use std::io::{self, Write};
use std::sync::mpsc;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, CheckButton, Entry, Label, Orientation};
use libadwaita as adw;

use idevice::usbmuxd::{UsbmuxdAddr, UsbmuxdConnection};

use isideload::{
    anisette::remote_v3::RemoteV3AnisetteProvider,
    auth::apple_account::{
        AppleAccount,
        TwoFactorCallbackParams,
        TwoFactorCallbackResponse,
    },
    dev::{
        certificates::DevelopmentCertificate,
        developer_session::DeveloperSession,
        teams::DeveloperTeam,
    },
    sideload::{
        SideloaderBuilder,
        TeamSelection,
        builder::MaxCertsBehavior,
    },
    util::keyring_storage::KeyringStorage,
};


use dopamine_linux::{get_secret, ipa_path};


fn get_cert_selection_gui(certificates: Vec<(String, String, String)>) -> Vec<String> {
    let (tx, rx) = mpsc::channel::<Vec<String>>();

    std::thread::spawn(move || {
        if let Err(error) = adw::init() {
            eprintln!("Nie udało się uruchomić libadwaita dla wyboru certyfikatów: {error}");
            return;
        }

        let app = Application::builder()
            .application_id("pl.krissu.DopamineCertSelection")
            .build();

        let tx_activate = tx.clone();
        app.connect_activate(move |app| {
            let window = ApplicationWindow::builder()
                .application(app)
                .title("Dopamine Sideload — certyfikaty")
                .default_width(520)
                .default_height(320)
                .build();

            let root = gtk4::Box::builder()
                .orientation(Orientation::Vertical)
                .spacing(12)
                .margin_top(20)
                .margin_bottom(20)
                .margin_start(20)
                .margin_end(20)
                .build();

            root.append(&Label::new(Some(
                "Osiągnięto limit certyfikatów Apple Developer.\nWybierz certyfikat(y), które chcesz unieważnić:"
            )));

            let mut checks: Vec<(CheckButton, String)> = Vec::new();
            for (name, machine, serial) in &certificates {
                let label = format!("{name} — {machine}");
                let check = CheckButton::with_label(&label);
                root.append(&check);
                checks.push((check, serial.clone()));
            }

            let submit = Button::with_label("Unieważnij zaznaczone i kontynuuj");
            let tx_submit = tx_activate.clone();
            let app_submit = app.clone();
            submit.connect_clicked(move |_| {
                let selected = checks
                    .iter()
                    .filter(|(check, _)| check.is_active())
                    .map(|(_, serial)| serial.clone())
                    .collect::<Vec<_>>();
                let _ = tx_submit.send(selected);
                app_submit.quit();
            });
            root.append(&submit);

            window.set_child(Some(&root));
            window.present();
        });

        app.run_with_args::<&str>(&[]);
    });

    match rx.recv() {
        Ok(selected) => selected,
        Err(error) => {
            eprintln!("Okno wyboru certyfikatów nie zwróciło wyboru: {error}");
            Vec::new()
        }
    }
}

