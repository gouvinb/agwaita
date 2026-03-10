//! Battery monitoring via UPower (D-Bus).

use crate::{
    dbus_const::{
        DBUS_INTERFACE,
        DBUS_PATH,
        DBUS_PROPERTIES_CHANGED_MEMBER,
        DBUS_PROPERTIES_INTERFACE,
    },
    runtime,
};
use futures::StreamExt;
use log::{
    debug,
    error,
    info,
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
    MessageStream,
};
use zvariant::OwnedValue;

#[derive(Debug, Clone)]
pub struct BatteryState {
    pub percentage: f64,
    pub is_charging: bool,
    pub is_present: bool,
}

pub struct BatteryService {
    state: Arc<Mutex<BatteryState>>,
}

impl BatteryService {
    const UPOWER_INTERFACE: &str = "org.freedesktop.UPower";
    const UPOWER_PATH_BASE: &str = "/org/freedesktop/UPower";
    const UPOWER_DISPLAY_DEVICES_PATH: &str = "/org/freedesktop/UPower/devices/DisplayDevice";

    pub async fn new() -> Self {
        let state = Self::get_current_state().await;

        debug!(
            "Battery service initialized: percentage={:.1}%, charging={}, present={}",
            state.percentage * 100.0,
            state.is_charging,
            state.is_present
        );

        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn get_state(&self) -> BatteryState {
        self.state.lock().unwrap().clone()
    }

    pub fn start_dbus_monitor<F>(&self, callback: F)
    where
        F: Fn(f64, bool, bool) + Send + 'static,
    {
        let state = Arc::clone(&self.state);

        runtime::spawn(async move {
            match Connection::system().await {
                Ok(connection) => {
                    info!("Battery D-Bus monitor connected");

                    let match_rule_str = format!(
                        "type='signal',sender='{upower_interface}',interface='{dbus_properties_interface}',member='{member}'",
                        upower_interface = Self::UPOWER_INTERFACE,
                        dbus_properties_interface = DBUS_PROPERTIES_INTERFACE,
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
                        error!("Failed to subscribe to UPower signals: {}", e);
                        return;
                    }

                    debug!("Subscribed to UPower PropertiesChanged signals");

                    let mut stream = MessageStream::from(&connection);

                    while let Some(msg) = stream.next().await {
                        if let Ok(msg) = msg {
                            let header = msg.header();

                            if let (Some(path), Some(interface), Some(member)) = (header.path(), header.interface(), header.member()) {
                                let path_str = path.as_str();
                                let interface_str = interface.as_str();
                                let member_str = member.as_str();

                                let is_valid_path = path_str == Self::UPOWER_DISPLAY_DEVICES_PATH
                                    || path_str == Self::UPOWER_PATH_BASE
                                    || path_str.starts_with(format!("{}/devices/battery_", Self::UPOWER_PATH_BASE).as_str());

                                if interface_str != DBUS_PROPERTIES_INTERFACE || member_str != DBUS_PROPERTIES_CHANGED_MEMBER || !is_valid_path {
                                    continue;
                                }
                                if let Ok((interface_name, changed_properties, _)) = msg
                                    .body()
                                    .deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>()
                                {
                                    let is_relevant = Self::is_revelant(interface_name, changed_properties);

                                    if !is_relevant {
                                        continue;
                                    }

                                    debug!("Battery state change detected on {}", path_str);

                                    let new_state = BatteryService::get_current_state().await;

                                    let mut current_state = state.lock().unwrap();
                                    let changed = (current_state.percentage - new_state.percentage).abs() > 0.001
                                        || current_state.is_charging != new_state.is_charging
                                        || current_state.is_present != new_state.is_present;

                                    if !changed {
                                        continue;
                                    }

                                    *current_state = new_state.clone();
                                    drop(current_state);

                                    callback(
                                        new_state.percentage,
                                        new_state.is_charging,
                                        new_state.is_present,
                                    );
                                }
                            }
                        }
                    }

                    info!("Battery D-Bus monitor stream ended");
                },
                Err(e) => {
                    error!(
                        "Failed to connect to system bus for battery monitoring: {}",
                        e
                    );
                },
            }
        });
    }

    fn is_revelant(interface_name: String, changed_properties: HashMap<String, OwnedValue>) -> bool {
        (interface_name == "org.freedesktop.UPower.Device" && (changed_properties.contains_key("Percentage") || changed_properties.contains_key("State")))
            || (interface_name == "org.freedesktop.UPower" && changed_properties.contains_key("OnBattery"))
    }

    async fn get_current_state() -> BatteryState {
        Self::get_current_state_async().await.unwrap_or_else(|| {
            error!("Failed to get battery state from UPower");
            BatteryState {
                percentage: 0.0,
                is_charging: false,
                is_present: false,
            }
        })
    }

    async fn get_current_state_async() -> Option<BatteryState> {
        use zbus::names::InterfaceName;

        let connection = Connection::system().await.ok()?;

        let proxy = zbus::fdo::PropertiesProxy::builder(&connection)
            .path(Self::UPOWER_DISPLAY_DEVICES_PATH)
            .ok()?
            .destination(Self::UPOWER_INTERFACE)
            .ok()?
            .build()
            .await
            .ok()?;

        let interface = InterfaceName::try_from("org.freedesktop.UPower.Device").ok()?;

        let percentage_value = proxy.get(interface.clone(), "Percentage").await.ok()?;
        let percentage: f64 = percentage_value.try_into().ok()?;

        let state_value = proxy.get(interface.clone(), "State").await.ok()?;
        let state_code: u32 = state_value.try_into().ok()?;

        let present_value = proxy.get(interface, "IsPresent").await.ok()?;
        let is_present: bool = present_value.try_into().unwrap_or(true);

        let is_charging = matches!(state_code, 1 | 5);

        Some(BatteryState {
            percentage: percentage / 100.0,
            is_charging,
            is_present,
        })
    }
}
