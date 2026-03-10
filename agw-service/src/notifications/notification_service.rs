//! Main notification service orchestrator
//!
//! This module provides the main NotificationService that acts as the
//! org.freedesktop.Notifications daemon.

use super::{
    daemon::Daemon,
    notification::Notification,
    types::ClosedReason,
};
use crate::signal::SignalHandler;
use log::info;
use std::{
    error::Error,
    pin::Pin,
    sync::Arc,
};
use tokio::runtime::Handle;
use zbus::Connection;

/// Main notification service
pub enum NotificationService {
    /// Running as daemon
    Daemon(Arc<Daemon>),
}

impl NotificationService {
    /// Initialize the notification service
    pub fn init() -> Pin<Box<dyn Future<Output = Result<Self, Box<dyn Error>>> + Send>> {
        Box::pin(async move {
            // Try to request the bus name
            let connection = Connection::session().await?;

            connection
                .request_name("org.freedesktop.Notifications")
                .await?;

            // We got the name, become daemon
            info!("Starting notification daemon");
            let mut daemon = Daemon::new();

            // Register the daemon on DBus synchronously
            daemon.register_on_dbus(connection).await?;

            let daemon = Arc::new(daemon);
            info!("Notification daemon registered on DBus");

            Ok(NotificationService::Daemon(daemon))
        })
    }

    /// Set the runtime handle for async operations (only applicable for Daemon mode)
    pub fn set_runtime_handle(&self, handle: Handle) {
        let NotificationService::Daemon(d) = self;
        d.set_runtime_handle(handle);
    }

    /// Get a notification by ID
    pub fn get_notif(&self, id: u32) -> Option<Notification> {
        match self {
            NotificationService::Daemon(d) => d.get_notif(id),
        }
    }

    /// Get all notifications
    pub fn notifications(&self) -> Vec<Notification> {
        match self {
            NotificationService::Daemon(d) => d.notifications(),
        }
    }

    /// Connect to notified signal
    pub fn connect_notified<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn((u32, bool)) + Send + 'static,
    {
        match self {
            NotificationService::Daemon(d) => d.connect_notified(callback),
        }
    }

    /// Connect to resolved signal
    pub fn connect_resolved<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn((u32, ClosedReason)) + Send + 'static,
    {
        match self {
            NotificationService::Daemon(d) => d.connect_resolved(callback),
        }
    }

    /// Check if running as daemon
    pub fn is_daemon(&self) -> bool {
        matches!(self, NotificationService::Daemon(_))
    }
}
