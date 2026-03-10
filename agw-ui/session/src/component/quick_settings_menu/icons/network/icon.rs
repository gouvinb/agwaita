use agw_service::network::NetworkType;
use gtk4::prelude::*;
use relm4::{
    ComponentParts,
    ComponentSender,
    SimpleComponent,
    gtk,
};

/// NetworkIcon - Displays network connection status
/// Matches the AGS TypeScript implementation for systemd-networkd/iwd
pub struct NetworkIcon {
    network_type: NetworkType,
    connected: bool,
    wifi_strength: u8, // 0-100
}

#[derive(Debug)]
pub enum NetworkIconInput {
    UpdateState(NetworkType, bool, u8), // (type, connected, wifi_strength)
}

#[relm4::component(pub)]
impl SimpleComponent for NetworkIcon {
    type Init = (NetworkType, bool, u8);
    type Input = NetworkIconInput;
    type Output = ();

    view! {
        #[root]
        gtk::Image {
            #[watch]
            set_icon_name: Some(Self::resolve_status_icon(model.network_type.clone(), model.connected, model.wifi_strength)),
            set_pixel_size: 16,
            #[watch]
            set_tooltip_text: Some(&Self::get_tooltip(model.network_type.clone(), model.connected, model.wifi_strength)),
        }
    }

    fn init(init: Self::Init, root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = NetworkIcon {
            network_type: init.0,
            connected: init.1,
            wifi_strength: init.2,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            NetworkIconInput::UpdateState(network_type, connected, wifi_strength) => {
                self.network_type = network_type;
                self.connected = connected;
                self.wifi_strength = wifi_strength;
            },
        }
    }
}

impl NetworkIcon {
    /// Resolves the network icon based on type, connection state, and wifi strength
    /// Matches the AGS implementation
    fn resolve_status_icon(network_type: NetworkType, connected: bool, wifi_strength: u8) -> &'static str {
        match network_type {
            NetworkType::Wired => {
                if connected {
                    "network-wired-symbolic"
                } else {
                    "network-wired-disconnected-symbolic"
                }
            },
            NetworkType::Wifi => {
                if !connected {
                    "network-wireless-offline-symbolic"
                } else if wifi_strength >= 80 {
                    "network-wireless-signal-excellent-symbolic"
                } else if wifi_strength >= 60 {
                    "network-wireless-signal-good-symbolic"
                } else if wifi_strength >= 40 {
                    "network-wireless-signal-ok-symbolic"
                } else if wifi_strength >= 20 {
                    "network-wireless-signal-weak-symbolic"
                } else {
                    "network-wireless-signal-none-symbolic"
                }
            },
            NetworkType::None => "network-wireless-disabled-symbolic",
        }
    }

    fn get_tooltip(network_type: NetworkType, connected: bool, wifi_strength: u8) -> String {
        match network_type {
            NetworkType::Wired => {
                if connected {
                    "Network: Wired".to_string()
                } else {
                    "Network: Wired (Disconnected)".to_string()
                }
            },
            NetworkType::Wifi => {
                if !connected {
                    "Network: WiFi (Offline)".to_string()
                } else {
                    format!("Network: WiFi ({}%)", wifi_strength)
                }
            },
            NetworkType::None => "Network: Disabled".to_string(),
        }
    }
}
