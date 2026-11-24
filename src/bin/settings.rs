use dopamine_linux::{data_dir, get_secret, set_secret};
use gtk4::glib::{self, ControlFlow};
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, Entry, Label, Orientation,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

const DEFAULT_SERVICE: &str = "dopamine-daemon.service";

fn service_active(service: &str) -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", service])
        .status()
        .is_ok_and(|status| status.success())
}

fn service_action(service: &str, action: &str) -> Result<(), String> {
    let mut command = Command::new("systemctl");
    command.args(["--user", action]);
    if action == "enable" || action == "disable" {
        command.arg("--now");
    }
    let output = command
        .arg(service)
        .output()
        .map_err(|error| format!("Nie udało się uruchomić systemctl: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn row(root: &GtkBox, title: &str) -> Label {
    let line = GtkBox::new(Orientation::Horizontal, 8);
    let heading = Label::new(Some(title));
    heading.set_width_chars(15);
    heading.set_halign(gtk4::Align::Start);
    let value = Label::new(Some("Sprawdzanie…"));
    value.set_halign(gtk4::Align::Start);
    value.set_hexpand(true);
    value.set_wrap(true);
    line.append(&heading);
    line.append(&value);
    root.append(&line);
    value
}

fn device_status() -> String {
    match Command::new("idevice_id").arg("-l").output() {
        Ok(output) if output.status.success() => {
            let count = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
            match count {
                0 => "Nie wykryto iPhone’a".into(),
                1 => "iPhone wykryty przez USB".into(),
                n => format!("Wykryto {n} urządzenia przez USB"),
            }
        }
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if detail.is_empty() {
                "Brak połączenia z usbmuxd".into()
            } else {
                format!("usbmuxd niedostępny: {detail}")
            }
        }
        Err(error) => format!("Nie można uruchomić idevice_id: {error}"),
    }
}

fn sideload_status(service: &str) -> String {
    if Command::new("pgrep")
        .args(["-f", "dopamine-sideload"])
        .status()
        .is_ok_and(|status| status.success())
    {
        return "Sideloader działa — trwa próba instalacji".into();
    }

    let output = match Command::new("journalctl")
        .args(["--user", "-u", service, "-n", "160", "--no-pager", "-o", "cat"])
        .output()
    {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).into_owned(),
        _ => return "Brak logów sideloadingu".into(),
    };
    let lines: Vec<&str> = output.lines().collect();
    let start = lines
        .iter()
        .rposition(|line| line.contains("URUCHAMIAM DOPAMINE SIDELOAD"))
        .unwrap_or(0);
    let last_run = lines[start..].join("\n");

    if last_run.contains("Brak poświadczenia 'apple-id'") {
        "Nie zapisano Apple ID — zapisz dane i wybierz «Ponów sideload»".into()
    } else if last_run.contains("Brak poświadczenia 'apple-password'") {
        "Nie zapisano hasła — zapisz dane i wybierz «Ponów sideload»".into()
    } else if last_run.contains("Logowanie nie powiodło się") {
        "Logowanie Apple nie powiodło się — szczegóły są w dzienniku".into()
    } else if last_run.contains("Dopamine zostało zainstalowane.") || last_run.contains("SUKCES!") {
        "Dopamine zainstalowane pomyślnie".into()
    } else if last_run.contains("BŁĄD INSTALACJI") {
        "Instalacja nie powiodła się — szczegóły są w dzienniku".into()
    } else if last_run.contains("URUCHAMIAM DOPAMINE SIDELOAD") {
        "Sideloader uruchomiony; oczekuje na wynik".into()
    } else {
        "Nie rozpoczęto jeszcze sideloadingu".into()
    }
}

