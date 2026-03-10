//! Topbar component and manager implementation.

use crate::{
    APP_SENDER,
    component::{
        info_center::InfoCenter,
        privacy_indicator::PrivacyIndicator,
        quick_settings_menu::QuickSettingsMenu,
        systemd_unit_failed_indicator::SystemdUnitFailedIndicator,
        systray_icons::SystemTrayIcons,
        workspaces::Workspaces,
    },
    system_state::{
        global_service::GlobalSystemService,
        messages::SystemStateUpdate,
    },
};
use agw_lib_outcome::error::AgwError;
use agw_service::{
    power_mode::PowerModeService,
    runtime,
};
use agw_ui_app_launcher::{
    AppLauncherWindow,
    AppLauncherWindowConfig,
};
use agw_ui_notifications::{
    NotificationPopup,
    NotificationPopupConfig,
    NotificationPopupInput,
};
use agw_ui_power_menu::{
    PowerMenuWindow,
    PowerMenuWindowConfig,
};
use futures::{
    StreamExt,
    channel::mpsc,
};
use gtk4::{
    gdk,
    glib,
    prelude::{
        BoxExt,
        Cast,
        DisplayExt,
        GtkWindowExt,
        ListModelExt,
        MonitorExt,
        OrientableExt,
        WidgetExt,
    },
};
use gtk4_layer_shell::{
    Edge,
    Layer,
    LayerShell,
};
use log::{
    debug,
    error,
    info,
};
use relm4::{
    self,
    Component,
    ComponentController,
    ComponentParts,
    ComponentSender,
    Controller,
    RelmWidgetExt,
    SimpleComponent,
    adw,
    gtk,
};
use std::{
    collections::{
        HashMap,
        HashSet,
    },
    sync::{
        Arc,
        Mutex,
        mpsc::{
            Receiver,
            Sender,
        },
    },
};

mod info_center;
mod privacy_indicator;
pub(crate) mod quick_settings_menu;
mod systemd_unit_failed_indicator;
mod systray_icons;
mod workspaces;

enum SystrayEvent {
    ItemRegistered(String),
    ItemUnregistered(String),
    IconChanged(String),
    StatusChanged(String, String),
    MenuLayoutChanged(String, systray_icons::Layout),
}

#[derive(Debug)]
pub enum TopbarManagerInput {
    Show,
    Hide,
    Toggle,
    MonitorAdded(String),
    MonitorRemoved(String),
    RefreshMonitors,
    StartSystrayWatcher,
    DndEnable,
    DndDisable,
    DndToggle,
    DndStatus(Sender<bool>),
    Quit,
}

struct TopbarManager {
    visible: bool,
    topbars: HashMap<String, Controller<TopbarApp>>,
    systray_watcher_started: bool,
    systray_senders: Arc<Mutex<HashMap<String, relm4::Sender<systray_icons::SystemTrayIconsInput>>>>,
    systray_watcher_proxy: Arc<Mutex<Option<systray_icons::StatusNotifierWatcherProxy<'static>>>>,
    systray_names: Arc<Mutex<HashSet<String>>>,
    global_service: Option<Arc<GlobalSystemService>>,
    main_loop: glib::MainLoop,
}

struct TopbarApp {
    info_center: Controller<InfoCenter>,
    workspaces: Controller<Workspaces>,
    systemd_unit_failed_indicator: Controller<SystemdUnitFailedIndicator>,
    privacy_indicator: Controller<PrivacyIndicator>,
    systray_icons: Controller<SystemTrayIcons>,
    quick_settings_menu: Controller<QuickSettingsMenu>,
}

#[relm4::component]
impl SimpleComponent for TopbarApp {
    type Input = ();
    type Output = ();
    type Init = (
        gdk::Monitor,
        Receiver<SystemStateUpdate>,
        Option<Arc<PowerModeService>>,
        Receiver<SystemStateUpdate>,
        Receiver<SystemStateUpdate>,
        Arc<GlobalSystemService>,
    );

