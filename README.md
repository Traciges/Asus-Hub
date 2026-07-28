<h1 align="center">
  <img src="assets/trayicon.png" alt="Ayuz icon" width="128" height="128"/>
  <br>
  Ayuz
</h1>

<p align="center">
  A centralized settings application for ASUS laptops on Linux - bringing together hardware controls, display tuning, audio profiles, and system management in one place.
</p>

<p align="center">
  <strong>Tested on:</strong> ASUS Zenbook S16 · Fedora 43 · KDE Plasma
</p>

---

> [!WARNING]
> **Disclaimer:** This is an unofficial, community-driven, open-source project. **"Ayuz" is NOT affiliated with, endorsed by, sponsored by, or connected to ASUSTeK Computer Inc. in any way.** "ASUS", "Zenbook", "ROG", "Vivobook", and "MyAsus" are registered trademarks of ASUSTeK Computer Inc. <br>
> The use of ASUS trademarks within this website and associated tools and libraries is only to provide a recognisable identifier to users to enable them to associate that these tools will work with ASUS laptops. <br>
> This software is provided as-is, without warranty, and uses community reverse-engineered backend tools. Use at your own risk.

---

## Screenshots

<table align="center">
  <tr>
    <td align="center" colspan="2"><img src="screenshots/Homepage.png" alt="Homepage" width="100%"/><br/><em>Homepage</em></td>
  </tr>
  <tr>
    <td align="center"><img src="screenshots/Audio.png" alt="Audio settings" width="100%"/><br/><em>Audio Settings</em></td>
    <td align="center"><img src="screenshots/Aura.png" alt="Aura settings" width="100%"/><br/><em>Aura Settings</em></td>
  </tr>
  <tr>
  <td align="center"><img src="screenshots/Display.png" alt="Display settings" width="100%"/><br/><em>Display Settings</em></td>
  <td align="center"><img src="screenshots/Keyboard.png" alt="Keyboard settings" width="100%"/><br/><em>Keyboard Settings</em></td>
</tr>
<tr>
  <td align="center"><img src="screenshots/System.png" alt="System settings" width="100%"/><br/><em>System Settings</em></td>
    <td align="center"><img src="screenshots/Touchpad.png" alt="Touchpad settings" width="100%"/><br/><em>Touchpad Settings</em></td>
</tr>
</table>

---

## Motivation

ASUS provides the MyAsus application for Windows, offering a unified interface to control display settings, battery care, fan profiles, keyboard backlight, and more. On Linux, no equivalent exists. <br>
Instead, the relevant controls are scattered across a variety of independent tools and configuration files:

- Battery charge limits via `asusctl`
- Audio effects via EasyEffects with manual preset management
- Display brightness quirks via `kscreen-doctor`
- Fan profiles via D-Bus calls to `asusd`
- Keyboard backlight via idle daemons like `swayidle`
- Aura RGB lighting via raw D-Bus calls to `asusd`
- AniMatrix support via `asusctl`
- OLED-specific care settings buried in KDE power management config files
- NumberPad support for illuminated numeric keypad via `asus-numberpad-driver`

Ayuz aims to consolidate all of these into a single, clean GTK4 interface - making it easy to manage your ASUS laptop on Linux without needing to know which tool controls which feature. <br>
The application is smart about availability: if a required tool or desktop environment is not detected, the corresponding setting is automatically disabled rather than silently failing.

---

## Features

### Display

| Feature                   | Description                                                                                                       | Requires                      |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| OLED Flicker-Free Dimming | Reduces OLED panel flickering at low brightness levels using a 10-100% slider                                     | KDE, `kscreen-doctor`         |
| Color Gamut               | Switch between Native, sRGB, DCI-P3, and Display P3 color profiles - bundled ICC files sourced directly from ASUS | KDE, `kscreen-doctor`         |
| Target Mode               | Dims unfocused windows using the KWin `diminactive` compositor effect                                             | KDE, `qdbus`, `kwriteconfig6` |
| OLED Pixel Refresh        | Activates a pixel refresh screensaver after inactivity to reduce burn-in risk                                     | KDE, `kwriteconfig6`          |
| Panel Auto-Hide           | Automatically hides the KDE panel to reduce static OLED elements                                                  | KDE, `qdbus`                  |
| Panel Transparency        | Sets the KDE panel to transparent or opaque                                                                       | KDE, `qdbus`                  |

