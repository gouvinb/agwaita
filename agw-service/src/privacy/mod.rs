//! Privacy monitoring service for microphone, camera, location, and screencast usage.
//!
//! This module provides event-driven monitoring of privacy-sensitive resources:
//! - Camera and microphone usage via PipeWire
//! - Location services via GeoClue2 D-Bus
//! - Screencast/screen recording detection
//!
//! ## Architecture
//! - `PrivacyService`: Core service with signal-based notifications
//! - `GeoclueManagerProxy`: D-Bus interface for location monitoring
//! - `PipeWireMonitor`: PipeWire stream monitoring for audio/video

pub mod geoclue;
pub mod pipewire;

use crate::{
    runtime,
    signal::{
        Signal,
        SignalHandler,
    },
};
pub use geoclue::GeoclueManagerProxy;
use log::{
    debug,
    error,
    warn,
};
pub use pipewire::PipeWireMonitor;
use std::{
    collections::HashSet,
    sync::{
        Arc,
        RwLock,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
};

/// Global flag to ensure only one privacy monitor is started.
static PRIVACY_MONITORING_STARTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrivacyUsage {
    pub camera: HashSet<String>,
    pub microphone: HashSet<String>,
    pub location: HashSet<String>,
    pub screencast: HashSet<String>,
}

pub struct PrivacyService {
    usage: Arc<RwLock<PrivacyUsage>>,
    usage_changed: Signal<PrivacyUsage>,
}

impl PrivacyService {
    pub fn new() -> Self {
        PrivacyService {
            usage: Arc::new(RwLock::new(PrivacyUsage::default())),
            usage_changed: Signal::new(),
        }
    }

    pub fn get_usage(&self) -> PrivacyUsage {
        self.usage.read().unwrap().clone()
    }

    pub fn connect_usage_changed<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn(PrivacyUsage) + Send + 'static,
    {
        self.usage_changed.connect(callback)
    }

    pub fn disconnect(&self, handler: SignalHandler) {
        self.usage_changed.disconnect(handler);
    }

    pub(crate) fn update_usage<F>(&self, updater: F)
    where
        F: FnOnce(&mut PrivacyUsage),
    {
        let mut usage = self.usage.write().unwrap();
        let before = usage.clone();
        updater(&mut *usage);
        if *usage == before {
            drop(usage);
            return;
        }
        let usage_clone = usage.clone();
        drop(usage);

        self.usage_changed.emit_sync(usage_clone);
    }

    /// Start monitoring all privacy resources
    ///
    /// This spawns async tasks to monitor:
    /// - GeoClue2 for location services
    /// - PipeWire for camera/microphone/screencast
    pub fn start_monitoring(&self) {
        if PRIVACY_MONITORING_STARTED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            debug!("Privacy monitoring already started");
            return;
        }

        let service_clone = self.clone();
        runtime::spawn(async move {
            Self::monitor_geoclue(service_clone).await;
        });

        self.start_pipewire_monitoring();
    }

    fn start_pipewire_monitoring(&self) {
        let service = self.clone();

        match PipeWireMonitor::new(move |camera_apps, mic_apps, screencast_apps| {
            service.update_usage(|usage| {
                usage.camera = camera_apps;
                usage.microphone = mic_apps;
                usage.screencast = screencast_apps;
            });
        }) {
            Ok(_monitor) => {
                debug!("PipeWire monitor started");
            },
            Err(e) => {
                error!("Failed to start PipeWire monitor: {}", e);
            },
        }
    }

    async fn monitor_geoclue(service: PrivacyService) {
        use futures::StreamExt;

        match zbus::Connection::system().await {
            Ok(conn) => match GeoclueManagerProxy::new(&conn).await {
                Ok(manager) => {
                    let initial_in_use = manager.in_use().await.unwrap_or(false);
                    debug!("GeoClue initial state: {}", initial_in_use);

                    if initial_in_use {
                        service.update_usage(|usage| {
                            usage
                                .location
                                .insert("∙ Location services active <i>(GeoClue)</i>".to_string());
                        });
                    }

                    let mut stream = manager.receive_in_use_changed().await;
                    while let Some(change) = stream.next().await {
                        if let Ok(in_use) = change.get().await {
                            debug!("GeoClue state changed: {}", in_use);
                            service.update_usage(|usage| {
                                if in_use {
                                    usage
                                        .location
                                        .insert("∙ Location services active <i>(GeoClue)</i>".to_string());
                                } else {
                                    usage.location.clear();
                                }
                            });
                        }
                    }
                },
                Err(e) => {
                    warn!("Failed to create GeoClue manager proxy: {}", e);
                },
            },
            Err(e) => {
                error!("Failed to connect to system bus: {}", e);
            },
        }
    }
}

impl Default for PrivacyService {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PrivacyService {
    fn clone(&self) -> Self {
        Self {
            usage: Arc::clone(&self.usage),
            usage_changed: self.usage_changed.clone(),
        }
    }
}
