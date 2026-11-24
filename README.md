# Dopamine Sideload for Linux

A Linux desktop helper for installing and updating **[Dopamine](https://github.com/opa334/Dopamine)** on a connected iPhone.

The project combines a USB device monitor, an Apple account sign-in flow, a GTK settings application, and an updater for the official Dopamine IPA. Sideloading is handled by the Rust `isideload` library.

> **Unofficial project.** This repository is not affiliated with or endorsed by the Dopamine project or Apple.

## Features

- **Automatic USB detection** — monitors `usbmuxd` and starts the sideloader when an iPhone is connected.
- **Desktop settings** — GTK 4 / libadwaita application available from the desktop application menu.
- **Apple account credentials** — stores the Apple ID and password using the system Secret Service keyring, rather than a plaintext configuration file.
- **Two-factor authentication** — displays a graphical prompt when Apple requests a trusted-device verification code.
- **Certificate management** — displays a graphical selection dialog when the Apple Developer certificate limit is reached.
- **Dopamine IPA updater** — downloads the latest IPA from the official `opa334/Dopamine` GitHub release, validates the archive, and checks the published SHA-256 digest when available.
- **Scheduled IPA checks** — a systemd user timer checks for a new Dopamine release daily.

## Requirements

- A Linux desktop session with **systemd user services**.
- A working **Secret Service** provider (for example, GNOME Keyring or KWallet configured to provide Secret Service).
- `usbmuxd` and the libimobiledevice utilities.
- GTK 4, libadwaita, D-Bus development files, and `pkg-config`.
- Rust and Cargo **1.88 or newer**. The installer can offer to install the stable toolchain with rustup if the system Rust is older.
- An active graphical user session for the settings application and interactive 2FA / certificate dialogs.

The installer supports package installation through **DNF, APT, Pacman, and Zypper**. Package-manager support does not mean every distribution has been tested; see the test status below.

## Installation

Open a terminal in your desktop session and run:

```bash
curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh \
  | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload bash
```

The installer:

1. Downloads the selected repository branch.
2. Checks for the required build and runtime dependencies.
3. Asks before installing missing system packages.
4. Builds the daemon, sideloader, and settings application locally.
5. Installs the binaries, desktop entry, icon, and systemd user units.
6. Downloads the latest Dopamine IPA and enables the daily updater timer.
7. Enables and starts the USB monitoring daemon.

The installer may request your password through `sudo` when system packages need to be installed.

### Select a branch

The default branch is `main`. To install another branch, set `DOPAMINE_REF`:

```bash
curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh \
  | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload DOPAMINE_REF=BRANCH_NAME bash
```

### Reinstall or update

Run the same installation command again to fetch the latest source from the selected branch, rebuild the applications, and replace the installed binaries and service files.

The installer keeps the application data directory, including saved credentials and the downloaded IPA. It does not currently provide an incremental binary update: each run builds the Rust applications again.

### Optional installation settings

- `DOPAMINE_START_DAEMON=0` — install without enabling or starting the USB monitoring daemon.
- `DOPAMINE_HOME=/path/to/directory` — use a custom application data directory.

Example:

```bash
DOPAMINE_START_DAEMON=0 \
  curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh \
  | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload bash
```

## First run

1. Open **Dopamine Sideload Settings** from your desktop application menu.
2. Enter your Apple ID and password, then choose **Save login details**.
3. Connect the iPhone over USB, unlock it, and accept **Trust This Computer** if prompted.
4. Keep the desktop session active. When Apple requests verification, enter the code in the 2FA dialog.
5. If the Apple Developer certificate limit is reached, review the listed certificates and select only certificates you are willing to revoke.

The daemon starts the sideloader automatically when it detects a supported USB connection. Keep the phone connected and unlocked during the installation process.

**Certificate warning:** revoking an Apple Development certificate can affect other apps or workflows that depend on it. Do not revoke a certificate unless you know it is safe to do so.

## How it is installed

The default application data directory is:

```text
~/.local/share/dopamine-sideload/
├── Dopamine.ipa
├── Dopamine.version
└── bin/
    ├── dopamine-daemon
    ├── dopamine-sideload
    ├── dopamine-settings
    └── dopamine-update-ipa
```

The installer also creates:

- `~/.local/bin/dopamine-settings` — command-line launcher for the settings application.
- `~/.local/share/applications/dopamine-settings.desktop` — desktop menu entry.
- `~/.config/systemd/user/dopamine-daemon.service` — USB monitoring service.
- `~/.config/systemd/user/dopamine-updater.service` and `dopamine-updater.timer` — scheduled IPA updater.

If `XDG_DATA_HOME` or `XDG_CONFIG_HOME` is set, the corresponding paths follow those locations.

## Useful commands

Check the daemon:

```bash
systemctl --user status dopamine-daemon.service
```

View daemon and sideloader logs:

```bash
journalctl --user -u dopamine-daemon.service -f
```

Restart USB monitoring:

```bash
systemctl --user restart dopamine-daemon.service
```

Check the USB device list:

```bash
idevice_id -l
```

Check the updater timer:

```bash
systemctl --user status dopamine-updater.timer
```

## Distribution test status

| Distribution family | Status |
| --- | --- |
| Fedora (DNF) | Tested |
| Xubuntu (APT / Ubuntu base) | Tested |
| Arch Linux (Pacman) | Package mapping implemented; testing pending |
| openSUSE (Zypper) | Package mapping implemented; testing pending |

Testing on one distribution does not guarantee compatibility with every release or desktop configuration in the same family. Reports from additional distributions are welcome.

## Troubleshooting

### The iPhone is not detected

- Unlock the iPhone and reconnect the USB cable.
- Accept the trust prompt on the iPhone.
- Check that `usbmuxd` is installed and running.
- Run `idevice_id -l` and confirm that the device UDID is listed.
- Inspect the daemon log with `journalctl --user -u dopamine-daemon.service -n 150 --no-pager`.

### A 2FA or certificate dialog does not appear

The dialogs require an active graphical desktop session. Start the daemon from a logged-in user session and make sure the session's GTK, D-Bus, and display environment is available to user services.

### Apple sign-in or installation fails

Inspect the daemon log for the Apple authentication or sideloading error. Apple account verification, Developer Program restrictions, certificate limits, and device trust state can prevent installation even when USB detection works.

## Project layout

```text
src/
├── main.rs          # Apple sign-in and Dopamine sideloading
├── lib.rs           # Shared paths and Secret Service access
└── bin/
    ├── daemon.rs    # usbmuxd device monitor
    └── settings.rs  # GTK desktop settings application

scripts/
├── bootstrap.sh     # Download and launch the installer
├── install.sh       # Dependencies, build, and installation
└── update-ipa.sh    # Dopamine IPA release updater
```

## Acknowledgements

- [Dopamine](https://github.com/opa334/Dopamine) by opa334.
- [isideload](https://crates.io/crates/isideload) for Apple account authentication and sideloading.
- [libimobiledevice](https://libimobiledevice.org/) and `usbmuxd` for Linux iOS device connectivity.
