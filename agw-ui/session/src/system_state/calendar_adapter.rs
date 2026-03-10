//! Calendar Service Adapter
//!
//! Bridges agw-service::calendar::CalendarService to UI SystemStateUpdate messages

use super::messages::SystemStateUpdate;
use agw_service::{
    calendar::CalendarService,
    runtime,
    signal::SignalHandler,
};
use log::debug;
use std::sync::mpsc::Sender;

/// Adapter that connects CalendarService signals to SystemStateUpdate messages
pub struct CalendarServiceAdapter {
    _service: CalendarService,
    _signal_handler: SignalHandler,
}

impl CalendarServiceAdapter {
    /// Create a new adapter that forwards calendar events to SystemStateUpdate
    pub fn new(service: CalendarService, sender: Sender<SystemStateUpdate>) -> Self {
        debug!("Creating CalendarServiceAdapter");

        // Connect to events_changed signal
        let signal_handler = service.connect_events_changed(move |_| {
            debug!("Calendar events changed, sending update");
            // Send empty Vec to signal that calendar data has changed
            // Components will call calendar_service.get_events_for_date() to fetch new data
            let _ = sender.send(SystemStateUpdate::CalendarEvents(Vec::new()));
        });

        // Start monitoring in background
        let service_clone = service.clone();
        runtime::spawn(async move {
            if let Err(e) = service_clone.start_monitoring().await {
                log::error!("Failed to start calendar monitoring: {}", e);
            } else {
                log::info!("Calendar monitoring started successfully");
            }
        });

        Self {
            _service: service,
            _signal_handler: signal_handler,
        }
    }
}
