//! Systemd user unit failure monitoring.
//!
//! Monitors failed systemd user units and provides notifications.

use log::{
    debug,
    warn,
};
use std::{
    sync::{
        Arc,
        Mutex,
        mpsc::Sender,
    },
    thread,
    time::Duration,
};

/// Systemd unit failure information.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemdFailedUnits {
    pub units: Vec<String>,
    pub count: usize,
}

/// Systemd failure monitoring service.
#[derive(Clone)]
pub struct SystemdFailedService {
    failed_units: Arc<Mutex<SystemdFailedUnits>>,
}

impl SystemdFailedService {
    pub fn new() -> Self {
        SystemdFailedService {
            failed_units: Arc::new(Mutex::new(SystemdFailedUnits::default())),
        }
    }

    pub fn get_failed_units(&self) -> SystemdFailedUnits {
        self.failed_units.lock().unwrap().clone()
    }

    fn update_failed_units(&self, units: Vec<String>) -> SystemdFailedUnits {
        let mut failed = self.failed_units.lock().unwrap();
        failed.units = units;
        failed.count = failed.units.len();
        failed.clone()
    }
}

/// Systemd failure monitor.
pub struct SystemdFailedMonitor {
    _thread_handle: Option<thread::JoinHandle<()>>,
}

impl SystemdFailedMonitor {
    pub fn new(service: SystemdFailedService, sender: Sender<crate::system_state::messages::SystemStateUpdate>) -> Self {
        let thread_handle = thread::spawn(move || {
            debug!("Systemd failed units monitor started");

            loop {
                match check_failed_units() {
                    Ok(units) => {
                        let failed = service.update_failed_units(units);
                        sender
                            .send(crate::system_state::messages::SystemStateUpdate::SystemdFailed(failed))
                            .ok();
                    },
                    Err(e) => {
                        warn!("Failed to check systemd units: {}", e);
                    },
                }

                thread::sleep(Duration::from_secs(5));
            }
        });

        SystemdFailedMonitor {
            _thread_handle: Some(thread_handle),
        }
    }
}

fn check_failed_units() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    use std::process::{
        Command,
        Stdio,
    };

    let output = Command::new("systemctl")
        .args(&["--user", "--failed", "--no-legend"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let units: Vec<String> = output_str
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && (line.starts_with('●') || line.contains("failed")))
        .map(|line| line.to_string())
        .collect();

    Ok(units)
}
