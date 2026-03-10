//! Airplane mode control via rfkill (polling).

use log::{
    debug,
    error,
};
use serde::Deserialize;
use std::{
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    thread,
    time::Duration,
};

/// Airplane Mode service - controls WiFi and Bluetooth via rfkill.
pub struct AirplaneModeService {
    last_state: Arc<Mutex<Option<bool>>>,
}

#[derive(Debug, Deserialize)]
struct RfkillOutput {
    rfkilldevices: Vec<RfkillDevice>,
}

#[derive(Debug, Deserialize)]
struct RfkillDevice {
    #[serde(rename = "type")]
    device_type: String,
    device: String,
    soft: String,
    hard: String,
}

impl AirplaneModeService {
    pub fn new() -> Self {
        let initial_state = Self::is_enabled_static();
        debug!(
            "AirplaneModeService initialized with state: {}",
            initial_state
        );
        Self {
            last_state: Arc::new(Mutex::new(Some(initial_state))),
        }
    }

    /// Check if airplane mode is enabled (all devices blocked).
    pub fn is_enabled(&self) -> bool {
        Self::is_enabled_static()
    }

    /// Static method to check state without instance.
    pub fn is_enabled_static() -> bool {
        match Self::get_status() {
            Ok(status) => status.iter().all(|device| !device.enabled),
            Err(e) => {
                error!("Failed to get airplane mode status: {}", e);
                false
            },
        }
    }

    /// Get detailed status of all rfkill devices.
    pub fn get_status() -> Result<Vec<DeviceStatus>, String> {
        let output = std::process::Command::new("rfkill")
            .arg("--json")
            .env("LANG", "C")
            .output()
            .map_err(|e| format!("Failed to execute rfkill: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "rfkill command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let rfkill_output: RfkillOutput = serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse rfkill output: {}", e))?;

        Ok(rfkill_output
            .rfkilldevices
            .into_iter()
            .map(|device| DeviceStatus {
                device_type: device.device_type,
                device_name: device.device,
                enabled: device.soft == "unblocked" && device.hard == "unblocked",
            })
            .collect())
    }

    /// Enable airplane mode (block wifi and bluetooth).
    pub fn enable() -> Result<(), String> {
        debug!("Enabling airplane mode (blocking wifi and bluetooth)");

        Self::block_device("wifi")?;
        Self::block_device("bluetooth")?;

        Ok(())
    }

    /// Disable airplane mode (unblock wifi and bluetooth).
    pub fn disable() -> Result<(), String> {
        debug!("Disabling airplane mode (unblocking wifi and bluetooth)");

        Self::unblock_device("wifi")?;
        Self::unblock_device("bluetooth")?;

        Ok(())
    }

    /// Toggle airplane mode.
    pub fn toggle(&self) -> Result<(), String> {
        if self.is_enabled() {
            Self::disable()
        } else {
            Self::enable()
        }
    }

    /// Create a monitor that polls for airplane mode state changes.
    pub fn monitor_airplane_mode<F>(&self, callback: F) -> AirplaneModeMonitor
    where
        F: Fn(bool) + Send + 'static,
    {
        let last_state = Arc::clone(&self.last_state);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);

        let handle = thread::spawn(move || {
            while !stop_thread.load(Ordering::SeqCst) {
                let current_state = Self::is_enabled_static();
                let mut last = last_state.lock().unwrap();

                if last.is_none() || last.unwrap() != current_state {
                    *last = Some(current_state);
                    debug!("Airplane mode state changed: {}", current_state);
                    callback(current_state);
                }

                thread::sleep(Duration::from_secs(2));
            }
        });

        AirplaneModeMonitor {
            stop,
            handle: Some(handle),
        }
    }

    /// Block a specific device type.
    fn block_device(device_type: &str) -> Result<(), String> {
        let output = std::process::Command::new("rfkill")
            .arg("block")
            .arg(device_type)
            .output()
            .map_err(|e| format!("Failed to block {}: {}", device_type, e))?;

        if !output.status.success() {
            return Err(format!(
                "Failed to block {}: {}",
                device_type,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        debug!("Blocked {}", device_type);
        Ok(())
    }

    /// Unblock a specific device type.
    fn unblock_device(device_type: &str) -> Result<(), String> {
        let output = std::process::Command::new("rfkill")
            .arg("unblock")
            .arg(device_type)
            .output()
            .map_err(|e| format!("Failed to unblock {}: {}", device_type, e))?;

        if !output.status.success() {
            return Err(format!(
                "Failed to unblock {}: {}",
                device_type,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        debug!("Unblocked {}", device_type);
        Ok(())
    }
}

impl Default for AirplaneModeService {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of a single rfkill device.
#[derive(Debug, Clone)]
pub struct DeviceStatus {
    pub device_type: String,
    pub device_name: String,
    pub enabled: bool,
}

/// Monitor for airplane mode state changes via polling.
pub struct AirplaneModeMonitor {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for AirplaneModeMonitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
            debug!("Airplane mode monitor stopped");
        }
    }
}
