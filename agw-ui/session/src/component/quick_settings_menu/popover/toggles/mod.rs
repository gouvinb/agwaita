pub mod dnd;
pub mod power_mode;

use agw_service::{
    accent_color::{
        AccentColor,
        AccentColorService,
    },
    airplane_mode::AirplaneModeService,
    dark_mode::DarkModeService,
    power_mode::PowerProfile,
};
use dnd::DndToggleService;
use gtk4::prelude::*;
use relm4::{
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    SimpleComponent,
    gtk,
};
use std::{
    process::Command,
    sync::Arc,
};

/// ToggleButtons - Grid of toggle buttons for quick settings
///
/// Layout with revealers:
/// Row 1: [AirplaneMode] [PowerMode]
///        PowerMode Revealer (spans 2 columns)
/// Row 2: [DarkMode] [AccentColor]
///        AccentColor Revealer (spans 2 columns)
/// Row 3: [DoNotDisturb] [Bluetooth]
pub struct ToggleButtons {
    airplane_mode_service: Option<Arc<AirplaneModeService>>,
    airplane_mode_enabled: bool,
    dark_mode_enabled: bool,
    dnd_service: DndToggleService,
    dnd_enabled: bool,
    power_mode_expanded: bool,
    power_mode_service: Option<Arc<power_mode::PowerModeService>>,
    power_profile: PowerProfile,
    accent_color_expanded: bool,
    accent_color: AccentColor,
}

impl ToggleButtons {
    fn is_active(&self, accent_color: AccentColor) -> bool {
        accent_color == self.accent_color
    }

    fn opacity(&self, accent_color: AccentColor) -> f64 {
        if self.is_active(accent_color) {
            1.0
        } else {
            0.5
        }
    }
}

pub enum ToggleButtonsInput {
    ToggleAirplaneMode,
    ToggleDarkMode,
    ToggleDnd,
    TogglePowerModeRevealer,
    SelectPowerProfile(power_mode::PowerProfile),
    ToggleAccentColorRevealer,
    SelectAccentColor(AccentColor),
    UpdateAirplaneMode(bool),
    UpdateDarkMode(bool),
    UpdateDnd(bool),
    UpdatePowerProfile(power_mode::PowerProfile),
    UpdateAccentColor(AccentColor),
    PowerModeServiceReady(Arc<power_mode::PowerModeService>),
    ResetRevealers, // Close all revealers
}