fn get_2fa_code_gui() -> String {
    let (tx, rx) = mpsc::channel::<String>();

    std::thread::spawn(move || {
        if let Err(error) = adw::init() {
            eprintln!("Nie udało się uruchomić libadwaita: {error}");
            let _ = tx.send(String::new());
            return;
        }

        let app = Application::builder()
            .application_id("pl.krissu.DopamineSideload")
            .build();

        let tx_activate = tx.clone();

        app.connect_activate(move |app| {
            let window = ApplicationWindow::builder()
                .application(app)
                .title("Dopamine Sideload")
                .default_width(420)
                .default_height(220)
                .build();

            let box_main = gtk4::Box::builder()
                .orientation(Orientation::Vertical)
                .spacing(12)
                .margin_top(24)
                .margin_bottom(24)
                .margin_start(24)
                .margin_end(24)
                .build();

            let title = Label::new(Some("🔐 Weryfikacja 2FA"));
            title.add_css_class("title-2");

            let info = Label::new(Some(
                "Wprowadź kod 2FA wysłany na zaufane urządzenie.",
            ));

            let entry = Entry::builder()
                .placeholder_text("Kod 2FA")
                .max_length(8)
                .activates_default(true)
                .build();

            let button = Button::with_label("Zatwierdź");
            button.set_receives_default(true);

            let tx_button = tx_activate.clone();
            let window_button = window.clone();
            let entry_button = entry.clone();

            button.connect_clicked(move |_| {
                let code = entry_button.text().trim().to_string();

                if !code.is_empty() {
                    let _ = tx_button.send(code);
                    window_button.close();
                }
            });

            let button_enter = button.clone();
            entry.connect_activate(move |_| {
                button_enter.emit_clicked();
            });

            box_main.append(&title);
            box_main.append(&info);
            box_main.append(&entry);
            box_main.append(&button);

            window.set_child(Some(&box_main));
            window.set_default_widget(Some(&button));
            window.present();
            entry.grab_focus();
        });

        // The sideloader receives --device-udid from the daemon. Do not pass
        // those CLI arguments to GTK/GApplication, which would reject them.
        app.run_with_args::<&str>(&[]);
    });

    match rx.recv() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Okno 2FA zostało zamknięte: {error}");
            String::new()
        }
    }
}

