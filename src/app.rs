
use crate::modele::IndexSemantique;
use crate::os;
use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const NOMBRE_SUGGESTIONS: usize = 6;

pub enum EtatModele {
    Chargement,
    Pret(IndexSemantique),
    Erreur(String),
}

pub struct RujiApp {
    recherche: String,
    toggle_requested: Arc<AtomicBool>,
    visible: bool,
    fenetre_precedente: Option<String>,
    selection_index: usize,
    modele: Arc<Mutex<EtatModele>>,
    derniere_recherche: String,
    resultats: Vec<String>,
    resultats_en_attente_du_modele: bool,
    nerdfont_actif: bool,
    theme_sombre: bool,
    derniere_nerdfont_actif: bool,
}

impl RujiApp {
    pub fn new(toggle_requested: Arc<AtomicBool>, modele: Arc<Mutex<EtatModele>>) -> Self {
        Self {
            recherche: String::new(),
            toggle_requested,
            visible: false,
            fenetre_precedente: None,
            selection_index: 0,
            modele,
            derniere_recherche: String::new(),
            resultats: Vec::new(),
            resultats_en_attente_du_modele: false,
            nerdfont_actif: true,
            theme_sombre: false,
            derniere_nerdfont_actif: true,
        }
    }

    fn choisir(&mut self, ctx: &egui::Context, emoji: &str) {
        self.visible = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        os::coller_texte(emoji, self.fenetre_precedente.take());
    }
}

