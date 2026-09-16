//! Device row factory component.

use crate::model::Device;
use relm4::{
    adw::{
        self,
        prelude::*,
    },
    factory::{
        DynamicIndex,
        FactoryComponent,
        FactorySender,
    },
    gtk,
};

#[derive(Debug)]
pub enum DeviceRowOutput {
    Clicked(String),
}

pub struct DeviceRow {
    pub device: Device,
}

#[relm4::factory(pub)]
impl FactoryComponent for DeviceRow {
    type CommandOutput = ();
    type Init = Device;
    type Input = Device;
    type Output = DeviceRowOutput;
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        adw::ActionRow {
            #[watch]
            set_title: &self.device.alias,
            #[watch]
            set_subtitle: &self.device.address,
            set_activatable: true,

            connect_activated[sender, address = self.device.address.clone()] => move |_| {
                sender.output(DeviceRowOutput::Clicked(address.clone())).ok();
            },

            add_prefix = &gtk::Box {
                set_spacing: 8,

                gtk::Image {
                    #[watch]
                    set_icon_name: Some(device_icon(self.device.icon.as_deref())),
                    set_icon_size: gtk::IconSize::Large,
                },
            },

            add_suffix = &gtk::Box {
                set_spacing: 8,

                gtk::Image {
                    #[watch]
                    set_icon_name: Some(if self.device.connected {
                        "network-wireless-signal-excellent-symbolic"
                    } else {
                        "network-wireless-offline-symbolic"
                    }),
                    #[watch]
                    set_opacity: if self.device.connected { 1.0 } else { 0.5 },
                    #[watch]
                    set_tooltip_text: Some(if self.device.connected { "Connected" } else { "Disconnected" }),
                },

                gtk::Image {
                    #[watch]
                    set_icon_name: Some(if self.device.paired {
                        "network-transmit-receive-symbolic"
                    } else {
                        "network-no-route-symbolic"
                    }),
                    #[watch]
                    set_opacity: if self.device.paired { 1.0 } else { 0.5 },
                    #[watch]
                    set_tooltip_text: Some(if self.device.paired { "Paired" } else { "Not paired" }),
                },

                gtk::Image {
                    #[watch]
                    set_icon_name: Some(if self.device.trusted {
                        "network-wireless-encrypted-symbolic"
                    } else {
                        "channel-insecure-symbolic"
                    }),
                    #[watch]
                    set_opacity: if self.device.trusted { 1.0 } else { 0.5 },
                    #[watch]
                    set_tooltip_text: Some(if self.device.trusted { "Trusted" } else { "Not trusted" }),
                },
            },
        }
    }

    fn init_model(device: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { device }
    }

    fn update(&mut self, device: Self::Input, _sender: FactorySender<Self>) {
        self.device = device;
    }
}

/// Map a BlueZ icon hint (freedesktop icon-naming-spec category) to a symbolic
/// GTK icon name. Mirrors `getDeviceIcon` from the original Ags implementation.
pub(crate) fn device_icon(icon_hint: Option<&str>) -> &'static str {
    let icon = icon_hint.unwrap_or("");

    if icon.contains("audio") || icon.contains("headset") || icon.contains("headphone") {
        "audio-headphones-symbolic"
    } else if icon.contains("phone") || icon.contains("modem") {
        "phone-symbolic"
    } else if icon.contains("computer") {
        "computer-symbolic"
    } else if icon.contains("input") || icon.contains("keyboard") {
        "input-keyboard-symbolic"
    } else if icon.contains("mouse") {
        "input-mouse-symbolic"
    } else {
        "bluetooth-symbolic"
    }
}
