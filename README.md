# Dopamine Sideload for Linux

A Linux desktop utility for installing and updating **[Dopamine](https://github.com/opa334/Dopamine)** on a USB-connected iPhone.

The project combines a USB device monitor, Apple account authentication, a GTK desktop settings application, and an updater for the official Dopamine IPA. Sideloading is powered by the Rust `isideload` library.

> **Unofficial project.** This repository is not affiliated with or endorsed by the Dopamine project or Apple.

## Features

- **Automatic USB detection** — monitors `usbmuxd` and starts sideloading when an iPhone is connected.
- **Desktop settings** — GTK 4 / libadwaita application available from the desktop application menu.
- **Credential storage** — stores the Apple ID and password through the system Secret Service keyring; credentials are not written to a plaintext configuration file.
- **Two-factor authentication** — displays a graphical prompt when Apple requests a trusted-device verification code.
- **Certificate management** — provides a graphical selection dialog when the Apple Developer certificate limit is reached.
- **Dopamine IPA updater** — downloads the latest IPA from the official `opa334/Dopamine` GitHub release, validates the archive, and verifies the published SHA-256 digest when available.
- **Scheduled update checks** — a systemd user timer checks for new Dopamine releases daily.

## Requirements

- A Linux desktop distribution with **systemd user services**.
- A working **Secret Service** provider (for example, GNOME Keyring or KWallet configured with Secret Service support).
- `usbmuxd` and the libimobiledevice utilities.
- GTK 4, libadwaita, D-Bus development files, and `pkg-config`.
- Rust and Cargo **1.88 or newer**. If the system Rust is older, the installer can offer to install the stable toolchain using rustup.
- An active graphical user session for the settings application and interactive 2FA / certificate dialogs.

The installer detects and supports these package managers:

| Package manager | Distribution family |
| --- | --- |
| APT | Debian, Ubuntu and derivatives |
| DNF | Fedora and derivatives |
| Pacman | Arch Linux and derivatives |
| Zypper | openSUSE |

## Installation

Run the following command from a terminal in your desktop session:

```bash
curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh \
  | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload bash
```

The installer:

1. Downloads the selected repository branch.
2. Checks for required build tools, libraries, and device utilities.
3. Asks for confirmation before installing missing system packages.
4. Builds the daemon, sideloader, and settings application locally.
5. Installs the binaries, desktop launcher, icon, and systemd user units.
6. Downloads the latest Dopamine IPA and enables the daily update timer.
7. Enables and starts the USB monitoring daemon.

The installer may request your password through `sudo` when system packages need to be installed.

### Install a specific branch

The default branch is `main`. Set `DOPAMINE_REF` to select another branch:

```bash
curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh \
  | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload DOPAMINE_REF=BRANCH_NAME bash
```

### Reinstall or update the application

Run the installation command again to download the latest source from the selected branch, rebuild the applications, and replace the installed binaries and service files.

Your application data directory is preserved, including saved credentials and the downloaded IPA. **The current installer performs a fresh source build on each run; incremental binary updates are not implemented yet.**

### Optional installer settings

| Variable | Description | Default |
| --- | --- | --- |
| `DOPAMINE_REPOSITORY` | GitHub repository in `owner/name` format | Required |
| `DOPAMINE_REF` | Branch to install | `main` |
| `DOPAMINE_HOME` | Application data and binary directory | `~/.local/share/dopamine-sideload` |
| `DOPAMINE_START_DAEMON` | Set to `0` to install without enabling or starting the daemon | `1` |

Example — install without starting the USB monitor:

```bash
curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh \
  | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload DOPAMINE_START_DAEMON=0 bash
```

## First-time setup

1. Open **Dopamine Sideload Settings** from your desktop application menu.
2. Enter your Apple ID and password, then select **Save login details**.
3. Connect your iPhone over USB, unlock it, and accept **Trust This Computer** if prompted.
4. Keep your desktop session active. Enter the verification code when the Apple 2FA dialog appears.
5. If the Apple Developer certificate limit is reached, review the certificates and select only those you are willing to revoke.

The daemon starts the sideloader automatically when it detects an eligible USB connection. Keep the iPhone connected and unlocked during installation.

> **Certificate warning:** revoking an Apple Development certificate can affect other apps or workflows that depend on it. Review the selection carefully before confirming.

## Settings and command-line tools

Launch **Dopamine Sideload Settings** from the desktop application menu. The application lets you:

- Enable or disable background iPhone monitoring.
- View daemon and USB device status.
- View the latest sideload result.
- View the locally installed Dopamine IPA version.
- Check whether a newer Dopamine release is available.
- Save or update Apple ID credentials.

The status display refreshes automatically.

The installer also provides these commands in `~/.local/bin`:

```bash
dopamine-settings
dopamine-update-ipa
```

## Service management and diagnostics

Check the daemon:

```bash
systemctl --user status dopamine-daemon.service
```

Follow daemon and sideloader logs:

```bash
journalctl --user -u dopamine-daemon.service -f
```

Restart USB monitoring:

```bash
systemctl --user restart dopamine-daemon.service
```

Check whether the iPhone is visible to libimobiledevice:

```bash
idevice_id -l
```

Check the scheduled IPA updater:

```bash
systemctl --user status dopamine-updater.timer
```

Manually check for and download the latest Dopamine IPA:

```bash
dopamine-update-ipa
```

The updater timer is scheduled to run daily at 04:00 local time.

## Installed files

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

Additional files:

| Path | Purpose |
| --- | --- |
| `~/.local/bin/dopamine-settings` | Settings application launcher |
| `~/.local/bin/dopamine-update-ipa` | IPA updater launcher |
| `~/.local/share/applications/dopamine-settings.desktop` | Desktop menu entry |
| `~/.config/systemd/user/dopamine-daemon.service` | USB monitoring service |
| `~/.config/systemd/user/dopamine-updater.service` | One-shot IPA updater service |
| `~/.config/systemd/user/dopamine-updater.timer` | Daily update timer |

If `XDG_DATA_HOME` or `XDG_CONFIG_HOME` is set, the corresponding paths use those locations.

## Distribution test status

| Distribution | Package manager | Status |
| --- | --- | --- |
| Fedora | DNF | Tested |
| Xubuntu (Ubuntu base) | APT | Tested |
| Arch Linux | Pacman | Package mapping implemented; testing pending |
| openSUSE | Zypper | Package mapping implemented; testing pending |

Testing on one distribution does not guarantee compatibility with every release or desktop configuration in the same distribution family. Reports and test results from additional distributions are welcome.

## Troubleshooting

### The iPhone is not detected

- Unlock the iPhone and reconnect the USB cable.
- Accept the trust prompt on the iPhone.
- Confirm that `usbmuxd` is installed and running.
- Run `idevice_id -l` and check whether the device UDID is listed.
- Inspect the daemon log:

  ```bash
  journalctl --user -u dopamine-daemon.service -n 150 --no-pager
  ```

### A 2FA or certificate dialog does not appear

The dialogs require an active graphical desktop session. Start the daemon from a logged-in user session and make sure the user service has access to the session's display and D-Bus environment.

### Apple sign-in or installation fails

Inspect the daemon log for the Apple authentication or sideloading error. Apple account verification, Developer Program restrictions, certificate limits, and device trust state can prevent installation even when USB detection works.

### Credentials cannot be saved

Make sure a Secret Service-compatible password manager is installed, running, and unlocked.

## Project structure

```text
src/
├── main.rs          # Apple sign-in and Dopamine sideloading
├── lib.rs           # Shared paths and Secret Service access
└── bin/
    ├── daemon.rs    # usbmuxd device monitor
    └── settings.rs  # GTK desktop settings application

scripts/
├── bootstrap.sh     # Download and launch the installer
├── install.sh       # Dependency checks, build, and installation
└── update-ipa.sh    # Dopamine IPA release updater
```

## Acknowledgements

- [Dopamine](https://github.com/opa334/Dopamine) by opa334.
- [isideload](https://crates.io/crates/isideload) for Apple account authentication and sideloading.
- [libimobiledevice](https://libimobiledevice.org/) and `usbmuxd` for iOS device connectivity on Linux.
