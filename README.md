# Agwaita

Previously, a GTK Shell configuration that combines Adwaita and AGS.

Now it's the same but in Rust.

## Dependencies

### System Dependencies and dev environment

- Latest stable Rust version (1.96.0)
- [Niri](https://yalter.github.io/niri/) - Wayland compositor (currently the main compositor used for development)
- GTK4 and Libadwaita system libraries
- gtk4-layer-shell - Layer Shell protocol library
- D-Bus for inter-process communication
- Gnome shell and `gnome-calendar` (for Gnome gsettings and `gnome-calendar` linked account).
- Rfkill - Tool for enabling and disabling wireless devices
- Systemd with `systemctl` and `loginctl` - Control the systemd system, service manager and the systemd login manager
- Bluez - Bluetooth protocol stack
- Nushell (for running `make.nu` build scripts)

### Rust Dependencies

#### Core

- `catalyser` - My personal crate for some utility functions and extensions
- `log` - Logging facade for structured logging throughout the application
- `pretty_env_logger` - Pretty, colorized logger for development

#### CLI

- `clap` - Command-line argument parser with derive macros for the `agwaita` CLI

#### System Integration

- `signal-hook` - Unix signal handling (SIGINT, SIGTERM) for graceful shutdown
- `inotify` - File system event monitoring (used for brightness control and avatar changes)

#### GTK/UI

- `gtk4` - GTK4 bindings for Rust (v4.20 features enabled)
- `relm4` - Elm-inspired reactive GUI framework built on GTK4/Libadwaita
- `relm4-components` - Pre-built components for Relm4
- `gtk4-layer-shell` - Layer Shell protocol support for Wayland compositors (topbar positioning)

#### D-Bus

- `zbus` - Pure Rust D-Bus implementation for system service communication (power profiles, system tray, etc.)
- `zvariant` - Type-safe D-Bus implementation details

#### Async Runtime

- `futures` - Async utilities and combinators
- `tokio` - Async runtime for D-Bus and other async operations

#### Utilities

- `serde` / `serde_json` - Serialization/deserialization
- `chrono` - Date and time handling with locale support
- `freedesktop-desktop-entry` - Desktop entry file parsing
- `nucleo-matcher` - String normalization ~~and fuzzy matching~~

#### Calendar Integration

- `calcard` - vCard and iCalendar parsing
- `rrule` - Recurring rule handling for calendar events

#### Compositor Integration

- `niri-ipc` - IPC client for Niri compositor

#### Media and Hardware

- `pipewire` - PipeWire integration for audio monitoring
- `libspa` - PipeWire SPA (Stream/Protocol/Audio) support
- `libpulse-binding` - PulseAudio/PipeWire bindings for audio management
- `brightness` - Brightness control
- `udev` - Linux device management

## Installation and running

1. Install the required dependencies
2. Clone this repository
3. Add to your PATH: $XDG_BIN_HOME or $HOME/.local/bin
4. At the root of the project:
    - Run the command `./make.nu -h` to see your options
    - Run the command `./make.nu hotrun` to test agwaita without installation
    - Run the command `./make.nu install` to install agwaita in the `$XDG_BIN_HOME` folder or the `$HOME/.local/bin` folder, which allows you to access the `agwaita` command in your PATH

### My Niri configuration

```kdl
// Startup
spawn-sh-at-startup "~/.config/niri/scripts/startup.sh"
// startup.sh: ---
// #!/usr/bin/env -S bash
//
// niri msg action spawn-sh -- "${XDG_CONFIG_HOME:-~/.config}/niri/scripts/shell"
// ---
// 
// ${XDG_CONFIG_HOME:-~/.config}/niri/scripts/shell: ---
// #!/usr/bin/nu
// 
// def main [] {
//     (
//         load-env { AGWAITA_LOG_LEVEL: trace };
//         agwaita session
//     ) e+o> $"($env.TMPDIR? | default "/tmp")/agwaita-(date now | format date "%Y-%m-%dT%H-%M-%S").log"
// }
// ---

// Layer rules
layer-rule {
    match namespace="^agwaita-bar$"
    match at-startup=true

    place-within-backdrop true

    background-effect {
        blur false
    }
}

layer-rule {
    match namespace="^agwaita-notifications$"
    match at-startup=true

    place-within-backdrop false

    background-effect {
        blur false
    }
}

// Bindings
binds {
    // start th app launcher
    Mod+D           hotkey-overlay-title="Run a Desktop Application"     { spawn-sh "agwaita app-launcher"; }

    // start the power menu
    Mod+Ctrl+Delete hotkey-overlay-title="Toggle power menu"             { spawn-sh "agwaita power-menu"; }

    // Toggle bar mode
    Mod+W           hotkey-overlay-title="Toggle bar mode"               { spawn-sh "agwaita bar toggle"; }

    // Notification
    Mod+Escape      hotkey-overlay-title="Dismiss last notification"     { spawn-sh "agwaita notification close-last"; }
    Mod+Ctrl+Escape hotkey-overlay-title="Toggle do not disturb"         { spawn-sh "agwaita notification notification dnd"; }
}
```

### Logging and Debugging

Agwaita uses structured logging for developers and system administrators to diagnose issues.

#### Log Levels

By default:
- _Debug builds_: Logs at `Debug` level and above
- _Release builds_: Logging is disabled

You can override the log level using the `AGWAITA_LOG_LEVEL` environment variable:

```bash
# Available levels: trace, debug, info, warn, error, off
AGWAITA_LOG_LEVEL=debug agwaita session
```

#### Reporting Issues with Logs

If you encounter a problem and need to report it, please provide debug logs:

1. Run Agwaita with debug logging enabled:
   ```bash
   AGWAITA_LOG_LEVEL=debug agwaita session 2>&1 | tee agwaita.log
   ```
2. Reproduce the issue
3. Stop Agwaita (Ctrl+C)
4. Attach the `agwaita.log` file to your issue report

Example log output:

```log
[2025-01-31T10:30:45.123Z INFO  agw_service] Starting Agwaita daemon
[2025-01-31T10:30:45.234Z INFO  agw_ui_session] Initializing global system state service
[2025-01-31T10:30:45.345Z DEBUG agw_ui_session::component] Initializing topbars for existing monitors
[2025-01-31T10:30:45.456Z INFO  agw_ui_session::component] Creating topbar for monitor: DP-1
```

## Philosophy and Vision

Agwaita follows a **simple, clear, and comprehensible** philosophy:

### Simplicity First

The primary goal is to create a shell that **works reliably** without unnecessary complexity. For now, this means:

- **No configuration files** - The shell works out of the box with sensible defaults
- **Focus on core functionality** - Get the essentials working well before adding customization
- Clear, readable code that's easy to understand and maintain

Configuration support may be added in the future, but only after the core functionality is solid and stable.

### GNOME Integration

Agwaita reuses specific GNOME resources and settings particularly for theming, accent colors, and desktop interface settings. D-Bus services like power-profiles-daemon, system tray protocols are also used to provide a seamless experience.

> [!NOTE]
> 
> Agwaita is developed alongside Niri compositor with GNOME installed on the system. The impact of running Agwaita without GNOME components present is currently unknown and may cause issues.

### Design Principles

1. One rust binary - Leverage Rust's safety and performance, and no additional file like css
2. GTK4/Libadwaita - Native Adwaita look and feel
3. Compositor-agnostic - Designed for Wayland compositors (tested primarily with Niri)
4. Reactive architecture - Using Relm4's Elm-inspired model for predictable state management

## Preview

<details>
<summary>General Preview</summary>

The initial Agwaita shell state, showcasing its integrated top bar. From left to right, the bar presents: 

- **Workspace indicators** (currently optimized for Niri): Interactive elements that display either the workspace index or its assigned name upon clicking.
- **Clock and date**: A clickable shortcut that opens the **Info Center**.
- **Systemd failure notification**: An indicator (with a counter) that appears when systemd units fail, providing additional details via tooltips and can recheck on click.
- **Privacy module indicator** (experimental coverage): Displays icons when activity is detected from the camera, microphone, GPS, or screen sharing, with further information available in tooltips.
- **System tray** (experimental: certain applications may not be captured): Holds application-specific icons and supports interactions with running background applications.
- **System status indicators**: A clickable suite of quick-access buttons for system settings (brightness, screen, Bluetooth, network, distraction-free mode, power, battery, and session avatar) that opens the **Quick Settings** panel, also featuring tooltips.

![General Preview](assets/img/shell.png)

</details>

<details>
<summary>App Launcher</summary>

An application launcher that provides a seamless way to search and launch applications. It features:

- **Automated Application Discovery**: Automatically scans `$XDG_DATA_DIRS` or `/usr/local/share:/usr/share` to maintain an up-to-date list of installed applications.
- **Fast & Responsive Search** (experimental): A real-time search engine with normalized results, ensuring quick and accurate application retrieval.
- **Advanced Metadata Matching** (experimental): Search results prioritize matches based on localized name, non-localized name, categories, and application comments.
- **Favorites Management**: Easily mark or unmark applications as favorites using the star icon, integrated with GSettings for persistent settings.
- **Sorting**: Applications are organized into groups (favorites listed first), with each group sorted alphabetically by name.

![App Launcher](assets/img/shell-app_launcher.png)

<video src="assets/video/shell-app_launcher.mp4" controls></video>

</details>

<details>
<summary>Info Center</summary>

A centralized hub to monitor recent system notifications and view your schedule at a glance. It features:

- **Persistent Notification History**: Unlike standard popups, notifications without a timeout remain accessible in a scrollable list, sorted from newest to oldest with colored borders for quick priority identification (Low: gray, Normal/Default: accent color, Urgent: red).
  - Experimental support for notification actions and icon resolution. 
  - Does not currently support HTML/Pango formatting in notification titles or descriptions.
- **GNOME Calendar Integration**: Synchronized with `gnome-calendar` and connected accounts, providing real-time updates (experimental) and highlighting days containing events.
- **Dynamic Agenda**: Displays event details (title, description, time, and color) directly from the calendar source for both the current day ("Today") and the selected date. The "Today" section appears dynamically when events are present.

![Info Center](assets/img/shell-info_center.png)

<video src="assets/video/shell-info_center.mp4" controls></video>

</details>

<details>
<summary>Notifications</summary>

Quick-access popups for system events and application alerts, designed to work in perfect harmony with the Info Center. It features:

- **Consistent Behavior**: Shares the same properties as the _**Persistent Notification History**_, but includes a 5-second visual fade for notifications without an explicit timeout (does not affect their persistence in the Info Center).
- **Seamless Synchronization**: Fully synced with the Info Center; e.g., dismissing a notification from the popup or the history removes it globally.

![Notifications](assets/img/shell-notifications_popup.png)

<video src="assets/video/shell-notifications.mp4" controls></video>

</details>

<details>
<summary>Power Menu</summary>

A streamlined menu for essential system session management, maintaining a UI/UX consistency with the App Launcher. It features:

- **System-Level Control**: Executes critical actions directly via `loginctl` and `systemctl`, providing reliable handling for `Lock screen`, `Suspend`, `Log-out`, `Reboot`, and `Shutdown`.
- **Consistent Navigation**: A simple, highly accessible interface that can be easily navigated using either the mouse or keyboard.

![Power Menu](assets/img/shell-power_menu.png)

<video src="assets/video/shell-power_menu.mp4" controls></video>

</details>

<details>
<summary>Quick Settings</summary>

A comprehensive suite of quick-access controls for system hardware, connectivity, and personalization. It features:

- **System Status & Power Quick Actions**: Displays battery levels alongside direct access to essential power actions (Lock, Suspend, Logout, etc.) for immediate execution without opening the power menu.
- **Hardware Controls**: Integrated, intuitive sliders for real-time brightness and volume adjustments (Pavucontrol redirection will be replaced by an internal audio manager later).
- **Connectivity & Services**: 
  - **Airplane Mode** (integrated via `rfkill`)
  - **Bluetooth management** (currently integrated via `bluetoothctl`, but to be replaced by an internal bluetooth manager).
  - **Power Mode**: An expandable selector (via `Revealer`) to switch between different power profiles.
- **Personalization & Theming**: 
  - Quick toggle for **Dark Mode**.
  - **Accent Color**: An expandable selector (via `Revealer`) to easily customize the system appearance using Adwaita's accent colors.
- **Focus Management**: A **Do Not Disturb** mode that suppresses intrusive notification popups while ensuring all alerts remain accessible in the Info Center history.

![Quick Settings](assets/img/shell-quick_settings.png)

<video src="assets/video/shell-quick_settings.mp4" controls></video>

</details>

## Roadmap

[//]: # (TODO: Don't forget to update the roadmap)
See [Agwaita's project](https://github.com/users/gouvinb/projects/4)

# Credits

- [GNOME](https://www.gnome.org/) for Adwaita and GTK
- [AGS](https://aylur.github.io/ags/) by Aylur for Ags, Gnim and Astal projects (I dropped the Ags project in favor of Rust, but I want to keep this credit)
- [Overskride](https://github.com/kaii-lb/overskride) by Kaii-lb for the design of her powerful bluetooth client
- [ashell](https://github.com/MalpenZibo/ashell) by MalpenZibo for Tray inspiration and implementation patterns
- [i3status-rust](https://github.com/greshake/i3status-rust) for PipeWire integration inspiration and implementation patterns

# Contributors

- __All contributors who published code that was incorporated into LLMs gemma 3 (27b) and 4 (26b).__

# License

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

> See [LICENSE.md](LICENSE.md)

> [!NOTE]
> 
> This project was initially licensed under Apache License 2.0, but was changed to GPL-3.0 due to dependencies with copyleft licenses that require GPL compatibility.

> [!NOTE]
> 
> According to the indirect contributors listed in the _[Credits](#credits) > [Contributor](#contributor)_ section, some licenses may not be properly complied with, through no fault of my own and without my knowledge; I apologize for this.
