//! Blocked devices management page.

use crate::{
    components::device_row::device_icon,
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
    factory::{
        DynamicIndex,
        FactoryComponent,
        FactorySender,
        FactoryVecDeque,
    },
    gtk,
};
use std::sync::Arc;

#[derive(Debug)]
pub enum BlockedDeviceRowOutput {
    Unblock(String),
}

#[derive(Debug)]
pub enum BlockedDevicesPageInput {
    StoreEvent(Box<BluetoothEvent>),
    UnblockDevice(String),
    SyncFromStore,
}

#[derive(Debug)]
pub enum BlockedDevicesPageOutput {
    TitleChanged(String),
}

pub struct BlockedDeviceRow {
    pub device: Device,
}

pub struct BlockedDevicesPageConfig {
    pub service_adapter: Arc<BluetoothServiceAdapter>,
}

pub struct BlockedDevicesPage {
    service_adapter: Arc<BluetoothServiceAdapter>,
    devices: FactoryVecDeque<BlockedDeviceRow>,
}

#[relm4::factory(pub)]
impl FactoryComponent for BlockedDeviceRow {
    type CommandOutput = ();
    type Init = Device;
    type Input = Device;
    type Output = BlockedDeviceRowOutput;
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        adw::ActionRow {
            #[watch]
            set_title: &self.device.alias,
            #[watch]
            set_subtitle: &self.device.address,
            set_activatable: false,

            add_prefix = &gtk::Box {
                set_spacing: 8,

                gtk::Image {
                    #[watch]
                    set_icon_name: Some(device_icon(self.device.icon.as_deref())),
                    set_icon_size: gtk::IconSize::Large,
                },
            },

            add_suffix = &gtk::Button {
                set_label: "Unblock",
                set_valign: gtk::Align::Center,
                add_css_class: "suggested-action",
                connect_clicked[sender, address = self.device.address.clone()] => move |_| {
                    sender.output(BlockedDeviceRowOutput::Unblock(address.clone())).ok();
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

#[relm4::component(pub)]
impl SimpleComponent for BlockedDevicesPage {
    type Input = BlockedDevicesPageInput;
    type Output = BlockedDevicesPageOutput;
    type Init = BlockedDevicesPageConfig;

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
                        #[local_ref]
                        devices_list_box -> gtk::ListBox {
                            set_selection_mode: gtk::SelectionMode::None,
                            add_css_class: "boxed-list",
                        },
                    },
                },
            },
        }
    }

    fn init(config: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let store = config.service_adapter.store();

        let mut devices = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |output| match output {
                BlockedDeviceRowOutput::Unblock(address) => BlockedDevicesPageInput::UnblockDevice(address),
            });

        {
            let mut guard = devices.guard();
            for device in store.get_devices().into_iter().filter(|d| d.blocked) {
                guard.push_back(device);
            }
        }

        let devices_list_box: &gtk::ListBox = devices.widget();
        devices_list_box.set_placeholder(Some(
            &adw::StatusPage::builder()
                .icon_name("channel-secure-symbolic")
                .title("No Blocked Devices")
                .description("Devices you block will appear here")
                .build(),
        ));
        let widgets = view_output!();

        let input_sender = sender.input_sender().clone();
        let store_for_thread = (*store).clone();
        std::thread::spawn(move || {
            let receiver = store_for_thread.subscribe();
            while let Ok(event) = receiver.recv() {
                input_sender
                    .send(BlockedDevicesPageInput::StoreEvent(Box::new(event)))
                    .ok();
            }
        });

        let model = Self {
            service_adapter: config.service_adapter,
            devices,
        };

        sender
            .output(BlockedDevicesPageOutput::TitleChanged(
                "Blocked Devices".to_string(),
            ))
            .ok();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            BlockedDevicesPageInput::StoreEvent(event) => match *event {
                BluetoothEvent::DevicesCleared => {
                    self.devices.guard().clear();
                },
                BluetoothEvent::DeviceAdded(_) | BluetoothEvent::DeviceChanged(_) | BluetoothEvent::DeviceRemoved(_) => {
                    self.rebuild_blocked_devices_list();
                },
                _ => {},
            },
            BlockedDevicesPageInput::SyncFromStore => {
                self.rebuild_blocked_devices_list();
            },
            BlockedDevicesPageInput::UnblockDevice(address) => {
                let service_adapter = Arc::clone(&self.service_adapter);
                let input_sender = sender.input_sender().clone();
                std::thread::spawn(move || {
                    if let Err(e) = service_adapter.set_device_blocked(&address, false) {
                        log::warn!("Failed to unblock device {}: {}", address, e);
                        input_sender
                            .send(BlockedDevicesPageInput::SyncFromStore)
                            .ok();
                    }
                });
            },
        }
    }
}

impl BlockedDevicesPage {
    fn rebuild_blocked_devices_list(&mut self) {
        let store = self.service_adapter.store();
        let mut guard = self.devices.guard();
        guard.clear();
        for device in store.get_devices().into_iter().filter(|d| d.blocked) {
            guard.push_back(device);
        }
    }
}
