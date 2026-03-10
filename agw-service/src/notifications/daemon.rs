//! Notification daemon implementing org.freedesktop.Notifications DBus interface

use super::{
    action::Action,
    notification::Notification,
    types::{
        ClosedReason,
        State,
    },
};
use crate::signal::{
    Signal,
    SignalHandler,
};
use log::{
    debug,
    error,
    info,
    warn,
};
use std::{
    collections::HashMap,
    error::Error,
    sync::{
        Arc,
        RwLock,
    },
    thread,
    time::{
        Duration,
        SystemTime,
        UNIX_EPOCH,
    },
};
use tokio::runtime::Handle;
use zbus::{
    Connection,
    interface,
    object_server::SignalEmitter,
};
use zvariant::{
    OwnedValue,
    Value,
};

const DAEMON_NAME: &str = "agwaita-notifications";
const DAEMON_VENDOR: &str = "agwaita";
const DAEMON_VERSION: &str = "0.1";
const SPEC_VERSION: &str = "1.2";

/// Notification daemon
pub struct Daemon {
    inner: Arc<DaemonInner>,
    /// DBus connection (kept alive to maintain registration)
    _connection: Arc<RwLock<Option<Connection>>>,
}

struct DaemonInner {
    /// Next notification ID
    next_id: RwLock<u32>,
    /// Active notifications
    notifications: RwLock<HashMap<u32, Notification>>,
    /// Signal emitted when a notification is added or replaced
    pub notified: Signal<(u32, bool)>,
    /// Signal emitted when a notification is resolved
    pub resolved: Signal<(u32, ClosedReason)>,
    /// Settings (stored as simple flags for now)
    ignore_timeout: RwLock<bool>,
    default_timeout: RwLock<i32>,
    server_capabilities: Vec<String>,
    /// DBus connection for emitting signals
    connection: RwLock<Option<Connection>>,
    /// Tokio runtime handle for spawning async tasks
    runtime_handle: RwLock<Option<Handle>>,
}

impl Daemon {
    /// Create a new daemon instance
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DaemonInner {
                next_id: RwLock::new(1),
                notifications: RwLock::new(HashMap::new()),
                notified: Signal::new(),
                resolved: Signal::new(),
                ignore_timeout: RwLock::new(false),
                default_timeout: RwLock::new(5000),
                server_capabilities: vec![
                    "actions".to_string(),
                    "body".to_string(),
                    "body-markup".to_string(),
                    "icon-static".to_string(),
                ],
                connection: RwLock::new(None),
                runtime_handle: RwLock::new(None),
            }),
            _connection: Arc::new(RwLock::new(None)),
        }
    }

    /// Register the daemon on DBus
    pub async fn register_on_dbus(&mut self, connection: Connection) -> Result<(), Box<dyn Error>> {
        let dbus_daemon = DBusDaemon::new(Arc::clone(&self.inner));

        connection
            .object_server()
            .at("/org/freedesktop/Notifications", dbus_daemon)
            .await?;

        info!("Notification daemon registered at /org/freedesktop/Notifications");

        // Store the connection in inner for signal emission
        *self.inner.connection.write().unwrap() = Some(connection.clone());

        // Store the connection to keep it alive
        *self._connection.write().unwrap() = Some(connection);

        Ok(())
    }

    /// Set the runtime handle for async operations
    pub fn set_runtime_handle(&self, handle: Handle) {
        *self.inner.runtime_handle.write().unwrap() = Some(handle);
        debug!("Daemon: Runtime handle set");
    }

    /// Get a notification by ID
    pub fn get_notif(&self, id: u32) -> Option<Notification> {
        self.inner.notifications.read().unwrap().get(&id).cloned()
    }

    /// Get all notifications
    pub fn notifications(&self) -> Vec<Notification> {
        self.inner
            .notifications
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    /// Connect to notified signal
    pub fn connect_notified<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn((u32, bool)) + Send + 'static,
    {
        self.inner.notified.connect(callback)
    }

    /// Connect to resolved signal
    pub fn connect_resolved<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn((u32, ClosedReason)) + Send + 'static,
    {
        self.inner.resolved.connect(callback)
    }
}