async fn get_2fa_code_gui_async() -> String {
    match tokio::task::spawn_blocking(get_2fa_code_gui).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Nie udało się uruchomić GUI 2FA: {error}");
            String::new()
        }
    }
}

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Nie udało się uruchomić rustls");

    isideload::init()
        .expect("Nie udało się zainicjalizować isideload");

    println!("=================================");
    println!("       DOPAMINE SIDELOADER");
    println!("=================================");
    println!();

    let selected_udid = std::env::args().skip(1).collect::<Vec<_>>();
    let selected_udid = selected_udid
        .windows(2)
        .find(|pair| pair[0] == "--device-udid")
        .map(|pair| pair[1].as_str());
    let ipa_path = match ipa_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    if !ipa_path.is_file() {
        eprintln!("Nie znaleziono IPA:");
        eprintln!("{}", ipa_path.display());
        return;
    }

    println!("IPA: {}", ipa_path.display());
    println!();

    let apple_id = match get_secret("apple-id") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Nie udało się odczytać Apple ID z systemowego magazynu haseł: {error}");
            return;
        }
    };

    let apple_password = match get_secret("apple-password") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Nie udało się odczytać hasła z systemowego magazynu haseł: {error}");
            return;
        }
    };

    println!();
    println!("Logowanie do Apple...");

    let get_2fa_code = async |_params: TwoFactorCallbackParams| {
        let code = get_2fa_code_gui_async().await;

        Ok(TwoFactorCallbackResponse::SubmitCode(code))
    };

    let account = AppleAccount::builder(&apple_id)
        .anisette_provider(
            RemoteV3AnisetteProvider::default()
                .unwrap()
                .set_serial_number("2".to_string()),
        )
        .login(&apple_password, get_2fa_code)
        .await;

    let mut account = match account {
        Ok(account) => account,

        Err(error) => {
            eprintln!();
            eprintln!("Logowanie nie powiodło się:");
            eprintln!("{error}");
            return;
        }
    };

    println!();
    println!("Zalogowano pomyślnie.");

    println!();
    println!("Tworzenie sesji deweloperskiej...");

    let dev_session =
        match DeveloperSession::from_account(&mut account).await {
            Ok(session) => session,

            Err(error) => {
                eprintln!("Nie udało się utworzyć sesji:");
                eprintln!("{error}");
                return;
            }
        };

    println!("Sesja deweloperska OK.");

    println!();
    println!("Łączenie z usbmuxd...");

    let mut usbmuxd = match UsbmuxdConnection::default().await {
        Ok(connection) => connection,

        Err(error) => {
            eprintln!("Nie udało się połączyć z usbmuxd:");
            eprintln!("{error}");
            return;
        }
    };

    let devices = match usbmuxd.get_devices().await {
        Ok(devices) => devices,

        Err(error) => {
            eprintln!("Nie udało się pobrać urządzeń:");
            eprintln!("{error}");
            return;
        }
    };

    if devices.is_empty() {
        eprintln!("Nie znaleziono żadnego iPhone'a.");
        return;
    }

    let usb_devices: Vec<_> = devices
        .iter()
        .filter(|device| matches!(device.connection_type, idevice::usbmuxd::Connection::Usb))
        .collect();
    let device = match selected_udid {
        Some(udid) => usb_devices.iter().copied().find(|device| device.udid == udid),
        None if usb_devices.len() == 1 => usb_devices.first().copied(),
        None if usb_devices.len() > 1 => {
            eprintln!("Podłączono kilka urządzeń USB; uruchom program z --device-udid <UDID>.");
            return;
        }
        None => None,
    };
    let device = match device {
        Some(device) => device,
        None => {
            eprintln!("Nie znaleziono wskazanego iPhone'a po USB.");
            return;
        }
    };

    println!();
    println!("Znaleziono iPhone'a:");
    println!("UDID: {}", device.udid);
    println!("Połączenie: USB");

    let provider = device.to_provider(
        UsbmuxdAddr::from_env_var().unwrap(),
        "dopamine-sideload",
    );

    println!("Połączono z urządzeniem.");

    let team_selection_prompt =
        |teams: &Vec<DeveloperTeam>| {
            println!();
            println!("Dostępne zespoły:");

            for (index, team) in teams.iter().enumerate() {
                println!(
                    "{}: {} ({})",
                    index + 1,
                    team.name
                        .as_deref()
                        .unwrap_or("<Unnamed>"),
                    team.team_id
                );
            }

            let mut input = String::new();

            print!("Wybierz numer zespołu: ");
            io::stdout().flush().unwrap();

            io::stdin()
                .read_line(&mut input)
                .unwrap();

            let selection =
                input.trim().parse::<usize>().ok()?;

            if selection == 0 ||
                selection > teams.len()
            {
                return None;
            }

            Some(
                teams[selection - 1]
                    .team_id
                    .clone()
            )
        };

    let cert_selection_prompt = async |certs: Vec<DevelopmentCertificate>| {
        let options = certs
            .into_iter()
            .map(|cert| {
                (
                    cert.name.unwrap_or_else(|| "<Unnamed>".to_string()),
                    cert.machine_name.unwrap_or_else(|| "<No Machine Name>".to_string()),
                    cert.serial_number.unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();

        let selected = tokio::task::spawn_blocking(move || {
            get_cert_selection_gui(options)
        })
        .await
        .unwrap_or_default();

        if selected.is_empty() {
            Ok(None)
        } else {
            Ok(Some(selected))
        }
    };

    println!();
    println!("Budowanie sideloadera...");

    let mut sideloader =
        SideloaderBuilder::new(
            dev_session,
            apple_id.to_string(),
        )
        .team_selection(
            TeamSelection::PromptOnce(
                team_selection_prompt
            )
        )
        .max_certs_behavior(
            MaxCertsBehavior::Prompt(
                cert_selection_prompt
            )
        )
        .storage(Box::new(
            KeyringStorage::new(
                "dopamine-sideload".to_string()
            )
        ))
        .machine_name(
            "dopamine-sideload".to_string()
        )
        .build();

    println!("Sideloader gotowy.");

    println!();
    println!("=================================");
    println!(" Instalowanie Dopamine...");
    println!("=================================");
    println!();

    let result = sideloader
        .install_app(
        &provider,
        ipa_path,
        true,
        None::<fn(f32) -> std::future::Ready<()>>,
        None,
    )
        .await;

    match result {
        Ok(_) => {
            println!();
            println!("=================================");
            println!("            SUKCES!");
            println!(" Dopamine zostało zainstalowane.");
            println!("=================================");
        }

        Err(error) => {
            eprintln!();
            eprintln!("=================================");
            eprintln!("       BŁĄD INSTALACJI");
            eprintln!("=================================");
            eprintln!("{error}");
        }
    }
}
