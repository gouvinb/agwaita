//! Device settings page.

use crate::{
    model::{
        BluetoothEvent,
        Device,
    },
    service::BluetoothServiceAdapter,
};
use relm4::{
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    SimpleComponent,
    adw::{
        self,
        prelude::*,
    },
    gtk,
};
use std::sync::Arc;

#[derive(Debug)]
pub enum DeviceSettingsPageInput {
    StoreEvent(Box<BluetoothEvent>),
    ShowDevice(String),
    ToggleConnected(bool),
    ToggleTrusted(bool),
    ToggleBlocked(bool),
    ApplyAlias(String),
    RemoveDeviceClicked,
    SyncFromStore,
}

pub struct DeviceSettingsPageConfig {
    pub service_adapter: Arc<BluetoothServiceAdapter>,
}

pub struct DeviceSettingsPage {
    service_adapter: Arc<BluetoothServiceAdapter>,
    current_device: Option<Device>,
}

#[relm4::component(pub)]
impl SimpleComponent for DeviceSettingsPage {
    type Input = DeviceSettingsPageInput;
    type Output = ();
    type Init = DeviceSettingsPageConfig;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_vexpand: true,

            adw::Clamp {
                set_maximum_size: 500,
                set_margin_all: 32,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 18,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_halign: gtk::Align::Center,
                        set_spacing: 20,

                        gtk::Image {
                            #[watch]
                            set_icon_name: Some(model.current_device.as_ref().and_then(|d| d.icon.as_deref()).unwrap_or("bluetooth-symbolic")),
                            set_pixel_size: 80,
                        },
                        gtk::Label {
                            #[watch]
                            set_label: model.current_device.as_ref().map(|d| d.alias.as_str()).unwrap_or("Device Settings"),
                        },
                    },

                    adw::PreferencesGroup {
                        set_title: "Connection Properties",

                        adw::SwitchRow {
                            set_title: "Connected",
                            #[watch]
                            set_active: model.current_device.as_ref().map(|d| d.connected).unwrap_or(false),
                            connect_active_notify[sender] => move |row| {
                                sender.input(DeviceSettingsPageInput::ToggleConnected(row.is_active()));
                            },
                        },
                    },

                    adw::PreferencesGroup {
                        set_title: "Device Properties",

                        adw::EntryRow {
                            set_title: "Device Name",
                            set_show_apply_button: true,
                            #[watch]
                            set_text: model.current_device.as_ref().map(|d| d.alias.as_str()).unwrap_or(""),
                            connect_apply[sender] => move |row| {
                                sender.input(DeviceSettingsPageInput::ApplyAlias(row.text().to_string()));
                            },
                        },

                        adw::SwitchRow {
                            set_title: "Trusted",
                            #[watch]
                            set_active: model.current_device.as_ref().map(|d| d.trusted).unwrap_or(false),
                            connect_active_notify[sender] => move |row| {
                                sender.input(DeviceSettingsPageInput::ToggleTrusted(row.is_active()));
                            },
                        },

                        adw::SwitchRow {
                            set_title: "Blocked",
                            #[watch]
                            set_active: model.current_device.as_ref().map(|d| d.blocked).unwrap_or(false),
                            connect_active_notify[sender] => move |row| {
                                sender.input(DeviceSettingsPageInput::ToggleBlocked(row.is_active()));
                            },
                        },
                    },

                    adw::PreferencesGroup {
                        set_title: "Status Information",

                        adw::ActionRow {
                            set_title: "Remote Device ID",
                            #[watch]
                            set_subtitle: model.current_device.as_ref().and_then(|d| d.modalias.as_deref()).unwrap_or("######"),
                        },
                        adw::ActionRow {
                            set_title: "Adapter",
                            #[watch]
                            set_subtitle: model.current_device.as_ref().map(|d| d.adapter.as_str()).unwrap_or("######"),
                        },
                        adw::ActionRow {
                            set_title: "Address",
                            #[watch]
                            set_subtitle: model.current_device.as_ref().map(|d| d.address.as_str()).unwrap_or("######"),
                        },
                        adw::ActionRow {
                            set_title: "Battery Level",
                            #[watch]
                            set_visible: model.current_device.as_ref().and_then(|d| d.battery_percentage).is_some(),
                            #[watch]
                            set_subtitle: &model.current_device.as_ref()
                                .and_then(|d| d.battery_percentage)
                                .map(|b| format!("{}%", b))
                                .unwrap_or_default(),

                            add_suffix = &gtk::LevelBar {
                                set_min_value: 0.0,
                                set_max_value: 1.0,
                                #[watch]
                                set_value: model.current_device.as_ref().and_then(|d| d.battery_percentage).unwrap_or(0) as f64 / 100.0,
                                add_offset_value: (gtk::LEVEL_BAR_OFFSET_LOW, 0.2),
                                add_offset_value: (gtk::LEVEL_BAR_OFFSET_HIGH, 1.0),
                                set_hexpand: true,
                                set_valign: gtk::Align::Center,
                            },
                        },
                    },

