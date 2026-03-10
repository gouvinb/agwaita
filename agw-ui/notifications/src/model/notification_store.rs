//! Centralized notification store with subscriber pattern.

use super::{
    Notification,
    NotificationVisibility,
};
use log::debug;
use std::sync::{
    Arc,
    Mutex,
    mpsc::{
        self,
        Sender,
    },
};

/// Events emitted by the notification store
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    Added(Notification),
    Updated(Notification),
    Closed(u32),
    ActionInvoked(u32, String),
}

/// Thread-safe notification store with subscriber pattern
#[derive(Debug)]
pub struct NotificationStore {
    notifications: Arc<Mutex<Vec<Notification>>>,
    subscribers: Arc<Mutex<Vec<Sender<NotificationEvent>>>>,
    next_id: Arc<Mutex<u32>>,
}

impl Clone for NotificationStore {
    fn clone(&self) -> Self {
        Self {
            notifications: Arc::clone(&self.notifications),
            subscribers: Arc::clone(&self.subscribers),
            next_id: Arc::clone(&self.next_id),
        }
    }
}

impl NotificationStore {
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(Mutex::new(Vec::new())),
            subscribers: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn subscribe(&self) -> mpsc::Receiver<NotificationEvent> {
        let (sender, receiver) = mpsc::channel();
        self.subscribers.lock().unwrap().push(sender);
        debug!("New notification subscriber registered");
        receiver
    }

    fn broadcast(&self, event: NotificationEvent) {
        let mut subs = self.subscribers.lock().unwrap();
        subs.retain(|sender| sender.send(event.clone()).is_ok());
    }

    pub fn add(&self, mut notification: Notification) -> u32 {
        let id = {
            let mut next_id = self.next_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        notification.id = id;
        notification.visibility = NotificationVisibility::Visible;

        let notif_clone = notification.clone();
        self.notifications.lock().unwrap().push(notification);

        debug!(
            "Notification added: id={}, summary={}",
            id, notif_clone.summary
        );
        self.broadcast(NotificationEvent::Added(notif_clone));
        id
    }

    pub fn update(&self, notification: Notification) {
        let mut notifications = self.notifications.lock().unwrap();
        if let Some(existing) = notifications.iter_mut().find(|n| n.id == notification.id) {
            let id = notification.id;
            *existing = notification.clone();
            drop(notifications);
            debug!("Notification updated: id={}", id);
            self.broadcast(NotificationEvent::Updated(notification));
        }
    }

    pub fn close(&self, id: u32) {
        let mut notifications = self.notifications.lock().unwrap();
        if let Some(pos) = notifications.iter().position(|n| n.id == id) {
            notifications.remove(pos);
            drop(notifications);
            self.broadcast(NotificationEvent::Closed(id));
            debug!("Notification closed: id={}", id);
        }
    }

    pub fn hide(&self, id: u32) {
        let mut notifications = self.notifications.lock().unwrap();
        if let Some(notification) = notifications.iter_mut().find(|n| n.id == id) {
            notification.visibility = NotificationVisibility::Hidden;
            let notif_clone = notification.clone();
            drop(notifications);
            self.broadcast(NotificationEvent::Updated(notif_clone));
            debug!("Notification hidden: id={}", id);
        }
    }

    pub fn invoke_action(&self, id: u32, action_id: &String) {
        self.broadcast(NotificationEvent::ActionInvoked(id, action_id.clone()));
        debug!(
            "Action invoked: notification_id={}, action_id={}",
            id, action_id
        );
    }

    pub fn get_all(&self) -> Vec<Notification> {
        self.notifications.lock().unwrap().clone()
    }

    pub fn get_visible(&self) -> Vec<Notification> {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .filter(|n| n.visibility == NotificationVisibility::Visible)
            .cloned()
            .collect()
    }

    pub fn count(&self) -> usize {
        self.notifications.lock().unwrap().len()
    }
}

impl Default for NotificationStore {
    fn default() -> Self {
        Self::new()
    }
}
