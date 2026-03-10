use gtk4::prelude::*;
use relm4::{
    ComponentParts,
    ComponentSender,
    SimpleComponent,
    gtk,
};

/// BrightnessIcon - Displays screen brightness level
/// Matches the icon resolution logic from AGS TypeScript implementation
pub struct BrightnessIcon {
    level: f64, // 0.0-1.0 (normalized value like AGS)
}

#[derive(Debug)]
pub enum BrightnessIconInput {
    UpdateLevel(f64), // Accepts 0.0-1.0 range
}

#[relm4::component(pub)]
impl SimpleComponent for BrightnessIcon {
    type Init = f64;
    type Input = BrightnessIconInput;
    type Output = ();

    view! {
        #[root]
        gtk::Image {
            set_pixel_size: 16,

            #[watch]
            set_icon_name: Some(Self::resolve_icon(model.level)),
            #[watch]
            set_tooltip_text: Some(&format!("Brightness: {}%", (model.level * 100.0) as u8)),
        }
    }

    fn init(level: Self::Init, root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = BrightnessIcon {
            level: level.clamp(0.0, 1.0),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            BrightnessIconInput::UpdateLevel(level) => {
                let new_level = level.clamp(0.0, 1.0);
                if (self.level - new_level).abs() > 0.01 {
                    self.level = new_level;
                }
            },
        }
    }
}

impl BrightnessIcon {
    /// Resolves the icon name based on brightness value
    /// Matches the AGS implementation:
    /// - < 0.20: night (very dim)
    /// - < 0.40: sunset (dim)
    /// - < 0.60: sunrise (medium)
    /// - >= 0.60: bright (full)
    fn resolve_icon(value: f64) -> &'static str {
        if value < 0.20 {
            "weather-clear-night-symbolic"
        } else if value < 0.40 {
            "daytime-sunset-symbolic"
        } else if value < 0.60 {
            "daytime-sunrise-symbolic"
        } else {
            "display-brightness-symbolic"
        }
    }
}
