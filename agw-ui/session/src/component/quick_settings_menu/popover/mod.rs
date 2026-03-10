// Module structure for popover components
pub mod header;
pub mod sliders;
pub mod toggles;

use crate::component::quick_settings_menu::icons::audio::AudioService;
use agw_service::{
    accent_color::AccentColor,
    airplane_mode::AirplaneModeService,
    brightness::BrightnessService,
    power_mode::{
        PowerModeService,
        PowerProfile,
    },
};
use gtk4::prelude::*;
use header::PopoverHeader;
use relm4::{
    Component,
    ComponentController,
    ComponentParts,
    ComponentSender,
    gtk,
};
use sliders::{
    audio::AudioSlider,
    brightness::BrightnessSlider,
};
use std::sync::Arc;
use toggles::ToggleButtons;

/// QuickSettingsPopover - Main popover content for quick settings
///
/// Structure:
/// - Header: Battery info + system buttons (lock, logout, reboot, shutdown)
/// - Audio slider
/// - Brightness slider
/// - Toggle buttons grid (airplane mode, power mode, dark mode, accent color, DND, bluetooth)
pub struct QuickSettingsPopover {
    header: relm4::Controller<PopoverHeader>,
    audio_slider: relm4::Controller<AudioSlider>,
    brightness_slider: relm4::Controller<BrightnessSlider>,
    toggle_buttons: relm4::Controller<ToggleButtons>,
}

pub enum QuickSettingsPopoverInput {
    UpdateBattery(f64, bool, bool),               // Forward to header
    UpdateAudio(f64, bool),                       // Forward to audio slider (volume, muted)
    UpdateBrightness(f64),                        // Forward to brightness slider
    UpdateAirplaneMode(bool),                     // Forward to toggle buttons
    UpdateDarkMode(bool),                         // Forward to toggle buttons
    UpdateDnd(bool),                              // Forward to toggle buttons
    UpdatePowerProfile(PowerProfile),             // Forward to toggle buttons
    PowerModeServiceReady(Arc<PowerModeService>), // Forward to toggle buttons
    UpdateAccentColor(AccentColor),               // Forward to toggle buttons
    ResetRevealers,                               // Close all revealers when popover closes
}

#[derive(Debug)]
pub enum QuickSettingsPopoverOutput {
    ToggleButtons(toggles::ToggleButtonsOutput),
    ClosePopover,
}

// Custom Debug to handle Arc<PowerModeService>
impl std::fmt::Debug for QuickSettingsPopoverInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpdateBattery(p, c, pr) => f
                .debug_tuple("UpdateBattery")
                .field(p)
                .field(c)
                .field(pr)
                .finish(),
            Self::UpdateAudio(v, m) => f.debug_tuple("UpdateAudio").field(v).field(m).finish(),
            Self::UpdateBrightness(l) => f.debug_tuple("UpdateBrightness").field(l).finish(),
            Self::UpdateAirplaneMode(v) => f.debug_tuple("UpdateAirplaneMode").field(v).finish(),
            Self::UpdateDarkMode(v) => f.debug_tuple("UpdateDarkMode").field(v).finish(),
            Self::UpdateDnd(v) => f.debug_tuple("UpdateDnd").field(v).finish(),
            Self::UpdatePowerProfile(p) => f.debug_tuple("UpdatePowerProfile").field(p).finish(),
            Self::PowerModeServiceReady(_) => write!(f, "PowerModeServiceReady(...)"),
            Self::UpdateAccentColor(c) => f.debug_tuple("UpdateAccentColor").field(c).finish(),
            Self::ResetRevealers => write!(f, "ResetRevealers"),
        }
    }
}

#[relm4::component(pub)]
impl Component for QuickSettingsPopover {
    type Init = (
        (f64, bool, bool),
        Arc<AudioService>,
        Arc<BrightnessService>,
        Option<Arc<AirplaneModeService>>,
        Option<Arc<PowerModeService>>,
    ); // (battery state, audio service, brightness service, airplane mode service, power mode service)
    type Input = QuickSettingsPopoverInput;
    type Output = QuickSettingsPopoverOutput;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_hexpand: true,