fn ipa_status() -> String {
    let dir = match data_dir() {
        Ok(dir) => dir,
        Err(error) => return error,
    };
    let ipa = dir.join("Dopamine.ipa");
    if !ipa.is_file() {
        return "Brak pliku Dopamine.ipa".into();
    }
    let version = std::fs::read_to_string(dir.join("Dopamine.version"))
        .unwrap_or_else(|_| "wersja nieznana".into())
        .trim()
        .to_string();
    let size_mb = std::fs::metadata(ipa)
        .map(|metadata| metadata.len() as f64 / 1_048_576.0)
        .unwrap_or_default();
    format!("Dopamine {version} — plik IPA, {size_mb:.1} MB")
}

fn refresh_status(
    service: &str,
    daemon: &Label,
    phone: &Label,
    sideload: &Label,
    ipa: &Label,
) {
    daemon.set_text(if service_active(service) {
        "Działa"
    } else {
        "Zatrzymany"
    });
    phone.set_text(&device_status());
    sideload.set_text(&sideload_status(service));
    ipa.set_text(&ipa_status());
}

fn updater_path() -> PathBuf {
    if let Some(path) = std::env::var_os("DOPAMINE_UPDATER") {
        return PathBuf::from(path);
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join("dopamine-update-ipa")))
        .unwrap_or_else(|| PathBuf::from("dopamine-update-ipa"))
}

fn check_version_async(sender: mpsc::Sender<String>, updater: PathBuf) {
    std::thread::spawn(move || {
        let result = Command::new(updater).arg("--check").output();
        let message = match result {
            Ok(output) if output.status.success() => {
                let response = String::from_utf8_lossy(&output.stdout);
                let fields: Vec<&str> = response.trim().split('|').collect();
                match fields.as_slice() {
                    ["AKTUALNA", version] => format!("IPA aktualna — Dopamine {version}"),
                    ["DOSTEPNA", local, remote] => format!("Dostępna aktualizacja: {local} → {remote}"),
                    ["BRAK", remote] => format!("Brak IPA — dostępna wersja: {remote}"),
                    _ => "Nie udało się odczytać wyniku sprawdzania wersji".into(),
                }
            }
            Ok(output) => {
                let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if detail.is_empty() { "Nie udało się sprawdzić aktualizacji".into() } else { format!("Błąd sprawdzania aktualizacji: {detail}") }
            }
            Err(error) => format!("Nie można uruchomić updatera: {error}"),
        };
        let _ = sender.send(message);
    });
}

