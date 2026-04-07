<h1 align="center">
  <img src="assets/trayicon.png" alt="Asus Hub icon" width="128" height="128"/>
  <br>
  Asus Hub
</h1>

<p align="center">
  A centralized settings application for ASUS laptops on Linux — bringing together hardware controls, display tuning, audio profiles, and system management in one place.
</p>

<p align="center">
  <strong>Tested on:</strong> ASUS Zenbook S16 · Fedora 43 · KDE Plasma
</p>

---

## Screenshots

<table align="center"><tr>
  <td align="center"><img src="screenshots/Display.png" alt="Display settings" width="100%"/><br/><em>Display Settings</em></td>
  <td align="center"><img src="screenshots/Keyboard.png" alt="Keyboard settings" width="100%"/><br/><em>Keyboard Settings</em></td>
</tr><tr>
  <td align="center"><img src="screenshots/Audio.png" alt="Audio settings" width="100%"/><br/><em>Audio Settings</em></td>
  <td align="center"><img src="screenshots/Touchpad.png" alt="Touchpad settings" width="100%"/><br/><em>Touchpad Settings</em></td>
</tr><tr>
  <td align="center" colspan="2"><img src="screenshots/System.png" alt="System settings" width="50%"/><br/><em>System Settings</em></td>
</tr></table>

---

## Motivation

ASUS provides the **MyAsus** application for Windows, offering a unified interface to control display settings, battery care, fan profiles, keyboard backlight, and more. On Linux, no equivalent exists.

Instead, the relevant controls are scattered across a variety of independent tools and configuration files:
- Battery charge limits via `asusctl`
- Audio effects via EasyEffects with manual preset management
- Display brightness quirks via `kscreen-doctor`
- Fan profiles via D-Bus calls to `asusd`
- Keyboard backlight via idle daemons like `swayidle`
- OLED-specific care settings buried in KDE power management config files

**Asus Hub** aims to consolidate all of these into a single, clean GTK4 interface — making it easy to manage your ASUS laptop on Linux without needing to know which tool controls which feature.

The application is smart about availability: if a required tool or desktop environment is not detected, the corresponding setting is automatically disabled rather than silently failing.

---

## Features

### Display

| Feature | Description | Requires |
|---------|-------------|----------|
| OLED Flicker-Free Dimming | Reduces OLED panel flickering at low brightness levels using a 10–100% slider | `kscreen-doctor` |
| Color Gamut | Switch between Native, sRGB, DCI-P3, and Display P3 color profiles — bundled ICC files sourced directly from ASUS | — |
| Target Mode *(KDE only)* | Dims unfocused windows using the KWin `diminactive` compositor effect | KDE, `qdbus` |
| OLED Pixel Refresh *(KDE only)* | Activates a pixel refresh screensaver after inactivity to reduce burn-in risk | KDE, `kwriteconfig6` |
| Panel Auto-Hide *(KDE only)* | Automatically hides the KDE panel to reduce static OLED elements | KDE, `qdbus` |
| Panel Transparency *(KDE only)* | Sets the KDE panel to transparent or opaque | KDE, `qdbus` |

> **Bundled color gamut presets:** Native, sRGB, DCI-P3 and Display P3
>
> These presets are the original ASUS color gamut profiles.

### Keyboard

| Feature | Description | Requires |
|---------|-------------|----------|
| Automatic Backlight | Uses the ambient light sensor to automatically adjust keyboard brightness | `iio-sensor-proxy` |
| Backlight Idle Mode | Turns off backlight after inactivity (1/2/5 min), configurable per power state | `swayidle` |
| FN Key Mode | Toggle between function key priority (F1–F12) and shortcut priority | `asusd` |

### Touchpad

| Feature | Description | Requires |
|---------|-------------|----------|
| Smart Gestures | Control volume, brightness, and media playback via touchpad edge swipes | — |
| Touchpad Toggle | Enable or disable the touchpad, with a 10-second auto-revert safety timer | KDE or GNOME |

### Audio

| Feature | Description | Requires |
|---------|-------------|----------|
| Volume & Boost | Control system volume from 0–150% via PipeWire | `wpctl` |
| Sound Profiles | Apply EasyEffects presets bundled with the app (see below) | `easyeffects` |