impl eframe::App for RujiApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        if cfg!(target_os = "windows") {
            if self.theme_sombre {
                [26.0 / 255.0, 23.0 / 255.0, 21.0 / 255.0, 1.0]
            } else {
                [250.0 / 255.0, 248.0 / 255.0, 244.0 / 255.0, 1.0]
            }
        } else {
            [0.0, 0.0, 0.0, 0.0]
        }
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.toggle_requested.swap(false, Ordering::SeqCst) {
            self.visible = !self.visible;

            if self.visible {
                self.fenetre_precedente = os::fenetre_active();
                self.selection_index = 0;

                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                os::forcer_focus_fenetre_app();
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        if self.visible && self.recherche.is_empty() {
            if let Some((x, y)) = os::position_souris() {
                let cible = egui::pos2(x as f32 + 12.0, y as f32 + 12.0);
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(cible));
            }
            ctx.request_repaint_after(Duration::from_millis(33));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.visible {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false));
            return;
        }

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.visible = false;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false));
            return;
        }

        ui.style_mut().visuals = if self.theme_sombre { egui::Visuals::dark() } else { egui::Visuals::light() };

        let (fond, accent, texte, texte_attenue, bordure) = if self.theme_sombre {
            (
                Color32::from_rgba_unmultiplied_const(26, 23, 21, 248),
                Color32::from_rgb(255, 140, 80),
                Color32::from_rgb(240, 235, 228),
                Color32::from_rgb(150, 143, 135),
                Color32::from_rgba_unmultiplied_const(255, 255, 255, 30),
            )
        } else {
            (
                Color32::from_rgba_unmultiplied_const(250, 248, 244, 252),
                Color32::from_rgb(232, 90, 20),
                Color32::from_rgb(30, 26, 23),
                Color32::from_rgb(140, 132, 124),
                Color32::from_rgba_unmultiplied_const(255, 255, 255, 200),
            )
        };

        let cadre = egui::Frame::NONE
            .fill(fond)
            .corner_radius(CornerRadius::same(24))
            .inner_margin(egui::Margin::symmetric(20, 18))
            .stroke(Stroke::new(1.0, bordure))
            .shadow(egui::Shadow {
                offset: [0, 8],
                blur: 28,
                spread: 0,
                color: Color32::from_black_alpha(90),
            });

        cadre.show(ui, |ui| {
            if cfg!(target_os = "windows") {
                ui.set_min_size(ui.available_size());
            }

            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("../assets/mascotte.png"))
                        .fit_to_exact_size(Vec2::splat(64.0))
                        .corner_radius(14.0),
                );
                ui.add_space(12.0);

                let largeur_menu = 32.0;
                let edit = egui::TextEdit::singleline(&mut self.recherche)
                    .hint_text(egui::RichText::new("Rechercher...").color(texte_attenue))
                    .font(egui::FontId::proportional(22.0))
                    .text_color(texte)
                    .frame(egui::Frame::NONE);

                let response = ui.add_sized(
                    Vec2::new((ui.available_width() - largeur_menu - 8.0).max(0.0), 64.0),
                    edit,
                );
                response.request_focus();

                ui.add_space(8.0);
                ui.menu_button(egui::RichText::new("⋮").size(20.0).color(texte_attenue), |ui| {
                    ui.checkbox(&mut self.nerdfont_actif, "Icônes Nerd Font");
                    ui.separator();
                    ui.selectable_value(&mut self.theme_sombre, false, "☀ Clair");
                    ui.selectable_value(&mut self.theme_sombre, true, "🌙 Sombre");
                });
            });

            ui.add_space(16.0);

            if self.recherche.is_empty() {
                ui.label(
                    egui::RichText::new("tapez pour chercher · échap pour fermer")
                        .color(texte_attenue)
                        .size(12.0),
                );
            } else {
                if self.recherche != self.derniere_recherche
                    || self.resultats_en_attente_du_modele
                    || self.nerdfont_actif != self.derniere_nerdfont_actif
                {
                    self.derniere_recherche = self.recherche.clone();
                    self.derniere_nerdfont_actif = self.nerdfont_actif;
                    self.selection_index = 0;
                    self.resultats = match self.modele.lock() {
                        Ok(mut guard) => match &mut *guard {
                            EtatModele::Pret(index) => {
                                self.resultats_en_attente_du_modele = false;
                                index.rechercher(&self.recherche, NOMBRE_SUGGESTIONS, self.nerdfont_actif)
                            }
                            EtatModele::Chargement | EtatModele::Erreur(_) => {
                                self.resultats_en_attente_du_modele = true;
                                Vec::new()
                            }
                        },
                        Err(_) => Vec::new(),
                    };
                }

                if self.resultats.is_empty() {
                    let message = match self.modele.lock() {
                        Ok(guard) => match &*guard {
                            EtatModele::Chargement => "Chargement du modèle en cours...".to_string(),
                            EtatModele::Erreur(e) => format!("Modèle indisponible : {e}"),
                            EtatModele::Pret(_) => "Aucun résultat.".to_string(),
                        },
                        Err(_) => "Aucun résultat.".to_string(),
                    };
                    ui.label(egui::RichText::new(message).color(texte_attenue).size(13.0));
                } else {
                    self.selection_index = self.selection_index.min(self.resultats.len() - 1);

                    if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                        self.selection_index = (self.selection_index + 1).min(self.resultats.len() - 1);
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                        self.selection_index = self.selection_index.saturating_sub(1);
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let emoji = self.resultats[self.selection_index].clone();
                        self.choisir(ui.ctx(), &emoji);
                        return;
                    }

                    let mut a_choisir: Option<String> = None;
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::splat(8.0);
                        for (i, emoji) in self.resultats.iter().enumerate() {
                            let btn_text = egui::RichText::new(emoji.as_str()).size(24.0);

                            let selectionne = i == self.selection_index;
                            let mut bouton = egui::Button::new(btn_text)
                                .corner_radius(CornerRadius::same(14))
                                .min_size(Vec2::splat(44.0))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE);
                            if selectionne {
                                bouton = bouton
                                    .fill(accent.gamma_multiply(0.22))
                                    .stroke(Stroke::new(1.0, accent));
                            }

                            if ui.add(bouton).clicked() {
                                a_choisir = Some(emoji.clone());
                            }
                        }
                    });
                    if let Some(emoji) = a_choisir {
                        self.choisir(ui.ctx(), &emoji);
                        return;
                    }
                }
            }
        });
    }
}
