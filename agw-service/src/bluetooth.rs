//! Bluetooth monitoring via BlueZ (D-Bus).

use crate::{
    dbus_const::{
        DBUS_INTERFACE,
        DBUS_OBJECT_MANAGER_INTERFACE,
        DBUS_PATH,
        DBUS_PROPERTIES_CHANGED_MEMBER,
        DBUS_PROPERTIES_INTERFACE,
    },
    runtime,
};
use log::{
    debug,
    error,
    info,
    warn,
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        Mutex,
    },
};
use zbus::{
    Connection,
    zvariant::{
        OwnedObjectPath,
        OwnedValue,
        Value,
    },
};

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
    const BLUEZ_INTERFACE: &str = "org.bluez";
    const BLUEZ_FIRST_CONTROLER_PATH: &str = "/org/bluez/hci0";

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
    pub fn set_powered(&self, powered: bool) -> Result<(), Box<dyn std::error::Error>> {
        runtime::runtime().block_on(async {
            let connection = Connection::system().await?;

            let proxy = zbus::Proxy::new(
                &connection,
                Self::BLUEZ_INTERFACE,
                Self::BLUEZ_FIRST_CONTROLER_PATH,
                DBUS_PROPERTIES_INTERFACE,
            )
            .await?;

            let value = Value::new(powered);
            proxy
                .call::<_, _, ()>("Set", &(Self::buez_adapter_interface(1), "Powered", value))
                .await?;

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

    /// Start monitoring bluetooth state changes via D-Bus signals (event-based, no polling).
    pub fn start_dbus_monitor<F>(&self, callback: F)
    where
        F: Fn(bool, u8) + Send + 'static,
    {
        let service = self.clone_service();

        std::thread::spawn(move || {
            runtime::runtime().block_on(async move {
                use futures::StreamExt;

                match Connection::system().await {
                    Ok(connection) => {
                        info!("Bluetooth D-Bus monitor connected");

                        let match_rule_str = format!(
                            "type='signal',sender='{sender}',interface='{interface}',member='{member}'",
                            sender = Self::BLUEZ_INTERFACE,
                            interface = DBUS_PROPERTIES_INTERFACE,
                            member = DBUS_PROPERTIES_CHANGED_MEMBER
                        );

                        if let Err(e) = connection
                            .call_method(
                                Some(DBUS_INTERFACE),
                                DBUS_PATH,
                                Some(DBUS_INTERFACE),
                                "AddMatch",
                                &(match_rule_str),
                            )
                            .await
                        {
                            error!("Failed to subscribe to BlueZ signals: {}", e);
                            return;
                        }

                        debug!("Subscribed to BlueZ PropertiesChanged signals");

                        let mut stream = zbus::MessageStream::from(&connection);

                        while let Some(msg) = stream.next().await {
                            if let Ok(msg) = msg {
                                let header = msg.header();

                                if let (Some(path), Some(interface), Some(member)) = (header.path(), header.interface(), header.member()) {
                                    let path_str = path.as_str();
                                    let interface_str = interface.as_str();
                                    let member_str = member.as_str();

                                    if interface_str == DBUS_PROPERTIES_INTERFACE
                                        && member_str == DBUS_PROPERTIES_CHANGED_MEMBER
                                        && (path_str.starts_with("/org/bluez/hci0/dev_") || path_str == Self::BLUEZ_FIRST_CONTROLER_PATH)
                                    {
                                        if let Ok((interface_name, changed_properties, _)) = msg
                                            .body()
                                            .deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>()
                                        {
                                            let is_relevant = (interface_name == "org.bluez.Device1" && changed_properties.contains_key("Connected"))
                                                || (interface_name == "org.bluez.Adapter1" && changed_properties.contains_key("Powered"));

                                            if is_relevant {
                                                debug!("Bluetooth state change detected on {}", path_str);

                                                if let Err(e) = service.refresh_state_async().await {
                                                    warn!("Failed to refresh bluetooth state: {}", e);
                                                    continue;
                                                }

                                                let powered = service.get_powered();
                                                let connected_count = service.get_connected_count();

                                                callback(powered, connected_count);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        info!("Bluetooth D-Bus monitor stream ended");
                    },
                    Err(e) => {
                        error!(
                            "Failed to connect to system bus for bluetooth monitoring: {}",
                            e
                        );
                    },
                }
            });
        });
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

    fn refresh_state(&self) -> Result<(), Box<dyn std::error::Error>> {
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

    pub(crate) async fn refresh_state_async(&self) -> Result<(), Box<dyn std::error::Error>> {
        let connection = Connection::system().await?;

        match self.get_adapter_powered(&connection).await {
            Ok(powered) => {
                *self.adapter_powered.lock().unwrap() = powered;
                debug!("Adapter powered: {}", powered);
            },
            Err(err) => {
                warn!("Failed to get adapter power state: {}", err);
            },
        }

        match self.get_device_list(&connection).await {
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

    fn buez_adapter_interface(id: u8) -> String {
        format!("org.bluez.Adapter{}", id)
    }

    async fn get_adapter_powered(&self, connection: &Connection) -> Result<bool, Box<dyn std::error::Error>> {
        let proxy = zbus::Proxy::new(
            connection,
            Self::BLUEZ_INTERFACE,
            Self::BLUEZ_FIRST_CONTROLER_PATH,
            DBUS_PROPERTIES_INTERFACE,
        )
        .await?;

        let variant: OwnedValue = proxy
            .call("Get", &("org.bluez.Adapter1", "Powered"))
            .await?;

        let powered = match variant.downcast_ref::<Value>() {
            Ok(Value::Bool(b)) => b,
            _ => bool::try_from(&variant).unwrap_or(false),
        };

        Ok(powered)
    }

    async fn get_device_list(&self, connection: &Connection) -> Result<Vec<BluetoothDevice>, Box<dyn std::error::Error>> {
        let proxy = zbus::Proxy::new(
            connection,
            Self::BLUEZ_INTERFACE,
            "/",
            DBUS_OBJECT_MANAGER_INTERFACE,
        )
        .await?;

        let managed_objects: HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>> = proxy.call("GetManagedObjects", &()).await?;

        let mut devices = Vec::new();

        for (path, interfaces) in managed_objects {
            if let Some(device_props) = interfaces.get("org.bluez.Device1") {
                if let Some(device) = self.parse_device(path.as_str(), device_props) {
                    devices.push(device);
                }
            }
        }

        Ok(devices)
    }

    fn parse_device(&self, path: &str, props: &HashMap<String, OwnedValue>) -> Option<BluetoothDevice> {
        let address = props
            .get("Address")
            .and_then(|v| <&str>::try_from(v).ok())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let alias = props
            .get("Alias")
            .and_then(|v| <&str>::try_from(v).ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| address.clone());

        let connected = props
            .get("Connected")
            .and_then(|v| bool::try_from(v).ok())
            .unwrap_or(false);

        let paired = props
            .get("Paired")
            .and_then(|v| bool::try_from(v).ok())
            .unwrap_or(false);

        let trusted = props
            .get("Trusted")
            .and_then(|v| bool::try_from(v).ok())
            .unwrap_or(false);

        let battery_percentage = props
            .get("BatteryPercentage")
            .and_then(|v| u8::try_from(v).ok());

        let rssi = props.get("RSSI").and_then(|v| i16::try_from(v).ok());

        Some(BluetoothDevice {
            path: path.to_string(),
            address,
            alias,
            connected,
            paired,
            trusted,
            battery_percentage,
            rssi,
        })
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
