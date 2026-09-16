//! Bluetooth monitoring and management via BlueZ (bluer).

use crate::{
    runtime,
    signal::{
        Signal,
        SignalHandler,
    },
};
pub use bluer::agent::AgentHandle;
use bluer::{
    Adapter,
    AdapterEvent,
    Address,
    Device,
    DeviceEvent,
    Modalias,
    Session,
    agent::{
        Agent,
        AuthorizeService,
        DisplayPasskey,
        DisplayPinCode,
        ReqError,
        RequestAuthorization,
        RequestConfirmation,
        RequestPasskey,
        RequestPinCode,
    },
};
use futures::Stream;
use log::{
    debug,
    error,
    info,
    warn,
};
use std::{
    fmt::{
        Display,
        Formatter,
    },
    pin::Pin,
    sync::{
        Arc,
        Mutex,
    },
};
use tokio::sync::{
    mpsc,
    oneshot,
};
use tokio_stream::{
    StreamExt,
    StreamMap,
};

/// Errors that can occur while interacting with the Bluetooth adapter or its devices.
#[derive(Debug)]
pub enum BluetoothError {
    Bluer(bluer::Error),
    InvalidAddress(String),
    NotMonitoring,
}

/// Discriminant and payload of incoming pairing requests handled by the pairing agent.
#[derive(Clone, Debug)]
pub enum PairingRequestKind {
    DisplayPasskey { passkey: u32, entered: u16 },
    DisplayPinCode { pincode: String },
    RequestConfirmation { passkey: u32 },
    RequestAuthorization,
    RequestPinCode,
    RequestPasskey,
    RequestServiceAuthorization { service: String },
}

/// Response returned to BlueZ when resolving a pairing request.
#[derive(Debug)]
pub enum PairingResponse {
    Confirm,
    Reject,
    PinCode(String),
    Passkey(u32),
}

/// Pairing request data emitted towards UI handlers.
#[derive(Clone, Debug)]
pub struct PairingRequest {
    pub device_address: String,
    pub kind: PairingRequestKind,
}

/// Pairing envelope bridging a pairing request with an optional response channel.
pub struct PairingEnvelope {
    pub request: PairingRequest,
    /// `None` for display-only requests (BlueZ does not wait for a response);
    /// `Some` for interactive variants that block until user responds.
    pub responder: Option<oneshot::Sender<PairingResponse>>,
}

/// Snapshot of a Bluetooth device's properties.
#[derive(Clone, Debug)]
pub struct BluetoothDevice {
    pub path: String,
    pub address: String,
    pub name: String,
    pub alias: String,
    pub icon: Option<String>,
    pub adapter: String,
    pub connected: bool,
    pub paired: bool,
    pub trusted: bool,
    pub blocked: bool,
    pub battery_percentage: Option<u8>,
    pub rssi: Option<i16>,
    pub modalias: Option<String>,
}

/// Snapshot of a Bluetooth adapter's properties.
#[derive(Clone, Debug)]
pub struct BluetoothAdapter {
    pub name: String,
    pub address: String,
    pub alias: String,
    pub powered: bool,
    pub discoverable: bool,
    pub discoverable_timeout: u32,
    pub discovering: bool,
}

/// Bluetooth service managing Bluetooth adapter state, device lifecycle, and pairing via BlueZ.
pub struct BluetoothService {
    adapter_powered: Arc<Mutex<bool>>,
    connected_devices: Arc<Mutex<Vec<BluetoothDevice>>>,
    selected_adapter: Arc<Mutex<Option<String>>>,
    adapter_changed: Signal<BluetoothAdapter>,
    device_added: Signal<BluetoothDevice>,
    device_changed: Signal<BluetoothDevice>,
    device_removed: Signal<String>,
    adapter_switch: Arc<tokio::sync::Notify>,
    monitored_adapter: Arc<Mutex<Option<Adapter>>>,
    scan_paused: Arc<Mutex<bool>>,
    scan_toggle: Arc<tokio::sync::Notify>,
    scan_override: Arc<Mutex<Option<bool>>>,
}

