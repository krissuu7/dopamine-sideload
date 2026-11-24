# Dopamine Sideload for Linux

A Linux desktop utility for installing [Dopamine](https://github.com/opa334/Dopamine) on a connected iPhone over USB.

The project combines an automatic USB device monitor, the sideloading process, a GTK settings application, and an IPA updater. It uses the Rust `isideload` library for Apple account authentication and app installation.

> **Disclaimer:** This is an unofficial, community-developed tool. It is not affiliated with, endorsed by, or supported by Dopamine's developers or Apple.

## Features

- Automatically detects connected iPhones through `usbmuxd`.
- Starts the sideloading process when a supported USB device is detected.
- Provides a GTK settings application for Apple ID credentials and daemon control.
- Displays device, daemon, sideloading, and local IPA status.
- Opens graphical prompts for Apple two-factor authentication and certificate selection.
- Stores Apple ID credentials through the system Secret Service API instead of plaintext files.
- Downloads the latest Dopamine IPA from the official Dopamine GitHub releases.
- Validates the downloaded IPA and verifies its SHA-256 digest when the release provides one.
- Checks for IPA updates automatically once a day.

## Requirements

- A Linux distribution with **systemd user services**.
- A working graphical desktop session with GTK 4 and libadwaita support.
- `usbmuxd` and `libimobiledevice`.
- A Secret Service-compatible password manager, such as GNOME Keyring.
- An Apple account and an iPhone supported by the selected Dopamine release.
- A USB cable and the iPhone unlocked and trusted by the computer.

The installer builds the application from source, so Rust/Cargo and the GTK, libadwaita, and D-Bus development packages are also required. Missing supported system packages can be installed by the installer after confirmation.

## Installation

Run the following command in a terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload bash
```

The installer:

1. Downloads the selected repository branch.
2. Checks for required build tools and libraries.
3. Offers to install missing dependencies using the detected package manager.
4. Builds the daemon, sideloader, and settings application locally.
5. Installs the application files and user-level systemd services.
6. Downloads the latest Dopamine IPA.
7. Enables the daily IPA update timer and starts the USB monitoring daemon.

The installer currently recognizes **DNF, APT, Pacman, and Zypper**.

### Install from another branch

Set `DOPAMINE_REF` to the branch name:

```sh
curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload DOPAMINE_REF=branch-name bash
```

### Install without starting the daemon

For development or testing:

```sh
curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload DOPAMINE_START_DAEMON=0 bash
```

## First launch

1. Open **Dopamine Sideload Settings** from your desktop application menu.
2. Enter your Apple ID and password, then select **Save login details**.
3. Connect your iPhone over USB, unlock it, and accept the **Trust This Computer** prompt if shown.
4. Keep the user session active while the daemon detects the device and starts the sideloader.
5. Follow the graphical prompts for Apple two-factor authentication or certificate management when they appear.

The daemon runs as a systemd **user** service. It monitors USB connections and launches the sideloader for the detected device. The sideloader authenticates with Apple, creates or reuses the required developer credentials, and installs the local IPA.

## Settings and commands

Launch the settings application from the desktop menu, or run:

```sh
dopamine-settings
```

Useful service commands:

```sh
systemctl --user status dopamine-daemon.service
systemctl --user restart dopamine-daemon.service
journalctl --user -u dopamine-daemon.service -f
```

The settings application can also enable or disable the daemon and retry the sideloading process.

## Updating

Run the same installation command again to fetch the latest repository version and rebuild/reinstall the application:

```sh
curl -fsSL https://raw.githubusercontent.com/krissuu7/dopamine-sideload/main/scripts/bootstrap.sh | DOPAMINE_REPOSITORY=krissuu7/dopamine-sideload bash
```

The IPA updater is separate from the application build. It checks the official [Dopamine releases](https://github.com/opa334/Dopamine/releases) daily and can also be run manually:

```sh
dopamine-update-ipa
```

## File locations

By default, application data is stored under:

```text
~/.local/share/dopamine-sideload/
├── bin/
│   ├── dopamine-daemon
│   ├── dopamine-sideload
│   ├── dopamine-settings
│   └── dopamine-update-ipa
├── Dopamine.ipa
└── Dopamine.version
```

The installation also creates user-level systemd units in `~/.config/systemd/user/`, a desktop entry, and an application icon. The data directory can be changed with `DOPAMINE_HOME`.

## Distribution status

The installer contains dependency mappings for Fedora/DNF, Debian and Ubuntu/APT, Arch Linux/Pacman, and openSUSE/Zypper.

**Testing status:** successful installation and sideloading have been tested on Fedora and Xubuntu (Ubuntu-based). The other package-manager paths are included but have not yet been independently verified. Package names and availability may vary by distribution release.

## Troubleshooting

### The iPhone is not detected

- Unlock the iPhone and accept the trust prompt.
- Check that `usbmuxd` is installed and running.
- Verify detection with:

  ```sh
  idevice_id -l
  ```

### The sideloading process fails

Inspect the daemon log:

```sh
journalctl --user -u dopamine-daemon.service -n 200 --no-pager
```

Check the reported Apple authentication, developer account, certificate, or device error before retrying.

### A graphical authentication prompt does not appear

The graphical prompts require an active desktop session. Run the daemon and sideloader from the logged-in user's systemd session, not from a root service or a headless session.

## Limitations

- A systemd user session and an active graphical login session are required for the normal desktop workflow.
- The iPhone must be connected over USB and trusted by the computer.
- Apple Developer account limits may require revoking an existing development certificate before another can be created.
- Multiple connected devices and multiple Apple developer teams may require additional selection or configuration.

## License

See the repository's license file. If no license file is present, all rights remain with the respective copyright holders.
