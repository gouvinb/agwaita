//! Power mode management via power-profiles-daemon (D-Bus).

use crate::runtime;
use futures::StreamExt;
use log::{
    debug,
    error,
};
use std::sync::{
    Arc,
    Mutex,
};
use zbus::{
    Connection,
    proxy,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerProfile {
    PowerSaver,
    Balanced,
    Performance,
}

impl PowerProfile {
    pub fn from_str(value: &str) -> Self {
        match value {
            "power-saver" => PowerProfile::PowerSaver,
            "performance" => PowerProfile::Performance,
            "balanced" | _ => PowerProfile::Balanced,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PowerProfile::PowerSaver => "power-saver",
            PowerProfile::Balanced => "balanced",
            PowerProfile::Performance => "performance",
        }
    }
}

#[proxy(
    interface = "net.hadess.PowerProfiles",
    default_service = "net.hadess.PowerProfiles",
    default_path = "/net/hadess/PowerProfiles"
)]
trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> zbus::Result<()>;
}

pub struct PowerModeService {
    active_profile: Arc<Mutex<PowerProfile>>,
    connection: Connection,
}

impl PowerModeService {
    pub async fn new() -> Self {
        let connection = Connection::system()
            .await
            .expect("Failed to connect to system bus");

        let active_profile = Self::get_current_profile(&connection).await;

        debug!(
            "PowerMode service initialized: active_profile={:?}",
            active_profile
        );

        Self {
            active_profile: Arc::new(Mutex::new(active_profile)),
            connection,
        }
    }

    pub fn get_active_profile(&self) -> PowerProfile {
        self.active_profile.lock().unwrap().clone()
    }

    pub async fn set_active_profile(&self, profile: PowerProfile) -> Result<(), String> {
        match PowerProfilesProxy::new(&self.connection).await {
            Ok(proxy) => match proxy.set_active_profile(profile.as_str()).await {
                Ok(_) => {
                    *self.active_profile.lock().unwrap() = profile.clone();
                    debug!("Power profile set to: {:?}", profile);
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to set power profile: {}", e);
                    Err(e.to_string())
                },
            },
            Err(e) => {
                error!("Failed to create PowerProfiles proxy: {}", e);
                Err(e.to_string())
            },
        }
    }

    /// Create a monitor that listens for power profile changes via D-Bus property signals.
    pub fn monitor_power_mode<F>(&self, callback: F) -> PowerModeMonitor
    where
        F: Fn(PowerProfile) + Send + 'static,
    {
        let active_profile = Arc::clone(&self.active_profile);
        let connection = self.connection.clone();

        let callback = Arc::new(Mutex::new(callback));
        let callback_clone = Arc::clone(&callback);

        let handle = runtime::spawn(async move {
            let proxy = match PowerProfilesProxy::new(&connection).await {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to create PowerProfiles proxy for monitoring: {}", e);
                    return;
                },
            };

            let mut property_stream = proxy.receive_active_profile_changed().await;

            loop {
                match property_stream.next().await {
                    Some(change) => {
                        if let Ok(profile_str) = change.get().await {
                            let new_profile = PowerProfile::from_str(&profile_str);

                            let mut state = active_profile.lock().unwrap();
                            if *state != new_profile {
                                *state = new_profile.clone();
                                debug!("Power profile changed via signal: {:?}", new_profile);

                                if let Ok(cb) = callback_clone.lock() {
                                    cb(new_profile);
                                }
                            }
                        }
                    },
                    None => {
                        debug!("Power profile property stream ended");
                        break;
                    },
                }
            }
        });

        PowerModeMonitor { callback, handle }
    }

    async fn get_current_profile(connection: &Connection) -> PowerProfile {
        match PowerProfilesProxy::new(connection).await {
            Ok(proxy) => {
                let profile_str = proxy
                    .active_profile()
                    .await
                    .unwrap_or_else(|_| "balanced".to_string());
                PowerProfile::from_str(&profile_str)
            },
            Err(e) => {
                error!("Failed to create PowerProfiles proxy: {}", e);
                PowerProfile::Balanced
            },
        }
    }
}

/// Monitor for power profile changes using D-Bus property signals.
#[allow(dead_code)] // public api
pub struct PowerModeMonitor {
    callback: Arc<Mutex<dyn Fn(PowerProfile) + Send>>,
    handle: tokio::task::JoinHandle<()>,
}

impl PowerModeMonitor {
    /// Check method kept for compatibility.
    pub fn check(&mut self) {
        // With signal-based monitoring, we don't need to poll.
    }
}