/// Monitor for bluetooth state changes via polling (kept for compatibility).
pub struct BluetoothMonitor {
    service: Arc<BluetoothService>,
    last_powered: bool,
    last_connected_count: u8,
    callback: Box<dyn Fn(bool, u8) + Send>,
}

impl Display for BluetoothError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bluer(e) => write!(f, "bluetooth adapter error: {}", e),
            Self::InvalidAddress(addr) => write!(f, "invalid bluetooth address: {}", addr),
            Self::NotMonitoring => write!(
                f,
                "no adapter currently monitored, start_dbus_monitor() first"
            ),
        }
    }
}

impl std::error::Error for BluetoothError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Bluer(e) => Some(e),
            Self::InvalidAddress(_) => None,
            Self::NotMonitoring => None,
        }
    }
}

impl From<bluer::Error> for BluetoothError {
    fn from(error: bluer::Error) -> Self {
        Self::Bluer(error)
    }
}

impl Default for BluetoothService {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothService {
    pub fn new() -> Self {
        let service = Self {
            adapter_powered: Arc::new(Mutex::new(false)),
            connected_devices: Arc::new(Mutex::new(Vec::new())),
            selected_adapter: Arc::new(Mutex::new(None)),
            adapter_changed: Signal::new(),
            device_added: Signal::new(),
            device_changed: Signal::new(),
            device_removed: Signal::new(),
            adapter_switch: Arc::new(tokio::sync::Notify::new()),
            monitored_adapter: Arc::new(Mutex::new(None)),
            scan_paused: Arc::new(Mutex::new(false)),
            scan_toggle: Arc::new(tokio::sync::Notify::new()),
            scan_override: Arc::new(Mutex::new(None)),
        };

        if let Err(e) = service.refresh_state() {
            error!("Failed to initialize bluetooth state: {}", e);
        }

        service
    }

    /// Force the scan (discovery) state on the next monitor connection for the current session.
    /// Consumed once: subsequent reconnections (adapter switch, monitor restart) fall back to
    /// the adapter's live discovery state.
    pub fn set_scan_override(&self, override_value: Option<bool>) {
        *self.scan_override.lock().unwrap() = override_value;
    }

    /// Get current adapter power state.
    pub fn get_powered(&self) -> bool {
        *self.adapter_powered.lock().unwrap()
    }

    /// Get count of connected devices.
    pub fn get_connected_count(&self) -> u8 {
        self.connected_devices
            .lock()
            .unwrap()
            .iter()
            .filter(|d| d.connected)
            .count() as u8
    }

    /// Get list of all devices.
    pub fn get_devices(&self) -> Vec<BluetoothDevice> {
        self.connected_devices.lock().unwrap().clone()
    }

    /// Set adapter power state.
    pub fn set_powered(&self, powered: bool) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let adapter = self.connect_adapter().await?;
            adapter.set_powered(powered).await?;

            info!("Bluetooth adapter powered: {}", powered);
            *self.adapter_powered.lock().unwrap() = powered;

