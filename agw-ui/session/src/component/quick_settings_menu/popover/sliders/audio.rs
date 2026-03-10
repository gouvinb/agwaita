use crate::component::quick_settings_menu::icons::audio::{
    AudioIcon,
    AudioService,
};
use gtk4::prelude::*;
use log::debug;
use relm4::{
    Component,
    ComponentController,
    ComponentParts,
    ComponentSender,
    gtk,
};
use std::sync::Arc;

/// AudioSlider - Volume control slider with icon
///
/// Reuses AudioIcon and AudioService from icons module
pub struct AudioSlider {
    audio_icon: relm4::Controller<AudioIcon>,
    audio_service: Arc<AudioService>,
    volume: f64,
    muted: bool,
}

#[derive(Debug)]
pub enum AudioSliderInput {
    UpdateVolume(f64),
    UpdateMuted(bool),
    VolumeChanged(f64),
    ToggleMute,
}

#[relm4::component(pub)]
impl Component for AudioSlider {
    type Init = Arc<AudioService>;
    type Input = AudioSliderInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 8,
            set_hexpand: true,

            // Mute button with icon
            gtk::Button {
                model.audio_icon.widget().clone(),

                connect_clicked[sender] => move |_| {
                    sender.input(AudioSliderInput::ToggleMute);
                },
            },

            // Volume slider (0-150%)
            #[name = "volume_scale"]
            gtk::Scale {
                set_hexpand: true,
                set_draw_value: false,
                set_range: (0.0, 1.5),
                set_increments: (0.01, 0.1),

                add_mark: (1.0, gtk::PositionType::Bottom, Some("")),
                add_mark: (1.5, gtk::PositionType::Bottom, Some("")),

                connect_value_changed[sender] => move |scale| {
                    sender.input(AudioSliderInput::VolumeChanged(scale.value()));
                },

                #[watch]
                set_value: model.volume,
            },

            // Percentage label (can show >100%)
            gtk::Label {
                set_width_chars: 4,

                #[watch]
                set_label: &format!("{}%", (model.volume * 100.0).round() as i32),
            },

            // Settings button to open pavucontrol
            gtk::Button {
                set_icon_name: "preferences-other-symbolic",
                connect_clicked => move |_| {
                    let _ = std::process::Command::new("pavucontrol").spawn();
                },
            },
        }
    }

    fn init(service: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        // Get initial state from service
        let volume = service.get_volume();
        let muted = service.is_muted();

        // Initialize audio icon
        let audio_icon = AudioIcon::builder()
            .launch((volume, muted))
            .forward(sender.input_sender(), |_| unreachable!());

        let model = AudioSlider {
            audio_icon,
            audio_service: service,
            volume,
            muted,
        };

        let widgets = view_output!();

        // Désactiver complètement le scroll sur le slider pour éviter les boucles infinies
        let scroll_controller = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll_controller.connect_scroll(|_, _, _| gtk::glib::Propagation::Stop);
        widgets.volume_scale.add_controller(scroll_controller);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, #[allow(unused_variables)] sender: ComponentSender<Self>, #[allow(unused_variables)] root: &Self::Root) {
        use crate::component::quick_settings_menu::icons::audio::AudioIconInput;

        match message {
            AudioSliderInput::UpdateVolume(volume) => {
                // Update from system
                debug!("UpdateVolume from system: {:.2}", volume);
                self.volume = volume;
                self.audio_icon.emit(AudioIconInput::UpdateVolume(volume));
            },
            AudioSliderInput::UpdateMuted(muted) => {
                self.muted = muted;
                self.audio_icon.emit(AudioIconInput::UpdateMuted(muted));
            },
            AudioSliderInput::VolumeChanged(volume) => {
                // Only update if the percentage value changed (avoid spam)
                let old_percent = (self.volume * 100.0) as i32;
                let new_percent = (volume * 100.0) as i32;

                self.volume = volume;
                self.audio_icon.emit(AudioIconInput::UpdateVolume(volume));

                if old_percent != new_percent {
                    self.audio_service.set_volume(volume);
                }
            },
            AudioSliderInput::ToggleMute => {
                self.audio_service.toggle_mute();
                // The state will be updated via SystemStateUpdate
            },
        }
    }
}