                    adw::PreferencesGroup {
                        gtk::Button {
                            set_label: "Remove Device",
                            add_css_class: "destructive-action",
                            connect_clicked[sender] => move |_| {
                                sender.input(DeviceSettingsPageInput::RemoveDeviceClicked);
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(config: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let store = config.service_adapter.store();

        let input_sender = sender.input_sender().clone();
        std::thread::spawn(move || {
            let receiver = store.subscribe();
            while let Ok(event) = receiver.recv() {
                input_sender
                    .send(DeviceSettingsPageInput::StoreEvent(Box::new(event)))
                    .ok();
            }
        });

        let model = Self {
            service_adapter: config.service_adapter,
            current_device: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DeviceSettingsPageInput::ShowDevice(address) => {
                self.current_device = self.service_adapter.store().get_device(&address);
            },
            DeviceSettingsPageInput::StoreEvent(event) => match *event {
                BluetoothEvent::DeviceAdded(device) | BluetoothEvent::DeviceChanged(device) => {
                    if self.current_device.as_ref().map(|d| d.address.as_str()) == Some(device.address.as_str()) {
                        self.current_device = Some(device);
                    }
                },
                BluetoothEvent::DeviceRemoved(address) if self.current_device.as_ref().map(|d| d.address.as_str()) == Some(address.as_str()) => {
                    self.current_device = None;
                },
                _ => {},
            },
            DeviceSettingsPageInput::SyncFromStore => {
                if let Some(address) = self.current_device.as_ref().map(|d| d.address.clone()) {
                    self.current_device = self.service_adapter.store().get_device(&address);
                }
            },
            DeviceSettingsPageInput::ToggleConnected(connected) => {
                if let Some(device) = self.current_device.as_mut() {
                    device.connected = connected;
                }
                let Some(address) = self.current_device.as_ref().map(|d| d.address.clone()) else {
                    return;
                };
                let service_adapter = Arc::clone(&self.service_adapter);
                let input_sender = sender.input_sender().clone();
                std::thread::spawn(move || {
                    let result = if connected {
                        service_adapter.connect_device(&address)
                    } else {
                        service_adapter.disconnect_device(&address)
                    };
                    if let Err(e) = result {
                        log::warn!("Failed to toggle connected for {}: {}", address, e);
                        input_sender
                            .send(DeviceSettingsPageInput::SyncFromStore)
                            .ok();
                    }
                });
            },
            DeviceSettingsPageInput::ToggleTrusted(trusted) => {
                if let Some(device) = self.current_device.as_mut() {
                    device.trusted = trusted;
                }
                let Some(address) = self.current_device.as_ref().map(|d| d.address.clone()) else {
                    return;
                };
                let service_adapter = Arc::clone(&self.service_adapter);
                let input_sender = sender.input_sender().clone();
                std::thread::spawn(move || {
                    if let Err(e) = service_adapter.set_device_trusted(&address, trusted) {
                        log::warn!("Failed to set trusted for {}: {}", address, e);
                        input_sender
                            .send(DeviceSettingsPageInput::SyncFromStore)
                            .ok();
                    }
                });
            },
            DeviceSettingsPageInput::ToggleBlocked(blocked) => {
                if let Some(device) = self.current_device.as_mut() {
                    device.blocked = blocked;
                }
                let Some(address) = self.current_device.as_ref().map(|d| d.address.clone()) else {
                    return;
                };
                let service_adapter = Arc::clone(&self.service_adapter);
                let input_sender = sender.input_sender().clone();
                std::thread::spawn(move || {
                    if let Err(e) = service_adapter.set_device_blocked(&address, blocked) {
                        log::warn!("Failed to set blocked for {}: {}", address, e);
                        input_sender
                            .send(DeviceSettingsPageInput::SyncFromStore)
                            .ok();
                    }
                });
            },
            DeviceSettingsPageInput::ApplyAlias(alias) => {
                if let Some(device) = self.current_device.as_mut() {
                    device.alias = alias.clone();
                }
                let Some(address) = self.current_device.as_ref().map(|d| d.address.clone()) else {
                    return;
                };
                let service_adapter = Arc::clone(&self.service_adapter);
                let input_sender = sender.input_sender().clone();
                std::thread::spawn(move || {
                    if let Err(e) = service_adapter.set_device_alias(&address, &alias) {
                        log::warn!("Failed to set alias for {}: {}", address, e);
                        input_sender
                            .send(DeviceSettingsPageInput::SyncFromStore)
                            .ok();
                    }
                });
            },
            DeviceSettingsPageInput::RemoveDeviceClicked => {
                let Some(address) = self.current_device.as_ref().map(|d| d.address.clone()) else {
                    return;
                };
                let service_adapter = Arc::clone(&self.service_adapter);
                std::thread::spawn(move || {
                    if let Err(e) = service_adapter.remove_device(&address) {
                        log::warn!("Failed to remove device {}: {}", address, e);
                    }
                });
                // Le retour à la page adaptateur se fait naturellement : le
                // DeviceRemoved du store met current_device à None ici, et
                // c'est main_window qui décide de rebasculer la page.
            },
        }
    }
}