            // Header with battery and system buttons
            model.header.widget().clone(),

            // Audio slider
            gtk::Box {
                model.audio_slider.widget().clone(),
            },

            // Brightness slider
            gtk::Box {
                model.brightness_slider.widget().clone(),
            },

            // Toggle buttons grid
            gtk::Box {
                model.toggle_buttons.widget().clone(),
            },
        },
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let (battery_state, audio_service, brightness_service, airplane_mode_service, power_mode_service) = init;

        // Initialize header with battery state
        let header = PopoverHeader::builder()
            .launch(battery_state)
            .forward(sender.output_sender(), |_| {
                QuickSettingsPopoverOutput::ClosePopover
            });

        // Initialize audio slider with service
        let audio_slider = AudioSlider::builder()
            .launch(audio_service)
            .forward(sender.input_sender(), |_| unreachable!());

        // Initialize brightness slider with service
        let brightness_slider = BrightnessSlider::builder()
            .launch(brightness_service)
            .forward(sender.input_sender(), |_| unreachable!());

        // Initialize toggle buttons with services
        let toggle_buttons = ToggleButtons::builder()
            .launch((airplane_mode_service, power_mode_service))
            .forward(
                sender.output_sender(),
                QuickSettingsPopoverOutput::ToggleButtons,
            );

        let model = QuickSettingsPopover {
            header,
            audio_slider,
            brightness_slider,
            toggle_buttons,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, #[allow(unused_variables)] sender: ComponentSender<Self>, #[allow(unused_variables)] root: &Self::Root) {
        use header::PopoverHeaderInput;
        use sliders::{
            audio::AudioSliderInput,
            brightness::BrightnessSliderInput,
        };
        use toggles::ToggleButtonsInput;

        match message {
            QuickSettingsPopoverInput::UpdateBattery(percentage, is_charging, is_present) => {
                self.header.emit(PopoverHeaderInput::UpdateBattery(
                    percentage,
                    is_charging,
                    is_present,
                ));
            },
            QuickSettingsPopoverInput::UpdateAudio(volume, muted) => {
                self.audio_slider
                    .emit(AudioSliderInput::UpdateVolume(volume));
                self.audio_slider.emit(AudioSliderInput::UpdateMuted(muted));
            },
            QuickSettingsPopoverInput::UpdateBrightness(brightness) => {
                self.brightness_slider
                    .emit(BrightnessSliderInput::UpdateLevel(brightness));
            },
            QuickSettingsPopoverInput::UpdateAirplaneMode(enabled) => {
                self.toggle_buttons
                    .emit(ToggleButtonsInput::UpdateAirplaneMode(enabled));
            },
            QuickSettingsPopoverInput::UpdateDarkMode(enabled) => {
                self.toggle_buttons
                    .emit(ToggleButtonsInput::UpdateDarkMode(enabled));
            },
            QuickSettingsPopoverInput::UpdateDnd(enabled) => {
                self.toggle_buttons
                    .emit(ToggleButtonsInput::UpdateDnd(enabled));
            },
            QuickSettingsPopoverInput::UpdatePowerProfile(profile) => {
                self.toggle_buttons
                    .emit(ToggleButtonsInput::UpdatePowerProfile(profile));
            },
            QuickSettingsPopoverInput::PowerModeServiceReady(service) => {
                self.toggle_buttons
                    .emit(ToggleButtonsInput::PowerModeServiceReady(service));
            },
            QuickSettingsPopoverInput::UpdateAccentColor(color) => {
                self.toggle_buttons
                    .emit(ToggleButtonsInput::UpdateAccentColor(color));
            },
            QuickSettingsPopoverInput::ResetRevealers => {
                self.toggle_buttons.emit(ToggleButtonsInput::ResetRevealers);
            },
        }
    }
}
