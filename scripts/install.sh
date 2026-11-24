#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
app_dir="${DOPAMINE_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/dopamine-sideload}"
bin_dir="$HOME/.local/bin"
unit_dir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
applications_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons_dir="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"

confirm() {
    local answer
    if [[ ! -r /dev/tty ]]; then
        echo "Instalator potrzebuje interaktywnego terminala, aby potwierdzić tę czynność." >&2
        return 1
    fi
    read -r -p "$1 [t/N] " answer </dev/tty
    [[ "$answer" =~ ^[TtYy]$ ]]
}

needs_packages=0
for command in cargo rustc pkg-config curl jq python3 systemctl usbmuxd; do
    command -v "$command" >/dev/null || needs_packages=1
done
pkg-config --exists gtk4 libadwaita-1 dbus-1 2>/dev/null || needs_packages=1

if (( needs_packages )); then
    if command -v dnf >/dev/null; then
        package_manager=dnf
        packages=(cargo rust pkgconf-pkg-config gtk4-devel libadwaita-devel dbus-devel curl jq python3 usbmuxd systemd)
    elif command -v apt-get >/dev/null; then
        package_manager=apt-get
        packages=(cargo rustc pkg-config libgtk-4-dev libadwaita-1-dev libdbus-1-dev curl jq python3 usbmuxd systemd)
    elif command -v pacman >/dev/null; then
        package_manager=pacman
        packages=(rust cargo pkgconf gtk4 libadwaita dbus curl jq python usbmuxd systemd)
    elif command -v zypper >/dev/null; then
        package_manager=zypper
        packages=(rust cargo pkg-config gtk4-devel libadwaita-devel dbus-1-devel curl jq python3 usbmuxd systemd)
    else
        echo "Nieobsługiwana dystrybucja pakietów. Zainstaluj Rust/Cargo, GTK4, libadwaita, D-Bus development files, pkg-config, curl, jq, Python 3, systemd i usbmuxd ręcznie." >&2
        exit 1
    fi

    printf 'Do zbudowania Dopamine potrzebne są pakiety: %s\n' "${packages[*]}"
    confirm "Zainstalować zależności przez $package_manager?" || { echo "Przerwano; zależności nie zostały zainstalowane." >&2; exit 1; }
    if (( EUID == 0 )); then
        privilege=()
    else
        command -v sudo >/dev/null || { echo "Do instalacji pakietów potrzebne jest sudo." >&2; exit 1; }
        privilege=(sudo)
    fi
    case "$package_manager" in
        dnf) "${privilege[@]}" dnf install -y "${packages[@]}" ;;
        apt-get) "${privilege[@]}" apt-get update; "${privilege[@]}" apt-get install -y "${packages[@]}" ;;
        pacman) "${privilege[@]}" pacman -S --needed --noconfirm "${packages[@]}" ;;
        zypper) "${privilege[@]}" zypper --non-interactive install "${packages[@]}" ;;
    esac
fi

for command in cargo rustc pkg-config curl jq python3 systemctl usbmuxd; do
    command -v "$command" >/dev/null || { echo "Po instalacji nadal brakuje komendy: $command" >&2; exit 1; }
done
pkg-config --exists gtk4 libadwaita-1 dbus-1 || { echo "Brakuje bibliotek deweloperskich GTK4/libadwaita lub D-Bus." >&2; exit 1; }

rust_version=$(rustc --version | awk '{print $2}')
if [[ "$rust_version" =~ ^([0-9]+)\.([0-9]+)\. ]] &&
    (( BASH_REMATCH[1] == 1 && BASH_REMATCH[2] < 88 )); then
    if ! command -v rustup >/dev/null; then
        confirm "Rust $rust_version jest za stary. Zainstalować stabilny toolchain przez rustup?" || { echo "Wymagany jest Rust 1.88 lub nowszy." >&2; exit 1; }
        curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
        # shellcheck disable=SC1091
        source "$HOME/.cargo/env"
    else
        rustup toolchain install stable --profile minimal
    fi
    cargo_build=(cargo +stable build --release --manifest-path "$root/Cargo.toml")
else
    cargo_build=(cargo build --release --manifest-path "$root/Cargo.toml")
fi

echo "Buduję trzy programy Rust..."
CARGO_TARGET_DIR="$root/target" "${cargo_build[@]}"
install -d -m 755 "$app_dir/bin" "$bin_dir" "$unit_dir" "$applications_dir" "$icons_dir"
install -m 755 "$root/target/release/dopamine-daemon" "$app_dir/bin/dopamine-daemon"
install -m 755 "$root/target/release/dopamine-sideload" "$app_dir/bin/dopamine-sideload"
install -m 755 "$root/target/release/dopamine-settings" "$app_dir/bin/dopamine-settings"
install -m 755 "$root/scripts/update-ipa.sh" "$app_dir/bin/dopamine-update-ipa"
ln -sfn "$app_dir/bin/dopamine-settings" "$bin_dir/dopamine-settings"
ln -sfn "$app_dir/bin/dopamine-update-ipa" "$bin_dir/dopamine-update-ipa"
install -m 644 "$root/packaging/icons/dopamine-settings.svg" "$icons_dir/dopamine-settings.svg"

cat > "$unit_dir/dopamine-daemon.service" <<EOF
[Unit]
Description=Dopamine iPhone USB monitor
After=graphical-session.target

[Service]
Type=simple
Environment=DOPAMINE_HOME=$app_dir
Environment=DOPAMINE_SIDELOADER=$app_dir/bin/dopamine-sideload
ExecStart=$app_dir/bin/dopamine-daemon
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
EOF

cat > "$unit_dir/dopamine-updater.service" <<EOF
[Unit]
Description=Update Dopamine IPA from upstream release

[Service]
Type=oneshot
Environment=DOPAMINE_HOME=$app_dir
ExecStart=$app_dir/bin/dopamine-update-ipa
EOF
cat > "$unit_dir/dopamine-updater.timer" <<'EOF'
[Unit]
Description=Check for Dopamine IPA updates daily

[Timer]
OnCalendar=*-*-* 04:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

cat > "$applications_dir/dopamine-settings.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Dopamine Sideload Settings
Comment=Credentials and iPhone monitoring settings
Exec=env DOPAMINE_HOME=$app_dir $app_dir/bin/dopamine-settings
Icon=dopamine-settings
Terminal=false
Categories=Utility;System;
StartupNotify=true
EOF

systemctl --user daemon-reload
if ! DOPAMINE_HOME="$app_dir" "$app_dir/bin/dopamine-update-ipa"; then
    echo "Uwaga: nie udało się pobrać IPA podczas instalacji. Timer spróbuje ponownie." >&2
fi
systemctl --user enable --now dopamine-updater.timer
if [[ "${DOPAMINE_START_DAEMON:-1}" == "1" ]]; then
    systemctl --user enable --now dopamine-daemon.service
else
    echo "Demon pozostawiono bez uruchamiania (DOPAMINE_START_DAEMON=0)."
fi
echo "Instalacja zakończona. Otwórz Dopamine Sideload Settings z menu aplikacji."
echo "Aplikacja i Dopamine.ipa znajdują się w: $app_dir"