fn main() {
    let app = Application::builder()
        .application_id("pl.krissu.DopamineSettings")
        .build();

    app.connect_activate(|app| {
        let service = std::env::var("DOPAMINE_SERVICE")
            .unwrap_or_else(|_| DEFAULT_SERVICE.to_string());
        let updater = updater_path();
        let root = GtkBox::new(Orientation::Vertical, 12);
        root.set_margin_top(22);
        root.set_margin_bottom(22);
        root.set_margin_start(22);
        root.set_margin_end(22);

        let title = Label::new(Some("Dopamine Sideload"));
        title.add_css_class("title-1");
        title.set_halign(gtk4::Align::Start);
        root.append(&title);

        let monitor = CheckButton::with_label("Monitoruj iPhone’a w tle");
        monitor.set_active(service_active(&service));
        let message = Label::new(Some("Gotowy"));
        message.set_halign(gtk4::Align::Start);
        message.set_wrap(true);
        let service_for_toggle = service.clone();
        let message_for_toggle = message.clone();
        monitor.connect_toggled(move |button| {
            match service_action(&service_for_toggle, if button.is_active() { "enable" } else { "disable" }) {
                Ok(()) => message_for_toggle.set_text(if button.is_active() { "Monitorowanie włączone." } else { "Monitorowanie wyłączone." }),
                Err(error) => message_for_toggle.set_text(&format!("Nie udało się zmienić usługi: {error}")),
            }
        });
        root.append(&monitor);

        let status_box = GtkBox::new(Orientation::Vertical, 7);
        status_box.add_css_class("card");
        let daemon = row(&status_box, "Demon");
        let phone = row(&status_box, "iPhone");
        let sideload = row(&status_box, "Sideload");
        let ipa = row(&status_box, "Lokalne IPA");
        let version = row(&status_box, "Aktualizacja");
        root.append(&status_box);

        let retry = Button::with_label("Ponów sideload teraz");
        let retry_service = service.clone();
        let retry_message = message.clone();
        retry.connect_clicked(move |_| {
            match service_action(&retry_service, "restart") {
                Ok(()) => retry_message.set_text("Demon uruchomiony ponownie; ponawiam wykrywanie telefonu."),
                Err(error) => retry_message.set_text(&format!("Nie udało się ponowić sideloadingu: {error}")),
            }
        });
        root.append(&retry);

        let version_button = Button::with_label("Sprawdź aktualizację Dopamine");
        let (version_tx, version_rx) = mpsc::channel::<String>();
        let tx_click = version_tx.clone();
        let updater_click = updater.clone();
        let version_click = version.clone();
        version_button.connect_clicked(move |_| {
            version_click.set_text("Sprawdzam release...");
            check_version_async(tx_click.clone(), updater_click.clone());
        });
        root.append(&version_button);

        let apple_id = Entry::builder()
            .placeholder_text("Apple ID")
            .text(get_secret("apple-id").unwrap_or_default())
            .build();
        let password = Entry::builder()
            .placeholder_text("Hasło Apple ID")
            .visibility(false)
            .build();
        if let Ok(saved) = get_secret("apple-password") {
            password.set_text(&saved);
        }
        let save = Button::with_label("Zapisz dane logowania");
        let save_id = apple_id.clone();
        let save_password = password.clone();
        let save_message = message.clone();
        let save_service = service.clone();
        save.connect_clicked(move |_| {
            let id = save_id.text().trim().to_string();
            let pass = save_password.text().to_string();
            if id.is_empty() || pass.is_empty() {
                save_message.set_text("Wpisz Apple ID i hasło.");
                return;
            }
            match set_secret("apple-id", &id).and_then(|_| set_secret("apple-password", &pass)) {
                Ok(()) => {
                    if service_active(&save_service) {
                        match service_action(&save_service, "restart") {
                            Ok(()) => save_message.set_text("Dane zapisane. Demon uruchomiony ponownie i ponawia wykrywanie telefonu."),
                            Err(error) => save_message.set_text(&format!("Dane zapisane, ale nie udało się zrestartować demona: {error}")),
                        }
                    } else {
                        save_message.set_text("Dane zapisane w menedżerze haseł systemu.");
                    }
                }
                Err(error) => save_message.set_text(&format!("Nie udało się zapisać danych: {error}")),
            }
        });
        root.append(&Label::new(Some("Dane Apple ID")));
        root.append(&apple_id);
        root.append(&password);
        root.append(&save);
        root.append(&message);

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Ustawienia Dopamine Sideload")
            .default_width(520)
            .default_height(560)
            .child(&root)
            .build();
        window.present();

        let service_refresh = service.clone();
        let daemon_refresh = daemon.clone();
        let phone_refresh = phone.clone();
        let sideload_refresh = sideload.clone();
        let ipa_refresh = ipa.clone();
        glib::timeout_add_local(Duration::from_secs(4), move || {
            refresh_status(
                &service_refresh,
                &daemon_refresh,
                &phone_refresh,
                &sideload_refresh,
                &ipa_refresh,
            );
            ControlFlow::Continue
        });
        refresh_status(&service, &daemon, &phone, &sideload, &ipa);

        let version_rx = Rc::new(RefCell::new(version_rx));
        let version_poll = version.clone();
        glib::timeout_add_local(Duration::from_millis(250), move || {
            if let Ok(result) = version_rx.borrow().try_recv() {
                version_poll.set_text(&result);
            }
            ControlFlow::Continue
        });
        version.set_text("Sprawdzanie release...");
        check_version_async(version_tx, updater);
    });

    app.run();
}
