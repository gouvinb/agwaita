use super::{
    messages::SystemStateUpdate,
    privacy_adapter::PrivacyServiceAdapter,
    systemd_failed::{
        SystemdFailedMonitor,
        SystemdFailedService,
    },
};
use crate::component::quick_settings_menu::icons::audio::{
    AudioMonitor,
    AudioService,
};
use agw_service::{
    accent_color::{
        AccentColorMonitor,
        AccentColorService,
    },
    airplane_mode::{
        AirplaneModeMonitor,
        AirplaneModeService,
    },
    battery::BatteryService,
    bluetooth::{
        BluetoothMonitor,
        BluetoothService,
    },
    brightness::BrightnessService,
    calendar::CalendarService,
    dark_mode::{
        DarkModeMonitor,
        DarkModeService,
    },
    dnd::{
        DndMonitor,
        DndService,
    },
    network::{
        NetworkMonitor,
        NetworkService,
    },
    notifications::NotificationService,
    power_mode::PowerModeService,
    runtime,
    time::TimeService,
    wm::WMService,
};
use agw_ui_app_launcher::{
    desktop_entries_service::DesktopEntriesService,
    favorites::FavoritesService,
};
use log::{
    debug,
    info,
};
use std::sync::{
    Arc,
    Mutex,
    mpsc,
};

/// Global singleton for system state monitoring
///
/// This service centralizes all system monitoring (brightness, audio, bluetooth, etc.)
/// and broadcasts updates to all subscribed topbars. This ensures only one set of
/// monitors runs regardless of the number of screens/topbars.
pub struct GlobalSystemService {
    /// List of all subscribers (topbars)
    subscribers: Arc<Mutex<Vec<mpsc::Sender<SystemStateUpdate>>>>,

    brightness_service: Arc<BrightnessService>,
    brightness_service_adapter: Arc<Mutex<Option<super::brightness_adapter::BrightnessServiceAdapter>>>,

    audio_service: Arc<AudioService>,
    audio_monitor: Arc<Mutex<Option<AudioMonitor>>>,

    bluetooth_service: Arc<BluetoothService>,
    #[allow(dead_code)] // Will be used in the future with bluetooth-manager
    bluetooth_monitor: Arc<Mutex<Option<BluetoothMonitor>>>,

    network_service: Arc<NetworkService>,
    #[allow(dead_code)] // Will be used in the future with networkd-manager
    network_monitor: Arc<Mutex<Option<NetworkMonitor>>>,

    dnd_service: Arc<DndService>,
    dnd_monitor: Arc<Mutex<Option<DndMonitor>>>,

    airplane_mode_service: Arc<AirplaneModeService>,
    airplane_mode_monitor: Arc<Mutex<Option<AirplaneModeMonitor>>>,

    power_mode_service: Arc<Mutex<Option<Arc<PowerModeService>>>>,

    battery_service: Arc<Mutex<Option<Arc<BatteryService>>>>,

    accent_color_monitor: Arc<Mutex<Option<AccentColorMonitor>>>,
    dark_mode_monitor: Arc<Mutex<Option<DarkModeMonitor>>>,

    privacy_service: Arc<agw_service::privacy::PrivacyService>,
    privacy_service_adapter: Arc<Mutex<Option<PrivacyServiceAdapter>>>,

    systemd_failed_service: Arc<SystemdFailedService>,
    systemd_failed_monitor: Arc<Mutex<Option<SystemdFailedMonitor>>>,

    notification_service: Arc<NotificationService>,
    notification_service_adapter: Arc<agw_ui_notifications::NotificationServiceAdapter>,

    calendar_service: Arc<CalendarService>,
    calendar_service_adapter: Arc<Mutex<Option<super::calendar_adapter::CalendarServiceAdapter>>>,

    time_service: Arc<TimeService>,

    favorites_service: Arc<Mutex<Option<FavoritesService>>>,

    desktop_entries_service: Arc<Mutex<Option<DesktopEntriesService>>>,

    wm_service: Arc<WMService>,
}

impl GlobalSystemService {
    /// Create a new GlobalSystemService
    ///
    /// This should be called once at application startup in the GTK main thread.
    /// Store the returned instance and pass it to components that need it.
    pub fn new_instance() -> Self {
        debug!("Creating new GlobalSystemService instance");
        Self::new()
    }