> **Bundled EasyEffects presets:** Movie, Music, Perfect EQ, Video, Voice, Custom
>
> These presets are **not** the original ASUS audio profiles — they are [Community Presets](https://github.com/wwmm/easyeffects/wiki/Community-presets) from the EasyEffects project, included for convenience. They are automatically installed to the EasyEffects preset directory on first use.

### System

| Feature | Description | Requires |
|---------|-------------|----------|
| Battery Maintenance Mode | Limit charging to 80% for long-term battery health | `asusd` |
| Full Charge | Charge to 100% with automatic revert to maintenance mode after 24 hours | `asusd` |
| Deep Sleep | Switch between `s2idle` and `deep` suspend modes | — |
| Fan Profiles | Switch between Performance, Balanced, and Quiet fan curves | `asusd` |

### General

- **Global search** — search across all settings with a keyboard shortcut
- **System tray** — minimize to tray, restore or quit from tray menu
- **Persistent configuration** — settings are saved to `~/.config/asus-hub/config.json` and restored on every launch
- **Multilingual UI** — English and German supported, switchable at runtime
- **Toast notifications** — errors and status messages shown as non-blocking toasts

---

## Dependencies

Asus Hub integrates with several external tools and system services. Install only the ones relevant to the features you want to use.

| Dependency | Purpose | Package (Fedora) |
|------------|---------|-----------------|
| [`asusctl`](https://gitlab.com/asus-linux/asusctl) | Battery care, fan profiles, FN key mode | `asusctl` (copr) |
| `asusd` | System daemon required by asusctl | bundled with `asusctl` |
| [`easyeffects`](https://github.com/wwmm/easyeffects) | Audio sound profiles | `easyeffects` |
| `kscreen-doctor` | OLED flicker-free dimming | `kscreen` |
| `iio-sensor-proxy` | Ambient light sensor for auto backlight | `iio-sensor-proxy` |
| `swayidle` | Keyboard backlight idle timer | `swayidle` |
| `wpctl` | PipeWire volume control | bundled with `pipewire-utils` |
| `qdbus` | KDE-specific D-Bus calls (KDE features) | `qt6-tools` |
| `kwriteconfig6` | KDE config file access (KDE features) | `kf6-kconfig` |
| `gsettings` | Touchpad toggle on GNOME | `glib2` |

> Features that depend on a missing tool or an incompatible desktop environment are automatically disabled in the UI.

---

## Hardware & OS Compatibility

The application has been developed and tested on:

- **Laptop:** ASUS Zenbook S16
- **OS:** Fedora 43
- **Desktop:** KDE Plasma (Wayland)

Other ASUS laptops are likely supported to varying degrees. Features relying on `asusd` (battery, fan, FN key) depend on your device being supported by asusctl. Check the [asusctl device support list](https://gitlab.com/asus-linux/asusctl) for compatibility.

Other Linux distributions should work as long as the relevant dependencies can be installed. Features are individually guarded against missing tools, so the app remains usable even if only some dependencies are available.

---

## Installation

### Prerequisites

- Rust toolchain (install via [rustup](https://rustup.rs))
- GTK4 and Libadwaita development libraries

**Fedora:**
```bash
sudo dnf install gtk4-devel libadwaita-devel
```

**Arch:**
```bash
sudo pacman -S gtk4 libadwaita
```

### Install external tools

Most tools are already included with a standard Fedora KDE installation (`kscreen-doctor`, `kwriteconfig6`, `wpctl`). The following need to be installed manually:

**asusctl** (via COPR — see [asus-linux.org](https://asus-linux.org) for full documentation):
```bash
sudo dnf copr enable lukenukem/asus-linux
sudo dnf update
sudo dnf install asusctl supergfxctl
sudo dnf update --refresh
sudo systemctl enable supergfxd.service
```
Reboot after installation.

**Remaining tools:**
```bash
sudo dnf install easyeffects iio-sensor-proxy swayidle
```

### Build

```bash
git clone https://github.com/Traciges/asus-hub
cd asus-hub
cargo build --release
./target/release/asus-hub
```

---

## Project Structure

```
asus-hub/
├── src/
│   ├── main.rs               # Entry point: CSS, i18n, tray, app init
│   ├── app.rs                # Main window, tab navigation, search integration
│   ├── tray.rs               # System tray (freedesktop.org SNI protocol)
│   ├── search.rs             # Global search index and result routing
│   ├── components/
│   │   ├── display/          # OLED dimming, color gamut, target mode, OLED care
│   │   ├── keyboard/         # Backlight auto/idle, FN key mode
│   │   ├── audio/            # Volume control, EasyEffects sound profiles
│   │   └── system/           # Battery care, fan profiles, deep sleep
│   └── services/
│       ├── dbus.rs           # Shared asusd D-Bus proxy (lazy singleton)
│       ├── commands.rs       # Shell command helpers
│       ├── config.rs         # JSON config persistence
│       └── edge_gestures.rs  # Touchpad evdev gesture engine
├── locales/
│   ├── en.yml                # English translations
│   └── de.yml                # German translations
├── assets/
│   ├── icm/                  # ICC color profile files
│   ├── presets/              # EasyEffects audio preset files
│   └── trayicon.png          # Application tray icon
└── Cargo.toml
```

**Tech stack:** Rust · GTK4 · Libadwaita · [Relm4](https://relm4.org) · [zbus](https://dbus2.github.io/zbus/) · Tokio

Each settings component is a self-contained Relm4 `Component` with its own state model, message types, and declarative view — making it straightforward to add new settings without touching unrelated code.

---

## Contributing

Contributions are welcome. If you own a different ASUS laptop model and want to add or fix support for specific features, feel free to open an issue or pull request.

When adding a new feature, follow the existing component pattern in `src/components/` and add corresponding translation keys to both `locales/en.yml` and `locales/de.yml`.

---

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).