> **Bundled color gamut presets:** Native, sRGB, DCI-P3 and Display P3 <br>
> These presets are the original ASUS color gamut profiles.

### Keyboard

| Feature             | Description                                                                    | Requires           |
| ------------------- | ------------------------------------------------------------------------------ | ------------------ |
| Automatic Backlight | Uses the ambient light sensor to automatically adjust keyboard brightness      | `iio-sensor-proxy` |
| Backlight Idle Mode | Turns off backlight after inactivity (1/2/5 min), configurable per power state | `swayidle`         |
| FN Key Mode         | Toggle between function key priority (F1-F12) and shortcut priority            | `asusd`            |
| FN+F Fan Hotkey     | Cycles fan profiles globally (Balanced → Performance → Quiet → Balanced) via the hardware FN+F key; shows an on-screen notification on each switch | `asusd`            |

### Aura RGB

| Feature           | Description                                                                                                                   | Requires |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------- | -------- |
| Lighting Mode     | Choose from 12 modes: Static, Breathe, Rainbow Cycle, Rainbow Wave, Star, Rain, Highlight, Laser, Ripple, Pulse, Comet, Flash | `asusd`  |
| Color Control     | Set primary and secondary colors (RGB) - secondary color available for Breathe and Star modes                                 | `asusd`  |
| Brightness        | Four brightness levels: Off, Low, Medium, High                                                                                | `asusd`  |
| Speed & Direction | Control animation speed (Low/Medium/High) and direction (Rainbow Wave only)                                                   | `asusd`  |
| Power Settings    | Configure LED behavior per zone for Boot, Awake, Sleep, and Shutdown states                                                   | `asusd`  |

### AniMatrix LED Panel

Controls for the AniMatrix LED matrix display found on select ROG laptops (e.g. ROG Zephyrus G14 GA401). The section is always visible but grayed out on unsupported hardware.

| Feature                  | Description                                                                                                      | Requires |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------- | -------- |
| Enable / Disable         | Turn the AniMatrix panel on or off                                                                               | `asusd`  |
| Brightness               | Four brightness levels: Off, Low, Medium, High                                                                   | `asusd`  |
| Built-in Animations      | Enable or disable firmware-driven animations and select which animation plays during Boot, Awake, Sleep, Shutdown | `asusd`  |
| Power Behavior           | Automatically turn off the panel when unplugged, suspended, or when the lid is closed                            | `asusd`  |
| GIF Gallery              | Stream one of 34 bundled ASUS animations directly to the panel; play/stop with a single button                   | `asusd`  |
| Profile Integration      | All AniMatrix settings are saved per profile and restored on profile switch                                       | `asusd`  |

> **GIF Gallery availability:** The gallery currently supports **GA401** hardware (ROG G14 2021), where the bundled ASUS GIF format (74×36 px) matches the panel exactly. GIF playback on other AniMatrix models (GA402, GU604, G635L, G835L) is planned for a future release once multi-device frame sizing is available. Firmware controls (brightness, built-in animations, power behavior) work on all supported models.

### Touchpad

| Feature         | Description                                                               | Requires                     |
| --------------- | ------------------------------------------------------------------------- | ---------------------------- |
| Smart Gestures  | Control volume, brightness, and media playback via touchpad edge swipes   | `brightnessctl`, `playerctl` |
| Touchpad Toggle | Enable or disable the touchpad, with a 10-second auto-revert safety timer | KDE or GNOME                 |
| While Typing    | Palm rejection: suppress Smart Gestures and/or disable the touchpad entirely while typing, with a configurable reactivation delay (0.1-5 s) after the last keystroke | user in `input` group; KDE or GNOME for the touchpad option |
| ASUS NumberPad  | Toggle the LED numeric keypad embedded in the touchpad. Emits standard Linux Numpad keycodes (`KEY_KP0`..`KEY_KPENTER`) through a virtual uinput device, so international keyboard layouts (e.g. German, Lithuanian) work natively. | `i2c-dev` + `uinput` kernel modules; user in `i2c` and `input` groups (see below) |

#### NumberPad Requirements

The NumberPad driver runs entirely in-process. It needs unprivileged read/write access to two kernel facilities:

