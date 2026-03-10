//! Send notifications to any notification server
//!
//! This module provides functionality to send notifications to any
//! org.freedesktop.Notifications compatible server, not just our daemon.

use super::{
    notification::Notification,
    types::State,
};
use crate::runtime;
use futures::StreamExt;
use std::{
    collections::HashMap,
    env::{
        current_exe,
        var,
    },
    error::Error,
    pin::Pin,
    time,
    time::SystemTime,
};
use zbus::{
    Connection,
    proxy,
};
use zvariant::OwnedValue;

/// Proxy to send notifications to any notification server
#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait NotificationsSender {
    /// Send a notification
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    /// Close a notification
    fn close_notification(&self, id: u32) -> zbus::Result<()>;

    /// Signal: notification was closed
    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;

    /// Signal: action was invoked
    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action: String) -> zbus::Result<()>;
}

/// Send a notification.
///
/// This function does not depend on Notifd and can be used with any notification server.
/// The notification passed to this function will have its state changed from `DRAFT` to `SENT`
/// after which it can no longer be mutated.
///
/// # Errors
///
/// Returns an error if:
/// - The notification is not in DRAFT state
/// - Connection to the notification server fails
/// - The server rejects the notification
pub fn send_notification(notification: Notification) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error>>> + Send>> {
    Box::pin(async move {
        // Check state
        if notification.state() != State::Draft {
            return Err("cannot send notification: not a draft".into());
        }

        // Connect to notification server
        let connection = Connection::session().await?;
        let proxy = NotificationsSenderProxy::new(&connection).await?;

        // Prepare actions array
        let actions: Vec<String> = notification
            .actions()
            .iter()
            .flat_map(|action| vec![action.id().to_string(), action.label().to_string()])
            .collect();

        // Prepare hints
        let hints = notification.hints();

        // Set default app_name if empty
        let app_name = if notification.app_name().is_empty() {
            var("APP_NAME")
                .or_else(|_| {
                    current_exe()
                        .ok()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                        .ok_or("Unknown")
                })
                .unwrap_or_else(|_| "Notifd".to_string())
        } else {
            notification.app_name()
        };

        // Send notification
        let notification_id = proxy
            .notify(
                app_name,
                notification.id(),
                notification.app_icon(),
                notification.summary(),
                notification.body(),
                actions,
                hints,
                notification.expire_timeout(),
            )
            .await?;

        // Update notification state
        notification.set_id(notification_id);
        notification.set_time(
            SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        );
        notification.set_state(State::Sent);

        // Listen for signals
        let proxy_clone = proxy.clone();
        let notification_clone = notification.clone();
        runtime::spawn(async move {
            let mut closed_stream = proxy_clone.receive_notification_closed().await.unwrap();
            let mut invoked_stream = proxy_clone.receive_action_invoked().await.unwrap();

            loop {
                tokio::select! {
                    Some(signal) = closed_stream.next() => {
                        let args = signal.args().unwrap();
                        if args.id == notification_id {
                            let reason = super::types::ClosedReason::from(args.reason);
                            notification_clone.inner.resolved.emit_sync(reason);
                            break;
                        }
                    }
                    Some(signal) = invoked_stream.next() => {
                        let args = signal.args().unwrap();
                        if args.id == notification_id {
                            notification_clone.inner.invoked.emit_sync(args.action.clone());
                        }
                    }
                    else => break,
                }
            }
        });

        Ok(())
    })
}

/// Builder-style API for sending notifications
pub struct NotificationSender {
    notification: Notification,
}

impl NotificationSender {
    /// Create a new notification sender
    pub fn new() -> Self {
        Self {
            notification: Notification::new(),
        }
    }

    /// Set the summary (title)
    pub fn summary(self, summary: impl Into<String>) -> Self {
        self.notification.set_summary(summary.into());
        self
    }

    /// Set the body (message)
    pub fn body(self, body: impl Into<String>) -> Self {
        self.notification.set_body(body.into());
        self
    }

    /// Set the app name
    pub fn app_name(self, app_name: impl Into<String>) -> Self {
        self.notification.set_app_name(app_name.into());
        self
    }

    /// Set the app icon
    pub fn app_icon(self, app_icon: impl Into<String>) -> Self {
        self.notification.set_app_icon(app_icon.into());
        self
    }

    /// Set the urgency level
    pub fn urgency(self, urgency: super::types::Urgency) -> Self {
        self.notification.set_urgency(urgency);
        self
    }

    /// Set the timeout in milliseconds
    pub fn timeout(self, timeout: i32) -> Self {
        self.notification.set_expire_timeout(timeout);
        self
    }

    /// Add an action
    pub fn add_action(self, action: super::action::Action) -> Self {
        self.notification.add_action(action);
        self
    }

    /// Set a hint
    pub fn hint(self, name: impl Into<String>, value: zvariant::OwnedValue) -> Self {
        self.notification.set_hint(&name.into(), value);
        self
    }

    /// Set the category hint
    pub fn category(self, category: impl Into<String>) -> Self {
        self.notification.set_category(category.into());
        self
    }

    /// Set the transient hint (notification won't be persisted)
    pub fn transient(self, transient: bool) -> Self {
        self.notification.set_transient(transient);
        self
    }

    /// Send the notification
    pub fn send(self) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error>>> + Send>> {
        send_notification(self.notification)
    }
}

impl Default for NotificationSender {
    fn default() -> Self {
        Self::new()
    }
}