impl DaemonInner {
    fn resolve(&self, id: u32, reason: ClosedReason) {
        if let Some(n) = self.notifications.write().unwrap().remove(&id) {
            n.inner.resolved.emit_sync(reason);
            self.resolved.emit_sync((id, reason));
            debug!("Notification {} resolved: {:?}", id, reason);
        }
    }
}

/// DBus interface implementation
struct DBusDaemon {
    inner: Arc<DaemonInner>,
}

impl DBusDaemon {
    fn new(inner: Arc<DaemonInner>) -> Self {
        Self { inner }
    }

    fn add_notification(&self, n: Notification) {
        let id = n.id();
        let inner = Arc::clone(&self.inner);

        // Connect signals
        let inner_clone = Arc::clone(&inner);
        let resident = n.resident();
        n.inner.invoked.connect(move |action_id| {
            debug!("Action invoked: {} on notification {}", action_id, id);

            // Emit DBus ActionInvoked signal
            if let (Some(conn), Some(handle)) = (
                inner_clone.connection.read().unwrap().as_ref(),
                inner_clone.runtime_handle.read().unwrap().as_ref(),
            ) {
                debug!(
                    "Spawning ActionInvoked signal emission task for id={}, action={}",
                    id, action_id
                );
                let conn_clone = conn.clone();
                let action_id_clone = action_id.clone();
                handle.spawn(async move {
                    debug!(
                        "ActionInvoked signal task started for id={}, action={}",
                        id, action_id_clone
                    );
                    let obj_server = conn_clone.object_server();
                    debug!("Got object server");
                    match obj_server
                        .interface::<_, DBusDaemon>("/org/freedesktop/Notifications")
                        .await
                    {
                        Ok(iface) => {
                            debug!("Got DBusDaemon interface, emitting signal");
                            if let Err(e) = DBusDaemon::action_invoked(iface.signal_emitter(), id, &action_id_clone).await {
                                error!("Failed to emit ActionInvoked signal: {}", e);
                            } else {
                                info!(
                                    "✓ Emitted ActionInvoked D-Bus signal: id={}, action={}",
                                    id, action_id_clone
                                );
                            }
                        },
                        Err(e) => {
                            error!("Failed to get DBusDaemon interface: {}", e);
                        },
                    }
                });
            } else {
                warn!("Cannot emit ActionInvoked signal: connection or runtime handle not available");
            }

            if !resident {
                inner_clone.resolve(id, ClosedReason::Closed);
            }
        });

        let inner_clone = Arc::clone(&inner);
        n.inner.dismissed.connect(move |_| {
            inner_clone.resolve(id, ClosedReason::DismissedByUser);
        });

        let inner_clone = Arc::clone(&inner);
        n.inner.expired.connect(move |_| {
            inner_clone.resolve(id, ClosedReason::Expired);
        });

        n.set_state(State::Received);
        self.inner.notifications.write().unwrap().insert(id, n);
    }

