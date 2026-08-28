
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub fn armer_raccourci(toggle_requested: Arc<AtomicBool>, ctx: eframe::egui::Context) {
    use signal_hook::consts::signal::SIGUSR1;
    use signal_hook::iterator::Signals;

    let mut signals = Signals::new([SIGUSR1]).expect("Impossible d'installer le handler SIGUSR1");
    std::thread::spawn(move || {
        for _ in signals.forever() {
            toggle_requested.store(true, Ordering::SeqCst);
            ctx.request_repaint();
        }
    });
}

const GNOME_KEYBIND_PATH: &str =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/ruji-toggle/";
const GNOME_SHORTCUT_NAME: &str = "Ruji toggle";
const GNOME_SHORTCUT_COMMAND: &str = "pkill -USR1 -x ruji";
const GNOME_SHORTCUT_BINDING: &str = "<Primary><Shift>e";

pub fn preparer_session() {
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    if session == "wayland" {
        eprintln!("Session Linux/Wayland détectée : bascule sur XWayland pour permettre le positionnement de la fenêtre.");
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
        }
    } else {
        eprintln!("Session Linux/X11 détectée : positionnement natif, aucun contournement nécessaire.");
    }

    installer_raccourci_gnome();
}

fn installer_raccourci_gnome() {
    let Ok(output) = Command::new("gsettings")
        .args([
            "get",
            "org.gnome.settings-daemon.plugins.media-keys",
            "custom-keybindings",
        ])
        .output()
    else {
        return;
    };
    if !output.status.success() {
        return;
    }

    let current = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if current.contains(GNOME_KEYBIND_PATH) {
        return;
    }

    let mut paths: Vec<String> = current
        .trim_start_matches('@')
        .trim()
        .trim_start_matches("as")
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    paths.push(GNOME_KEYBIND_PATH.to_string());
    let new_list = format!(
        "[{}]",
        paths.iter().map(|p| format!("'{p}'")).collect::<Vec<_>>().join(", ")
    );

    let schema = format!(
        "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{GNOME_KEYBIND_PATH}"
    );

    let ok = Command::new("gsettings")
        .args([
            "set",
            "org.gnome.settings-daemon.plugins.media-keys",
            "custom-keybindings",
            &new_list,
        ])
        .status()
        .is_ok_and(|s| s.success())
        && Command::new("gsettings")
            .args(["set", &schema, "name", GNOME_SHORTCUT_NAME])
            .status()
            .is_ok_and(|s| s.success())
        && Command::new("gsettings")
            .args(["set", &schema, "command", GNOME_SHORTCUT_COMMAND])
            .status()
            .is_ok_and(|s| s.success())
        && Command::new("gsettings")
            .args(["set", &schema, "binding", GNOME_SHORTCUT_BINDING])
            .status()
            .is_ok_and(|s| s.success());

    if ok {
        println!("Raccourci Ctrl+Shift+E installé automatiquement dans GNOME.");
    } else {
        eprintln!("Avertissement: échec de l'installation automatique du raccourci GNOME.");
    }
}

pub fn position_souris() -> Option<(i32, i32)> {
    position_souris_extension_gnome().or_else(position_souris_x11)
}

fn position_souris_extension_gnome() -> Option<(i32, i32)> {
    use std::sync::Mutex;
    use zbus::blocking::Connection;

    static CONNEXION: Mutex<Option<Connection>> = Mutex::new(None);

    let mut etat = CONNEXION.lock().ok()?;
    if etat.is_none() {
        *etat = Connection::session().ok();
    }
    let connexion = etat.as_ref()?;

    let reponse = connexion
        .call_method(
            Some("org.ruji.Pointer"),
            "/org/ruji/Pointer",
            Some("org.ruji.Pointer"),
            "GetPosition",
            &(),
        )
        .ok()?;
    reponse.body().deserialize::<(i32, i32)>().ok()
}

fn position_souris_x11() -> Option<(i32, i32)> {
    use std::sync::Mutex;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::ConnectionExt;
    use x11rb::rust_connection::RustConnection;

    struct EtatX11 {
        conn: RustConnection,
        root: u32,
    }

    static ETAT: Mutex<Option<EtatX11>> = Mutex::new(None);

    let mut etat = ETAT.lock().ok()?;
    if etat.is_none() {
        let (conn, ecran) = RustConnection::connect(None).ok()?;
        let root = conn.setup().roots[ecran].root;
        *etat = Some(EtatX11 { conn, root });
    }

    let EtatX11 { conn, root } = etat.as_ref()?;
    let reponse = conn.query_pointer(*root).ok().and_then(|c| c.reply().ok());

    match reponse {
        Some(r) => Some((r.root_x as i32, r.root_y as i32)),
        None => {
            eprintln!("[debug] position_souris: connexion X11 perdue, reconnexion au prochain appel.");
            *etat = None;
            None
        }
    }
}

pub fn fenetre_active() -> Option<String> {
    Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

pub fn forcer_focus_fenetre_app() {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(80));
        let _ = Command::new("xdotool")
            .args(["search", "--name", "^Ruji$", "windowfocus", "--sync"])
            .status();
    });
}

pub fn coller_texte(texte: &str, fenetre_precedente: Option<String>) {
    use arboard::{LinuxClipboardKind, SetExtLinux};

    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        eprintln!("Avertissement: impossible d'accéder au presse-papier");
        return;
    };
    if let Err(e) = clipboard.set_text(texte) {
        eprintln!("Avertissement: échec de la copie dans le presse-papier: {e}");
        return;
    }
    if let Err(e) = clipboard.set().clipboard(LinuxClipboardKind::Primary).text(texte) {
        eprintln!("Avertissement: échec de la copie dans la sélection primaire: {e}");
    }

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(120));
        if let Some(id) = fenetre_precedente {
            let _ = Command::new("xdotool").args(["windowactivate", &id]).status();
            std::thread::sleep(Duration::from_millis(60));
        }
        if let Err(e) = coller_via_uinput() {
            eprintln!("Avertissement: échec de la simulation du collage (uinput): {e}");
        }
    });
}

fn coller_via_uinput() -> Result<(), String> {
    use evdev::uinput::VirtualDevice;
    use evdev::{AttributeSet, EventType, InputEvent, KeyCode};
    use std::sync::Mutex;

    static CLAVIER: Mutex<Option<VirtualDevice>> = Mutex::new(None);

    let mut guard = CLAVIER.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        let mut touches = AttributeSet::<KeyCode>::new();
        touches.insert(KeyCode::KEY_LEFTSHIFT);
        touches.insert(KeyCode::KEY_INSERT);
        let clavier = VirtualDevice::builder()
            .map_err(|e| format!("{e} (l'utilisateur est-il dans le groupe 'input' ?)"))?
            .name("Ruji clavier virtuel")
            .with_keys(&touches)
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?;
        *guard = Some(clavier);
        std::thread::sleep(Duration::from_millis(200));
    }

    let clavier = guard.as_mut().ok_or("clavier virtuel indisponible")?;
    let bas = |touche: KeyCode| InputEvent::new(EventType::KEY.0, touche.code(), 1);
    let haut = |touche: KeyCode| InputEvent::new(EventType::KEY.0, touche.code(), 0);
    clavier
        .emit(&[bas(KeyCode::KEY_LEFTSHIFT), bas(KeyCode::KEY_INSERT)])
        .map_err(|e| e.to_string())?;
    clavier
        .emit(&[haut(KeyCode::KEY_INSERT), haut(KeyCode::KEY_LEFTSHIFT)])
        .map_err(|e| e.to_string())
}
