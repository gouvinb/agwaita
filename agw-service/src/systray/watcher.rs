//! StatusNotifierWatcher implementation
//!
//! Implements org.kde.StatusNotifierWatcher D-Bus interface

use crate::runtime;
use futures::StreamExt;
use log::{
    debug,
    info,
    warn,
};
use std::error::Error;
use zbus::{
    Connection,
    fdo::{
        DBusProxy,
        RequestNameFlags,
        RequestNameReply,
    },
    interface,
    message::Header,
    names::{
        BusName,
        UniqueName,
        WellKnownName,
    },
    object_server::SignalEmitter,
};

const NAME: WellKnownName = WellKnownName::from_static_str_unchecked("org.kde.StatusNotifierWatcher");
const OBJECT_PATH: &str = "/StatusNotifierWatcher";

#[derive(Debug, Default)]
pub struct StatusNotifierWatcher {
    items: Vec<(UniqueName<'static>, String)>,
}

impl StatusNotifierWatcher {
    pub async fn start_server() -> Result<(Connection, Vec<String>), Box<dyn Error + Send + Sync>> {
        let connection = Connection::session().await?;

        let existing_items = if let Ok(existing_watcher) = StatusNotifierWatcherProxy::new(&connection).await {
            debug!("Found existing StatusNotifierWatcher, querying items");
            existing_watcher
                .registered_status_notifier_items()
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        if !existing_items.is_empty() {
            debug!("Captured {} existing tray items", existing_items.len());
        }

        connection
            .object_server()
            .at(OBJECT_PATH, StatusNotifierWatcher::default())
            .await?;

        let interface = connection
            .object_server()
            .interface::<_, StatusNotifierWatcher>(OBJECT_PATH)
            .await?;

        let dbus_proxy = DBusProxy::new(&connection).await?;
        let mut name_owner_changed_stream = dbus_proxy.receive_name_owner_changed().await?;

        let flags = RequestNameFlags::ReplaceExisting | RequestNameFlags::AllowReplacement;
        let reply = dbus_proxy.request_name(NAME, flags.into()).await?;
        match reply {
            RequestNameReply::PrimaryOwner => {
                info!("System tray watcher started");
            },
            RequestNameReply::AlreadyOwner => {
                debug!("Already owner of bus name");
            },
            RequestNameReply::InQueue => {
                warn!("Bus name already owned, waiting in queue");
            },
            _ => {
                warn!("Unexpected bus name reply: {:?}", reply);
            },
        }

        if !existing_items.is_empty() {
            let mut interface_guard = interface.get_mut().await;
            for item in &existing_items {
                let service_name = if let Some(idx) = item.find('/') {
                    &item[..idx]
                } else {
                    item.as_str()
                };

                if let Ok(bus_name) = service_name.try_into() {
                    if let Ok(owner) = dbus_proxy.get_name_owner(bus_name).await {
                        interface_guard
                            .items
                            .push((owner.clone().into(), item.clone()));
                        debug!("Pre-registered tray item: {}", item);
                    }
                }
            }
        }

        let emitter = SignalEmitter::new(&connection, OBJECT_PATH)?;
        StatusNotifierWatcher::status_notifier_host_registered(&emitter).await?;
        debug!("Emitted StatusNotifierHostRegistered signal");

        let internal_connection = connection.clone();
        runtime::spawn(async move {
            let mut have_bus_name = false;
            let unique_name = internal_connection.unique_name().map(|x| x.to_owned());

            while let Some(evt) = name_owner_changed_stream.next().await {
                let args = match evt.args() {
                    Ok(args) => args,
                    Err(_) => continue,
                };

                if args.name.as_ref() == NAME {
                    if args.new_owner.as_ref() == unique_name.as_deref() {
                        debug!("Acquired bus name: {NAME}");
                        have_bus_name = true;
                    } else if have_bus_name {
                        warn!("Lost bus name: {NAME}");
                        have_bus_name = false;
                    }
                } else if let BusName::Unique(name) = &args.name {
                    let mut interface = interface.get_mut().await;
                    if let Some(idx) = interface
                        .items
                        .iter()
                        .position(|(unique_name, _)| unique_name == name)
                    {
                        let service = interface.items.remove(idx).1;
                        let emitter = SignalEmitter::new(&internal_connection, OBJECT_PATH).unwrap();
                        StatusNotifierWatcher::status_notifier_item_unregistered(&emitter, &service)
                            .await
                            .ok();
                        debug!("Auto-unregistered disconnected item: {}", service);
                    }
                }
            }
        });

        Ok((connection, existing_items))
    }
}

#[interface(
    name = "org.kde.StatusNotifierWatcher",
    proxy(
        gen_blocking = false,
        default_service = "org.kde.StatusNotifierWatcher",
        default_path = "/StatusNotifierWatcher",
    )
)]
impl StatusNotifierWatcher {
    /// Registers a StatusNotifierItem with the watcher.
    ///
    /// According to the StatusNotifierItem specification:
    /// - If `service` starts with '/', it's a path and needs to be prefixed with sender
    /// - Otherwise, it's a bus name and can be used as-is
    async fn register_status_notifier_item(&mut self, service: &str, #[zbus(header)] header: Header<'_>, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        let sender = header.sender().unwrap();
        let service = if service.starts_with('/') {
            format!("{sender}{service}")
        } else {
            service.to_string()
        };

        debug!("Registering StatusNotifierItem: {}", service);

        // Avoid duplicate registrations
        if self.items.iter().any(|(_, s)| s == &service) {
            debug!("Item {} already registered, skipping", service);
            return;
        }

        self.items.push((sender.to_owned(), service.clone()));

        Self::status_notifier_item_registered(&emitter, &service)
            .await
            .ok();
    }

    /// Registers a StatusNotifierHost.
    ///
    /// This is called by hosts (like our application) to register themselves.
    #[allow(unused_variables)]
    async fn register_status_notifier_host(&mut self, service: &str, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        debug!("StatusNotifierHost registered: {}", service);
        Self::status_notifier_host_registered(&emitter).await.ok();
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.iter().map(|(_, x)| x.clone()).collect()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(emitter: &SignalEmitter<'_>, service: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(emitter: &SignalEmitter<'_>, service: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_unregistered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}