    /// Create a new GlobalSystemService (private, use instance() instead)
    fn new() -> Self {
        debug!("Creating GlobalSystemService");

        // Initialize all services
        let brightness_service = Arc::new(BrightnessService::new());
        let audio_service = Arc::new(AudioService::new());
        let bluetooth_service = Arc::new(BluetoothService::new());
        let network_service = Arc::new(NetworkService::new());
        let dnd_service = Arc::new(DndService::new());
        let airplane_mode_service = Arc::new(AirplaneModeService::new());
        let power_mode_service = Arc::new(Mutex::new(None::<Arc<PowerModeService>>));
        let battery_service = Arc::new(Mutex::new(None::<Arc<BatteryService>>));
        let privacy_service = Arc::new(agw_service::privacy::PrivacyService::new());
        let systemd_failed_service = Arc::new(SystemdFailedService::new());
        let calendar_service = Arc::new(CalendarService::new());
        let time_service = Arc::new(TimeService::new());

        // Initialize the global message handler for notifications
        // agw_ui_notifications::message::init(notification_service.clone());

        // Initialize NotificationService (daemon or proxy mode) and adapter
        let (notification_service, notification_service_adapter) = {
            runtime::runtime().block_on(async {
                match NotificationService::init().await {
                    Ok(notification_service) => {
                        info!("NotificationService initialized successfully");

                        let notification_service = Arc::new(notification_service);

                        // Create NotificationStore and adapter
                        let store = Arc::new(agw_ui_notifications::NotificationStore::new());

                        // Initialize global notification store for message passing
                        agw_ui_notifications::message::init(Arc::clone(&store));

                        let adapter = agw_ui_notifications::NotificationServiceAdapter::new(Arc::clone(&notification_service), store);

                        (notification_service, Arc::new(adapter))
                    },
                    Err(e) => {
                        log::error!("Failed to initialize NotificationService: {}", e);
                        panic!("NotificationService is required for the session");
                    },
                }
            })
        };

        // Initialize favorites service
        let favorites_service = match FavoritesService::new() {
            Ok(service) => {
                info!("FavoritesService initialized successfully");
                Arc::new(Mutex::new(Some(service)))
            },
            Err(e) => {
                debug!("Failed to initialize FavoritesService: {}", e);
                Arc::new(Mutex::new(None))
            },
        };

        // Initialize desktop entries service
        let desktop_entries_service = match DesktopEntriesService::new() {
            Ok(service) => {
                info!("DesktopEntriesService initialized successfully");
                Arc::new(Mutex::new(Some(service)))
            },
            Err(e) => {
                debug!("Failed to initialize DesktopEntriesService: {}", e);
                Arc::new(Mutex::new(None))
            },
        };

        let wm_service = runtime::runtime().block_on(async { WMService::init().await });

        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            brightness_service,
            brightness_service_adapter: Arc::new(Mutex::new(None)),
            audio_service,
            audio_monitor: Arc::new(Mutex::new(None)),
            bluetooth_service,
            bluetooth_monitor: Arc::new(Mutex::new(None)),
            network_service,
            network_monitor: Arc::new(Mutex::new(None)),
            dnd_service,
            dnd_monitor: Arc::new(Mutex::new(None)),
            airplane_mode_service,
            airplane_mode_monitor: Arc::new(Mutex::new(None)),
            power_mode_service,
            battery_service,
            accent_color_monitor: Arc::new(Mutex::new(None)),
            dark_mode_monitor: Arc::new(Mutex::new(None)),
            privacy_service,
            privacy_service_adapter: Arc::new(Mutex::new(None)),
            systemd_failed_service,
            systemd_failed_monitor: Arc::new(Mutex::new(None)),
            notification_service,
            notification_service_adapter,
            calendar_service,
            calendar_service_adapter: Arc::new(Mutex::new(None)),
            time_service,
            favorites_service,
            desktop_entries_service,
            wm_service,
        }
    }

    /// Subscribe to system state updates
    ///
    /// Returns a receiver that will receive SystemStateUpdate messages
    /// whenever the system state changes.
    ///
    /// The initial state is sent immediately after subscription.
    pub fn subscribe(&self) -> mpsc::Receiver<SystemStateUpdate> {
        let (sender, receiver) = mpsc::channel();

        debug!("New subscriber registered");

        // Send current state immediately to the new subscriber
        self.send_initial_state(&sender);

        // Add to subscribers list for future updates
        self.subscribers.lock().unwrap().push(sender);

        receiver
    }

    /// Send the current system state to a new subscriber
    fn send_initial_state(&self, sender: &mpsc::Sender<SystemStateUpdate>) {
        debug!("Sending initial state to new subscriber");

        // Get current state from all services
        let brightness = self.brightness_service.get_brightness();
        let _ = sender.send(SystemStateUpdate::Brightness(brightness));

        let volume = self.audio_service.get_volume();
        let muted = self.audio_service.is_muted();
        let _ = sender.send(SystemStateUpdate::Audio(volume, muted));

        let bluetooth_powered = self.bluetooth_service.get_powered();
        let bluetooth_connected = self.bluetooth_service.get_connected_count();
        let _ = sender.send(SystemStateUpdate::Bluetooth(
            bluetooth_powered,
            bluetooth_connected,
        ));

        let network_state = self.network_service.get_state();
        let _ = sender.send(SystemStateUpdate::Network(
            network_state.network_type,
            network_state.connected,
            network_state.wifi_strength,
        ));

        let dnd_enabled = self.dnd_service.get_dont_disturb();
        let _ = sender.send(SystemStateUpdate::Dnd(dnd_enabled));

        let dark_mode_enabled = DarkModeService::is_enabled();
        let _ = sender.send(SystemStateUpdate::DarkMode(dark_mode_enabled));

        let airplane_mode_enabled = self.airplane_mode_service.is_enabled();
        let _ = sender.send(SystemStateUpdate::AirplaneMode(airplane_mode_enabled));

        // Power profile - may not be available yet
        if let Some(ref service) = *self.power_mode_service.lock().unwrap() {
            let profile = service.get_active_profile();
            let _ = sender.send(SystemStateUpdate::PowerProfile(profile));
        }

        // Battery - may not be available yet
        if let Some(ref service) = *self.battery_service.lock().unwrap() {
            let state = service.get_state();
            let _ = sender.send(SystemStateUpdate::Battery(
                state.percentage,
                state.is_charging,
                state.is_present,
            ));
        }

        // Accent color
        let accent_color = AccentColorService::get_accent_color();
        let _ = sender.send(SystemStateUpdate::AccentColor(accent_color));

        // Privacy usage
        let privacy_usage = self.privacy_service.get_usage();
        let _ = sender.send(SystemStateUpdate::Privacy(privacy_usage));

        // Systemd failed units
        let systemd_failed = self.systemd_failed_service.get_failed_units();
        let _ = sender.send(SystemStateUpdate::SystemdFailed(systemd_failed));

        debug!("Initial state sent to subscriber");
    }

    /// Start monitoring system state
    ///
    /// This should be called once at application startup.
    /// It initializes all monitors and sets up callbacks to broadcast updates.
    pub fn start(&self) {
        debug!("Starting GlobalSystemService monitors");

        // Setup brightness monitor
        self.setup_brightness_monitor();

        // Setup audio monitor
        self.setup_audio_monitor();

        // Setup bluetooth monitor
        self.setup_bluetooth_monitor();

        // Setup network monitor
        self.setup_network_monitor();

        // Setup DND monitor
        self.setup_dnd_monitor();

        // Setup airplane mode monitor
        self.setup_airplane_mode_monitor();

        // Setup power mode monitor (async)
        self.setup_power_mode_monitor();

        // Setup battery monitor (async)
        self.setup_battery_monitor();

        // Setup accent color monitor
        self.setup_accent_color_monitor();

        // Setup dark mode monitor
        self.setup_dark_mode_monitor();

        // Setup privacy monitor
        self.setup_privacy_monitor();

        // Setup systemd failed monitor
        self.setup_systemd_failed_monitor();

        // Setup calendar monitor
        self.setup_calendar_monitor();

        // Start time service tick
        self.time_service.start();

        // Setup desktop entries monitor
        self.setup_desktop_entries_monitor();

        debug!("GlobalSystemService monitors started");
    }

    fn setup_brightness_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let service = self.brightness_service.clone();

        // Create a channel to forward brightness updates to all subscribers
        let (sender, receiver) = mpsc::channel::<SystemStateUpdate>();

        // Spawn a thread to forward brightness updates to all subscribers
        std::thread::spawn(move || {
            while let Ok(update) = receiver.recv() {
                let mut subs = subscribers.lock().unwrap();
                subs.retain(|sender| sender.send(update.clone()).is_ok());
            }
        });

        let adapter = super::brightness_adapter::BrightnessServiceAdapter::new((*service).clone(), sender);

        *self.brightness_service_adapter.lock().unwrap() = Some(adapter);
        debug!("Brightness monitor started");
    }

    fn setup_audio_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let service = self.audio_service.clone();

        // Setup event-based monitor (no polling needed!)
        let monitor = service.monitor_audio(move |volume, muted| {
            debug!("Audio changed: {:.1}%, muted={}", volume * 100.0, muted);
            let update = SystemStateUpdate::Audio(volume, muted);

            let mut subs = subscribers.lock().unwrap();
            subs.retain(|sender| sender.send(update.clone()).is_ok());
        });

        *self.audio_monitor.lock().unwrap() = Some(monitor);
    }

    fn setup_bluetooth_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let service = self.bluetooth_service.clone();

        // Start D-Bus event-based monitor (no polling!)
        service.start_dbus_monitor(move |powered, connected_count| {
            debug!(
                "Bluetooth changed: powered={}, connected={}",
                powered, connected_count
            );
            let update = SystemStateUpdate::Bluetooth(powered, connected_count);

            let mut subs = subscribers.lock().unwrap();
            subs.retain(|sender| sender.send(update.clone()).is_ok());
        });

        info!("Bluetooth D-Bus monitor started (event-based)");
    }

    fn setup_network_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let service = self.network_service.clone();

        // Get and broadcast initial state
        let initial_state = service.get_state();
        debug!(
            "Initial network: type={:?}, connected={}, strength={}%",
            initial_state.network_type, initial_state.connected, initial_state.wifi_strength
        );

        let update = SystemStateUpdate::Network(
            initial_state.network_type,
            initial_state.connected,
            initial_state.wifi_strength,
        );
        let mut subs = subscribers.lock().unwrap();
        subs.retain(|sender| sender.send(update.clone()).is_ok());
        drop(subs);

        // Start hybrid monitor: D-Bus events + conditional WiFi polling
        service.start_hybrid_monitor(move |network_type, connected, wifi_strength| {
            debug!(
                "Network changed: type={:?}, connected={}, strength={}%",
                network_type, connected, wifi_strength
            );
            let update = SystemStateUpdate::Network(network_type, connected, wifi_strength);

            let mut subs = subscribers.lock().unwrap();
            subs.retain(|sender| sender.send(update.clone()).is_ok());
        });

        info!("Network hybrid monitor started (D-Bus + conditional polling)");
    }

    fn setup_dnd_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let service = self.dnd_service.clone();

        // Setup event-based monitor (no polling needed!)
        let monitor = service.monitor_dnd(move |dont_disturb| {
            debug!("DND changed: enabled={}", dont_disturb);
            let update = SystemStateUpdate::Dnd(dont_disturb);

            let mut subs = subscribers.lock().unwrap();
            subs.retain(|sender| sender.send(update.clone()).is_ok());
        });

        *self.dnd_monitor.lock().unwrap() = Some(monitor);
    }

    fn setup_airplane_mode_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let service = self.airplane_mode_service.clone();

        // Setup polling monitor (rfkill doesn't support events)
        let monitor = service.monitor_airplane_mode(move |enabled| {
            debug!("Airplane mode changed: enabled={}", enabled);
            let update = SystemStateUpdate::AirplaneMode(enabled);

            let mut subs = subscribers.lock().unwrap();
            subs.retain(|sender| sender.send(update.clone()).is_ok());
        });

        *self.airplane_mode_monitor.lock().unwrap() = Some(monitor);
        info!("Airplane mode monitor started (polling every 2s)");
    }

    fn setup_power_mode_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let power_mode_service_clone = self.power_mode_service.clone();

        runtime::spawn(async move {
            let service = Arc::new(PowerModeService::new().await);
            let current_profile = service.get_active_profile();

            debug!("Initial power profile: {:?}", current_profile);

            // Send initial state
            let update = SystemStateUpdate::PowerProfile(current_profile);
            let mut subs = subscribers.lock().unwrap();
            subs.retain(|sender| sender.send(update.clone()).is_ok());
            drop(subs);

            // Send service ready notification
            debug!("Broadcasting PowerModeServiceReady to all subscribers");
            let service_ready = SystemStateUpdate::PowerModeServiceReady(service.clone());
            let mut subs = subscribers.lock().unwrap();
            subs.retain(|sender| sender.send(service_ready.clone()).is_ok());
            drop(subs);

            // Setup monitor
            let subscribers_clone = subscribers.clone();
            let _monitor = service.monitor_power_mode(move |profile| {
                debug!("Power profile changed: {:?}", profile);
                let update = SystemStateUpdate::PowerProfile(profile);

                let mut subs = subscribers_clone.lock().unwrap();
                subs.retain(|sender| sender.send(update.clone()).is_ok());
            });

            *power_mode_service_clone.lock().unwrap() = Some(service);
        });
    }

    fn setup_battery_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let battery_service_clone = self.battery_service.clone();

        runtime::spawn(async move {
            let service = Arc::new(BatteryService::new().await);
            let battery_state = service.get_state();

            debug!(
                "Initial battery: {:.1}%, charging={}, present={}",
                battery_state.percentage * 100.0,
                battery_state.is_charging,
                battery_state.is_present
            );

            // Send initial state
            let update = SystemStateUpdate::Battery(
                battery_state.percentage,
                battery_state.is_charging,
                battery_state.is_present,
            );
            let mut subs = subscribers.lock().unwrap();
            subs.retain(|sender| sender.send(update.clone()).is_ok());
            drop(subs);

            // Start D-Bus event-based monitor (no polling!)
            let subscribers_clone = subscribers.clone();
            service.start_dbus_monitor(move |percentage, is_charging, is_present| {
                debug!(
                    "Battery changed: {:.1}%, charging={}, present={}",
                    percentage * 100.0,
                    is_charging,
                    is_present
                );
                let update = SystemStateUpdate::Battery(percentage, is_charging, is_present);

                let mut subs = subscribers_clone.lock().unwrap();
                subs.retain(|sender| sender.send(update.clone()).is_ok());
            });

            *battery_service_clone.lock().unwrap() = Some(service);

            info!("Battery D-Bus monitor started (event-based)");
        });
    }

    fn setup_accent_color_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let accent_color_monitor_ref = self.accent_color_monitor.clone();

        // Get initial color
        let initial_color = AccentColorService::get_accent_color();
        debug!("Initial accent color: {:?}", initial_color);

        // Start monitoring
        let monitor = AccentColorService::monitor_accent_color(move |color| {
            debug!("Accent color changed to: {:?}", color);
            let update = SystemStateUpdate::AccentColor(color);

            let mut subs = subscribers.lock().unwrap();
            subs.retain(|sender| sender.send(update.clone()).is_ok());
        });

        *accent_color_monitor_ref.lock().unwrap() = monitor;
    }

    fn setup_dark_mode_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let dark_mode_monitor_ref = self.dark_mode_monitor.clone();

        let initial_state = DarkModeService::is_enabled();
        debug!("Initial dark mode: {}", initial_state);

        let monitor = DarkModeService::monitor_dark_mode(move |enabled| {
            debug!("Dark mode changed: enabled={}", enabled);
            let update = SystemStateUpdate::DarkMode(enabled);

            let mut subs = subscribers.lock().unwrap();
            subs.retain(|sender| sender.send(update.clone()).is_ok());
        });

        *dark_mode_monitor_ref.lock().unwrap() = monitor;
    }
    fn setup_privacy_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let service = self.privacy_service.clone();

        // Create a channel to forward privacy updates to all subscribers
        let (sender, receiver) = mpsc::channel::<SystemStateUpdate>();

        // Spawn a thread to forward privacy updates to all subscribers
        std::thread::spawn(move || {
            while let Ok(update) = receiver.recv() {
                let mut subs = subscribers.lock().unwrap();
                subs.retain(|sender| sender.send(update.clone()).is_ok());
            }
        });

        let adapter = PrivacyServiceAdapter::new((*service).clone(), sender);

        *self.privacy_service_adapter.lock().unwrap() = Some(adapter);
        debug!("Privacy monitor started");
    }

    fn setup_systemd_failed_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let service = self.systemd_failed_service.clone();

        let (sender, receiver) = mpsc::channel::<SystemStateUpdate>();

        std::thread::spawn(move || {
            while let Ok(update) = receiver.recv() {
                let mut subs = subscribers.lock().unwrap();
                subs.retain(|sender| sender.send(update.clone()).is_ok());
            }
        });

        let monitor = SystemdFailedMonitor::new((*service).clone(), sender);

        *self.systemd_failed_monitor.lock().unwrap() = Some(monitor);
        debug!("Systemd failed monitor started");
    }

    fn setup_calendar_monitor(&self) {
        let subscribers = self.subscribers.clone();
        let service = self.calendar_service.clone();

        // Create a channel to forward calendar updates to all subscribers
        let (sender, receiver) = mpsc::channel::<SystemStateUpdate>();

        // Spawn a thread to forward calendar updates to all subscribers
        std::thread::spawn(move || {
            while let Ok(update) = receiver.recv() {
                let mut subs = subscribers.lock().unwrap();
                subs.retain(|sender| sender.send(update.clone()).is_ok());
            }
        });

        let adapter = super::calendar_adapter::CalendarServiceAdapter::new((*service).clone(), sender);

        *self.calendar_service_adapter.lock().unwrap() = Some(adapter);
        debug!("Calendar monitor started");
    }

    /// Get DND service
    pub fn dnd_service(&self) -> Arc<DndService> {
        Arc::clone(&self.dnd_service)
    }

    /// Get Airplane Mode service
    pub fn airplane_mode_service(&self) -> Arc<AirplaneModeService> {
        Arc::clone(&self.airplane_mode_service)
    }

    /// Get Power Mode service (may not be available yet)
    pub fn power_mode_service(&self) -> Option<Arc<PowerModeService>> {
        self.power_mode_service.lock().unwrap().clone()
    }

    /// Get Notification service
    pub fn notification_service(&self) -> Arc<NotificationService> {
        Arc::clone(&self.notification_service)
    }

    pub fn notification_service_adapter(&self) -> &Arc<agw_ui_notifications::NotificationServiceAdapter> {
        &self.notification_service_adapter
    }

    /// Get Calendar service
    /// Get calendar service reference
    pub fn calendar_service(&self) -> &Arc<CalendarService> {
        &self.calendar_service
    }

    /// Get Time service reference
    pub fn time_service(&self) -> &Arc<TimeService> {
        &self.time_service
    }

    /// Get Favorites service Arc (may not be available if GSettings is not accessible)
    pub fn favorites_service_arc(&self) -> Arc<Mutex<Option<FavoritesService>>> {
        self.favorites_service.clone()
    }

    /// Get Desktop Entries service Arc (may not be available if initialization failed)
    pub fn desktop_entries_service_arc(&self) -> Arc<Mutex<Option<DesktopEntriesService>>> {
        self.desktop_entries_service.clone()
    }

    /// Get WM service (window manager integration)
    pub fn wm_service(&self) -> Arc<WMService> {
        self.wm_service.clone()
    }

    /// Get Tray service (system tray integration) - may not be initialized yet
    pub fn audio_service(&self) -> Arc<AudioService> {
        self.audio_service.clone()
    }

    pub fn brightness_service(&self) -> Arc<BrightnessService> {
        self.brightness_service.clone()
    }

    fn setup_desktop_entries_monitor(&self) {
        if let Some(ref service) = *self.desktop_entries_service.lock().unwrap() {
            service.start_monitoring();
            info!("Desktop entries monitoring started");
        } else {
            debug!("Desktop entries service not available, monitoring not started");
        }
    }
}
