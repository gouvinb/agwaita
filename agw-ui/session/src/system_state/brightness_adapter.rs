//! Brightness Service Adapter
//!
//! Bridges agw-service::brightness::BrightnessService to UI SystemStateUpdate messages

use super::messages::SystemStateUpdate;
use agw_service::{
    brightness::BrightnessService,
    signal::SignalHandler,
};
use log::debug;
use std::sync::mpsc::Sender;

/// Adapter that connects BrightnessService signals to SystemStateUpdate messages
pub struct BrightnessServiceAdapter {
    _service: BrightnessService,
    _signal_handler: SignalHandler,
}

impl BrightnessServiceAdapter {
    /// Create a new adapter that forwards brightness changes to SystemStateUpdate
    pub fn new(service: BrightnessService, sender: Sender<SystemStateUpdate>) -> Self {
        debug!("Creating BrightnessServiceAdapter");

        // Connect to brightness_changed signal
        let signal_handler = service.connect_brightness_changed(move |level| {
            debug!("Brightness changed: {:.1}%", level * 100.0);
            let _ = sender.send(SystemStateUpdate::Brightness(level));
        });

        // Start monitoring
        service.start_monitoring();

        Self {
            _service: service,
            _signal_handler: signal_handler,
        }
    }
}
