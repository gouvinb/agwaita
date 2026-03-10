use gtk4::prelude::*;
use log::error;
use relm4::{
    ComponentParts,
    ComponentSender,
    SimpleComponent,
    gtk,
};

/// AudioIcon - Displays audio volume level
/// Matches the AGS TypeScript implementation with volume ranges
pub struct AudioIcon {
    volume: f64, // 0.0-1.5 (supports overamplification)
    muted: bool,
}

#[derive(Debug)]
pub enum AudioIconInput {
    UpdateVolume(f64), // 0.0-1.0
    UpdateMuted(bool),
}

#[relm4::component(pub)]
impl SimpleComponent for AudioIcon {
    type Init = (f64, bool);
    type Input = AudioIconInput;
    type Output = ();

    view! {
        #[root]
        gtk::Image {
            #[watch]
            set_icon_name: Some(Self::resolve_icon(model.volume, model.muted)),
            set_pixel_size: 16,
            #[watch]
            set_tooltip_text: Some(&Self::get_tooltip(model.volume, model.muted)),
        }
    }

    fn init(init: Self::Init, root: Self::Root, #[allow(unused_variables)] sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = AudioIcon {
            volume: init.0.clamp(0.0, 1.5),
            muted: init.1,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, #[allow(unused_variables)] sender: ComponentSender<Self>) {
        match message {
            AudioIconInput::UpdateVolume(volume) => {
                self.volume = volume.clamp(0.0, 1.5);
            },
            AudioIconInput::UpdateMuted(muted) => {
                self.muted = muted;
            },
        }
    }
}

impl AudioIcon {
    /// Resolves the icon name based on volume and mute status
    /// Matches the AGS implementation:
    /// - muted or 0%: muted
    /// - < 34%: low
    /// - < 67%: medium
    /// - <= 100%: high
    /// - > 100%: overamplified
    fn resolve_icon(volume: f64, muted: bool) -> &'static str {
        if muted {
            return "audio-volume-muted-symbolic";
        }

        let volume_percent = volume * 100.0;

        match volume_percent {
            0.0 => "audio-volume-muted-symbolic",
            0.0..=33.3 => "audio-volume-low-symbolic",
            33.3..=66.6 => "audio-volume-medium-symbolic",
            66.6..=100.0 => "audio-volume-high-symbolic",
            100.0..=f64::MAX => "audio-volume-overamplified-symbolic",
            _ => {
                error!("Invalid volume value: {}", volume_percent);
                "audio-volume-muted-symbolic"
            },
        }
    }

    fn get_tooltip(volume: f64, muted: bool) -> String {
        if muted {
            "Volume: Muted".to_string()
        } else {
            format!("Volume: {}%", (volume * 100.0) as u8)
        }
    }
}