// Custom Debug to handle Arc<PowerModeService>
impl std::fmt::Debug for ToggleButtonsInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToggleAirplaneMode => write!(f, "ToggleAirplaneMode"),
            Self::ToggleDarkMode => write!(f, "ToggleDarkMode"),
            Self::ToggleDnd => write!(f, "ToggleDnd"),
            Self::TogglePowerModeRevealer => write!(f, "TogglePowerModeRevealer"),
            Self::SelectPowerProfile(p) => write!(f, "SelectPowerProfile({:?})", p),
            Self::ToggleAccentColorRevealer => write!(f, "ToggleAccentColorRevealer"),
            Self::SelectAccentColor(c) => write!(f, "SelectAccentColor({:?})", c),
            Self::UpdateAirplaneMode(v) => write!(f, "UpdateAirplaneMode({})", v),
            Self::UpdateDarkMode(v) => write!(f, "UpdateDarkMode({})", v),
            Self::UpdateDnd(v) => write!(f, "UpdateDnd({})", v),
            Self::UpdatePowerProfile(p) => write!(f, "UpdatePowerProfile({:?})", p),
            Self::UpdateAccentColor(c) => write!(f, "UpdateAccentColor({:?})", c),
            Self::PowerModeServiceReady(_) => write!(f, "PowerModeServiceReady(...)"),
            Self::ResetRevealers => write!(f, "ResetRevealers"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ToggleButtonsOutput {
    PowerProfileChanged(power_mode::PowerProfile),
    ClosePopover,
}

#[relm4::component(pub)]
impl SimpleComponent for ToggleButtons {
    type Init = (
        Option<Arc<AirplaneModeService>>,
        Option<Arc<power_mode::PowerModeService>>,
    );
    type Input = ToggleButtonsInput;
    type Output = ToggleButtonsOutput;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 4,
            set_hexpand: true,

            // Row 1: Airplane Mode + Power Mode
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                set_homogeneous: true,

                #[name = "airplane_mode_button"]
                gtk::ToggleButton {
                    set_hexpand: true,

                    #[watch]
                    set_active: model.airplane_mode_enabled,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

                        gtk::Image {
                            #[watch]
                            set_icon_name: match model.airplane_mode_enabled {
                                true => Some("airplane-mode-symbolic"),
                                false => Some("airplane-mode-disabled-symbolic"),
                            },
                        },
                        gtk::Label {
                            set_label: "Airplane Mode",
                        }
                    }
                },
                #[name = "power_mode_button"]
                gtk::ToggleButton {
                    set_hexpand: true,

                    #[watch]
                    set_active: model.power_mode_expanded,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

                        gtk::Image {
                            #[watch]
                            set_icon_name: match model.power_profile {
                                PowerProfile::PowerSaver => Some("power-profile-power-saver-symbolic"),
                                PowerProfile::Balanced => Some("power-profile-balanced-symbolic"),
                                PowerProfile::Performance => Some("power-profile-performance-symbolic"),
                            },
                        },
                        gtk::Label {
                            set_hexpand: true,
                            set_label: "Power Mode",
                            set_align: gtk::Align::Start,
                        },
                        gtk::Image {
                            #[watch]
                            set_icon_name: match model.power_mode_expanded {
                                true => Some("pan-down-symbolic"),
                                false => Some("pan-end-symbolic"),
                            },
                        },
                    }
                },
            },

            // Power Mode Revealer (spans full width)
            #[name = "power_mode_revealer"]
            gtk::Revealer {
                set_transition_type: gtk::RevealerTransitionType::SlideDown,
                #[watch]
                set_reveal_child: model.power_mode_expanded,

                gtk::Box {
                    add_css_class: "card",
                    inline_css: "padding: 8px;",
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_margin_vertical: 4,

                    gtk::Box {
                        set_margin_horizontal: 8,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 16,

                        gtk::Image {
                            set_icon_name: Some("resources-symbolic"),
                            set_icon_size: gtk::IconSize::Large,
                        },
                        gtk::Label {
                            add_css_class: "title-1",
                            set_label: "Power Mode",
                        },
                    },

                    #[name = "power_saver_button"]
                    gtk::ToggleButton {
                        set_label: "Power Saver",
                        #[watch]
                        set_active: model.power_profile == PowerProfile::PowerSaver,
                    },

                    #[name = "balanced_button"]
                    gtk::ToggleButton {
                        set_label: "Balanced",
                        #[watch]
                        set_active: model.power_profile == PowerProfile::Balanced,
                    },

                    #[name = "performance_button"]
                    gtk::ToggleButton {
                        set_label: "Performance",
                        #[watch]
                        set_active: model.power_profile == PowerProfile::Performance,
                    },
                },
            },

            // Row 2: Dark Mode + Accent Color
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                set_homogeneous: true,

                #[name = "dark_mode_button"]
                gtk::ToggleButton {
                    set_hexpand: true,

                    #[watch]
                    set_active: model.dark_mode_enabled,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

                        gtk::Image {
                            #[watch]
                            set_icon_name: match model.dark_mode_enabled {
                                true => Some("night-light-symbolic"),
                                false => Some("night-light-disabled-symbolic"),
                            },
                        },
                        gtk::Label {
                            set_label: "Dark Mode",
                        }
                    }
                },
                #[name = "accent_color_button"]
                gtk::ToggleButton {
                    set_label: "Accent Color",
                    set_hexpand: true,

                    #[watch]
                    set_active: model.accent_color_expanded,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

                        gtk::Image {
                            inline_css: "color: var(--accent-color);",
                            set_icon_name: Some("org.gnome.Settings-color-symbolic"),
                        },
                        gtk::Label {
                            set_hexpand: true,
                            set_label: "Accent Color",
                            set_align: gtk::Align::Start,
                        },
                        gtk::Image {
                            #[watch]
                            set_icon_name: match model.accent_color_expanded {
                                true => Some("pan-down-symbolic"),
                                false => Some("pan-end-symbolic"),
                            },
                        },
                    }

                },
            },

            // Accent Color Revealer (spans full width)
            #[name = "accent_color_revealer"]
            gtk::Revealer {
                set_transition_type: gtk::RevealerTransitionType::SlideDown,
                #[watch]
                set_reveal_child: model.accent_color_expanded,

                gtk::Box {
                    add_css_class: "card",
                    inline_css: "padding: 8px;",
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_margin_vertical: 4,

                    gtk::Box {
                        set_margin_horizontal: 8,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 16,

                        gtk::Image {
                            inline_css: "color: var(--accent-color);",
                            set_icon_name: Some("org.gnome.Settings-color-symbolic"),
                            set_icon_size: gtk::IconSize::Large,
                        },
                        gtk::Label {
                            add_css_class: "title-1",
                            set_label: "Accent Color",
                        },
                    },

                    gtk::FlowBox {
                        set_column_spacing: 4,
                        set_row_spacing: 4,
                        set_homogeneous: true,
                        set_selection_mode: gtk::SelectionMode::None,
                        set_max_children_per_line: 7,

                        #[name = "blue_button"]
                        gtk::ToggleButton {
                            inline_css: "background-color: var(--accent-blue);",

                            #[watch]
                            set_active: model.is_active(AccentColor::Blue),
                            #[watch]
                            set_opacity: model.opacity(AccentColor::Blue),
                        },

                        #[name = "teal_button"]
                        gtk::ToggleButton {
                            inline_css: "background-color: var(--accent-teal);",

                            #[watch]
                            set_active: model.is_active(AccentColor::Teal),
                            #[watch]
                            set_opacity: model.opacity(AccentColor::Teal),
                        },

                        #[name = "green_button"]
                        gtk::ToggleButton {
                            inline_css: "background-color: var(--accent-green);",

                            #[watch]
                            set_active: model.is_active(AccentColor::Green),
                            #[watch]
                            set_opacity: model.opacity(AccentColor::Green),
                        },

                        #[name = "yellow_button"]
                        gtk::ToggleButton {
                            inline_css: "background-color: var(--accent-yellow);",

                            #[watch]
                            set_active: model.is_active(AccentColor::Yellow),
                            #[watch]
                            set_opacity: model.opacity(AccentColor::Yellow),
                        },

                        #[name = "orange_button"]
                        gtk::ToggleButton {
                            inline_css: "background-color: var(--accent-orange);",

                            #[watch]
                            set_active: model.is_active(AccentColor::Orange),
                            #[watch]
                            set_opacity: model.opacity(AccentColor::Orange),
                        },

                        #[name = "red_button"]
                        gtk::ToggleButton {
                            inline_css: "background-color: var(--accent-red);",

                            #[watch]
                            set_active: model.is_active(AccentColor::Red),
                            #[watch]
                            set_opacity: model.opacity(AccentColor::Red),
                        },

                        #[name = "pink_button"]
                        gtk::ToggleButton {
                            inline_css: "background-color: var(--accent-pink);",

                            #[watch]
                            set_active: model.is_active(AccentColor::Pink),
                            #[watch]
                            set_opacity: model.opacity(AccentColor::Pink),
                        },

                        #[name = "purple_button"]
                        gtk::ToggleButton {
                            inline_css: "background-color: var(--accent-purple);",

                            #[watch]
                            set_active: model.is_active(AccentColor::Purple),
                            #[watch]
                            set_opacity: model.opacity(AccentColor::Purple),
                        },

                        #[name = "slate_button"]
                        gtk::ToggleButton {
                            inline_css: "background-color: var(--accent-slate);",

                            #[watch]
                            set_active: model.is_active(AccentColor::Slate),
                            #[watch]
                            set_opacity: model.opacity(AccentColor::Slate),
                        },
                    },

                }
            },

            // Row 3: Do Not Disturb + Bluetooth
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                set_homogeneous: true,

                #[name = "dnd_button"]
                gtk::ToggleButton {
                    set_hexpand: true,

                    #[watch]
                    set_active: model.dnd_enabled,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

                        gtk::Image {
                            #[watch]
                            set_icon_name: match model.dnd_enabled {
                                true => Some("notifications-disabled-symbolic"),
                                false => Some("org.gnome.Settings-notifications-symbolic"),
                            },
                        },
                        gtk::Label {
                            set_label: "Do Not Disturb",
                        }
                    }
                },

                #[name = "bluetooth_button"]
                gtk::Button {
                    set_label: "Bluetooth",
                    set_hexpand: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

                        gtk::Image {
                            set_icon_name: Some("org.gnome.Settings-bluetooth-symbolic"),
                        },
                        gtk::Label {
                            set_label: "Bluetooth",
                        }
                    }
                },
            },
        }
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let (airplane_mode_service, power_mode_service) = init;

        let dnd_service = DndToggleService::new();
        let dnd_enabled = dnd_service.is_enabled();

        // Read airplane mode state - use service if available, otherwise check directly
        let airplane_mode_enabled = airplane_mode_service
            .as_ref()
            .map(|s| s.is_enabled())
            .unwrap_or_else(|| AirplaneModeService::is_enabled_static());

        // Read initial power profile from service
        let power_profile = power_mode_service
            .as_ref()
            .map(|s| s.get_active_profile())
            .unwrap_or(PowerProfile::Balanced);

        // Read initial accent color
        let accent_color = AccentColorService::get_accent_color();

        let model = ToggleButtons {
            airplane_mode_service,
            airplane_mode_enabled,
            dark_mode_enabled: DarkModeService::is_enabled(),
            dnd_service,
            dnd_enabled,
            power_mode_expanded: false,
            power_mode_service,
            power_profile,
            accent_color_expanded: false,
            accent_color,
        };
        let widgets = view_output!();

        // Connect power profile button signals
        let sender_clone = sender.input_sender().clone();
        widgets.power_saver_button.connect_clicked(move |button| {
            if button.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectPowerProfile(
                        PowerProfile::PowerSaver,
                    ))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.balanced_button.connect_clicked(move |button| {
            if button.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectPowerProfile(
                        PowerProfile::Balanced,
                    ))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.performance_button.connect_clicked(move |button| {
            if button.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectPowerProfile(
                        PowerProfile::Performance,
                    ))
                    .ok();
            }
        });

        // Connect button signals
        let sender_clone = sender.input_sender().clone();
        widgets.airplane_mode_button.connect_clicked(move |_| {
            sender_clone
                .send(ToggleButtonsInput::ToggleAirplaneMode)
                .ok();
        });

        let sender_clone = sender.input_sender().clone();
        widgets.dark_mode_button.connect_clicked(move |_| {
            sender_clone.send(ToggleButtonsInput::ToggleDarkMode).ok();
        });

        let sender_clone = sender.input_sender().clone();
        widgets.dnd_button.connect_clicked(move |_| {
            sender_clone.send(ToggleButtonsInput::ToggleDnd).ok();
        });

        let sender_clone = sender.input_sender().clone();
        widgets.power_mode_button.connect_clicked(move |_| {
            sender_clone
                .send(ToggleButtonsInput::TogglePowerModeRevealer)
                .ok();
        });

        let sender_clone = sender.input_sender().clone();
        widgets.accent_color_button.connect_clicked(move |_| {
            sender_clone
                .send(ToggleButtonsInput::ToggleAccentColorRevealer)
                .ok();
        });

        // Connect accent color button signals
        let sender_clone = sender.input_sender().clone();
        widgets.blue_button.connect_clicked(move |btn| {
            if btn.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectAccentColor(AccentColor::Blue))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.teal_button.connect_clicked(move |btn| {
            if btn.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectAccentColor(AccentColor::Teal))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.green_button.connect_clicked(move |btn| {
            if btn.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectAccentColor(AccentColor::Green))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.yellow_button.connect_clicked(move |btn| {
            if btn.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectAccentColor(AccentColor::Yellow))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.orange_button.connect_clicked(move |btn| {
            if btn.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectAccentColor(AccentColor::Orange))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.red_button.connect_clicked(move |btn| {
            if btn.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectAccentColor(AccentColor::Red))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.pink_button.connect_clicked(move |btn| {
            if btn.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectAccentColor(AccentColor::Pink))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.purple_button.connect_clicked(move |btn| {
            if btn.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectAccentColor(AccentColor::Purple))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.slate_button.connect_clicked(move |btn| {
            if btn.is_active() {
                sender_clone
                    .send(ToggleButtonsInput::SelectAccentColor(AccentColor::Slate))
                    .ok();
            }
        });

        // Connect bluetooth button signal
        let output_sender = sender.output_sender().clone();
        widgets.bluetooth_button.connect_clicked(move |_| {
            // Launch bluetoothctl
            Command::new("niri")
                .args(&[
                    "msg",
                    "-j",
                    "action",
                    "spawn-sh",
                    "--",
                    "/usr/bin/ghostty --gtk-single-instance=true -e bluetoothctl",
                ])
                .output()
                .ok();
            // Request popover to close
            output_sender.send(ToggleButtonsOutput::ClosePopover).ok();
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ToggleButtonsInput::ToggleAirplaneMode => {
                if let Some(ref service) = self.airplane_mode_service {
                    if let Err(e) = service.toggle() {
                        log::error!("Failed to toggle airplane mode: {}", e);
                        return;
                    }
                    self.airplane_mode_enabled = service.is_enabled();
                } else {
                    // Fallback: create temporary service instance
                    let service = AirplaneModeService::new();
                    if let Err(e) = service.toggle() {
                        log::error!("Failed to toggle airplane mode: {}", e);
                        return;
                    }
                    self.airplane_mode_enabled = service.is_enabled();
                }
            },
            ToggleButtonsInput::ToggleDarkMode => {
                if let Err(e) = DarkModeService::toggle() {
                    log::error!("Failed to toggle dark mode: {}", e);
                    return;
                }
                self.dark_mode_enabled = DarkModeService::is_enabled();
            },
            ToggleButtonsInput::ToggleDnd => {
                self.dnd_service.toggle();
                self.dnd_enabled = self.dnd_service.is_enabled();
            },
            ToggleButtonsInput::TogglePowerModeRevealer => {
                self.power_mode_expanded = !self.power_mode_expanded;
            },
            ToggleButtonsInput::SelectPowerProfile(profile) => {
                // User selected a power profile - change it via service
                if let Some(ref service) = self.power_mode_service {
                    let service_clone = Arc::clone(service);
                    let profile_clone = profile.clone();
                    gtk4::glib::MainContext::default().spawn_local(async move {
                        if let Err(e) = service_clone.set_active_profile(profile_clone).await {
                            log::error!("Failed to set power profile: {}", e);
                        }
                    });
                } else {
                    log::warn!("PowerModeService not available, cannot change profile");
                }
                // Update local state immediately for responsiveness
                self.power_profile = profile.clone();
                // Notify parent component of the change
                sender
                    .output(ToggleButtonsOutput::PowerProfileChanged(profile))
                    .ok();
            },
            ToggleButtonsInput::ToggleAccentColorRevealer => {
                self.accent_color_expanded = !self.accent_color_expanded;
            },
            ToggleButtonsInput::SelectAccentColor(color) => {
                // User selected an accent color
                if let Err(e) = AccentColorService::set_accent_color(color) {
                    log::error!("Failed to set accent color: {}", e);
                    return;
                }
                // Update local state immediately for responsiveness
                self.accent_color = color;
            },
            ToggleButtonsInput::UpdateAirplaneMode(enabled) => {
                self.airplane_mode_enabled = enabled;
            },
            ToggleButtonsInput::UpdateDarkMode(enabled) => {
                self.dark_mode_enabled = enabled;
            },
            ToggleButtonsInput::UpdateDnd(enabled) => {
                self.dnd_enabled = enabled;
            },
            ToggleButtonsInput::UpdateAccentColor(color) => {
                // External update from monitor
                self.accent_color = color;
            },
            ToggleButtonsInput::UpdatePowerProfile(profile) => {
                // External update from monitor
                self.power_profile = profile;
            },
            ToggleButtonsInput::PowerModeServiceReady(service) => {
                // Power mode service is now ready
                log::info!("PowerModeService is now ready (received from GlobalSystemService)");
                self.power_mode_service = Some(service.clone());
                // Update current profile from the service
                self.power_profile = service.get_active_profile();
            },
            ToggleButtonsInput::ResetRevealers => {
                // Close all revealers when popover closes
                self.power_mode_expanded = false;
                self.accent_color_expanded = false;
            },
        }
    }
}
