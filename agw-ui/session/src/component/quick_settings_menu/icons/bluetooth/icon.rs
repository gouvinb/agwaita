use gtk4::prelude::*;
use relm4::{
    ComponentParts,
    ComponentSender,
    SimpleComponent,
    gtk,
};

/// BluetoothIcon - Displays Bluetooth connection status
/// Matches the AGS TypeScript implementation with power state and device count
pub struct BluetoothIcon {
    powered: bool,
    connected_count: u8,
}

#[derive(Debug)]
pub enum BluetoothIconInput {
    UpdateState(bool, u8), // (powered, connected_count)
}

#[relm4::component(pub)]
impl SimpleComponent for BluetoothIcon {
    type Init = (bool, u8);
    type Input = BluetoothIconInput;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 0,

            gtk::Image {
                set_pixel_size: 16,

                #[watch]
                set_icon_name: Some(Self::resolve_icon(model.powered, model.connected_count)),
                #[watch]
                set_tooltip_text: Some(&Self::get_tooltip(model.powered, model.connected_count)),
            },

            gtk::Label {
                #[watch]
                set_visible: model.connected_count > 0,
                #[watch]
                set_markup: &format!("<span font_size=\"xx-small\" baseline_shift=\"superscript\">{}</span>", model.connected_count),
            },
        }
    }

    fn init(init: Self::Init, root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = BluetoothIcon {
            powered: init.0,
            connected_count: init.1,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            BluetoothIconInput::UpdateState(powered, connected_count) => {
                self.powered = powered;
                self.connected_count = connected_count;
            },
        }
    }
}

impl BluetoothIcon {
    /// Resolves the icon name based on power state and connected devices
    /// Matches the AGS implementation:
    /// - !powered: bluetooth-hardware-disabled-symbolic
    /// - powered but no devices: bluetooth-disabled-symbolic
    /// - powered with devices: bluetooth-active-symbolic
    fn resolve_icon(powered: bool, connected_count: u8) -> &'static str {
        if !powered {
            "bluetooth-hardware-disabled-symbolic"
        } else if connected_count > 0 {
            "bluetooth-active-symbolic"
        } else {
            "bluetooth-disabled-symbolic"
        }
    }

    fn get_tooltip(powered: bool, connected_count: u8) -> String {
        if !powered {
            "Bluetooth: Disabled".to_string()
        } else if connected_count > 0 {
            format!("Bluetooth: {} device(s) connected", connected_count)
        } else {
            "Bluetooth: Enabled".to_string()
        }
    }
}
