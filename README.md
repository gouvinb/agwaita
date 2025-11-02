# Agwaita

A GTK Shell configuration that combines Adwaita and AGS.

## Dependencies

- [Ags](https://aylur.github.io/ags/)
- [Niri](https://yalter.github.io/niri/) (because it's my main WM at the moment)
- glib2 for using gsettings
- pavucontrol (because I haven't yet developed the part for managing sound/microphone devices)
- [overskride](https://github.com/kaii-lb/overskride) (because I haven't developed the Bluetooth device management part yet)
- kvantum with kvantum-qt5 and kvantum-theme-libadwaita for using kvantummanager
- swaylock and a preparation script `$XDG_LIB_HOME/desktop-scripts/prelock` (the latter is not published because it is only used to take a screenshot and pixelate it for swaylock, and can be replaced by anything else)
- systemd for using:
    - systemctl
    - loginctl
    - systemd-networkd (because I never liked NetworkManager)
- nushell for the make.nu file

## Installation

1. Install the required dependencies
2. Clone this repository
3. Put in the PATH: XDG_BIN_HOME or the `$HOME/.local/bin` folder
4. At the root of the project:
    - Run the command `./make.nu -h` to see your options
    - Run the command `./make.nu -h` to see your options
    - Run the command `./make.nu hotrun` to test ags-shell without installation
    -  Run the command `./make.nu install` to install ags-shell in the $XDG_BIN_HOME folder or the `$HOME/.local/bin` folder, which allows you to access the `ags-shell` command in your PATH

## Preview

[demo.mp4](assets/demo.mp4)

### Notifications system

![Notifications.png](assets/Notifications.png)

### Status bar
![status-bar.png](assets/status-bar.png)

### Notifications center

![status-bar_notifications-center-empty.png](assets/status-bar_notifications-center-empty.png)
![status-bar_notifications-center-nonempty.png](assets/status-bar_notifications-center-nonempty.png)

### Quick settings

![status-bar_quick-settings.png](assets/status-bar_quick-settings.png)

## Roadmap

- [-] Status bar
- [ ] Application launcher
- [x] Notification system

### Status Bar

#### Left side

- [-] A component for displaying and managing Niri, Sway, and Hyprland workspaces (Niri prioritized)

#### Center

- [-] A component to display the date and time with subcomponents
    - [x] A notification center on the left
    - [x] `Gtk.Calendar` on the right
    - [x] Displays the remaining events of the day below `Gtk.Calendar`

#### Right side

- [x] Systemd unit fail
- [x] Systray with AstalTray
- [-] A button consisting of several icons representing different parts of the OS status.
    - [-] Icons :
        - [ ] privacy (I had a very specific idea when writing this line, but I've forgotten it since)
        - [x] brightness
        - [x] volume
        - [x] bluetooth
        - [x] network
        - [x] battery
        - [x] power mode
        - [x] Avatar
    - [-] This button will open a submenu to display components for interacting with various parts of the system
        - [x] Several buttons for
            - [x] Lock the session
            - [x] Session ended
            - [x] Restart the PC
            - [x] Turn off the PC
        - [x] A slider for brightness
        - [-] A slider for audio with a dropdown menu to select the default sink
            - [x] Slider
            - [-] Sink (for now, ags-shell redirects to pavucontrol)
        - [x] A toggle button for airplane mode
        - [x] A button for Power mode with a dropdown menu to choose the power supply mode
            - [x] Button
            - [x] Dropdown
        - [x] A button for dark mode
        - [x] An accent color button with a dropdown menu to choose one of the colors supported by Adwaita
            - [x] Button
            - [x] Dropdown
        - [x] A button for do not disturb mode
        - [-] A button for Bluetooth with a dropdown menu to manages devices
            - [x] Button
            - [-] Dropdown (for now, ags-shell redirects to overskride)

### Application launcher

- [ ] Application grid sorted alphabetically (locale considered)
- [ ] Search bar

### Notification system

- [x] Takes into account the DontDisturb function
- [x] Scrollable
- [x] Multimonitor
- [x] Priority Management
- [x] Action management
- [x] Request handler to interact with notifications

# License

Copyright 2025 Gouvinb

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

> See [LICENSE.md](LICENSE.md)
