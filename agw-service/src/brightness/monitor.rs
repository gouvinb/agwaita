//! Brightness monitoring - Hybrid approach (inotify + udev)
//!
//! - inotify: Monitors brightness file changes (from external tools)
//! - udev: Detects device hotplug/removal (not used for value changes)
//!
//! Note: udev does NOT generate events for sysfs file modifications,
//! only for device add/remove. Therefore inotify is necessary.

use crate::signal::Signal;
use inotify::{
    Inotify,
    WatchMask,
};
use log::{
    error,
    info,
    warn,
};
use std::{
    error::Error,
    fs,
    path::{
        Path,
        PathBuf,
    },
    sync::{
        Arc,
        Mutex,
    },
    thread,
    time::{
        Duration,
        Instant,
    },
};
use udev::{
    EventType,
    MonitorBuilder,
};

/// Debounce duration after set_brightness (ignore inotify events during this time)
const DEBOUNCE_DURATION: Duration = Duration::from_millis(167);

/// Start hybrid monitoring for brightness changes
pub fn start_udev_monitor(
    device_path: PathBuf,
    max_brightness: u32,
    current_brightness: Arc<Mutex<u32>>,
    signal: Signal<f64>,
    last_set_time: Arc<Mutex<Option<Instant>>>,
) {
    let device_path_clone = device_path.clone();
    let current_brightness_clone = Arc::clone(&current_brightness);
    let signal_clone = signal.clone();
    let last_set_time_clone = Arc::clone(&last_set_time);

    thread::spawn(move || {
        info!(
            "Starting inotify monitor for brightness file: {:?}",
            device_path_clone
        );

        match inotify_monitor_loop(
            &device_path_clone,
            max_brightness,
            &current_brightness_clone,
            &signal_clone,
            &last_set_time_clone,
        ) {
            Ok(_) => {
                info!("inotify brightness monitor stopped gracefully");
            },
            Err(e) => {
                error!("inotify brightness monitor error: {}", e);
            },
        }
    });

    thread::spawn(move || {
        info!(
            "Starting udev monitor for backlight device hotplug: {:?}",
            device_path
        );

        match udev_monitor_loop(&device_path, max_brightness, &current_brightness, &signal) {
            Ok(_) => {
                info!("udev brightness monitor stopped gracefully");
            },
            Err(e) => {
                error!("udev brightness monitor error: {}", e);
            },
        }
    });
}

fn udev_monitor_loop(device_path: &Path, max_brightness: u32, current_brightness: &Arc<Mutex<u32>>, signal: &Signal<f64>) -> Result<(), Box<dyn Error>> {
    let socket = MonitorBuilder::new()?
        .match_subsystem("backlight")?
        .listen()?;

    let initial = *current_brightness.lock().unwrap();
    let normalized = initial as f64 / max_brightness as f64;
    signal.emit_sync(normalized);

    for event in socket.iter().filter(|event| event.syspath() == device_path) {
        match event.event_type() {
            EventType::Change => {
                if let Some(new_value) = read_brightness_value(&device_path.join("brightness")) {
                    let old_value = *current_brightness.lock().unwrap();

                    if new_value != old_value {
                        *current_brightness.lock().unwrap() = new_value;

                        let normalized = new_value as f64 / max_brightness as f64;
                        signal.emit_sync(normalized);
                    }
                }
            },
            EventType::Remove => {
                warn!("Backlight device removed: {:?}", device_path);
                break;
            },
            _ => {},
        }
    }

    Ok(())
}

/// Read brightness value from sysfs file
fn read_brightness_value(path: &Path) -> Option<u32> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
}

/// Monitor brightness file with inotify (detects external changes)
fn inotify_monitor_loop(
    device_path: &Path,
    max_brightness: u32,
    current_brightness: &Arc<Mutex<u32>>,
    signal: &Signal<f64>,
    last_set_time: &Arc<Mutex<Option<Instant>>>,
) -> Result<(), Box<dyn Error>> {
    let brightness_file = device_path.join("brightness");

    let mut inotify = Inotify::init()?;
    let _watch = inotify.watches().add(&brightness_file, WatchMask::MODIFY)?;

    if let Some(initial_value) = read_brightness_value(&brightness_file) {
        let normalized = initial_value as f64 / max_brightness as f64;
        signal.emit_sync(normalized);
    }

    let mut buffer = [0u8; 4096];

    loop {
        let events = inotify.read_events_blocking(&mut buffer)?;

        events
            .filter(|_| {
                let should_ignore = match *last_set_time.lock().unwrap() {
                    Some(last_set) => last_set.elapsed() < DEBOUNCE_DURATION,
                    None => false,
                };

                return !should_ignore;
            })
            .filter_map(|_| read_brightness_value(&brightness_file))
            .filter(|new_value| {
                let old_value = *current_brightness.lock().unwrap();
                return *new_value != old_value;
            })
            .for_each(|new_value| {
                *current_brightness.lock().unwrap() = new_value;

                let normalized = new_value as f64 / max_brightness as f64;
                signal.emit_sync(normalized);
            });
    }
}
