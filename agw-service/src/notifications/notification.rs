//! Notification structure

use super::{
    action::Action,
    types::{
        ClosedReason,
        State,
        Urgency,
    },
};
use crate::signal::Signal;
use log::{
    debug,
    error,
    warn,
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        RwLock,
    },
};
use zvariant::{
    OwnedValue,
    Value,
};

/// A desktop notification
pub struct Notification {
    // Internal fields
    pub(super) inner: Arc<NotificationInner>,
}

pub(super) struct NotificationInner {
    id: RwLock<u32>,
    app_name: RwLock<String>,
    app_icon: RwLock<String>,
    summary: RwLock<String>,
    body: RwLock<String>,
    hints: RwLock<HashMap<String, OwnedValue>>,
    expire_timeout: RwLock<i32>,
    actions: RwLock<Vec<Action>>,
    /// Handlers for action invocations (kept alive to maintain connections)
    action_handlers: RwLock<Vec<crate::signal::SignalHandler>>,

    /// State of the notification
    state: RwLock<State>,
    /// Unix time of when the notification was sent or received
    time: RwLock<i64>,

    /// Emitted when this notification is resolved
    pub resolved: Signal<ClosedReason>,
    /// Emitted when an action is invoked
    pub invoked: Signal<String>,
    /// Internal signal for dismissal
    pub(super) dismissed: Signal<()>,
    /// Internal signal for expiration
    pub(super) expired: Signal<()>,
}

impl Notification {
    /// Create a new draft notification
    pub fn new() -> Self {
        Self {
            inner: Arc::new(NotificationInner {
                id: RwLock::new(0),
                app_name: RwLock::new(String::new()),
                app_icon: RwLock::new(String::new()),
                summary: RwLock::new(String::new()),
                body: RwLock::new(String::new()),
                hints: RwLock::new(HashMap::new()),
                expire_timeout: RwLock::new(-1),
                actions: RwLock::new(Vec::new()),
                action_handlers: RwLock::new(Vec::new()),
                state: RwLock::new(State::Draft),
                time: RwLock::new(0),
                resolved: Signal::new(),
                invoked: Signal::new(),
                dismissed: Signal::new(),
                expired: Signal::new(),
            }),
        }
    }

    // Getters
    pub fn id(&self) -> u32 {
        *self.inner.id.read().unwrap()
    }

    pub fn app_name(&self) -> String {
        self.inner.app_name.read().unwrap().clone()
    }

    pub fn app_icon(&self) -> String {
        self.inner.app_icon.read().unwrap().clone()
    }

    pub fn summary(&self) -> String {
        self.inner.summary.read().unwrap().clone()
    }

    pub fn body(&self) -> String {
        self.inner.body.read().unwrap().clone()
    }

    pub fn expire_timeout(&self) -> i32 {
        *self.inner.expire_timeout.read().unwrap()
    }

    pub fn actions(&self) -> Vec<Action> {
        self.inner.actions.read().unwrap().clone()
    }

    pub fn hints(&self) -> HashMap<String, OwnedValue> {
        self.inner.hints.read().unwrap().clone()
    }

    pub fn state(&self) -> State {
        *self.inner.state.read().unwrap()
    }

    pub fn time(&self) -> i64 {
        *self.inner.time.read().unwrap()
    }

    // Setters (only work in Draft state)
    pub fn set_id(&self, value: u32) {
        self.set_field(|| {
            *self.inner.id.write().unwrap() = value;
        });
    }

    pub fn set_app_name(&self, value: String) {
        self.set_field(|| {
            *self.inner.app_name.write().unwrap() = value;
        });
    }

    pub fn set_app_icon(&self, value: String) {
        self.set_field(|| {
            *self.inner.app_icon.write().unwrap() = value;
        });
    }

    pub fn set_summary(&self, value: String) {
        self.set_field(|| {
            *self.inner.summary.write().unwrap() = value;
        });
    }

    pub fn set_body(&self, value: String) {
        self.set_field(|| {
            *self.inner.body.write().unwrap() = value;
        });
    }

    pub fn set_expire_timeout(&self, value: i32) {
        self.set_field(|| {
            *self.inner.expire_timeout.write().unwrap() = value;
        });
    }

    // Standard hints
    pub fn image(&self) -> String {
        self.get_str_hint("image-path")
    }

    pub fn set_image(&self, value: String) {
        self.set_hint("image-path", Value::from(value).try_into().unwrap());
    }

    pub fn action_icons(&self) -> bool {
        self.get_bool_hint("action-icons")
    }

    pub fn set_action_icons(&self, value: bool) {
        self.set_hint("action-icons", Value::from(value).try_into().unwrap());
    }

    pub fn category(&self) -> String {
        self.get_str_hint("category")
    }

    pub fn set_category(&self, value: String) {
        self.set_hint("category", Value::from(value).try_into().unwrap());
    }

    pub fn desktop_entry(&self) -> String {
        self.get_str_hint("desktop-entry")
    }