    async fn cache_image(&self, _image_data: OwnedValue, _app_name: &str) -> Option<String> {
        // TODO: Implement image caching
        // Image data format: (iiibiiay) - width, height, rowstride, has_alpha, bits_per_sample, channels, data
        None
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl DBusDaemon {
    /// Notify - main method to send a notification
    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::fdo::Result<u32> {
        // Handle image data caching if present
        let mut hints = hints;
        if let Some(image_data) = hints.remove("image-data") {
            if let Some(cached_path) = self.cache_image(image_data, &app_name).await {
                hints.insert(
                    "image-path".to_string(),
                    Value::from(cached_path).try_into().unwrap(),
                );
            }
        }

        // Remove deprecated hints
        hints.remove("image_data");
        hints.remove("icon_data");

        // Determine timeout
        // If expire_timeout < 0, it means no timeout (notification persists)
        // If expire_timeout == 0, use default timeout for transient notifications
        // Otherwise, use the specified timeout
        let default_timeout = *self.inner.default_timeout.read().unwrap();
        let timeout = if expire_timeout == 0 {
            default_timeout
        } else {
            expire_timeout
        };

        // Determine if replacing existing notification
        let replaced = self
            .inner
            .notifications
            .read()
            .unwrap()
            .contains_key(&replaces_id);
        let id = if replaced {
            replaces_id
        } else {
            let mut next_id = self.inner.next_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        // Create notification
        let n = Notification::new();
        n.set_id(id);
        n.set_app_name(app_name.clone());
        n.set_app_icon(app_icon);
        n.set_summary(summary);
        n.set_body(body);
        n.set_expire_timeout(timeout);
        n.set_time(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        );

        // Add hints
        hints.iter().for_each(|(key, value)| {
            n.set_hint(key, value.clone());
        });

        // Add actions
        Action::new_list(actions).iter().for_each(|action| {
            n.add_action(action.clone());
        });

        // Add to notifications
        self.add_notification(n.clone());

        // Emit notified signal
        info!(
            "Notification received: id={}, summary='{}', body='{}'",
            id,
            n.summary(),
            n.body()
        );
        self.inner.notified.emit_sync((id, replaced));

        // Schedule expiration if needed
        let ignore_timeout = *self.inner.ignore_timeout.read().unwrap();
        if !ignore_timeout && timeout > 0 {
            let inner_clone = Arc::clone(&self.inner);
            let timeout_ms = timeout as u64;
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(timeout_ms));
                if !*inner_clone.ignore_timeout.read().unwrap() {
                    inner_clone.resolve(id, ClosedReason::Expired);
                }
            });
        }

        Ok(id)
    }

    /// GetCapabilities - returns list of server capabilities
    fn get_capabilities(&self) -> Vec<String> {
        self.inner.server_capabilities.clone()
    }

    /// GetNotification - returns notification data by ID
    fn get_notification(&self, id: u32) -> zbus::fdo::Result<HashMap<String, OwnedValue>> {
        let notif = self
            .inner
            .notifications
            .read()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| zbus::fdo::Error::Failed("notification not found".to_string()))?;

        let mut data = HashMap::new();
        let actions: Vec<String> = notif
            .actions()
            .iter()
            .flat_map(|a| vec![a.id().to_string(), a.label().to_string()])
            .collect();

        data.insert(
            "id".to_string(),
            Value::from(notif.id()).try_into().unwrap(),
        );
        data.insert(
            "app_name".to_string(),
            Value::from(notif.app_name()).try_into().unwrap(),
        );
        data.insert(
            "app_icon".to_string(),
            Value::from(notif.app_icon()).try_into().unwrap(),
        );
        data.insert(
            "summary".to_string(),
            Value::from(notif.summary()).try_into().unwrap(),
        );
        data.insert(
            "body".to_string(),
            Value::from(notif.body()).try_into().unwrap(),
        );
        data.insert(
            "expire_timeout".to_string(),
            Value::from(notif.expire_timeout()).try_into().unwrap(),
        );
        data.insert(
            "time".to_string(),
            Value::from(notif.time()).try_into().unwrap(),
        );
        data.insert(
            "actions".to_string(),
            Value::from(actions).try_into().unwrap(),
        );
        data.insert(
            "hints".to_string(),
            Value::from(notif.hints()).try_into().unwrap(),
        );

        Ok(data)
    }

    /// CloseNotification - closes a notification
    fn close_notification(&self, id: u32) -> zbus::fdo::Result<()> {
        self.inner.resolve(id, ClosedReason::Closed);
        Ok(())
    }

    /// GetServerInformation - returns server information
    fn get_server_information(&self) -> (String, String, String, String) {
        (
            DAEMON_NAME.to_string(),
            DAEMON_VENDOR.to_string(),
            DAEMON_VERSION.to_string(),
            SPEC_VERSION.to_string(),
        )
    }

    /// NotificationClosed signal
    #[zbus(signal)]
    async fn notification_closed(ctxt: &SignalEmitter<'_>, id: u32, reason: u32) -> zbus::Result<()>;

    /// ActionInvoked signal
    #[zbus(signal)]
    async fn action_invoked(ctxt: &SignalEmitter<'_>, id: u32, action_key: &str) -> zbus::Result<()>;
}

impl Default for Daemon {
    fn default() -> Self {
        Self::new()
    }
}