- **`/dev/i2c-N`** - the touchpad's I2C slave receives a 13-byte magic packet to turn the LED grid on or off.
- **`/dev/uinput`** - a virtual keyboard emits Numpad keycodes when you tap a cell.

1. Load the required kernel modules (persist via `/etc/modules-load.d/ayuz.conf` if you want them at boot):

   ```bash
   sudo modprobe i2c-dev uinput
   ```

2. Add your user to the `i2c` and `input` groups, then restart your device:

   ```bash
   sudo usermod -aG i2c,input $USER
   ```

3. Install the bundled udev rules. Distro packages (Fedora/Debian/RPM/DEB) install them automatically to `/usr/lib/udev/rules.d/99-ayuz-numberpad.rules` so you don't need to do anything if you run one of these.
**For source builds and AppImage users**, manual installation is required:

   ```bash
   # If using the AppImage, download the rules file first:
   wget [https://raw.githubusercontent.com/Traciges/Ayuz/main/packaging/udev/99-ayuz-numberpad.rules](https://raw.githubusercontent.com/Traciges/Ayuz/main/packaging/udev/99-ayuz-numberpad.rules)

   # Copy it to the system folder and reload udev:
   sudo cp 99-ayuz-numberpad.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules && sudo udevadm trigger
   ```

If any of these are missing the NumberPad row in the Touchpad page is disabled and shows an actionable error.

### Audio

| Feature        | Description                                                | Requires      |
| -------------- | ---------------------------------------------------------- | ------------- |
| Volume & Boost | Control system volume from 0-150% via PipeWire             | `wpctl`       |
| Sound Profiles | Apply EasyEffects presets bundled with the app (see below) | `easyeffects` |

