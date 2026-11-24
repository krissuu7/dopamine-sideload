use futures_util::StreamExt;
use idevice::usbmuxd::{UsbmuxdConnection, UsbmuxdDevice, UsbmuxdListenEvent};
use tokio::process::{Child, Command};
use tokio::time::{sleep, Duration};

const SIDELOADER: &str = "dopamine-sideload";

fn start_sideload(
    device: &UsbmuxdDevice,
    dopamine_process: &mut Option<Child>,
    active_udid: &mut Option<String>,
    active_device_id: &mut Option<u32>,
) {
    if !matches!(device.connection_type, idevice::usbmuxd::Connection::Usb) {
        return;
    }
    println!("✓ Połączenie USB.");
    if dopamine_process.is_some() {
        println!(
            "ℹ Sideloader już działa dla urządzenia {}.",
            active_udid.as_deref().unwrap_or("nieznanego")
        );
        return;
    }

    println!();
    println!("=================================");
    println!("   URUCHAMIAM DOPAMINE SIDELOAD");
    println!("=================================");
    println!();

    let sideloader = std::env::var_os("DOPAMINE_SIDELOADER")
        .unwrap_or_else(|| SIDELOADER.into());
    match Command::new(sideloader)
        .arg("--device-udid")
        .arg(&device.udid)
        .spawn()
    {
        Ok(child) => {
            println!("✓ dopamine-sideload uruchomiony.");
            *dopamine_process = Some(child);
            *active_udid = Some(device.udid.clone());
            *active_device_id = Some(device.device_id);
        }
        Err(error) => println!("✗ Nie udało się uruchomić dopamine-sideload: {error}"),
    }
}

#[tokio::main]
async fn main() {
    println!("=================================");
    println!("       DOPAMINE DAEMON");
    println!("=================================");
    println!();
    println!("Daemon działa.");
    println!("Czekam na iPhone'a przez USB...");

    loop {
        println!();
        println!("Próba połączenia z usbmuxd...");

        let mut usbmuxd = match UsbmuxdConnection::default().await {
            Ok(connection) => {
                println!("✓ Połączono z usbmuxd.");
                connection
            }

            Err(error) => {
                println!("✗ usbmuxd niedostępny: {error}");
                println!("Ponawiam za 2 sekundy...");
                sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        let connected_devices = match usbmuxd.get_devices().await {
            Ok(devices) => devices,
            Err(error) => {
                println!("Nie udało się pobrać początkowej listy urządzeń: {error}");
                Vec::new()
            }
        };

        let mut events = match usbmuxd.listen().await {
            Ok(events) => {
                println!("✓ Nasłuchiwanie usbmuxd aktywne.");
                events
            }

            Err(error) => {
                println!("✗ Nie udało się uruchomić listen: {error}");
                println!("Ponawiam za 2 sekundy...");
                sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        let mut dopamine_process: Option<Child> = None;
        let mut active_udid: Option<String> = None;
        let mut active_device_id: Option<u32> = None;

        for device in &connected_devices {
            println!("Urządzenie podłączone przed startem nasłuchu:");
            println!("UDID: {}", device.udid);
            start_sideload(
                device,
                &mut dopamine_process,
                &mut active_udid,
                &mut active_device_id,
            );
        }

        loop {
            tokio::select! {
                event = events.next() => {
                    match event {
                        Some(Ok(UsbmuxdListenEvent::Connected(device))) => {
                            println!();
                            println!("Wykryto urządzenie:");
                            println!("UDID: {}", device.udid);
                            println!("Połączenie: {:?}", device.connection_type);

                            start_sideload(
                                &device,
                                &mut dopamine_process,
                                &mut active_udid,
                                &mut active_device_id,
                            );
                        }

                        Some(Ok(UsbmuxdListenEvent::Disconnected(device_id))) => {
                            println!();
                            println!("Urządzenie odłączone: {device_id}");

                            if active_device_id == Some(device_id) {
                                println!("✗ Aktywne urządzenie zostało odłączone.");

                                active_udid = None;
                                active_device_id = None;

                                if let Some(mut child) = dopamine_process.take() {
                                    println!("✗ Zatrzymuję dopamine-sideload...");

                                    if let Err(error) = child.kill().await {
                                        println!(
                                            "✗ Nie udało się zatrzymać dopamine-sideload: {error}"
                                        );
                                    } else {
                                        let _ = child.wait().await;
                                        println!("✓ dopamine-sideload został zatrzymany.");
                                    }
                                }

                                println!("Wracam do nasłuchiwania...");
                            } else {
                                println!("Odłączono urządzenie, dla którego sideloader nie był uruchomiony.");
                            }
                        }

                        Some(Err(error)) => {
                            println!();
                            println!("✗ Błąd połączenia z usbmuxd: {error}");
                            println!("Wracam do pętli reconnect...");

                            if let Some(mut child) = dopamine_process.take() {
                                println!("✗ Zatrzymuję dopamine-sideload...");

                                if let Err(error) = child.kill().await {
                                    println!(
                                        "✗ Nie udało się zatrzymać dopamine-sideload: {error}"
                                    );
                                } else {
                                    let _ = child.wait().await;
                                    println!("✓ dopamine-sideload został zatrzymany.");
                                }
                            }
                            break;
                        }

                        None => {
                            println!();
                            println!("Połączenie z usbmuxd zostało zamknięte.");

                            if let Some(mut child) = dopamine_process.take() {
                                println!("✗ Zatrzymuję dopamine-sideload...");

                                if let Err(error) = child.kill().await {
                                    println!(
                                        "✗ Nie udało się zatrzymać dopamine-sideload: {error}"
                                    );
                                } else {
                                    let _ = child.wait().await;
                                    println!("✓ dopamine-sideload został zatrzymany.");
                                }
                            }
                            break;
                        }
                    }
                }

                result = async {
                    match dopamine_process.as_mut() {
                        Some(child) => Some(child.wait().await),
                        None => None,
                    }
                }, if dopamine_process.is_some() => {
                    if let Some(result) = result {
                        match result {
                            Ok(status) => {
                                println!();
                                println!("dopamine-sideload zakończył działanie.");
                                println!("Status: {status}");
                            }

                            Err(error) => {
                                println!();
                                println!(
                                    "✗ Błąd podczas oczekiwania na dopamine-sideload: {error}"
                                );
                            }
                        }

                        dopamine_process = None;

                        if active_udid.is_some() {
                            println!("Urządzenie nadal jest podłączone.");
                            println!("Czekam na kolejne zdarzenie...");
                        }
                    }
                }
            }
        }

        println!();
        println!("Ponawiam połączenie z usbmuxd za 2 sekundy...");
        sleep(Duration::from_secs(2)).await;
    }
}
