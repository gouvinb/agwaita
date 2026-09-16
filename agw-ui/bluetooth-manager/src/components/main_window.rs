//! Main Bluetooth manager window.

use crate::{
    components::{
        adapter_settings_page::{
            AdapterSettingsPage,
            AdapterSettingsPageConfig,
            AdapterSettingsPageOutput,
        },
        blocked_devices_page::{
            BlockedDevicesPage,
            BlockedDevicesPageConfig,
        },
        device_settings_page::{
            DeviceSettingsPage,
            DeviceSettingsPageConfig,
            DeviceSettingsPageInput,
        },
        pairing_dialog::{
            PairingDialog,
            PairingDialogConfig,
            PairingDialogInput,
        },
        sidebar::{
            Sidebar,
            SidebarConfig,
            SidebarOutput,
        },
    },
    model::{
        Adapter,
        BluetoothEvent,
        PairingRequest,
    },
    service::BluetoothServiceAdapter,
};
use relm4::{
    Component,
    ComponentController,
    ComponentParts,
    ComponentSender,
    Controller,
    SimpleComponent,
    adw::{
        self,
        prelude::*,
    },
    gtk,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum BluetoothManagerWindowInput {
    AdaptersLoaded(Vec<Adapter>),
    ShowAdapterSettings,
    ShowBlockedDevices,
    ShowDeviceSettings(String),
    PairingRequested(PairingRequest),
    PairingCleared,
}

pub struct BluetoothManagerWindowConfig {
    pub service_adapter: Arc<BluetoothServiceAdapter>,
}

pub struct BluetoothManagerWindow {
    root_stack: gtk::Stack,
    main_stack: gtk::Stack,
    _sidebar: Controller<Sidebar>,
    _adapter_settings_page: Controller<AdapterSettingsPage>,
    _blocked_devices_page: Controller<BlockedDevicesPage>,
    device_settings_page: Controller<DeviceSettingsPage>,
    pairing_dialog: Controller<PairingDialog>,
}

#[relm4::component(pub)]
impl SimpleComponent for BluetoothManagerWindow {
    type Input = BluetoothManagerWindowInput;
    type Output = ();
    type Init = BluetoothManagerWindowConfig;

    view! {
        #[root]
        adw::Window {
            set_title: Some("Bluetooth manager"),
            set_default_width: 475,
            set_default_height: 575,
            set_width_request: 475,
            set_height_request: 475,

            connect_close_request => move |_| {
                relm4::main_application().quit();
                gtk::glib::Propagation::Stop
            },

            #[name = "toast_overlay"]
            adw::ToastOverlay {
                #[name = "root_stack"]
                gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::Crossfade,

                    #[name = "split_view"]
                    add_named[Some("main")] = &adw::OverlaySplitView {
                        set_pin_sidebar: false,
                        set_enable_hide_gesture: true,
                        set_enable_show_gesture: true,
                        set_sidebar_position: gtk::PackType::Start,
                        set_sidebar_width_fraction: 0.4,
                        set_min_sidebar_width: 350.0,
                        set_max_sidebar_width: 350.0,

                        #[wrap(Some)]
                        set_sidebar = &gtk::Box {},

                        #[wrap(Some)]
                        set_content = &adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                #[name = "show_sidebar_button"]
                                pack_start = &gtk::ToggleButton {
                                    set_icon_name: "sidebar-show-symbolic",
                                    set_active: true,
                                    set_tooltip_text: Some("Hide Sidebar"),
                                },
                            },

                            #[wrap(Some)]
                            #[name = "main_stack"]
                            set_content = &gtk::Stack {
                                set_transition_type: gtk::StackTransitionType::SlideLeftRight,
                            },
                        },
                    },

                    add_named[Some("no_adapter")] = &adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {},

                        #[wrap(Some)]
                        set_content = &adw::StatusPage {
                            set_icon_name: Some("bluetooth-disabled-symbolic"),
                            set_title: "No Bluetooth Adapter Found",
                            set_description: Some("Make sure Bluetooth is enabled and a compatible adapter is connected"),
                        },
                    },
                },
            },
        }
    }

    fn init(config: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let sidebar = Sidebar::builder()
            .launch(SidebarConfig {
                service_adapter: Arc::clone(&config.service_adapter),
            })
            .forward(sender.input_sender(), |output| match output {
                SidebarOutput::ShowAdapterSettings => BluetoothManagerWindowInput::ShowAdapterSettings,
                SidebarOutput::ShowDeviceSettings(address) => BluetoothManagerWindowInput::ShowDeviceSettings(address),
            });

        let adapter_settings_page = AdapterSettingsPage::builder()
            .launch(AdapterSettingsPageConfig {
                service_adapter: Arc::clone(&config.service_adapter),
            })
            .forward(sender.input_sender(), |output| match output {
                AdapterSettingsPageOutput::ShowBlockedDevices => BluetoothManagerWindowInput::ShowBlockedDevices,
            });

        let blocked_devices_page = BlockedDevicesPage::builder()
            .launch(BlockedDevicesPageConfig {
                service_adapter: Arc::clone(&config.service_adapter),
            })
            .detach();

        let device_settings_page = DeviceSettingsPage::builder()
            .launch(DeviceSettingsPageConfig {
                service_adapter: Arc::clone(&config.service_adapter),
            })
            .detach();

        let pairing_dialog = PairingDialog::builder()
            .launch(PairingDialogConfig {
                service_adapter: Arc::clone(&config.service_adapter),
                parent_window: root.clone(),
            })
            .detach();

        let input_sender = sender.input_sender().clone();
        let store_for_thread = config.service_adapter.store();
        std::thread::spawn(move || {
            let receiver = store_for_thread.subscribe();
            while let Ok(event) = receiver.recv() {
                match event {
                    BluetoothEvent::AdaptersLoaded(adapters) => {
                        input_sender
                            .send(BluetoothManagerWindowInput::AdaptersLoaded(adapters))
                            .ok();
                    },
                    BluetoothEvent::PairingRequested(req) => {
                        input_sender
                            .send(BluetoothManagerWindowInput::PairingRequested(req))
                            .ok();
                    },
                    BluetoothEvent::PairingCleared => {
                        input_sender
                            .send(BluetoothManagerWindowInput::PairingCleared)
                            .ok();
                    },
                    _ => {},
                }
            }
        });

        let widgets = view_output!();

        if config.service_adapter.store().get_adapters().is_empty() {
            widgets.root_stack.set_visible_child_name("no_adapter");
        } else {
            widgets.root_stack.set_visible_child_name("main");
        }

        widgets.split_view.set_sidebar(Some(sidebar.widget()));

        widgets.main_stack.add_named(
            adapter_settings_page.widget(),
            Some("adapter_settings_page"),
        );
        widgets
            .main_stack
            .add_named(blocked_devices_page.widget(), Some("blocked_devices_page"));
        widgets
            .main_stack
            .add_named(device_settings_page.widget(), Some("device_settings_page"));

        // Breakpoint : sidebar repliable sous 700px, comme le comportement Ags.
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            700.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(&widgets.split_view, "collapsed", Some(&true.to_value()));
        breakpoint.add_setter(&widgets.split_view, "show-sidebar", Some(&false.to_value()));
        breakpoint.add_setter(
            &widgets.show_sidebar_button,
            "active",
            Some(&false.to_value()),
        );
        root.add_breakpoint(breakpoint);

        widgets
            .split_view
            .bind_property("show-sidebar", &widgets.show_sidebar_button, "active")
            .bidirectional()
            .build();

        widgets
            .main_stack
            .set_visible_child_name("adapter_settings_page");

        let model = Self {
            root_stack: widgets.root_stack.clone(),
            main_stack: widgets.main_stack.clone(),
            _sidebar: sidebar,
            _adapter_settings_page: adapter_settings_page,
            _blocked_devices_page: blocked_devices_page,
            device_settings_page,
            pairing_dialog,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            BluetoothManagerWindowInput::AdaptersLoaded(adapters) => {
                if adapters.is_empty() {
                    self.root_stack.set_visible_child_name("no_adapter");
                } else {
                    self.root_stack.set_visible_child_name("main");
                }
            },
            BluetoothManagerWindowInput::ShowAdapterSettings => {
                self.main_stack
                    .set_visible_child_name("adapter_settings_page");
            },
            BluetoothManagerWindowInput::ShowBlockedDevices => {
                self.main_stack
                    .set_visible_child_name("blocked_devices_page");
            },
            BluetoothManagerWindowInput::ShowDeviceSettings(address) => {
                self.device_settings_page
                    .emit(DeviceSettingsPageInput::ShowDevice(address));
                self.main_stack
                    .set_visible_child_name("device_settings_page");
            },
            BluetoothManagerWindowInput::PairingRequested(req) => {
                self.pairing_dialog.emit(PairingDialogInput::Show(req));
            },
            BluetoothManagerWindowInput::PairingCleared => {
                self.pairing_dialog.emit(PairingDialogInput::Close);
            },
        }
    }
}
