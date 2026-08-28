#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod modele;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod os;
#[cfg(target_os = "windows")]
#[path = "win.rs"]
mod os;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod os {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    pub fn preparer_session() {}

    pub fn armer_raccourci(_toggle_requested: Arc<AtomicBool>, _ctx: eframe::egui::Context) {}

    pub fn position_souris() -> Option<(i32, i32)> {
        use mouse_position::mouse_position::Mouse;
        match Mouse::get_mouse_position() {
            Mouse::Position { x, y } => Some((x, y)),
            Mouse::Error => None,
        }
    }

    pub fn fenetre_active() -> Option<String> {
        None
    }

    pub fn forcer_focus_fenetre_app() {}

    pub fn coller_texte(_texte: &str, _fenetre_precedente: Option<String>) {}
}

use app::{EtatModele, RujiApp};
use eframe::egui;
use modele::IndexSemantique;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

fn preparer_dossier_modeles() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    let dossier = base.join("ruji").join("models");
    let _ = std::fs::create_dir_all(&dossier);

    let fichiers: [(&str, &[u8]); 3] = [
        ("model.onnx", include_bytes!("../models/model.onnx")),
        ("tokenizer.json", include_bytes!("../models/tokenizer.json")),
        ("metadata.json", include_bytes!("../models/metadata.json")),
    ];
    for (nom, contenu) in fichiers {
        let chemin = dossier.join(nom);
        let a_jour = std::fs::metadata(&chemin)
            .map(|m| m.len() as usize == contenu.len())
            .unwrap_or(false);
        if !a_jour {
            if let Err(e) = std::fs::write(&chemin, contenu) {
                eprintln!("Avertissement: échec d'écriture de {nom}: {e}");
            }
        }
    }
    dossier
}

fn main() -> eframe::Result<()> {
    os::preparer_session();

    let icone = eframe::icon_data::from_png_bytes(include_bytes!("../assets/mascotte.png"))
        .expect("assets/mascotte.png invalide");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 190.0])
            .with_decorations(false)
            .with_transparent(!cfg!(target_os = "windows"))
            .with_always_on_top()
            .with_visible(false)
            .with_icon(icone)
            .with_override_redirect(true),
        ..Default::default()
    };

    println!("🦀 Ruji démarré en arrière-plan !");
    println!("Appuyez sur Ctrl+Shift+E pour afficher/masquer la capsule.");

    eframe::run_native(
        "Ruji",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            cc.egui_ctx.add_font(egui::epaint::text::FontInsert::new(
                "nerd_font",
                egui::FontData::from_static(include_bytes!("../assets/nerdfont.ttf")),
                vec![
                    egui::epaint::text::InsertFontFamily {
                        family: egui::FontFamily::Proportional,
                        priority: egui::epaint::text::FontPriority::Lowest,
                    },
                    egui::epaint::text::InsertFontFamily {
                        family: egui::FontFamily::Monospace,
                        priority: egui::epaint::text::FontPriority::Lowest,
                    },
                ],
            ));

            let toggle_requested = Arc::new(AtomicBool::new(false));
            os::armer_raccourci(toggle_requested.clone(), cc.egui_ctx.clone());

            let modele = Arc::new(Mutex::new(EtatModele::Chargement));
            let modele_pour_thread = modele.clone();
            std::thread::spawn(move || {
                let dossier_modeles = preparer_dossier_modeles();
                let resultat = IndexSemantique::charger(&dossier_modeles);
                let nouvel_etat = match resultat {
                    Ok(index) => {
                        println!("Modèle de recherche sémantique chargé.");
                        EtatModele::Pret(index)
                    }
                    Err(e) => {
                        eprintln!("Avertissement: échec du chargement du modèle: {e}");
                        EtatModele::Erreur(e)
                    }
                };
                if let Ok(mut guard) = modele_pour_thread.lock() {
                    *guard = nouvel_etat;
                }
            });

            Ok(Box::new(RujiApp::new(toggle_requested, modele)))
        }),
    )
}
