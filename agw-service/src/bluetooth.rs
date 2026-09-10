//! Bluetooth monitoring via BlueZ (bluer).

use crate::runtime;
use bluer::{
    Adapter,
    AdapterEvent,
    Address,
    Device,
    DeviceEvent,
    Session,
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
use tokio_stream::{
    StreamExt,
    StreamMap,
};

/// Errors that can occur while interacting with the Bluetooth adapter or its devices.
#[derive(Debug)]
pub enum BluetoothError {
    Bluer(bluer::Error),
}

impl Display for BluetoothError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bluer(e) => write!(f, "bluetooth adapter error: {}", e),
        }
    }
}

impl std::error::Error for BluetoothError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Bluer(e) => Some(e),
        }
    }
}

impl From<bluer::Error> for BluetoothError {
    fn from(error: bluer::Error) -> Self {
        Self::Bluer(error)
    }
}

/// BluetoothService - Manages bluetooth adapter and device monitoring via BlueZ.
pub struct BluetoothService {
    adapter_powered: Arc<Mutex<bool>>,
    connected_devices: Arc<Mutex<Vec<BluetoothDevice>>>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)] // Some fields will be used in the future with bluetooth-manager
pub struct BluetoothDevice {
    pub path: String,
    pub address: String,
    pub alias: String,
    pub connected: bool,
    pub paired: bool,
    pub trusted: bool,
    pub battery_percentage: Option<u8>,
    pub rssi: Option<i16>,
}

impl BluetoothService {
    pub fn new() -> Self {
        let service = Self {
            adapter_powered: Arc::new(Mutex::new(false)),
            connected_devices: Arc::new(Mutex::new(Vec::new())),
        };

        if let Err(e) = service.refresh_state() {
            error!("Failed to initialize bluetooth state: {}", e);
        }

        service
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
    #[allow(dead_code)] // Will be used in the future with bluetooth-manager
    pub fn get_devices(&self) -> Vec<BluetoothDevice> {
        self.connected_devices.lock().unwrap().clone()
    }

    /// Set adapter power state.
    #[allow(dead_code)] // Will be used in the future with bluetooth-manager
    pub fn set_powered(&self, powered: bool) -> Result<(), BluetoothError> {
        runtime::runtime().block_on(async {
            let session = Session::new().await?;
            let adapter = session.default_adapter().await?;
            adapter.set_powered(powered).await?;

            info!("Bluetooth adapter powered: {}", powered);
            *self.adapter_powered.lock().unwrap() = powered;

            Ok(())
        })
    }

    /// Toggle adapter power state.
    #[allow(dead_code)] // Will be used in the future with bluetooth-manager
    pub fn toggle_powered(&self) {
        let current = self.get_powered();
        if let Err(e) = self.set_powered(!current) {
            error!("Failed to toggle bluetooth: {}", e);
        }
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
                let adapter = match Self::connect_adapter().await {
                    Ok(adapter) => adapter,
                    Err(e) => {
                        error!(
                            "Failed to connect to bluetooth adapter for monitoring: {}",
                            e
                        );
                        return;
                    },
                };

                // discover_devices() also drives the discovery session (Partie 2), so scanning
                // for new devices keeps working while we track known ones below.
                let mut adapter_events = match adapter.discover_devices().await {
                    Ok(events) => events,
                    Err(e) => {
                        error!("Failed to start bluetooth discovery: {}", e);
                        return;
                    },
                };

                info!("Bluetooth adapter monitor connected");

                // Keyed by address so a removed device's event stream is dropped automatically,
                // avoiding leaked tasks or dangling subscriptions.
                let mut device_events: StreamMap<Address, Pin<Box<dyn Stream<Item = DeviceEvent> + Send>>> = StreamMap::new();

                loop {
                    tokio::select! {
                        event = adapter_events.next() => {
                            let Some(event) = event else { break; };
                            debug!("Bluetooth adapter event: {:?}", event);
                            Self::handle_adapter_event(&adapter, event, &mut device_events).await;
                        },
                        Some((address, event)) = device_events.next(), if !device_events.is_empty() => {
                            debug!("Bluetooth device {} event: {:?}", address, event);
                        },
                    }

                    if let Err(e) = service.refresh_state_async().await {
                        warn!("Failed to refresh bluetooth state: {}", e);
                        continue;
                    }

                    callback(service.get_powered(), service.get_connected_count());
                }

                info!("Bluetooth adapter monitor stream ended");
            });
        });
    }

    async fn handle_adapter_event(
        adapter: &Adapter,
        event: AdapterEvent,
        device_events: &mut StreamMap<Address, Pin<Box<dyn Stream<Item = DeviceEvent> + Send>>>,
    ) {
        match event {
            AdapterEvent::DeviceAdded(address) => Self::track_device(adapter, address, device_events).await,
            AdapterEvent::DeviceRemoved(address) => {
                device_events.remove(&address);
            },
            AdapterEvent::PropertyChanged(_) => {},
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

    /// Create a monitor that checks for bluetooth state changes.
    #[allow(dead_code)] // Will be used in the future with bluetooth-manager
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
        let adapter = Self::connect_adapter().await?;

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

    async fn connect_adapter() -> Result<Adapter, BluetoothError> {
        let session = Session::new().await?;
        let adapter = session.default_adapter().await?;
        Ok(adapter)
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

        let alias = device.alias().await.unwrap_or_else(|_| address.clone());
        let connected = device.is_connected().await.unwrap_or(false);
        let paired = device.is_paired().await.unwrap_or(false);
        let trusted = device.is_trusted().await.unwrap_or(false);
        let battery_percentage = device.battery_percentage().await.ok().flatten();
        let rssi = device.rssi().await.ok().flatten();

        Some(BluetoothDevice {
            path: Self::device_dbus_path(device.adapter_name(), &address),
            address,
            alias,
            connected,
            paired,
            trusted,
            battery_percentage,
            rssi,
        })
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
        }
    }
}

/// Monitor for bluetooth state changes.
pub struct BluetoothMonitor {
    #[allow(dead_code)] // Will be used in the future with bluetooth-manager
    service: Arc<BluetoothService>,
    last_powered: bool,
    last_connected_count: u8,
    callback: Box<dyn Fn(bool, u8) + Send>,
}

impl BluetoothMonitor {
    /// Check for bluetooth state changes and call callback if changed.
    #[allow(dead_code)] // Will be used in the future with bluetooth-manager
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
