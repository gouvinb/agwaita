//! Network monitoring via systemd-networkd/iwd (D-Bus + polling).

use crate::{
    dbus_const::{
        DBUS_INTERFACE,
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
    process::Command,
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkType {
    Wired,
    Wifi,
    None,
}

#[derive(Debug, Clone)]
pub struct NetworkState {
    pub network_type: NetworkType,
    pub connected: bool,
    pub wifi_strength: u8,
    pub interface: Option<String>,
}

/// NetworkService - Manages network state monitoring using systemd-networkd/iwd.
/// Uses `ip` and `iw` commands to detect network state.
pub struct NetworkService {
    network_type: Arc<Mutex<NetworkType>>,
    connected: Arc<Mutex<bool>>,
    wifi_strength: Arc<Mutex<u8>>,
    interface: Arc<Mutex<Option<String>>>,
}

impl NetworkService {
    pub fn new() -> Self {
        let service = Self {
            network_type: Arc::new(Mutex::new(NetworkType::None)),
            connected: Arc::new(Mutex::new(false)),
            wifi_strength: Arc::new(Mutex::new(0)),
            interface: Arc::new(Mutex::new(None)),
        };

        if let Some(state) = service.get_network_state() {
            *service.network_type.lock().unwrap() = state.network_type;
            *service.connected.lock().unwrap() = state.connected;
            *service.wifi_strength.lock().unwrap() = state.wifi_strength;
            *service.interface.lock().unwrap() = state.interface;
        }

        service
    }

    /// Get current network state.
    pub fn get_state(&self) -> NetworkState {
        NetworkState {
            network_type: self.network_type.lock().unwrap().clone(),
            connected: *self.connected.lock().unwrap(),
            wifi_strength: *self.wifi_strength.lock().unwrap(),
            interface: self.interface.lock().unwrap().clone(),
        }
    }

    /// Start hybrid monitoring: D-Bus events + conditional WiFi strength polling.
    pub fn start_hybrid_monitor<F>(&self, callback: F)
    where
        F: Fn(NetworkType, bool, u8) + Send + 'static + Clone,
    {
        let service = self.clone_service();
        let callback = Arc::new(Mutex::new(callback));
        let wifi_polling_active = Arc::new(AtomicBool::new(false));

        let service_dbus = service.clone_service();
        let callback_dbus = Arc::clone(&callback);
        let wifi_polling_ref = Arc::clone(&wifi_polling_active);

        std::thread::spawn(move || {
            runtime::runtime().block_on(async move {
                use futures::StreamExt;
                use zbus::Connection;

                match Connection::system().await {
                    Ok(connection) => {
                        info!("Network D-Bus monitor connected");

                        let match_rule_str = format!(
                            "type='signal',interface='{interface}',member='{member}',path_namespace='/net/connman/iwd'",
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
                            error!("Failed to subscribe to iwd signals: {}", e);
                            return;
                        }

                        info!("Subscribed to iwd PropertiesChanged signals");

                        let mut stream = zbus::MessageStream::from(&connection);

                        while let Some(msg) = stream.next().await {
                            if let Ok(msg) = msg {
                                let header = msg.header();

                                if let (Some(_path), Some(interface), Some(member)) = (header.path(), header.interface(), header.member()) {
                                    let interface_str = interface.as_str();
                                    let member_str = member.as_str();

                                    if interface_str == DBUS_PROPERTIES_INTERFACE && member_str == DBUS_PROPERTIES_CHANGED_MEMBER {
                                        use zbus::zvariant::OwnedValue;

                                        if let Ok((interface_name, changed_properties, _)) = msg.body().deserialize::<(
                                            String,
                                            std::collections::HashMap<String, OwnedValue>,
                                            Vec<String>,
                                        )>() {
                                            let is_station_state = interface_name == "net.connman.iwd.Station" && changed_properties.contains_key("State");
                                            let is_network_connected =
                                                interface_name == "net.connman.iwd.Network" && changed_properties.contains_key("Connected");
                                            let is_adapter_powered = interface_name == "net.connman.iwd.Adapter" && changed_properties.contains_key("Powered");

                                            if is_station_state || is_network_connected || is_adapter_powered {
                                                service_dbus.refresh_state();
                                                let state = service_dbus.get_state();

                                                let should_poll = state.network_type == NetworkType::Wifi && state.connected;
                                                wifi_polling_ref.store(should_poll, Ordering::Relaxed);

                                                if let Ok(cb) = callback_dbus.lock() {
                                                    cb(state.network_type, state.connected, state.wifi_strength);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        info!("Network D-Bus monitor stream ended");
                    },
                    Err(e) => {
                        error!(
                            "Failed to connect to system bus for network monitoring: {}",
                            e
                        );
                    },
                }
            });
        });

        let service_poll = service.clone_service();
        let callback_poll = Arc::clone(&callback);
        let wifi_polling_poll = Arc::clone(&wifi_polling_active);

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3));

                if wifi_polling_poll.load(Ordering::Relaxed) {
                    service_poll.refresh_state();
                    let state = service_poll.get_state();

                    if state.network_type == NetworkType::Wifi && state.connected {
                        if let Ok(cb) = callback_poll.lock() {
                            cb(state.network_type, state.connected, state.wifi_strength);
                        }
                    }
                }
            }
        });

        info!("Network hybrid monitor started (D-Bus events + conditional WiFi polling)");
    }

    /// Create a monitor that checks for network state changes.
    #[allow(dead_code)] // Will be used in the future with networkd-manager
    pub fn monitor_network<F>(&self, callback: F) -> NetworkMonitor
    where
        F: Fn(NetworkType, bool, u8) + Send + 'static,
    {
        let state = self.get_state();
        NetworkMonitor {
            service: Arc::new(self.clone_service()),
            last_type: state.network_type,
            last_connected: state.connected,
            last_strength: state.wifi_strength,
            callback: Box::new(callback),
        }
    }

    fn get_network_state(&self) -> Option<NetworkState> {
        let output = Command::new("ip")
            .args(&["-br", "link", "show", "up"])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.trim().split('\n').collect();

        let wired_interfaces: Vec<&str> = lines
            .iter()
            .filter(|line| !line.starts_with("lo") && (line.contains("eth") || line.contains("enp") || line.contains("eno")))
            .copied()
            .collect();

        let wifi_interfaces: Vec<&str> = lines
            .iter()
            .filter(|line| !line.starts_with("lo") && (line.contains("wlan") || line.contains("wlp")))
            .copied()
            .collect();

        let route_output = Command::new("ip")
            .args(&["route", "show", "default"])
            .output()
            .ok()?;

        let route_stdout = String::from_utf8_lossy(&route_output.stdout);
        let has_default_route = !route_stdout.trim().is_empty();

        let wifi_interface_default = wifi_interfaces.first();
        let wired_interface_default = wired_interfaces.first();

        if !has_default_route {
            if let Some(wifi_iface) = wifi_interface_default {
                let interface_name = wifi_iface.split_whitespace().next()?.to_string();

                match Command::new("iw")
                    .args(&["dev", &interface_name, "link"])
                    .output()
                {
                    Ok(iw_output) => {
                        let iw_stdout = String::from_utf8_lossy(&iw_output.stdout);

                        if !iw_stdout.contains("Not connected") && iw_stdout.contains("Connected to") {
                            let strength = Self::parse_wifi_strength(&iw_stdout);
                            return Some(NetworkState {
                                network_type: NetworkType::Wifi,
                                connected: true,
                                wifi_strength: strength,
                                interface: Some(interface_name),
                            });
                        }
                    },
                    Err(_) => {},
                }

                return Some(NetworkState {
                    network_type: NetworkType::Wifi,
                    connected: false,
                    wifi_strength: 0,
                    interface: Some(interface_name),
                });
            }

            if let Some(wired_iface) = wired_interface_default {
                let interface_name = wired_iface.split_whitespace().next()?.to_string();
                return Some(NetworkState {
                    network_type: NetworkType::Wired,
                    connected: false,
                    wifi_strength: 0,
                    interface: Some(interface_name),
                });
            }
            return Some(NetworkState {
                network_type: NetworkType::None,
                connected: false,
                wifi_strength: 0,
                interface: None,
            });
        }

        if wired_interface_default.is_some() && (route_stdout.contains("eth") || route_stdout.contains("enp") || route_stdout.contains("eno")) {
            let interface_name = wired_interface_default?
                .split_whitespace()
                .next()?
                .to_string();
            return Some(NetworkState {
                network_type: NetworkType::Wired,
                connected: true,
                wifi_strength: 0,
                interface: Some(interface_name),
            });
        }

        if let Some(wifi_iface) = wifi_interface_default {
            let interface_name = wifi_iface.split_whitespace().next()?.to_string();

            match Command::new("iw")
                .args(&["dev", &interface_name, "link"])
                .output()
            {
                Ok(iw_output) => {
                    let iw_stdout = String::from_utf8_lossy(&iw_output.stdout);

                    if iw_stdout.contains("Not connected") {
                        return Some(NetworkState {
                            network_type: NetworkType::Wifi,
                            connected: false,
                            wifi_strength: 0,
                            interface: Some(interface_name),
                        });
                    }

                    let strength = Self::parse_wifi_strength(&iw_stdout);

                    debug!(
                        "WiFi interface {} signal: {} dBm ({}%)",
                        interface_name,
                        Self::extract_signal_dbm(&iw_stdout).unwrap_or(0),
                        strength
                    );

                    return Some(NetworkState {
                        network_type: NetworkType::Wifi,
                        connected: true,
                        wifi_strength: strength,
                        interface: Some(interface_name),
                    });
                },
                Err(e) => {
                    warn!("Failed to query iw for interface {}: {}", interface_name, e);
                    return Some(NetworkState {
                        network_type: NetworkType::Wifi,
                        connected: true,
                        wifi_strength: 50,
                        interface: Some(interface_name),
                    });
                },
            }
        }

        Some(NetworkState {
            network_type: NetworkType::None,
            connected: false,
            wifi_strength: 0,
            interface: None,
        })
    }

    fn refresh_state(&self) -> bool {
        if let Some(state) = self.get_network_state() {
            *self.network_type.lock().unwrap() = state.network_type;
            *self.connected.lock().unwrap() = state.connected;
            *self.wifi_strength.lock().unwrap() = state.wifi_strength;
            *self.interface.lock().unwrap() = state.interface;
            true
        } else {
            error!("Failed to get network state");
            false
        }
    }

    fn clone_service(&self) -> Self {
        Self {
            network_type: Arc::clone(&self.network_type),
            connected: Arc::clone(&self.connected),
            wifi_strength: Arc::clone(&self.wifi_strength),
            interface: Arc::clone(&self.interface),
        }
    }

    fn parse_wifi_strength(iw_output: &str) -> u8 {
        if let Some(signal_dbm) = Self::extract_signal_dbm(iw_output) {
            let strength = ((signal_dbm + 90) as f64 / 60.0 * 100.0)
                .max(0.0)
                .min(100.0);
            strength as u8
        } else {
            50
        }
    }

    fn extract_signal_dbm(iw_output: &str) -> Option<i32> {
        for line in iw_output.lines() {
            if line.contains("signal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if *part == "signal:" && i + 1 < parts.len() {
                        if let Ok(dbm) = parts[i + 1].parse::<i32>() {
                            return Some(dbm);
                        }
                    }
                }
            }
        }
        None
    }
}

/// Monitor for network state changes.
#[allow(dead_code)] // Some fields will be used in the future with networkd-manager
pub struct NetworkMonitor {
    service: Arc<NetworkService>,
    last_type: NetworkType,
    last_connected: bool,
    last_strength: u8,
    callback: Box<dyn Fn(NetworkType, bool, u8) + Send>,
}

impl NetworkMonitor {
    /// Check for network state changes and call callback if changed.
    #[allow(dead_code)] // Will be used in the future with networkd-manager
    pub fn check(&mut self) {
        if !self.service.refresh_state() {
            return;
        }

        let state = self.service.get_state();

        let type_changed = state.network_type != self.last_type;
        let connected_changed = state.connected != self.last_connected;
        let strength_changed = state.wifi_strength.abs_diff(self.last_strength) > 5;

        if type_changed || connected_changed || strength_changed {
            debug!(
                "Network state changed: type={:?}, connected={}, strength={}%",
                state.network_type, state.connected, state.wifi_strength
            );
            (self.callback)(
                state.network_type.clone(),
                state.connected,
                state.wifi_strength,
            );
            self.last_type = state.network_type;
            self.last_connected = state.connected;
            self.last_strength = state.wifi_strength;
        }
    }
}
