use crate::{
    desktop_entries::DesktopEntryScanner,
    model::DesktopEntry,
};
use log::{
    debug,
    info,
    warn,
};
use std::{
    sync::{
        Arc,
        Mutex,
        mpsc,
    },
    thread,
    time::Duration,
};

/// Update message for desktop entries changes
#[derive(Debug, Clone)]
pub enum DesktopEntriesUpdate {
    EntriesChanged,
}

/// Service for managing desktop entries with automatic reload on filesystem changes
pub struct DesktopEntriesService {
    scanner: Arc<DesktopEntryScanner>,
    subscribers: Arc<Mutex<Vec<mpsc::Sender<DesktopEntriesUpdate>>>>,
}

impl DesktopEntriesService {
    /// Create new service and perform initial scan
    pub fn new() -> Result<Self, String> {
        let scanner = DesktopEntryScanner::new();

        // Initial scan
        scanner.scan()?;

        // Setup inotify watchers
        if let Err(e) = scanner.setup_inotify() {
            warn!("Failed to setup inotify watchers: {}", e);
        }

        let scanner = Arc::new(scanner);

        Ok(Self {
            scanner,
            subscribers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Get current desktop entries (sorted)
    pub fn get_entries(&self) -> Vec<DesktopEntry> {
        self.scanner.get_entries()
    }

    /// Set favorites list
    pub fn set_favorites(&self, favorites: Vec<String>) {
        self.scanner.set_favorites(favorites);
    }

    /// Get scanner Arc for direct access
    pub fn scanner(&self) -> Arc<DesktopEntryScanner> {
        self.scanner.clone()
    }

    /// Subscribe to desktop entries updates
    pub fn subscribe(&self) -> mpsc::Receiver<DesktopEntriesUpdate> {
        let (tx, rx) = mpsc::channel();

        let mut subs = self.subscribers.lock().unwrap();
        subs.push(tx);

        rx
    }

    /// Start monitoring for filesystem changes
    pub fn start_monitoring(&self) {
        let scanner = self.scanner.clone();
        let subscribers = self.subscribers.clone();

        thread::spawn(move || {
            info!("Desktop entries monitoring thread started");

            let mut buffer = [0u8; 4096];

            loop {
                // Blocking read on inotify - will wait until an event occurs
                let mut inotify_guard = scanner.inotify.lock().unwrap();

                if let Some(ref mut inotify) = *inotify_guard {
                    match inotify.read_events_blocking(&mut buffer) {
                        Ok(events) => {
                            let mut changed = false;

                            for event in events {
                                if let Some(name) = event.name {
                                    let name_str = name.to_string_lossy();

                                    // Only react to .desktop files
                                    if name_str.ends_with(".desktop") {
                                        debug!("Desktop file changed: {:?}", name_str);
                                        changed = true;
                                    }
                                }
                            }

                            if changed {
                                info!("Desktop entries changed, rescanning...");

                                // Drop inotify lock before rescanning
                                drop(inotify_guard);

                                // Rescan all entries
                                if let Err(e) = scanner.scan() {
                                    warn!("Failed to rescan desktop entries: {}", e);
                                }

                                // Notify all subscribers
                                let mut subs = subscribers.lock().unwrap();
                                subs.retain(|sender| sender.send(DesktopEntriesUpdate::EntriesChanged).is_ok());
                            } else {
                                drop(inotify_guard);
                            }
                        },
                        Err(e) => {
                            warn!("Inotify read error: {}", e);
                            drop(inotify_guard);
                            thread::sleep(Duration::from_secs(1));
                        },
                    }
                } else {
                    drop(inotify_guard);
                    break;
                }
            }
        });
    }
}

impl Default for DesktopEntriesService {
    fn default() -> Self {
        Self::new().expect("Failed to create DesktopEntriesService")
    }
}
