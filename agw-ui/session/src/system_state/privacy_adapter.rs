//! Privacy service adapter for UI integration.
//!
//! This adapter bridges the agw-service::privacy::PrivacyService with the UI layer,
//! converting signal-based notifications to message-passing for GTK/Relm4 integration.

use super::messages::SystemStateUpdate;
use agw_service::{
    privacy::{
        PrivacyService,
        PrivacyUsage,
    },
    signal::SignalHandler,
};
use log::debug;
use std::sync::mpsc::Sender;

/// Adapter for integrating PrivacyService with the UI message system
pub struct PrivacyServiceAdapter {
    _service: PrivacyService,
    _signal_handler: SignalHandler,
}

impl PrivacyServiceAdapter {
    /// Create a new PrivacyServiceAdapter
    ///
    /// # Arguments
    /// * `service` - The PrivacyService instance from agw-service
    /// * `sender` - Channel sender for broadcasting privacy updates to UI
    pub fn new(service: PrivacyService, sender: Sender<SystemStateUpdate>) -> Self {
        let service_clone = service.clone();

        let signal_handler = service_clone.connect_usage_changed(move |usage: PrivacyUsage| {
            debug!(
                "Privacy usage changed - camera: {}, mic: {}, location: {}, screencast: {}",
                usage.camera.len(),
                usage.microphone.len(),
                usage.location.len(),
                usage.screencast.len()
            );
            sender.send(SystemStateUpdate::Privacy(usage)).ok();
        });

        service.start_monitoring();

        Self {
            _service: service,
            _signal_handler: signal_handler,
        }
    }
}