    view! {
        gtk::Window {
            set_namespace: Some("agwaita-bar"),
            set_layer: Layer::Top,
            auto_exclusive_zone_enable: (),
            set_anchor: (Edge::Top, true),
            set_anchor: (Edge::Left, true),
            set_anchor: (Edge::Right, true),
            set_margin_all: 0,

            set_default_height: 40,
            set_hexpand: true,

            gtk::CenterBox {
                set_valign: gtk::Align::Center,
                set_orientation: gtk::Orientation::Horizontal,
                set_margin_horizontal: 4,

                #[wrap(Some)]
                set_start_widget = model.workspaces.widget(),

                #[wrap(Some)]
                set_center_widget = model.info_center.widget(),

                #[wrap(Some)]
                set_end_widget = &gtk::Box {
                    set_spacing: 4,

                    model.systemd_unit_failed_indicator.widget(),
                    model.privacy_indicator.widget(),
                    model.systray_icons.widget(),
                    model.quick_settings_menu.widget(),
                },

            }
        }
    }

    fn init(init: Self::Init, root: Self::Root, #[allow(unused_variables)] sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let (monitor, system_state_receiver, power_mode_service, privacy_receiver, systemd_receiver, global_service) = init;
        let monitor_connector = monitor
            .connector()
            .map(|s| s.to_string())
            .unwrap_or_default();
        info!("Initializing topbar for monitor: {}", monitor_connector);

        root.init_layer_shell();
        root.set_monitor(Some(&monitor));

        let info_center: Controller<InfoCenter> = InfoCenter::builder()
            .launch(global_service.clone())
            .detach();

        let workspaces: Controller<Workspaces> = Workspaces::builder()
            .launch((Some(monitor_connector.clone()), global_service.wm_service()))
            .detach();

        let systemd_unit_failed_indicator: Controller<SystemdUnitFailedIndicator> = SystemdUnitFailedIndicator::builder()
            .launch(systemd_receiver)
            .detach();

        let privacy_indicator: Controller<PrivacyIndicator> = PrivacyIndicator::builder()
            .launch(privacy_receiver)
            .detach();

        let systray_icons: Controller<SystemTrayIcons> = SystemTrayIcons::builder().launch(()).detach();

        let quick_settings_menu: Controller<QuickSettingsMenu> = QuickSettingsMenu::builder()
            .launch((
                system_state_receiver,
                global_service.audio_service(),
                global_service.brightness_service(),
                power_mode_service,
            ))
            .detach();

        let model = TopbarApp {
            info_center,
            workspaces,
            systemd_unit_failed_indicator,
            privacy_indicator,
            systray_icons,
            quick_settings_menu,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

impl TopbarManager {
    fn new(global_service: Arc<GlobalSystemService>, main_loop: glib::MainLoop) -> Self {
        TopbarManager {
            visible: true,
            topbars: HashMap::new(),
            systray_watcher_started: false,
            systray_senders: Arc::new(Mutex::new(HashMap::new())),
            systray_watcher_proxy: Arc::new(Mutex::new(None)),
            systray_names: Arc::new(Mutex::new(HashSet::new())),
            global_service: Some(global_service),
            main_loop,
        }
    }

    fn handle_message(&mut self, msg: TopbarManagerInput) {
        match msg {
            TopbarManagerInput::Show => {
                self.visible = true;
                for topbar in self.topbars.values() {
                    topbar.widget().set_visible(true);
                }
            },
            TopbarManagerInput::Hide => {
                self.visible = false;
                for topbar in self.topbars.values() {
                    topbar.widget().set_visible(false);
                }
            },
            TopbarManagerInput::Toggle => {
                self.visible = !self.visible;
                for topbar in self.topbars.values() {
                    topbar.widget().set_visible(self.visible);
                }
            },
            TopbarManagerInput::MonitorAdded(connector) => {
                debug!("Creating topbar for monitor: {}", connector);

                if !self.topbars.contains_key(&connector) {
                    let display = gdk::Display::default().expect("Failed to get default display");
                    let monitors = display.monitors();

                    for i in 0..monitors.n_items() {
                        if let Some(monitor) = monitors
                            .item(i)
                            .and_then(|obj| obj.downcast::<gdk::Monitor>().ok())
                        {
                            if monitor.connector().map(|s| s.to_string()).as_deref() == Some(connector.as_str()) {
                                info!("Creating topbar for monitor: {}", connector);

                                let global_service = self
                                    .global_service
                                    .as_ref()
                                    .expect("GlobalSystemService should be initialized");

                                let receiver = global_service.subscribe();
                                let privacy_receiver = global_service.subscribe();
                                let systemd_receiver = global_service.subscribe();
                                let power_mode_service = global_service.power_mode_service();

                                let topbar = TopbarApp::builder()
                                    .launch((
                                        monitor,
                                        receiver,
                                        power_mode_service,
                                        privacy_receiver,
                                        systemd_receiver,
                                        global_service.clone(),
                                    ))
                                    .detach();

                                topbar.widget().set_visible(self.visible);
                                let systray_sender = topbar.model().systray_icons.sender().clone();
                                if let Ok(mut senders) = self.systray_senders.lock() {
                                    senders.insert(connector.clone(), systray_sender.clone());
                                }
                                if self.systray_watcher_started {
                                    debug!("Systray resync requested for monitor: {}", connector);
                                    self.resync_systray_for_sender(systray_sender);
                                }
                                self.topbars.insert(connector.clone(), topbar);
                                break;
                            }
                        }
                    }
                }
            },
            TopbarManagerInput::MonitorRemoved(connector) => {
                debug!("Removing topbar for monitor: {}", connector);

                if let Ok(mut senders) = self.systray_senders.lock() {
                    senders.remove(&connector);
                }

                if let Some(topbar) = self.topbars.remove(&connector) {
                    topbar.widget().close();
                    drop(topbar);
                }
            },
            TopbarManagerInput::RefreshMonitors => {
                debug!("Refreshing monitors");
                let display = gdk::Display::default().expect("Failed to get default display");
                let monitors = display.monitors();
                let mut current_monitors = std::collections::HashSet::new();
                let mut has_pending_monitor = false;

                for i in 0..monitors.n_items() {
                    if let Some(monitor) = monitors
                        .item(i)
                        .and_then(|obj| obj.downcast::<gdk::Monitor>().ok())
                    {
                        if let Some(connector) = monitor.connector().map(|s| s.to_string()) {
                            debug!("Found monitor with connector: {}", connector);
                            current_monitors.insert(connector.clone());
                            self.handle_message(TopbarManagerInput::MonitorAdded(connector));
                        } else {
                            debug!("Found monitor without connector (likely not ready yet)");
                            has_pending_monitor = true;
                        }
                    }
                }

                let to_remove: Vec<String> = self
                    .topbars
                    .keys()
                    .filter(|k| !current_monitors.contains(*k))
                    .cloned()
                    .collect();

                for connector in to_remove {
                    self.handle_message(TopbarManagerInput::MonitorRemoved(connector));
                }

                if has_pending_monitor {
                    debug!("Scheduling retry for pending monitors");
                    if let Some(sender) = crate::APP_SENDER.get() {
                        let sender = sender.clone();
                        glib::timeout_add_once(std::time::Duration::from_millis(500), move || {
                            debug!("Retrying RefreshMonitors for pending monitors");
                            let _ = sender.send(TopbarManagerInput::RefreshMonitors);
                        });
                    }
                }
            },
            TopbarManagerInput::StartSystrayWatcher => {
                if !self.systray_watcher_started {
                    debug!(
                        "Starting global systray watcher with {} topbars",
                        self.topbars.len()
                    );
                    self.start_systray_watcher();
                    self.systray_watcher_started = true;
                }
            },
            TopbarManagerInput::DndEnable => {
                debug!("Received DndEnable message");
                if let Some(global_service) = &self.global_service {
                    debug!("Calling set_dont_disturb(true)");
                    if let Err(e) = global_service.dnd_service().set_dont_disturb(true) {
                        log::error!("Failed to enable DND: {}", e);
                    } else {
                        debug!("DND enabled successfully");
                    }
                } else {
                    log::error!("No global_service available for DND");
                }
            },
            TopbarManagerInput::DndDisable => {
                if let Some(global_service) = &self.global_service {
                    if let Err(e) = global_service.dnd_service().set_dont_disturb(false) {
                        log::error!("Failed to disable DND: {}", e);
                    }
                }
            },
            TopbarManagerInput::DndToggle => {
                if let Some(global_service) = &self.global_service {
                    global_service.dnd_service().toggle_dont_disturb();
                }
            },
            TopbarManagerInput::DndStatus(sender) => {
                if let Some(global_service) = &self.global_service {
                    let status = global_service.dnd_service().get_dont_disturb();
                    sender.send(status).ok();
                }
            },
            TopbarManagerInput::Quit => {
                info!("Received Quit message, quitting GTK main loop");
                self.main_loop.quit();
            },
        }
    }

    /// Starts a global system tray watcher that broadcasts to all topbars.
    /// This must be called after all topbars are created to avoid race conditions.
    fn start_systray_watcher(&self) {
        use agw_service::systray::StatusNotifierWatcher;
        use systray_icons::{
            StatusNotifierItem,
            SystemTrayIconsInput,
        };

        let systray_senders = self.systray_senders.clone();
        let systray_watcher_proxy = self.systray_watcher_proxy.clone();
        let systray_names_for_events = self.systray_names.clone();
        let systray_names_for_runtime = self.systray_names.clone();

        debug!(
            "Starting system tray watcher for {} topbars",
            self.topbars.len()
        );

        let (event_sender, mut event_receiver) = mpsc::unbounded::<SystrayEvent>();
        let event_sender_for_listener = event_sender.clone();

        relm4::spawn_local(async move {
            while let Some(event) = event_receiver.next().await {
                let topbar_systray_senders: Vec<_> = match systray_senders.lock() {
                    Ok(senders) => senders.values().cloned().collect(),
                    Err(_) => Vec::new(),
                };
                match event {
                    SystrayEvent::ItemRegistered(item_name) => {
                        if let Ok(mut names) = systray_names_for_events.lock() {
                            names.insert(item_name.clone());
                        }
                        let event_sender = event_sender_for_listener.clone();
                        if let Ok(item) = StatusNotifierItem::new(item_name.clone()).await {
                            debug!("Loaded tray item: {}", item.display_name);
                            for sender in &topbar_systray_senders {
                                let _ = sender.send(SystemTrayIconsInput::ItemRegistered(item.clone()));
                            }

                            Self::spawn_item_listener(
                                item.item_proxy.clone(),
                                item.menu_proxy.clone(),
                                item.service_name.clone(),
                                event_sender,
                            );
                        }
                    },
                    SystrayEvent::ItemUnregistered(service_name) => {
                        debug!("Tray item unregistered: {}", service_name);
                        if let Ok(mut names) = systray_names_for_events.lock() {
                            names.remove(&service_name);
                        }
                        for sender in &topbar_systray_senders {
                            let _ = sender.send(SystemTrayIconsInput::ItemUnregistered(service_name.clone()));
                        }
                    },
                    SystrayEvent::IconChanged(service_name) => {
                        for sender in &topbar_systray_senders {
                            let _ = sender.send(SystemTrayIconsInput::IconChanged(service_name.clone()));
                        }
                    },
                    SystrayEvent::StatusChanged(service_name, status) => {
                        for sender in &topbar_systray_senders {
                            let _ = sender.send(SystemTrayIconsInput::StatusChanged(
                                service_name.clone(),
                                status.clone(),
                            ));
                        }
                    },
                    SystrayEvent::MenuLayoutChanged(service_name, layout) => {
                        for sender in &topbar_systray_senders {
                            let _ = sender.send(SystemTrayIconsInput::MenuLayoutChanged(
                                service_name.clone(),
                                layout.clone(),
                            ));
                        }
                    },
                }
            }
        });

        runtime::spawn(async move {
            match StatusNotifierWatcher::start_server().await {
                Ok((conn, _existing_items)) => {
                    if let Ok(watcher) = systray_icons::StatusNotifierWatcherProxy::new(&conn).await {
                        if let Ok(mut proxy) = systray_watcher_proxy.lock() {
                            *proxy = Some(watcher.clone());
                        }
                        let current_items = watcher
                            .registered_status_notifier_items()
                            .await
                            .unwrap_or_default();
                        if let Ok(mut names) = systray_names_for_runtime.lock() {
                            names.extend(current_items.iter().cloned());
                        }
                        for item_name in current_items {
                            let _ = event_sender.unbounded_send(SystrayEvent::ItemRegistered(item_name));
                        }

                        debug!("System tray ready, listening for events");

                        let registered_result = watcher.receive_status_notifier_item_registered().await;
                        let unregistered_result = watcher.receive_status_notifier_item_unregistered().await;

                        if let (Ok(mut registered_stream), Ok(mut unregistered_stream)) = (registered_result, unregistered_result) {
                            use futures::{
                                StreamExt,
                                future::FutureExt,
                            };

                            loop {
                                futures::select! {
                                    signal = registered_stream.next().fuse() => {
                                        if let Some(signal) = signal {
                                            if let Ok(args) = signal.args() {
                                                let item_name = args.service.to_string();
                                                debug!("New tray item registered: {}", item_name);
                                                let _ = event_sender.unbounded_send(SystrayEvent::ItemRegistered(item_name));
                                            }
                                        } else {
                                            break;
                                        }
                                    }
                                    signal = unregistered_stream.next().fuse() => {
                                        if let Some(signal) = signal {
                                            if let Ok(args) = signal.args() {
                                                let service_name = args.service.to_string();
                                                let _ = event_sender.unbounded_send(SystrayEvent::ItemUnregistered(service_name));
                                            }
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to start system tray watcher: {}", e);
                },
            }
        });
    }

    fn spawn_item_listener(
        item_proxy: systray_icons::StatusNotifierItemProxy<'static>,
        menu_proxy: systray_icons::DBusMenuProxy<'static>,
        service_name: String,
        event_sender: mpsc::UnboundedSender<SystrayEvent>,
    ) {
        runtime::spawn(async move {
            let mut new_icon = match item_proxy.receive_new_icon().await {
                Ok(s) => s.boxed(),
                Err(_) => futures::stream::pending().boxed(),
            }
            .fuse();
            let mut new_attention_icon = match item_proxy.receive_new_attention_icon().await {
                Ok(s) => s.boxed(),
                Err(_) => futures::stream::pending().boxed(),
            }
            .fuse();
            let mut new_overlay_icon = match item_proxy.receive_new_overlay_icon().await {
                Ok(s) => s.boxed(),
                Err(_) => futures::stream::pending().boxed(),
            }
            .fuse();
            let mut new_status = match item_proxy.receive_new_status().await {
                Ok(s) => s.boxed(),
                Err(_) => futures::stream::pending().boxed(),
            }
            .fuse();
            let mut menu_layout_updated = match menu_proxy.receive_layout_updated().await {
                Ok(s) => s.boxed(),
                Err(_) => futures::stream::pending().boxed(),
            }
            .fuse();

            loop {
                futures::select! {
                    sig = new_icon.next() => {
                        if sig.is_some() {
                            let _ = event_sender.unbounded_send(SystrayEvent::IconChanged(service_name.clone()));
                        } else { break; }
                    },
                    sig = new_attention_icon.next() => {
                        if sig.is_some() {
                            let _ = event_sender.unbounded_send(SystrayEvent::IconChanged(service_name.clone()));
                        } else { break; }
                    },
                    sig = new_overlay_icon.next() => {
                        if sig.is_some() {
                            let _ = event_sender.unbounded_send(SystrayEvent::IconChanged(service_name.clone()));
                        } else { break; }
                    },
                    sig = new_status.next() => {
                        if let Some(sig) = sig {
                            if let Ok(args) = sig.args() {
                                let _ = event_sender.unbounded_send(SystrayEvent::StatusChanged(service_name.clone(), args.status.clone()));
                            }
                        } else { break; }
                    },
                    sig = menu_layout_updated.next() => {
                        if sig.is_some() {
                            if let Ok((_, layout)) = menu_proxy.get_layout(0, -1, &[]).await {
                                let _ = event_sender.unbounded_send(SystrayEvent::MenuLayoutChanged(service_name.clone(), layout));
                            }
                        } else { break; }
                    },
                }
            }
        });
    }

    fn resync_systray_for_sender(&self, sender: relm4::Sender<systray_icons::SystemTrayIconsInput>) {
        use systray_icons::{
            StatusNotifierItem,
            SystemTrayIconsInput,
            is_name_active,
            watcher_register_host,
            watcher_registered_items,
        };

        let systray_names = self.systray_names.clone();
        let watcher_proxy = self.systray_watcher_proxy.clone();
        relm4::spawn_local(async move {
            let watcher = match watcher_proxy.lock() {
                Ok(proxy) => proxy.clone(),
                Err(_) => None,
            };
            let Some(watcher) = watcher else {
                return;
            };

            watcher_register_host(watcher.clone(), "agwaita-session").await;

            let watcher_items = watcher_registered_items(watcher.clone()).await;
            let mut current_items = std::collections::HashSet::new();
            current_items.extend(watcher_items.into_iter());
            if let Ok(names) = systray_names.lock() {
                current_items.extend(names.iter().cloned());
            }
            if let Ok(mut names) = systray_names.lock() {
                names.clear();
                names.extend(current_items.iter().cloned());
            }

            debug!("Systray resync: {} item(s) to load", current_items.len());

            for item_name in current_items {
                if !is_name_active(&item_name).await {
                    continue;
                }
                match StatusNotifierItem::new(item_name.clone()).await {
                    Ok(item) => {
                        let _ = sender.send(SystemTrayIconsInput::ItemRegistered(item));
                    },
                    Err(e) => {
                        error!("Failed to load tray item {}: {}", item_name, e);
                    },
                }
            }
        });
    }
}

/// Run the topbar application.
///
/// Initializes GTK/Adwaita, creates the global system state service,
///  sets up topbars for all monitors, and starts the GTK main loop.
///
/// # Errors
/// Returns an error if GTK/Adwaita initialization fails.
pub fn run_app() -> Result<(), AgwError> {
    info!("Starting topbar service");
    let _ = adw::init().map_err(|err| AgwError::new(1, err.to_string()));

    info!("Initializing global system state service");
    let global_service = GlobalSystemService::new_instance();

    info!("Starting system state monitors");
    global_service.start();

    // Wrap global_service in Arc for shared access
    let global_service_arc = Arc::new(global_service);

    // Get notification service
    let _notification_service = global_service_arc.notification_service();

    // Clone for app launcher
    let global_service_for_launcher = global_service_arc.clone();

    // Create main loop early so we can pass it to TopbarManager
    debug!("Creating GTK main loop");
    let main_loop = glib::MainLoop::new(None, false);

    let (sender, receiver) = relm4::channel::<TopbarManagerInput>();

    APP_SENDER.set(sender.clone()).ok();

    let mut manager = TopbarManager::new(global_service_arc.clone(), main_loop.clone());

    debug!("Initializing topbars for existing monitors");
    manager.handle_message(TopbarManagerInput::RefreshMonitors);

    // Start systray watcher after topbars are created
    sender.send(TopbarManagerInput::StartSystrayWatcher).ok();

    // Get DND service before moving manager into async block
    let dnd_service = manager.global_service.as_ref().unwrap().dnd_service();
    let initial_dnd = dnd_service.get_dont_disturb();

    debug!("Starting monitor change listener");
    let display = gdk::Display::default().expect("Failed to get default display");
    let monitors = display.monitors();
    debug!("Current monitors count: {}", monitors.n_items());

    let monitors_static = Box::leak(Box::new(monitors));
    let sender_clone = sender.clone();
    monitors_static.connect_items_changed(move |_list, _pos, _removed, _added| {
        debug!(
            "Monitors changed - pos: {}, removed: {}, added: {}",
            _pos, _removed, _added
        );
        sender_clone.send(TopbarManagerInput::RefreshMonitors).ok();
    });

    glib::spawn_future_local(async move {
        debug!("Message handler loop started");
        while let Some(msg) = receiver.recv().await {
            debug!("Handling message: {:?}", msg);
            manager.handle_message(msg);
        }
        debug!("Message handler loop ended");
    });

    // Notifd daemon is already started in GlobalSystemService initialization
    // No need for separate daemon thread - it's integrated into global_service_arc

    // Create notification popup window after a short delay
    let global_service_for_popup = global_service_arc.clone();
    let dnd_service_for_monitor = dnd_service.clone();

    glib::timeout_add_seconds_local_once(1, move || {
        info!("Creating notification popup window");
        let notification_store = global_service_for_popup
            .notification_service_adapter()
            .store();

        // Re-read DND state at creation time to ensure it's up-to-date
        let current_dnd = dnd_service_for_monitor.get_dont_disturb();
        debug!(
            "Popup creation: initial_dnd={}, current_dnd={}",
            initial_dnd, current_dnd
        );

        let popup_config = NotificationPopupConfig {
            store: notification_store,
            dnd_enabled: current_dnd,
        };
        let connector = relm4::ComponentBuilder::<NotificationPopup>::default().launch(popup_config);

        // Monitor DND changes and forward to popup
        let popup_sender = connector.sender().clone();
        debug!("Setting up DND monitor callback");
        let _dnd_monitor = dnd_service_for_monitor.monitor_dnd(move |enabled| {
            debug!("=== DND MONITOR CALLBACK INVOKED === enabled={}", enabled);
            match popup_sender.send(NotificationPopupInput::DndChanged(enabled)) {
                Ok(_) => debug!("Successfully sent DndChanged({}) to popup", enabled),
                Err(e) => error!("Failed to send DndChanged to popup: {:?}", e),
            }
        });
        debug!("DND monitor created and stored");

        // IMPORTANT: Don't detach() - leak the connector and monitor to keep them alive forever
        // This is acceptable because the notification popup lives for the entire session
        Box::leak(Box::new(connector));
        Box::leak(Box::new(_dnd_monitor));
    });

    // Create power menu window
    glib::timeout_add_seconds_local_once(1, move || {
        info!("Creating power menu window");
        let power_menu_config = PowerMenuWindowConfig::default();
        let power_menu = relm4::ComponentBuilder::<PowerMenuWindow>::default().launch(power_menu_config);

        // Store sender globally for power_menu_toggle() to use
        agw_ui_power_menu::message::POWER_MENU_SENDER
            .set(power_menu.sender().clone())
            .ok();

        // Keep the controller alive
        Box::leak(Box::new(power_menu));
    });

    // Create app launcher window
    glib::timeout_add_seconds_local_once(1, move || {
        info!("Creating app launcher window");

        // Get desktop entries service from global service
        let desktop_entries_service = global_service_for_launcher.desktop_entries_service_arc();

        // Get favorites from global service and update desktop entries service
        let favorites_arc = global_service_for_launcher.favorites_service_arc();
        if let Ok(favorites_guard) = favorites_arc.lock() {
            if let Some(ref favorites_service) = *favorites_guard {
                let favorites = favorites_service.get_favorites();
                let favorites_count = favorites.len();

                // Update desktop entries service with favorites
                if let Ok(de_guard) = desktop_entries_service.lock() {
                    if let Some(ref de_service) = *de_guard {
                        de_service.set_favorites(favorites.into_iter().collect());
                        info!("Loaded {} favorites for app launcher", favorites_count);
                    }
                }
            }
        }

        // Pass WMService directly to app launcher
        let wm_service = global_service_for_launcher.wm_service();

        let launcher_config = AppLauncherWindowConfig {
            desktop_entries_service,
            favorites_service: Some(global_service_for_launcher.favorites_service_arc()),
            wm_service,
        };

        let launcher = relm4::ComponentBuilder::<AppLauncherWindow>::default().launch(launcher_config);

        // Store sender globally for app_launcher_toggle() to use
        agw_ui_app_launcher::message::APP_LAUNCHER_SENDER
            .set(launcher.sender().clone())
            .ok();

        // Keep the controller alive
        Box::leak(Box::new(launcher));
    });

    main_loop.run();
    info!("Topbar service stopped");

    Ok(())
}
