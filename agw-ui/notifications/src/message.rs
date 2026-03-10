//! Global message passing for notification control.

use crate::model::NotificationStore;
use log::debug;
use std::sync::OnceLock;

static NOTIFICATION_STORE: OnceLock<std::sync::Arc<NotificationStore>> = OnceLock::new();

pub fn init(store: std::sync::Arc<NotificationStore>) {
    let _ = NOTIFICATION_STORE.set(store);
}

pub fn close_last() {
    if let Some(store) = NOTIFICATION_STORE.get() {
        let notifications = store.get_all();

        if let Some(last) = notifications.last() {
            let id = last.id;
            debug!("Closing last notification: id={}", id);
            store.close(id);
        } else {
            debug!("No notifications to close");
        }
    } else {
        log::error!("Notification store not initialized");
    }
}
