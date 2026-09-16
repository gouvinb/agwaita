//! Bluetooth manager sidebar: adapter dropdown, settings shortcut, device list.

use crate::{
    components::device_row::{
        DeviceRow,
        DeviceRowOutput,
    },
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
    factory::FactoryVecDeque,
    gtk,
};
use std::sync::Arc;

#[derive(Debug)]
pub enum SidebarInput {
    StoreEvent(Box<BluetoothEvent>),
    AdapterDropdownChanged,
    ShowAdapterSettingsClicked,
    DeviceRow(DeviceRowOutput),
    ToggleScan(bool),
}

#[derive(Debug)]
pub enum SidebarOutput {
    ShowAdapterSettings,
    ShowDeviceSettings(String),
}

pub struct SidebarConfig {
    pub service_adapter: Arc<BluetoothServiceAdapter>,
}

pub struct Sidebar {
    service_adapter: Arc<BluetoothServiceAdapter>,
    adapters: Vec<Adapter>,
    current_adapter_name: Option<String>,
    devices: FactoryVecDeque<DeviceRow>,
    adapter_dropdown: adw::ComboRow,
    adapter_model: gtk::StringList,
    scan_toggle_button: gtk::Switch,
    scan_switch_handler: gtk::glib::SignalHandlerId,
    discovering: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for Sidebar {
    type Input = SidebarInput;
    type Output = SidebarOutput;
    type Init = SidebarConfig;

    view! {
        #[root]
        adw::ToolbarView {
            set_top_bar_style: adw::ToolbarStyle::Flat,

            add_top_bar = &adw::HeaderBar {},

            #[wrap(Some)]
            set_content = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 20,
                set_margin_all: 12,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,

                    gtk::Label {
                        set_label: "Adapter",
                        add_css_class: "heading",
                        set_hexpand: true,
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Button {
                        set_icon_name: "preferences-system-symbolic",
                        set_tooltip_text: Some("Adapter Settings"),
                        connect_clicked[sender] => move |_| {
                            sender.input(SidebarInput::ShowAdapterSettingsClicked);
                        },
                    },
                },

                adw::PreferencesGroup {
                    #[name = "adapter_dropdown"]
                    adw::ComboRow {
                        set_title: "Current adapter",

                        #[wrap(Some)]
                        set_model = &gtk::StringList::new(&[]),
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,

                    gtk::Label {
                        set_label: "Devices",
                        add_css_class: "heading",
                        set_hexpand: true,
                        set_halign: gtk::Align::Start,
                    },

                    #[name = "scan_toggle_button"]
                    gtk::Switch {
                        set_valign: gtk::Align::Center,
                        set_tooltip_text: Some("Toggle scanning"),
                    },
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

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
            .forward(sender.input_sender(), SidebarInput::DeviceRow);

        // État initial : le store peut déjà contenir des adaptateurs/devices
        // chargés par `BluetoothServiceAdapter::new()` avant que ce composant
        // n'existe (pas d'événement rejoué automatiquement pour un nouvel abonné).
        let adapters = store.get_adapters();
        let current_adapter_name = store.get_selected_adapter();
        let discovering = current_adapter_name
            .as_deref()
            .and_then(|name| adapters.iter().find(|a| a.name == name))
            .map(|a| a.discovering)
            .unwrap_or(false);
        {
            let mut guard = devices.guard();
            for device in store.get_devices() {
                guard.push_back(device);
            }
        }

        let devices_list_box: &gtk::ListBox = devices.widget();
        devices_list_box.set_placeholder(Some(
            &adw::StatusPage::builder()
                .icon_name("bluetooth-disabled-symbolic")
                .title("No Devices Found")
                .description("No Bluetooth devices are currently known to this adapter")
                .build(),
        ));
        let widgets = view_output!();

        let adapter_dropdown = widgets.adapter_dropdown.clone();
        let adapter_model = gtk::StringList::new(&[]);
        adapter_dropdown.set_model(Some(&adapter_model));
        Self::rebuild_adapter_model(
            &adapter_dropdown,
            &adapter_model,
            &adapters,
            current_adapter_name.as_deref(),
        );

        let scan_toggle_button = widgets.scan_toggle_button.clone();
        let scan_switch_handler = scan_toggle_button.connect_state_set(gtk::glib::clone!(
            #[strong]
            sender,
            move |_, active| {
                sender.input(SidebarInput::ToggleScan(active));
                gtk::glib::Propagation::Proceed
            }
        ));

        adapter_dropdown.connect_selected_notify(gtk::glib::clone!(
            #[strong]
            sender,
            move |_| sender.input(SidebarInput::AdapterDropdownChanged)
        ));

        // Relais des événements du store vers l'Input du composant (mêmes
        // conventions que NotificationList : subscribe() + thread dédié).
        let input_sender = sender.input_sender().clone();
        let store_for_thread = (*store).clone();
        std::thread::spawn(move || {
            let receiver = store_for_thread.subscribe();
            while let Ok(event) = receiver.recv() {
                input_sender
                    .send(SidebarInput::StoreEvent(Box::new(event)))
                    .ok();
            }
        });

        let model = Self {
            service_adapter: config.service_adapter,
            adapters,
            current_adapter_name,
            devices,
            adapter_dropdown,
            adapter_model,
            scan_toggle_button,
            scan_switch_handler,
            discovering,
        };
        model.sync_scan_switch(discovering);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SidebarInput::StoreEvent(event) => match *event {
                BluetoothEvent::AdaptersLoaded(adapters) => {
                    self.adapters = adapters;
                    if let Some(name) = &self.current_adapter_name {
                        self.discovering = self
                            .adapters
                            .iter()
                            .find(|a| &a.name == name)
                            .map(|a| a.discovering)
                            .unwrap_or(false);
                        self.sync_scan_switch(self.discovering);
                    }
                    Self::rebuild_adapter_model(
                        &self.adapter_dropdown,
                        &self.adapter_model,
                        &self.adapters,
                        self.current_adapter_name.as_deref(),
                    );
                },
                BluetoothEvent::AdapterChanged(adapter) => {
                    if self.current_adapter_name.as_deref() == Some(adapter.name.as_str()) {
                        self.discovering = adapter.discovering;
                        self.sync_scan_switch(self.discovering);
                    }
                    if let Some(existing) = self.adapters.iter_mut().find(|a| a.name == adapter.name) {
                        *existing = adapter;
                    }
                    Self::rebuild_adapter_model(
                        &self.adapter_dropdown,
                        &self.adapter_model,
                        &self.adapters,
                        self.current_adapter_name.as_deref(),
                    );
                },
                BluetoothEvent::AdapterSelected(name) => {
                    self.current_adapter_name = Some(name.clone());
                    self.discovering = self
                        .adapters
                        .iter()
                        .find(|a| a.name == name)
                        .map(|a| a.discovering)
                        .unwrap_or(false);
                    self.sync_scan_switch(self.discovering);
                    Self::rebuild_adapter_model(
                        &self.adapter_dropdown,
                        &self.adapter_model,
                        &self.adapters,
                        Some(&name),
                    );
                },
                BluetoothEvent::DevicesCleared => {
                    self.devices.guard().clear();
                },
                BluetoothEvent::DeviceAdded(_) | BluetoothEvent::DeviceChanged(_) | BluetoothEvent::DeviceRemoved(_) => {
                    self.rebuild_devices_list();
                },
                _ => {},
            },
            SidebarInput::AdapterDropdownChanged => {
                let selected = self.adapter_dropdown.selected() as usize;
                if let Some(adapter) = self.adapters.get(selected)
                    && self.current_adapter_name.as_deref() != Some(adapter.name.as_str())
                {
                    let name = adapter.name.clone();
                    let service_adapter = Arc::clone(&self.service_adapter);
                    std::thread::spawn(move || {
                        if let Err(e) = service_adapter.select_adapter(&name) {
                            log::warn!("Failed to select adapter {}: {}", name, e);
                        }
                    });
                }
            },
            SidebarInput::ShowAdapterSettingsClicked => {
                sender.output(SidebarOutput::ShowAdapterSettings).ok();
            },
            SidebarInput::DeviceRow(DeviceRowOutput::Clicked(address)) => {
                sender
                    .output(SidebarOutput::ShowDeviceSettings(address))
                    .ok();
            },
            SidebarInput::ToggleScan(active) => {
                self.discovering = active; // optimiste
                self.scan_toggle_button.set_active(active);
                let service_adapter = Arc::clone(&self.service_adapter);
                std::thread::spawn(move || {
                    let result = if active {
                        service_adapter.start_discovery()
                    } else {
                        service_adapter.stop_discovery()
                    };
                    if let Err(e) = result {
                        log::warn!("Failed to toggle discovery: {}", e);
                    }
                });
            },
        }
    }
}

impl Sidebar {
    fn sync_scan_switch(&self, discovering: bool) {
        self.scan_toggle_button
            .block_signal(&self.scan_switch_handler);
        self.scan_toggle_button.set_active(discovering);
        self.scan_toggle_button
            .unblock_signal(&self.scan_switch_handler);
    }

    /// Rebuild the dropdown's string model and select the matching entry,
    /// without re-triggering `AdapterDropdownChanged` needlessly (comparaison
    /// avant sélection faite côté appelant `AdapterDropdownChanged`, ici on ne
    /// fait qu'un `set_selected` qui redéclenche `notify::selected` — sans
    /// gravité puisque `AdapterDropdownChanged` est un no-op si le nom est déjà
    /// celui sélectionné).
    fn rebuild_adapter_model(dropdown: &adw::ComboRow, model: &gtk::StringList, adapters: &[Adapter], current: Option<&str>) {
        let labels: Vec<&str> = adapters.iter().map(|a| a.alias.as_str()).collect();
        let old_len = model.n_items();
        model.splice(0, old_len, &labels);

        if let Some(current) = current
            && let Some(index) = adapters.iter().position(|a| a.name == current)
        {
            dropdown.set_selected(index as u32);
        }
    }

    /// Rebuild the devices `FactoryVecDeque` from the store's already sorted
    /// list (see `BluetoothStore::get_devices`), instead of applying
    /// `DeviceAdded`/`DeviceChanged` incrementally, which never repositions
    /// an item to its correct rank.
    fn rebuild_devices_list(&mut self) {
        let store = self.service_adapter.store();
        let mut guard = self.devices.guard();
        guard.clear();
        for device in store.get_devices() {
            guard.push_back(device);
        }
    }
}