            Ok(())
        })
    }

    /// Toggle adapter power state.
    pub fn toggle_powered(&self) {
        let current = self.get_powered();
        if let Err(e) = self.set_powered(!current) {
            error!("Failed to toggle bluetooth: {}", e);
        }
    }

    /// Set adapter discoverable state.
    pub fn set_discoverable(&self, discoverable: bool) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let adapter = self.connect_adapter().await?;
            adapter.set_discoverable(discoverable).await?;

            info!("Bluetooth adapter discoverable: {}", discoverable);
            Ok(())
        })
    }

    /// Set adapter discoverable timeout in seconds.
    pub fn set_discoverable_timeout(&self, timeout_seconds: u32) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let adapter = self.connect_adapter().await?;
            adapter.set_discoverable_timeout(timeout_seconds).await?;

            info!(
                "Bluetooth adapter discoverable timeout: {}s",
                timeout_seconds
            );
            Ok(())
        })
    }

    /// Set adapter alias.
    pub fn set_adapter_alias(&self, alias: &str) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let adapter = self.connect_adapter().await?;
            adapter.set_alias(alias.to_string()).await?;

            info!("Bluetooth adapter alias set to: {}", alias);
            Ok(())
        })
    }

    /// Set whether a device is trusted.
    pub fn set_device_trusted(&self, address: &str, trusted: bool) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let device = self.get_device(address).await?;
            device.set_trusted(trusted).await?;
            Ok(())
        })
    }

    /// Set whether a device is blocked.
    pub fn set_device_blocked(&self, address: &str, blocked: bool) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let device = self.get_device(address).await?;
            device.set_blocked(blocked).await?;
            Ok(())
        })
    }

    /// Set a device's alias.
    pub fn set_device_alias(&self, address: &str, alias: &str) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let device = self.get_device(address).await?;
            device.set_alias(alias.to_string()).await?;
            Ok(())
        })
    }

    /// Connect to a device.
    pub fn connect_device(&self, address: &str) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let device = self.get_device(address).await?;
            device.connect().await?;

            info!("Connected to bluetooth device: {}", address);
            Ok(())
        })
    }

    /// Disconnect from a device.
    pub fn disconnect_device(&self, address: &str) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let device = self.get_device(address).await?;
            device.disconnect().await?;

            info!("Disconnected from bluetooth device: {}", address);
            Ok(())
        })
    }

    /// Remove (unpair) a device from the current adapter.
    pub fn remove_device(&self, address: &str) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let addr: Address = address
                .parse()
                .map_err(|_| BluetoothError::InvalidAddress(address.to_string()))?;
            let adapter = self.connect_adapter().await?;
            adapter.remove_device(addr).await?;

            info!("Removed bluetooth device: {}", address);
            Ok(())
        })
    }

    /// Connect to the adapter changed signal.
    pub fn connect_adapter_changed<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn(BluetoothAdapter) + Send + 'static,
    {
        self.adapter_changed.connect(callback)
    }

    /// Connect to the device added signal.
    pub fn connect_device_added<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn(BluetoothDevice) + Send + 'static,
    {
        self.device_added.connect(callback)
    }

    /// Connect to the device changed signal.
    pub fn connect_device_changed<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn(BluetoothDevice) + Send + 'static,
    {
        self.device_changed.connect(callback)
    }

    /// Connect to the device removed signal.
    pub fn connect_device_removed<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn(String) + Send + 'static,
    {
        self.device_removed.connect(callback)
    }

    /// Start monitoring bluetooth state changes via the adapter and per-device event streams
    /// (event-based, no polling).
    pub fn start_dbus_monitor<F>(&self, callback: F)
    where
        F: Fn(bool, u8) + Send + 'static,
    {
        let service = self.clone_service();

        std::thread::spawn(move || {
            runtime::runtime().block_on(async move {
                'reconnect: loop {
                    let adapter = match service.connect_adapter().await {
                        Ok(adapter) => adapter,
                        Err(e) => {
                            error!(
                                "Failed to connect to bluetooth adapter for monitoring: {}",
                                e
                            );
                            *service.monitored_adapter.lock().unwrap() = None;
                            return;
                        },
                    };

                    *service.monitored_adapter.lock().unwrap() = Some(adapter.clone());

                    let is_discovering = adapter.is_discovering().await.unwrap_or(false);
                    let override_scan = service.scan_override.lock().unwrap().take();
                    let scan_active = override_scan.unwrap_or(is_discovering);
                    *service.scan_paused.lock().unwrap() = !scan_active;

                    // events() never auto-terminates, unlike discover_devices() which wraps it
                    // and stops the whole stream as soon as Discovering flips back to false
                    // (frequent, normal BlueZ/kernel LE scan cycles, or our own stop_discovery()).
                    let mut adapter_events = match adapter.events().await {
                        Ok(events) => events,
                        Err(e) => {
                            error!("Failed to subscribe to bluetooth adapter events: {}", e);
                            return;
                        },
                    };

                    info!("Bluetooth adapter monitor connected to {}", adapter.name());

                    // When scan is explicitly requested via CLI override (--scan=true), start a discovery session.
                    // Otherwise, do not force discovery: follow the live state and let start_discovery()/stop_discovery()
                    // control discovery sessions on demand.
                    let mut discovery_guard = if override_scan == Some(true) {
                        Self::discovery_stream_if_active(&adapter, &service).await
                    } else {
                        None
                    };

                    // Keyed by address so a removed device's event stream is dropped automatically,
                    // avoiding leaked tasks or dangling subscriptions.
                    let mut device_events: StreamMap<Address, Pin<Box<dyn Stream<Item = DeviceEvent> + Send>>> = StreamMap::new();

                    // events() doesn't replay already-known devices either (unlike
                    // discover_devices()) — useful right after a select_adapter() switch, which
                    // clears the device list on the UI side.
                    if let Ok(known_devices) = Self::get_device_list(&adapter).await {
                        for device in known_devices {
                            if let Ok(addr) = device.address.parse::<Address>() {
                                Self::track_device(&adapter, addr, &mut device_events).await;
                            }
                            service.device_added.emit(device).await;
                        }
                    }

                    loop {
                        tokio::select! {
                            _ = service.adapter_switch.notified() => {
                                info!("Adapter switch requested, reconnecting monitor to {:?}", service.current_adapter_name());
                                continue 'reconnect;
                            },
                            _ = service.scan_toggle.notified() => {
                                if *service.scan_paused.lock().unwrap() {
                                    info!("Bluetooth discovery paused on {}", adapter.name());
                                    discovery_guard = None; // dropping the session token really stops BlueZ discovery
                                } else if discovery_guard.is_none() {
                                    info!("Bluetooth discovery resumed on {}", adapter.name());
                                    discovery_guard = Self::discovery_stream_if_active(&adapter, &service).await;
                                }
                            },
                            event = adapter_events.next() => {
                                let Some(event) = event else {
                                    warn!("Bluetooth adapter event stream ended unexpectedly, reconnecting");
                                    continue 'reconnect;
                                };
                                debug!("Bluetooth adapter event: {:?}", event);
                                Self::handle_adapter_event(&adapter, event, &mut device_events, &service).await;
                            },
                            Some((address, event)) = device_events.next(), if !device_events.is_empty() => {
                                debug!("Bluetooth device {} event: {:?}", address, event);
                                if let Ok(device) = adapter.device(address)
                                    && let Some(parsed) = Self::parse_device(&device).await
                                {
                                    service.device_changed.emit(parsed).await;
                                }
                            },
                            // Only kept alive to hold the discovery session's RAII token; its events
                            // duplicate adapter_events (same underlying D-Bus subscription), so we
                            // just drain and discard them here.
                            maybe_event = async {
                                match discovery_guard.as_mut() {
                                    Some(stream) => stream.next().await,
                                    None => std::future::pending::<Option<AdapterEvent>>().await,
                                }
                            } => {
                                let _ = maybe_event;
                            },
                        }

                        if let Err(e) = service.refresh_state_async().await {
                            warn!("Failed to refresh bluetooth state: {}", e);
                            continue;
                        }

                        callback(service.get_powered(), service.get_connected_count());
                    }
                }
            });
        });
    }

    async fn handle_adapter_event(
        adapter: &Adapter,
        event: AdapterEvent,
        device_events: &mut StreamMap<Address, Pin<Box<dyn Stream<Item = DeviceEvent> + Send>>>,
        service: &BluetoothService,
    ) {
        match event {
            AdapterEvent::DeviceAdded(address) => {
                Self::track_device(adapter, address, device_events).await;
                if let Ok(device) = adapter.device(address)
                    && let Some(parsed) = Self::parse_device(&device).await
                {
                    service.device_added.emit(parsed).await;
                }
            },
            AdapterEvent::DeviceRemoved(address) => {
                device_events.remove(&address);
                service.device_removed.emit(address.to_string()).await;
            },
            AdapterEvent::PropertyChanged(_) => {
                if let Some(info) = Self::parse_adapter(adapter).await {
                    service.adapter_changed.emit(info).await;
                }
            },
        }
    }

    async fn track_device(adapter: &Adapter, address: Address, device_events: &mut StreamMap<Address, Pin<Box<dyn Stream<Item = DeviceEvent> + Send>>>) {
        let Ok(device) = adapter.device(address) else {
            return;
        };

        match device.events().await {
            Ok(events) => {
                device_events.insert(address, Box::pin(events));
            },
            Err(e) => {
                warn!(
                    "Failed to subscribe to events for device {}: {}",
                    address, e
                );
            },
        }
    }

    /// Open a discovery session on `adapter`, unless discovery is currently paused
    /// (`scan_paused`). `bluer`'s `Adapter` has no public `start_discovery`/`stop_discovery`:
    /// the discovery session is only controlled by creating/dropping the `discover_devices()`
    /// stream on the exact same D-Bus connection as the monitor. The returned stream is only
    /// meant to be held alive (its RAII token keeps discovery active) — its items duplicate
    /// what our own `adapter.events()` subscription already receives.
    async fn discovery_stream_if_active(adapter: &Adapter, service: &BluetoothService) -> Option<Pin<Box<dyn Stream<Item = AdapterEvent> + Send>>> {
        if *service.scan_paused.lock().unwrap() {
            return None;
        }

        match adapter.discover_devices().await {
            Ok(events) => Some(Box::pin(events)),
            Err(e) => {
                warn!("Failed to start bluetooth discovery: {}", e);
                None
            },
        }
    }

    /// Create a monitor that checks for bluetooth state changes.
    pub fn monitor_bluetooth<F>(&self, callback: F) -> BluetoothMonitor
    where
        F: Fn(bool, u8) + Send + 'static,
    {
        BluetoothMonitor {
            service: Arc::new(self.clone_service()),
            last_powered: self.get_powered(),
            last_connected_count: self.get_connected_count(),
            callback: Box::new(callback),
        }
    }

    fn refresh_state(&self) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            match self.refresh_state_async().await {
                Ok(_) => {
                    debug!("Bluetooth state refreshed successfully");
                    Ok(())
                },
                Err(e) => {
                    warn!("Failed to refresh bluetooth state: {}", e);
                    Err(e)
                },
            }
        })
    }

    pub(crate) async fn refresh_state_async(&self) -> Result<(), BluetoothError> {
        let adapter = self.connect_adapter().await?;

        match adapter.is_powered().await {
            Ok(powered) => {
                *self.adapter_powered.lock().unwrap() = powered;
                debug!("Adapter powered: {}", powered);
            },
            Err(err) => {
                warn!("Failed to get adapter power state: {}", err);
            },
        }

        match Self::get_device_list(&adapter).await {
            Ok(devices) => {
                *self.connected_devices.lock().unwrap() = devices;
                debug!(
                    "Found {} bluetooth devices",
                    self.connected_devices.lock().unwrap().len()
                );
            },
            Err(err) => {
                warn!("Failed to get device list: {}", err);
            },
        }

        Ok(())
    }

    async fn connect_adapter(&self) -> Result<Adapter, BluetoothError> {
        let session = Session::new().await?;
        let selected = self.selected_adapter.lock().unwrap().clone();

        let adapter = match selected {
            Some(name) => session.adapter(&name)?,
            None => session.default_adapter().await?,
        };

        Ok(adapter)
    }

    /// Resolve a `bluer::Device` from a text address, on the currently selected adapter.
    async fn get_device(&self, address: &str) -> Result<Device, BluetoothError> {
        let addr: Address = address
            .parse()
            .map_err(|_| BluetoothError::InvalidAddress(address.to_string()))?;
        let adapter = self.connect_adapter().await?;
        Ok(adapter.device(addr)?)
    }

    /// List all Bluetooth adapters known to BlueZ.
    pub fn list_adapters(&self) -> Result<Vec<BluetoothAdapter>, BluetoothError> {
        runtime::runtime().block_on(async {
            let session = Session::new().await?;
            let names = session.adapter_names().await?;
            let mut adapters = Vec::with_capacity(names.len());

            for name in names {
                let adapter = session.adapter(&name)?;
                if let Some(info) = Self::parse_adapter(&adapter).await {
                    adapters.push(info);
                }
            }

            Ok(adapters)
        })
    }

    /// Name of the explicitly selected adapter, if any (`None` = BlueZ default adapter).
    pub fn current_adapter_name(&self) -> Option<String> {
        self.selected_adapter.lock().unwrap().clone()
    }

    /// Select which adapter subsequent calls should target, then refresh state for it.
    pub fn select_adapter(&self, name: &str) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let session = Session::new().await?;
            session.adapter(name)?; // fails fast if the adapter doesn't exist

            *self.selected_adapter.lock().unwrap() = Some(name.to_string());
            self.adapter_switch.notify_one();

            self.refresh_state_async().await
        })
    }

    /// Resume discovery on the adapter currently monitored by the D-Bus monitor.
    ///
    /// `bluer`'s `Adapter` has no public `start_discovery`/`stop_discovery`: the discovery
    /// session is only controlled by creating/dropping the `discover_devices()` stream on the
    /// exact same D-Bus connection. We therefore toggle a flag and wake the monitor loop, which
    /// creates that stream, genuinely starting BlueZ discovery — without touching the
    /// independent, never-ending `adapter.events()` stream the monitor is otherwise built on.
    pub fn start_discovery(&self) -> Result<(), BluetoothError> {
        self.monitored_adapter_name()?;
        *self.scan_paused.lock().unwrap() = false;
        self.scan_toggle.notify_one();
        Ok(())
    }

    /// Pause discovery on the adapter currently monitored by the D-Bus monitor.
    pub fn stop_discovery(&self) -> Result<(), BluetoothError> {
        self.monitored_adapter_name()?;
        *self.scan_paused.lock().unwrap() = true;
        self.scan_toggle.notify_one();
        Ok(())
    }

    /// Register a default BlueZ pairing agent on D-Bus.
    ///
    /// Returns an `AgentHandle` managing registration RAII and a receiver channel
    /// for handling incoming pairing envelopes.
    pub fn register_pairing_agent(&self) -> Result<(AgentHandle, mpsc::UnboundedReceiver<PairingEnvelope>), BluetoothError> {
        runtime::runtime().block_on(async {
            let (tx, rx) = mpsc::unbounded_channel();
            let session = Session::new().await?;

            let agent = Agent {
                request_default: true,
                display_passkey: Some(Box::new({
                    let tx = tx.clone();
                    move |req: DisplayPasskey| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            let _ = tx.send(PairingEnvelope {
                                request: PairingRequest {
                                    device_address: req.device.to_string(),
                                    kind: PairingRequestKind::DisplayPasskey {
                                        passkey: req.passkey,
                                        entered: req.entered,
                                    },
                                },
                                responder: None,
                            });
                            Ok(())
                        })
                    }
                })),
                display_pin_code: Some(Box::new({
                    let tx = tx.clone();
                    move |req: DisplayPinCode| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            let _ = tx.send(PairingEnvelope {
                                request: PairingRequest {
                                    device_address: req.device.to_string(),
                                    kind: PairingRequestKind::DisplayPinCode {
                                        pincode: req.pincode,
                                    },
                                },
                                responder: None,
                            });
                            Ok(())
                        })
                    }
                })),
                request_confirmation: Some(Box::new({
                    let tx = tx.clone();
                    move |req: RequestConfirmation| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            let (resp_tx, resp_rx) = oneshot::channel();
                            let envelope = PairingEnvelope {
                                request: PairingRequest {
                                    device_address: req.device.to_string(),
                                    kind: PairingRequestKind::RequestConfirmation {
                                        passkey: req.passkey,
                                    },
                                },
                                responder: Some(resp_tx),
                            };
                            if tx.send(envelope).is_err() {
                                return Err(ReqError::Rejected);
                            }
                            match resp_rx.await {
                                Ok(PairingResponse::Confirm) => Ok(()),
                                _ => Err(ReqError::Rejected),
                            }
                        })
                    }
                })),
                request_authorization: Some(Box::new({
                    let tx = tx.clone();
                    move |req: RequestAuthorization| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            let (resp_tx, resp_rx) = oneshot::channel();
                            let envelope = PairingEnvelope {
                                request: PairingRequest {
                                    device_address: req.device.to_string(),
                                    kind: PairingRequestKind::RequestAuthorization,
                                },
                                responder: Some(resp_tx),
                            };
                            if tx.send(envelope).is_err() {
                                return Err(ReqError::Rejected);
                            }
                            match resp_rx.await {
                                Ok(PairingResponse::Confirm) => Ok(()),
                                _ => Err(ReqError::Rejected),
                            }
                        })
                    }
                })),
                request_pin_code: Some(Box::new({
                    let tx = tx.clone();
                    move |req: RequestPinCode| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            let (resp_tx, resp_rx) = oneshot::channel();
                            let envelope = PairingEnvelope {
                                request: PairingRequest {
                                    device_address: req.device.to_string(),
                                    kind: PairingRequestKind::RequestPinCode,
                                },
                                responder: Some(resp_tx),
                            };
                            if tx.send(envelope).is_err() {
                                return Err(ReqError::Rejected);
                            }
                            match resp_rx.await {
                                Ok(PairingResponse::PinCode(pincode)) => Ok(pincode),
                                _ => Err(ReqError::Rejected),
                            }
                        })
                    }
                })),
                request_passkey: Some(Box::new({
                    let tx = tx.clone();
                    move |req: RequestPasskey| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            let (resp_tx, resp_rx) = oneshot::channel();
                            let envelope = PairingEnvelope {
                                request: PairingRequest {
                                    device_address: req.device.to_string(),
                                    kind: PairingRequestKind::RequestPasskey,
                                },
                                responder: Some(resp_tx),
                            };
                            if tx.send(envelope).is_err() {
                                return Err(ReqError::Rejected);
                            }
                            match resp_rx.await {
                                Ok(PairingResponse::Passkey(passkey)) => Ok(passkey),
                                _ => Err(ReqError::Rejected),
                            }
                        })
                    }
                })),
                authorize_service: Some(Box::new({
                    let tx = tx.clone();
                    move |req: AuthorizeService| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            let (resp_tx, resp_rx) = oneshot::channel();
                            let envelope = PairingEnvelope {
                                request: PairingRequest {
                                    device_address: req.device.to_string(),
                                    kind: PairingRequestKind::RequestServiceAuthorization {
                                        service: req.service.to_string(),
                                    },
                                },
                                responder: Some(resp_tx),
                            };
                            if tx.send(envelope).is_err() {
                                return Err(ReqError::Rejected);
                            }
                            match resp_rx.await {
                                Ok(PairingResponse::Confirm) => Ok(()),
                                _ => Err(ReqError::Rejected),
                            }
                        })
                    }
                })),
                ..Default::default()
            };

            let handle = session.register_agent(agent).await?;
            Ok((handle, rx))
        })
    }

    /// Name of the adapter currently monitored by the D-Bus monitor, if any.
    fn monitored_adapter_name(&self) -> Result<String, BluetoothError> {
        self.monitored_adapter
            .lock()
            .unwrap()
            .as_ref()
            .map(|adapter| adapter.name().to_string())
            .ok_or(BluetoothError::NotMonitoring)
    }

    async fn get_device_list(adapter: &Adapter) -> Result<Vec<BluetoothDevice>, BluetoothError> {
        let addresses = adapter.device_addresses().await?;
        let mut devices = Vec::with_capacity(addresses.len());

        for address in addresses {
            let device = adapter.device(address)?;
            if let Some(device) = Self::parse_device(&device).await {
                devices.push(device);
            }
        }

        Ok(devices)
    }

    async fn parse_device(device: &Device) -> Option<BluetoothDevice> {
        let address = device.address().to_string();
        let adapter = device.adapter_name().to_string();

        let name = device
            .name()
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| address.clone());
        let alias = device.alias().await.unwrap_or_else(|_| name.clone());
        let icon = device.icon().await.ok().flatten();
        let connected = device.is_connected().await.unwrap_or(false);
        let paired = device.is_paired().await.unwrap_or(false);
        let trusted = device.is_trusted().await.unwrap_or(false);
        let blocked = device.is_blocked().await.unwrap_or(false);
        let battery_percentage = device.battery_percentage().await.ok().flatten();
        let rssi = device.rssi().await.ok().flatten();
        let modalias = device
            .modalias()
            .await
            .ok()
            .flatten()
            .map(|m| Self::format_modalias(&m));

        Some(BluetoothDevice {
            path: Self::device_dbus_path(&adapter, &address),
            address,
            name,
            alias,
            icon,
            adapter,
            connected,
            paired,
            trusted,
            blocked,
            battery_percentage,
            rssi,
            modalias,
        })
    }

    async fn parse_adapter(adapter: &Adapter) -> Option<BluetoothAdapter> {
        let name = adapter.name().to_string();
        let address = adapter.address().await.ok()?.to_string();
        let alias = adapter.alias().await.unwrap_or_else(|_| name.clone());
        let powered = adapter.is_powered().await.unwrap_or(false);
        let discoverable = adapter.is_discoverable().await.unwrap_or(false);
        let discoverable_timeout = adapter.discoverable_timeout().await.unwrap_or(0);
        let discovering = adapter.is_discovering().await.unwrap_or(false);

        Some(BluetoothAdapter {
            name,
            address,
            alias,
            powered,
            discoverable,
            discoverable_timeout,
            discovering,
        })
    }

    // bluer's Modalias doesn't implement Display, so we rebuild the standard
    // "source:vXXXXpXXXXdXXXX" modalias string from its parsed fields.
    fn format_modalias(modalias: &Modalias) -> String {
        format!(
            "{}:v{:04X}p{:04X}d{:04X}",
            modalias.source, modalias.vendor, modalias.product, modalias.device
        )
    }

    // bluer keeps the BlueZ object path private; consumers relying on `path` still get the
    // canonical BlueZ layout, reconstructed from the adapter name and device address.
    fn device_dbus_path(adapter_name: &str, address: &str) -> String {
        format!(
            "/org/bluez/{}/dev_{}",
            adapter_name,
            address.replace(':', "_")
        )
    }

    fn clone_service(&self) -> Self {
        Self {
            adapter_powered: Arc::clone(&self.adapter_powered),
            connected_devices: Arc::clone(&self.connected_devices),
            selected_adapter: Arc::clone(&self.selected_adapter),
            adapter_changed: self.adapter_changed.clone(),
            device_added: self.device_added.clone(),
            device_changed: self.device_changed.clone(),
            device_removed: self.device_removed.clone(),
            adapter_switch: Arc::clone(&self.adapter_switch),
            monitored_adapter: Arc::clone(&self.monitored_adapter),
            scan_paused: Arc::clone(&self.scan_paused),
            scan_toggle: Arc::clone(&self.scan_toggle),
            scan_override: Arc::clone(&self.scan_override),
        }
    }
}

