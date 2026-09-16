//! Adapter bridging agw_service::bluetooth::BluetoothService and the UI BluetoothStore.

use crate::model::{
    Adapter,
    BluetoothStore,
    Device,
};
use agw_service::{
    bluetooth::{
        AgentHandle,
        BluetoothAdapter,
        BluetoothDevice,
        BluetoothError,
        BluetoothService,
        PairingResponse,
    },
    signal::SignalHandler,
};
use std::sync::{
    Arc,
    Mutex,
};
use tokio::sync::oneshot;

/// Adapter that bridges BluetoothService signals to BluetoothStore events,
/// and exposes UI-facing actions that delegate to the service.
pub struct BluetoothServiceAdapter {
    bluetooth_service: Arc<BluetoothService>,
    store: Arc<BluetoothStore>,
    _handlers: Vec<SignalHandler>,
    _agent_handle: Option<AgentHandle>,
    pending_responder: Arc<Mutex<Option<oneshot::Sender<PairingResponse>>>>,
}

impl BluetoothServiceAdapter {
    /// Create a new adapter, load initial state, and start the DBus monitor.
    pub fn new(bluetooth_service: Arc<BluetoothService>, store: Arc<BluetoothStore>) -> Self {
        let mut handlers = Vec::new();

        let store_clone = Arc::clone(&store);
        handlers.push(bluetooth_service.connect_adapter_changed(move |adapter| {
            store_clone.update_adapter(to_ui_adapter(adapter));
        }));

        let store_clone = Arc::clone(&store);
        handlers.push(bluetooth_service.connect_device_added(move |device| {
            store_clone.upsert_device(to_ui_device(device));
        }));

        let store_clone = Arc::clone(&store);
        handlers.push(bluetooth_service.connect_device_changed(move |device| {
            store_clone.upsert_device(to_ui_device(device));
        }));

        let store_clone = Arc::clone(&store);
        handlers.push(bluetooth_service.connect_device_removed(move |address| {
            store_clone.remove_device(&address);
        }));

        log::info!("BluetoothServiceAdapter: connected to BluetoothService signals");

        let pending_responder = Arc::new(Mutex::new(None));
        let pending_responder_for_agent = Arc::clone(&pending_responder);
        let store_for_agent = Arc::clone(&store);

        let agent_handle = match bluetooth_service.register_pairing_agent() {
            Ok((handle, mut rx)) => {
                std::thread::spawn(move || {
                    while let Some(envelope) = rx.blocking_recv() {
                        let responder = envelope.responder;
                        *pending_responder_for_agent.lock().unwrap() = responder;
                        store_for_agent.set_pending_pairing_request(envelope.request);
                    }
                    log::info!("BluetoothServiceAdapter: pairing agent channel closed");
                });
                Some(handle)
            },
            Err(e) => {
                log::error!(
                    "BluetoothServiceAdapter: failed to register pairing agent: {}",
                    e
                );
                None
            },
        };

        let adapter = Self {
            bluetooth_service,
            store,
            _handlers: handlers,
            _agent_handle: agent_handle,
            pending_responder,
        };

        adapter.load_initial_state();
        adapter.bluetooth_service.start_dbus_monitor(|_, _| {
            // Le tray a son propre appel à start_dbus_monitor avec son propre
            // callback ; celui-ci sert uniquement à faire tourner la boucle
            // d'événements pour alimenter les signaux ci-dessus, l'état
            // powered/connected_count n'est pas consommé ici.
        });

        adapter
    }

    /// Select which adapter the service (and monitor) should target.
    pub fn select_adapter(&self, name: &str) -> Result<(), BluetoothError> {
        self.bluetooth_service.select_adapter(name)?;
        self.store.select_adapter(name.to_string());
        Ok(())
    }

    pub fn set_powered(&self, powered: bool) -> Result<(), BluetoothError> {
        self.bluetooth_service.set_powered(powered)
    }

    pub fn set_discoverable(&self, discoverable: bool) -> Result<(), BluetoothError> {
        self.bluetooth_service.set_discoverable(discoverable)
    }

    pub fn set_discoverable_timeout(&self, timeout_seconds: u32) -> Result<(), BluetoothError> {
        self.bluetooth_service
            .set_discoverable_timeout(timeout_seconds)
    }

    pub fn set_adapter_alias(&self, alias: &str) -> Result<(), BluetoothError> {
        self.bluetooth_service.set_adapter_alias(alias)
    }

