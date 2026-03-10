//! Adapter between agw_service::notifications::NotificationService and the UI NotificationStore

use crate::model::{
    Notification,
    NotificationEvent,
    NotificationStore,
    NotificationUrgency,
};
use agw_service::notifications::NotificationService;
use std::sync::Arc;

/// Adapter that bridges NotificationService signals to NotificationStore events
pub struct NotificationServiceAdapter {
    notification_service: Arc<NotificationService>,
    store: Arc<NotificationStore>,
    _handlers: Vec<agw_service::signal::SignalHandler>,
    _store_event_thread: std::thread::JoinHandle<()>,
}

impl NotificationServiceAdapter {
    /// Create a new adapter
    pub fn new(notification_service: Arc<NotificationService>, store: Arc<NotificationStore>) -> Self {
        let mut handlers = Vec::new();

        // Connect to notified signal
        let store_clone = Arc::clone(&store);
        let notification_service_clone = Arc::clone(&notification_service);
        let notified_handler = notification_service.connect_notified(move |(id, _replaced)| {
            // Get the notification from NotificationService
            if let Some(notif) = notification_service_clone.get_notif(id) {
                log::debug!(
                    "NotificationServiceAdapter: Converting notification id={}",
                    id
                );

                // Convert to UI Notification
                let app_icon = notif.app_icon();
                let body_text = notif.body();
                let image_path = notif.image();
                let desktop_entry = notif.desktop_entry();
                let expire_timeout = notif.expire_timeout();

                let ui_notif = Notification {
                    id,
                    app_name: notif.app_name(),
                    app_icon: if app_icon.is_empty() {
                        None
                    } else {
                        Some(app_icon)
                    },
                    desktop_entry: if desktop_entry.is_empty() {
                        None
                    } else {
                        Some(desktop_entry)
                    },
                    summary: notif.summary(),
                    body: if body_text.is_empty() {
                        None
                    } else {
                        Some(body_text)
                    },
                    image: if image_path.is_empty() {
                        None
                    } else {
                        Some(image_path)
                    },
                    time: chrono::DateTime::from_timestamp(notif.time(), 0)
                        .map(|dt| dt.with_timezone(&chrono::Local))
                        .unwrap_or_else(|| chrono::Local::now()),
                    urgency: match notif.urgency() {
                        agw_service::notifications::Urgency::Low => NotificationUrgency::Low,
                        agw_service::notifications::Urgency::Normal => NotificationUrgency::Normal,
                        agw_service::notifications::Urgency::Critical => NotificationUrgency::Critical,
                    },
                    actions: notif
                        .actions()
                        .iter()
                        .map(|a| crate::model::NotificationAction {
                            id: a.id().to_string(),
                            label: a.label().to_string(),
                        })
                        .collect(),
                    timeout_sec: if expire_timeout > 0 {
                        Some(expire_timeout / 1000)
                    } else {
                        None
                    },
                    visibility: crate::model::NotificationVisibility::Visible,
                };

                // Add to store (which will broadcast to subscribers)
                store_clone.add(ui_notif);
            }
        });
        handlers.push(notified_handler);

        // Connect to resolved signal
        let store_clone = Arc::clone(&store);
        let resolved_handler = notification_service.connect_resolved(move |(id, reason)| {
            log::debug!(
                "NotificationServiceAdapter: Notification {} resolved: {:?}",
                id,
                reason
            );
            store_clone.close(id);
        });
        handlers.push(resolved_handler);

        // Subscribe to store events to propagate ActionInvoked to NotificationService
        let store_receiver = store.subscribe();
        let notification_service_for_events = Arc::clone(&notification_service);
        let store_event_thread = std::thread::spawn(move || {
            // Use the shared runtime so zbus always has a live Tokio reactor.
            let handle = agw_service::runtime::runtime().handle().clone();
            notification_service_for_events.set_runtime_handle(handle);

            log::info!("NotificationServiceAdapter: Store event listener thread started");
            while let Ok(event) = store_receiver.recv() {
                log::debug!(
                    "NotificationServiceAdapter: Received store event: {:?}",
                    event
                );
                match event {
                    NotificationEvent::ActionInvoked(notif_id, action_id) => {
                        log::info!(
                            "NotificationServiceAdapter: Propagating action invocation to NotificationService: notif_id={}, action_id={}",
                            notif_id,
                            action_id
                        );
                        // Get the notification from NotificationService and invoke the action
                        if let Some(notif) = notification_service_for_events.get_notif(notif_id) {
                            log::debug!(
                                "NotificationServiceAdapter: Found notification in NotificationService, searching for action {}",
                                action_id
                            );
                            // Find the matching action and invoke it
                            let actions = notif.actions();
                            log::debug!(
                                "NotificationServiceAdapter: Notification has {} actions",
                                actions.len()
                            );
                            for action in actions {
                                log::debug!(
                                    "NotificationServiceAdapter: Checking action: id={}",
                                    action.id()
                                );
                                if action.id() == action_id {
                                    log::info!("NotificationServiceAdapter: Invoking action on NotificationService notification");
                                    action.invoke();
                                    log::info!("NotificationServiceAdapter: Action invoked successfully");
                                    break;
                                }
                            }
                        } else {
                            log::warn!(
                                "NotificationServiceAdapter: Notification {} not found in NotificationService",
                                notif_id
                            );
                        }
                    },
                    _ => {
                        // Ignore other events (we only care about ActionInvoked)
                    },
                }
            }
            log::warn!("NotificationServiceAdapter: Store event thread terminated");
        });

        log::info!("NotificationServiceAdapter: Connected to NotificationService signals and store events");

        Self {
            notification_service,
            store,
            _handlers: handlers,
            _store_event_thread: store_event_thread,
        }
    }

    /// Get the notification store
    pub fn store(&self) -> Arc<NotificationStore> {
        Arc::clone(&self.store)
    }

    /// Get the NotificationService instance
    pub fn notification_service(&self) -> &Arc<NotificationService> {
        &self.notification_service
    }
}
