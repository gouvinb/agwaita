//! Adapter settings page.

use crate::{
    model::{
        Adapter,
        BluetoothEvent,
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
pub enum AdapterSettingsPageInput {
    StoreEvent(Box<BluetoothEvent>),
    TogglePowered(bool),
    ToggleDiscoverable(bool),
    ApplyTimeout(u32),
    ApplyAlias(String),
    SyncFromStore,
}

#[derive(Debug)]
pub enum AdapterSettingsPageOutput {
    ShowBlockedDevices,
}

pub struct AdapterSettingsPageConfig {
    pub service_adapter: Arc<BluetoothServiceAdapter>,
}

pub struct AdapterSettingsPage {
    service_adapter: Arc<BluetoothServiceAdapter>,
    current_adapter: Option<Adapter>,
}

#[relm4::component(pub)]
impl SimpleComponent for AdapterSettingsPage {
    type Input = AdapterSettingsPageInput;
    type Output = AdapterSettingsPageOutput;
    type Init = AdapterSettingsPageConfig;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_vexpand: true,

            adw::Clamp {
                set_maximum_size: 500,
                set_margin_all: 32,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 20,

                    adw::PreferencesGroup {
                        set_title: "Bluetooth Adapter Status",

                        adw::SwitchRow {
                            set_title: "Powered",
                            #[watch]
                            set_active: model.current_adapter.as_ref().map(|a| a.powered).unwrap_or(false),
                            connect_active_notify[sender] => move |row| {
                                sender.input(AdapterSettingsPageInput::TogglePowered(row.is_active()));
                            },
                        },

                        adw::SwitchRow {
                            set_title: "Discoverable",
                            set_subtitle: "visible to others?",
                            #[watch]
                            set_active: model.current_adapter.as_ref().map(|a| a.discoverable).unwrap_or(false),
                            connect_active_notify[sender] => move |row| {
                                sender.input(AdapterSettingsPageInput::ToggleDiscoverable(row.is_active()));
                            },
                        },
                    },

                    adw::PreferencesGroup {
                        set_title: "Adapter Properties",

                        adw::SpinRow {
                            set_title: "Discoverable Timeout",
                            set_subtitle: "in seconds",
                            set_adjustment: Some(&gtk::Adjustment::new(0.0, 0.0, 3600.0, 1.0, 10.0, 0.0)),
                            #[watch]
                            set_value: model.current_adapter.as_ref().map(|a| a.discoverable_timeout as f64).unwrap_or(0.0),
                            connect_value_notify[sender] => move |row| {
                                sender.input(AdapterSettingsPageInput::ApplyTimeout(row.value() as u32));
                            },
                        },

                        adw::EntryRow {
                            set_title: "Adapter Name",
                            set_show_apply_button: true,
                            #[watch]
                            set_text: model.current_adapter.as_ref().map(|a| a.alias.as_str()).unwrap_or(""),
                            connect_apply[sender] => move |row| {
                                sender.input(AdapterSettingsPageInput::ApplyAlias(row.text().to_string()));
                            },
                        },
                    },

                    adw::PreferencesGroup {
                        set_title: "Device Management",

                        adw::ActionRow {
                            set_title: "Blocked Devices",
                            set_activatable: true,

                            connect_activated[sender] => move |_| {
                                sender.output(AdapterSettingsPageOutput::ShowBlockedDevices).ok();
                            },

                            add_suffix = &gtk::Image {
                                set_icon_name: Some("go-next-symbolic"),
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(config: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let store = config.service_adapter.store();
        let current_adapter = store
            .get_selected_adapter()
            .and_then(|name| store.get_adapters().into_iter().find(|a| a.name == name));

        let input_sender = sender.input_sender().clone();
        std::thread::spawn(move || {
            let receiver = store.subscribe();
            while let Ok(event) = receiver.recv() {
                input_sender
                    .send(AdapterSettingsPageInput::StoreEvent(Box::new(event)))
                    .ok();
            }
        });

        let model = Self {
            service_adapter: config.service_adapter,
            current_adapter,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AdapterSettingsPageInput::StoreEvent(event) => match *event {
                BluetoothEvent::AdaptersLoaded(adapters) => {
                    if let Some(name) = self.current_adapter.as_ref().map(|a| a.name.clone()) {
                        self.current_adapter = adapters.into_iter().find(|a| a.name == name);
                    }
                },
                BluetoothEvent::AdapterChanged(adapter) => {
                    if self.current_adapter.as_ref().map(|a| a.name.as_str()) == Some(adapter.name.as_str()) {
                        self.current_adapter = Some(adapter);
                    }
                },
                BluetoothEvent::AdapterSelected(name) => {
                    self.current_adapter = self
                        .service_adapter
                        .store()
                        .get_adapters()
                        .into_iter()
                        .find(|a| a.name == name);
                },
                _ => {}, // événements device sans rapport : ne touchent pas self.current_adapter
            },
            AdapterSettingsPageInput::SyncFromStore => {
                if let Some(name) = self.current_adapter.as_ref().map(|a| a.name.clone()) {
                    self.current_adapter = self
                        .service_adapter
                        .store()
                        .get_adapters()
                        .into_iter()
                        .find(|a| a.name == name);
                }
            },
            AdapterSettingsPageInput::TogglePowered(powered) => {
                if let Some(adapter) = self.current_adapter.as_mut() {
                    adapter.powered = powered;
                }
                let service_adapter = Arc::clone(&self.service_adapter);
                let input_sender = sender.input_sender().clone();
                std::thread::spawn(move || {
                    if let Err(e) = service_adapter.set_powered(powered) {
                        log::warn!("Failed to set powered: {}", e);
                        input_sender
                            .send(AdapterSettingsPageInput::SyncFromStore)
                            .ok();
                    }
                });
            },
            AdapterSettingsPageInput::ToggleDiscoverable(discoverable) => {
                if let Some(adapter) = self.current_adapter.as_mut() {
                    adapter.discoverable = discoverable;
                }
                let service_adapter = Arc::clone(&self.service_adapter);
                let input_sender = sender.input_sender().clone();
                std::thread::spawn(move || {
                    if let Err(e) = service_adapter.set_discoverable(discoverable) {
                        log::warn!("Failed to set discoverable: {}", e);
                        input_sender
                            .send(AdapterSettingsPageInput::SyncFromStore)
                            .ok();
                    }
                });
            },
            AdapterSettingsPageInput::ApplyTimeout(timeout) => {
                if let Some(adapter) = self.current_adapter.as_mut() {
                    adapter.discoverable_timeout = timeout;
                }
                let service_adapter = Arc::clone(&self.service_adapter);
                let input_sender = sender.input_sender().clone();
                std::thread::spawn(move || {
                    if let Err(e) = service_adapter.set_discoverable_timeout(timeout) {
                        log::warn!("Failed to set discoverable timeout: {}", e);
                        input_sender
                            .send(AdapterSettingsPageInput::SyncFromStore)
                            .ok();
                    }
                });
            },
            AdapterSettingsPageInput::ApplyAlias(alias) => {
                if let Some(adapter) = self.current_adapter.as_mut() {
                    adapter.alias = alias.clone();
                }
                let service_adapter = Arc::clone(&self.service_adapter);
                let input_sender = sender.input_sender().clone();
                std::thread::spawn(move || {
                    if let Err(e) = service_adapter.set_adapter_alias(&alias) {
                        log::warn!("Failed to set adapter alias: {}", e);
                        input_sender
                            .send(AdapterSettingsPageInput::SyncFromStore)
                            .ok();
                    }
                });
            },
        }
    }
}