    pub fn set_device_trusted(&self, address: &str, trusted: bool) -> Result<(), BluetoothError> {
        self.bluetooth_service.set_device_trusted(address, trusted)
    }

    pub fn set_device_blocked(&self, address: &str, blocked: bool) -> Result<(), BluetoothError> {
        self.bluetooth_service.set_device_blocked(address, blocked)
    }

    pub fn set_device_alias(&self, address: &str, alias: &str) -> Result<(), BluetoothError> {
        self.bluetooth_service.set_device_alias(address, alias)
    }

    pub fn connect_device(&self, address: &str) -> Result<(), BluetoothError> {
        self.bluetooth_service.connect_device(address)
    }

    pub fn disconnect_device(&self, address: &str) -> Result<(), BluetoothError> {
        self.bluetooth_service.disconnect_device(address)
    }

    pub fn remove_device(&self, address: &str) -> Result<(), BluetoothError> {
        self.bluetooth_service.remove_device(address)?;
        self.store.force_remove_device(address);
        Ok(())
    }

    pub fn start_discovery(&self) -> Result<(), BluetoothError> {
        self.bluetooth_service.start_discovery()
    }

    pub fn stop_discovery(&self) -> Result<(), BluetoothError> {
        self.bluetooth_service.stop_discovery()
    }

    /// Respond to an active pairing request and clear the pending request in the store.
    pub fn respond_pairing(&self, response: PairingResponse) {
        if let Some(responder) = self.pending_responder.lock().unwrap().take() {
            let _ = responder.send(response);
        }
        self.store.clear_pending_pairing_request();
    }

    /// Cancel an active pairing attempt, rejecting any pending response and attempting
    /// to disconnect/remove the device to interrupt pairing in BlueZ cleanly.
    ///
    /// # Limitation Note
    /// BlueZ / `bluer` does not actively notify agents when pairing is cancelled externally
    /// or on the remote device. This method defensively rejects pending responders and cleans
    /// up BlueZ device state.
    pub fn cancel_pairing(&self, device_address: &str) {
        if let Some(responder) = self.pending_responder.lock().unwrap().take() {
            let _ = responder.send(PairingResponse::Reject);
        }
        self.store.clear_pending_pairing_request();
        if let Err(e) = self.bluetooth_service.disconnect_device(device_address) {
            log::debug!(
                "BluetoothServiceAdapter: disconnect on cancel pairing: {}",
                e
            );
        }
        if let Err(e) = self.bluetooth_service.remove_device(device_address) {
            log::debug!("BluetoothServiceAdapter: remove on cancel pairing: {}", e);
        }
    }

    /// Get the Bluetooth store.
    pub fn store(&self) -> Arc<BluetoothStore> {
        Arc::clone(&self.store)
    }

    /// Get the BluetoothService instance.
    pub fn bluetooth_service(&self) -> &Arc<BluetoothService> {
        &self.bluetooth_service
    }

    fn load_initial_state(&self) {
        match self.bluetooth_service.list_adapters() {
            Ok(adapters) => {
                let ui_adapters: Vec<Adapter> = adapters.iter().cloned().map(to_ui_adapter).collect();
                self.store.load_adapters(ui_adapters);

                if let Some(name) = self
                    .bluetooth_service
                    .current_adapter_name()
                    .or_else(|| adapters.first().map(|a| a.name.clone()))
                {
                    self.store.select_adapter(name);
                }
            },
            Err(e) => log::warn!("BluetoothServiceAdapter: failed to list adapters: {}", e),
        }

        for device in self.bluetooth_service.get_devices() {
            self.store.upsert_device(to_ui_device(device));
        }
    }
}

fn to_ui_adapter(adapter: BluetoothAdapter) -> Adapter {
    Adapter {
        name: adapter.name,
        address: adapter.address,
        alias: adapter.alias,
        powered: adapter.powered,
        discoverable: adapter.discoverable,
        discoverable_timeout: adapter.discoverable_timeout,
        discovering: adapter.discovering,
    }
}

fn to_ui_device(device: BluetoothDevice) -> Device {
    Device {
        path: device.path,
        address: device.address,
        name: device.name,
        alias: device.alias,
        icon: device.icon,
        adapter: device.adapter,
        connected: device.connected,
        paired: device.paired,
        trusted: device.trusted,
        blocked: device.blocked,
        battery_percentage: device.battery_percentage,
        rssi: device.rssi,
        modalias: device.modalias,
    }
}