    pub fn set_desktop_entry(&self, value: String) {
        self.set_hint("desktop-entry", Value::from(value).try_into().unwrap());
    }

    pub fn resident(&self) -> bool {
        self.get_bool_hint("resident")
    }

    pub fn set_resident(&self, value: bool) {
        self.set_hint("resident", Value::from(value).try_into().unwrap());
    }

    pub fn sound_file(&self) -> String {
        self.get_str_hint("sound-file")
    }

    pub fn set_sound_file(&self, value: String) {
        self.set_hint("sound-file", Value::from(value).try_into().unwrap());
    }

    pub fn sound_name(&self) -> String {
        self.get_str_hint("sound-name")
    }

    pub fn set_sound_name(&self, value: String) {
        self.set_hint("sound-name", Value::from(value).try_into().unwrap());
    }

    pub fn suppress_sound(&self) -> bool {
        self.get_bool_hint("suppress-sound")
    }

    pub fn set_suppress_sound(&self, value: bool) {
        self.set_hint("suppress-sound", Value::from(value).try_into().unwrap());
    }

    pub fn transient(&self) -> bool {
        self.get_bool_hint("transient")
    }

    pub fn set_transient(&self, value: bool) {
        self.set_hint("transient", Value::from(value).try_into().unwrap());
    }

    pub fn x(&self) -> i32 {
        self.get_int_hint("x")
    }

    pub fn set_x(&self, value: i32) {
        self.set_hint("x", Value::from(value).try_into().unwrap());
    }

    pub fn y(&self) -> i32 {
        self.get_int_hint("y")
    }

    pub fn set_y(&self, value: i32) {
        self.set_hint("y", Value::from(value).try_into().unwrap());
    }

    pub fn urgency(&self) -> Urgency {
        self.get_hint("urgency")
            .and_then(|v| -> Option<u8> { v.try_into().ok() })
            .map(Urgency::from)
            .unwrap_or(Urgency::Normal)
    }

    pub fn set_urgency(&self, value: Urgency) {
        self.set_hint("urgency", Value::from(value as u8).try_into().unwrap());
    }

    // Actions
    pub fn add_action(&self, action: Action) -> &Self {
        if self.state() != State::Draft {
            error!("cannot add action: notification is not a draft");
            return self;
        }

        // Connect action's invoked signal to notification's invoked signal
        let action_id = action.id().to_string();
        let notif_invoked = self.inner.invoked.clone();
        let handler = action.connect_invoked(move |_| {
            debug!(
                "Action {} invoked, emitting notification invoked signal",
                action_id
            );
            notif_invoked.emit_sync(action_id.clone());
        });

        // Store the handler to keep the connection alive
        self.inner.action_handlers.write().unwrap().push(handler);
        self.inner.actions.write().unwrap().push(action);
        self
    }

    // Hints
    pub fn set_hint(&self, name: &str, value: OwnedValue) -> &Self {
        if self.state() != State::Draft {
            error!("cannot set hint '{}': notification is not a draft", name);
            return self;
        }

        self.inner
            .hints
            .write()
            .unwrap()
            .insert(name.to_string(), value);
        self
    }

    pub fn get_hint(&self, name: &str) -> Option<OwnedValue> {
        self.inner.hints.read().unwrap().get(name).cloned()
    }

    fn get_str_hint(&self, name: &str) -> String {
        self.get_hint(name)
            .and_then(|v| v.try_into().ok())
            .unwrap_or_default()
    }

    fn get_int_hint(&self, name: &str) -> i32 {
        self.get_hint(name)
            .and_then(|v| v.try_into().ok())
            .unwrap_or(0)
    }

    fn get_bool_hint(&self, name: &str) -> bool {
        self.get_hint(name)
            .and_then(|v| v.try_into().ok())
            .unwrap_or(false)
    }

    // Public methods

    /// Resolve this notification with DismissedByUser reason
    pub fn dismiss(&self) {
        if self.state() == State::Received {
            self.inner.dismissed.emit_sync(());
        } else {
            warn!("notification cannot be dismissed: not a received notification");
        }
    }

    /// Resolve this notification with Expired reason
    pub fn expire(&self) {
        if self.state() == State::Received {
            self.inner.expired.emit_sync(());
        } else {
            warn!("notification cannot be expired: not a received notification");
        }
    }

    /// Invoke an action
    pub fn invoke(&self, action_id: &str) {
        if self.state() == State::Received {
            self.inner.invoked.emit_sync(action_id.to_string());
        } else {
            warn!("action cannot be invoked: not a received notification");
        }
    }

    // Internal helpers
    fn set_field<F: FnOnce()>(&self, func: F) {
        if self.state() != State::Draft {
            error!("cannot modify field: notification is not a draft");
            return;
        }
        func();
    }

    pub(crate) fn set_state(&self, state: State) {
        *self.inner.state.write().unwrap() = state;
    }

    pub(crate) fn set_time(&self, time: i64) {
        *self.inner.time.write().unwrap() = time;
    }
}

impl Default for Notification {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Notification {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