> **Bundled EasyEffects presets:** Movie, Music, Perfect EQ, Video, Voice, Custom <br>
> These presets are **not** the original ASUS audio profiles - they are [Community Presets](https://github.com/wwmm/easyeffects/wiki/Community-presets) from the EasyEffects project, included for convenience. They are automatically installed to the EasyEffects preset directory on first use.

### System

| Feature                  | Description                                                                                                                                                   | Requires      |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- |
| Battery Maintenance Mode | Limit charging to 80% for long-term battery health                                                                                                            | `asusd`       |
| Full Charge              | Charge to 100% with automatic revert to maintenance mode after 24 hours                                                                                       | `asusd`       |
| Deep Sleep               | Switch between `s2idle` and `deep` suspend modes                                                                                                              | -             |
| Fan Profiles             | Switch between Performance, Balanced, and Quiet fan curves                                                                                                    | `asusd`       |
| GPU Mode                 | Switch between GPU modes: Hybrid, Integrated, Nvidia (No Modeset), VFIO, ASUS eGPU, and ASUS MUX Discrete. Switching GPU modes requires a full system reboot. | `supergfxctl` |
| GPU Memory Allocation    | Reserve system RAM for the integrated GPU (UMA Frame Buffer). Options: Auto, 1-8 GB. Requires a supported BIOS. Changes require a full system reboot.         | `asusd`       |

### General

- **Profiles** - create and manage multiple named configuration profiles, each with a custom icon. Settings are automatically saved to the currently selected profile and instantly restored when switching
- **Global search** - search across all settings with a keyboard shortcut
- **System tray** - minimize to tray, restore or quit from tray menu
- **Autostart** - optional autostart with the system; when enabled, the app launches hidden (`--hidden`) and only appears in the tray. Managed via a `.desktop` file at `~/.config/autostart/de.guido.ayuz.desktop`
- **Persistent configuration** - settings are saved to `~/.config/ayuz/config.json` and restored on every launch
- **Multilingual UI** - English, German, and Brazilian Portuguese supported, switchable at runtime
- **Toast notifications** - errors and status messages shown as non-blocking toasts

---

## Dependencies

Ayuz integrates with several external tools and system services. Install only the ones relevant to the features you want to use.

| Dependency                                                 | Purpose                                                          | Package (Fedora)              |
| ---------------------------------------------------------- | ---------------------------------------------------------------- | ----------------------------- |
| [`asusctl`](https://gitlab.com/asus-linux/asusctl)         | Battery care, fan profiles, FN key mode                          | `asusctl` (copr)              |
| `asusd`                                                    | System daemon required by asusctl                                | bundled with `asusctl`        |
| [`supergfxctl`](https://gitlab.com/asus-linux/supergfxctl) | GPU mode switching (Hybrid, Integrated, Nvidia, VFIO, eGPU, MUX) | `supergfxctl` (copr)          |
| [`easyeffects`](https://github.com/wwmm/easyeffects)       | Audio sound profiles                                             | `easyeffects`                 |
| `kscreen-doctor`                                           | OLED flicker-free dimming                                        | `kscreen`                     |
| `iio-sensor-proxy`                                         | Ambient light sensor for auto backlight                          | `iio-sensor-proxy`            |
| `swayidle`                                                 | Keyboard backlight idle timer                                    | `swayidle`                    |
| `wpctl`                                                    | PipeWire volume control                                          | bundled with `pipewire-utils` |
| `qdbus`                                                    | KDE-specific D-Bus calls (KDE features)                          | `qt6-tools`                   |
| `kwriteconfig6`                                            | KDE config file access (KDE features)                            | `kf6-kconfig`                 |
| `gsettings`                                                | Touchpad toggle on GNOME                                         | `glib2`                       |
| `brightnessctl`                                            | Smart Gestures                                                   | `brightnessctl`               |
| `playerctl`                                                | Media playback control                                           | `playerctl`                   |

> Features that depend on a missing tool or an incompatible desktop environment are automatically disabled in the UI.

---

## Hardware & OS Compatibility

The application has been developed and tested on:

- **Laptop:** ASUS Zenbook S16
- **OS:** Fedora 43
- **Desktop:** KDE Plasma (Wayland)

Other ASUS laptops are likely supported to varying degrees. Features relying on `asusd` (battery, fan, FN key) depend on your device being supported by asusctl. Check the [asusctl device support list](https://gitlab.com/asus-linux/asusctl) for compatibility. <br>
Other Linux distributions should work as long as the relevant dependencies can be installed. Features are individually guarded against missing tools, so the app remains usable even if only some dependencies are available.

**AniMatrix models:** The AniMatrix LED panel section is supported on the following hardware, detected automatically via DMI board name:

| Model identifier | Device |
| ---------------- | ------ |
| `GA401I`, `GA401Q` | ROG Zephyrus G14 (2021) |
| `GA402R`, `GA402X`, `GA402N` | ROG Zephyrus G14 (2022) |
| `GU604V` | ROG Zephyrus Duo 16 |
| `G635L` | ROG Strix G16 |
| `G835L` | ROG Strix SCAR 16 |

On all other models the AniMatrix section is shown as grayed out.

---

## Installation

### Package Managers:

**Fedora (Copr - Community Maintained)** <br>
A community member has packaged Ayuz for Fedora via Copr, providing automatic rebuilds and updates. <br>
**Note:** This repository is maintained by the community [SkyR0ver](https://github.com/SkyR0ver/ayuz-rpm), not officially by the upstream project. Currently supported on Fedora 43+.

```bash
sudo dnf copr enable lukenukem/asus-linux
sudo dnf copr enable skyr0ver/asus-hub
sudo dnf install ayuz
sudo systemctl enable --now supergfxd.service
```

<br>

**Arch Linux (AUR)** <br>
The application is available in the Arch User Repository. Install using your preferred AUR helper:

| Package    | Description                                              |
| ---------- | -------------------------------------------------------- |
| `ayuz`     | Compiles the latest stable release from source           |
| `ayuz-bin` | Downloads and installs the pre-compiled binary (fastest) |
| `ayuz-git` | Compiles the latest commit from the main branch          |

```bash
yay -S ayuz
```

<br>

**NixOS (Flakes)** <br>
Add `ayuz` to your `flake.nix` inputs: `ayuz.url = "github:Traciges/Ayuz";`.

- **NixOS Module (Recommended):** Automatically configures required system services (`asusd`, `supergfxd`), udev rules, and polkit policies.
  ```nix
  { inputs, ... }: {
    imports = [ inputs.ayuz.nixosModules.default ];
    services.ayuz.enable = true;
    services.ayuz.supportMyAsusKey = true; # Rebind MyAsus/ROG key to launch Ayuz
    services.ayuz.fnKeyMode = "shortcut"; # Set the initial Fn key lock state
  }
  ```
- **Home Manager Module:** For per-user installation, optional autostart, and configuration.
  ```nix
  { inputs, ... }: {
    imports = [ inputs.ayuz.homeManagerModules.default ];
    programs.ayuz.enable = true;
    programs.ayuz.autostart = true; # Start minimized on login
    programs.ayuz.settings = {
      # accepts either a Nix attribute set or a raw JSON string
      language = "en";
    };
  }
  ```
- **Traditional (Flakeless) usage:** You can use `builtins.getFlake` to use the modules or package directly.
  ```nix
  { pkgs, ... }:
  let
    ayuz-flake = builtins.getFlake "github:Traciges/Ayuz";
  in {
    imports = [ ayuz-flake.nixosModules.default ];
    services.ayuz = { ... };
    home-manager.users.username = {
      imports = [ ayuz-flake.homeManagerModules.default ];
      programs.ayuz = { ... };
    };
  }
  ```

### Install from GitHub Releases:

### 1. Prerequisites

**Note:** If you plan to use the AppImage (see Step 5), you can skip this step entirely. The AppImage already bundles all the necessary UI dependencies.

- Rust toolchain (install via [rustup](https://rustup.rs))
- GTK4 and Libadwaita development libraries

**Fedora:**

```bash
sudo dnf install gtk4-devel libadwaita-devel gtk4-layer-shell-devel
```

**Arch:**

```bash
sudo pacman -S gtk4 libadwaita gtk4-layer-shell
```

### 2. Install external tools

Most tools are already included with a standard Fedora KDE installation (`kscreen-doctor`, `kwriteconfig6`, `wpctl`). The following need to be installed manually: <br>
**asusctl** (via COPR - see [asus-linux.org](https://asus-linux.org) for full documentation):

```bash
sudo dnf copr enable lukenukem/asus-linux
sudo dnf update
sudo dnf install asusctl supergfxctl
sudo dnf update --refresh
sudo systemctl enable supergfxd.service
```

### 3. Reboot after installation

### 4. Remaining tools:

```bash
sudo dnf install easyeffects iio-sensor-proxy swayidle brightnessctl playerctl
```

### 5. Download & Install

**Manual Download:**

Download the package matching your distribution from the [GitHub Releases](https://github.com/Traciges/ayuz/releases) page:

- **Fedora / RPM-based:**

  ```bash
  sudo dnf install ./ayuz-1.1.9-1.x86_64.rpm
  ```

- **Debian / Ubuntu / DEB-based:**

  ```bash
  sudo apt install ./ayuz_1.1.9-1_amd64.deb
  ```

- **AppImage (any distribution):**
  ```bash
  chmod +x ayuz-1.1.9-1.AppImage
  ./ayuz-1.1.9-1.AppImage
  ```

**Note for AppImage users:** Because the AppImage runs without root privileges, it cannot install the necessary hardware permissions for the ASUS NumberPad automatically. If you want to use the NumberPad feature, please follow the manual `udev` rules installation in the NumberPad Requirements section above.

### Uninstall

- **Fedora, CentOS oder RHEL (via RPM/DNF):**

  ```bash
  sudo dnf remove ayuz
  ```

- **Ubuntu, Debian oder Linux Mint (via DEB/APT):**

  ```bash
  sudo apt remove ayuz
  ```

- **openSUSE (via RPM/Zypper):**
  ```bash
  sudo zypper remove ayuz
  ```

### Build from source

```bash
git clone https://github.com/Traciges/ayuz
cd ayuz
cargo build --release
./target/release/ayuz
```

### Build an RPM

```bash
cargo install cargo-rpm
cargo generate-rpm
```

### Build a DEB

```bash
cargo install cargo-deb
cargo deb
```

### Build an AppImage

Requires [`appimagetool`](https://github.com/AppImage/AppImageKit/releases) on your `$PATH`.

```bash
cargo install cargo-appimage
cargo appimage
./target/appimage/ayuz-1.1.6-1.AppImage
```

---

## Contributing

Contributions are welcome. If you own a different ASUS laptop model and want to add or fix support for specific features, feel free to open an issue or pull request.

When adding a new feature, follow the existing component pattern in `src/components/` and add corresponding translation keys to both `locales/en.yml` and `locales/de.yml`.

---

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).
