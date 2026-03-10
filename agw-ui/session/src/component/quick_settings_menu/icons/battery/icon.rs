use gtk4::prelude::{
    BoxExt,
    WidgetExt,
};
use relm4::{
    Component,
    ComponentParts,
    ComponentSender,
    gtk,
};

#[derive(Debug)]
pub struct BatteryIcon {
    percentage: f64,
    is_charging: bool,
    is_present: bool,
}

#[derive(Debug)]
pub enum BatteryIconInput {
    UpdateBattery(f64, bool, bool), // percentage, is_charging, is_present
}

#[relm4::component(pub)]
impl Component for BatteryIcon {
    type Init = (f64, bool, bool);
    type Input = BatteryIconInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_spacing: 4,

            gtk::Image {
                set_pixel_size: 16,

                #[watch]
                set_icon_name: Some(Self::resolve_icon(model.percentage, model.is_charging, model.is_present)),
                #[watch]
                set_tooltip_text: Some(&Self::get_tooltip(model.percentage, model.is_charging, model.is_present)),
            },

            gtk::Label {
                #[watch]
                set_label: &Self::get_percentage_label(model.percentage, model.is_present),
            },
        }
    }

    fn init(init: Self::Init, root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let (percentage, is_charging, is_present) = init;

        let model = BatteryIcon {
            percentage,
            is_charging,
            is_present,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            BatteryIconInput::UpdateBattery(percentage, is_charging, is_present) => {
                self.percentage = percentage;
                self.is_charging = is_charging;
                self.is_present = is_present;
            },
        }
    }
}

impl BatteryIcon {
    /// Resolve icon name based on battery state
    /// Matches the AGS TypeScript implementation
    fn resolve_icon(percentage: f64, is_charging: bool, is_present: bool) -> &'static str {
        if !is_present {
            return "battery-missing-symbolic";
        }

        let percent = (percentage * 100.0).round() as u8;

        if is_charging {
            if percent >= 100 {
                "battery-full-charging-symbolic"
            } else if percent >= 90 {
                "battery-level-90-charging-symbolic"
            } else if percent >= 80 {
                "battery-level-80-charging-symbolic"
            } else if percent >= 70 {
                "battery-level-70-charging-symbolic"
            } else if percent >= 60 {
                "battery-level-60-charging-symbolic"
            } else if percent >= 50 {
                "battery-level-50-charging-symbolic"
            } else if percent >= 40 {
                "battery-level-40-charging-symbolic"
            } else if percent >= 30 {
                "battery-level-30-charging-symbolic"
            } else if percent >= 20 {
                "battery-level-20-charging-symbolic"
            } else if percent >= 10 {
                "battery-level-10-charging-symbolic"
            } else {
                "battery-level-0-charging-symbolic"
            }
        } else {
            if percent >= 100 {
                "battery-level-100-symbolic"
            } else if percent >= 90 {
                "battery-level-90-symbolic"
            } else if percent >= 80 {
                "battery-level-80-symbolic"
            } else if percent >= 70 {
                "battery-level-70-symbolic"
            } else if percent >= 60 {
                "battery-level-60-symbolic"
            } else if percent >= 50 {
                "battery-level-50-symbolic"
            } else if percent >= 40 {
                "battery-level-40-symbolic"
            } else if percent >= 30 {
                "battery-level-30-symbolic"
            } else if percent >= 20 {
                "battery-level-20-symbolic"
            } else if percent >= 10 {
                "battery-level-10-symbolic"
            } else {
                "battery-level-0-symbolic"
            }
        }
    }

    /// Get percentage label text
    fn get_percentage_label(percentage: f64, is_present: bool) -> String {
        if is_present {
            format!("{}%", (percentage * 100.0).round() as u8)
        } else {
            "!".to_string()
        }
    }

    /// Get tooltip text for the battery
    fn get_tooltip(percentage: f64, is_charging: bool, is_present: bool) -> String {
        if !is_present {
            "Battery: Not Present".to_string()
        } else if is_charging {
            format!(
                "Battery: {}% (Charging)",
                (percentage * 100.0).round() as u8
            )
        } else {
            format!("Battery: {}%", (percentage * 100.0).round() as u8)
        }
    }
}