impl BluetoothMonitor {
    /// Check for bluetooth state changes and call callback if changed.
    pub fn check(&mut self) {
        if let Err(e) = self.service.refresh_state() {
            debug!("Failed to refresh bluetooth state: {}", e);
            return;
        }

        let current_powered = self.service.get_powered();
        let current_connected_count = self.service.get_connected_count();

        if current_powered != self.last_powered || current_connected_count != self.last_connected_count {
            debug!(
                "Bluetooth state changed: powered={}, connected={}",
                current_powered, current_connected_count
            );
            (self.callback)(current_powered, current_connected_count);
            self.last_powered = current_powered;
            self.last_connected_count = current_connected_count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_envelope_display() {
        let envelope = PairingEnvelope {
            request: PairingRequest {
                device_address: "00:11:22:33:44:55".to_string(),
                kind: PairingRequestKind::DisplayPinCode {
                    pincode: "123456".to_string(),
                },
            },
            responder: None,
        };

        assert_eq!(envelope.request.device_address, "00:11:22:33:44:55");
        assert!(matches!(
            envelope.request.kind,
            PairingRequestKind::DisplayPinCode { ref pincode } if pincode == "123456"
        ));
        assert!(envelope.responder.is_none());
    }

    #[tokio::test]
    async fn test_pairing_envelope_interactive_response() {
        let (tx, rx) = oneshot::channel();
        let envelope = PairingEnvelope {
            request: PairingRequest {
                device_address: "AA:BB:CC:DD:EE:FF".to_string(),
                kind: PairingRequestKind::RequestConfirmation { passkey: 654321 },
            },
            responder: Some(tx),
        };

        assert!(envelope.responder.is_some());
        if let Some(responder) = envelope.responder {
            responder.send(PairingResponse::Confirm).unwrap();
        }

        let response = rx.await.unwrap();
        assert!(matches!(response, PairingResponse::Confirm));
    }
}
