//! Centralized Bluetooth store with subscriber pattern.
//!
//! # Architecture and Design Choices
//!
//! - **Upsert by MAC `address`**: BlueZ events (`DeviceAdded` / `DeviceChanged`) do not guarantee
//!   that a device is strictly new (they replay known devices upon adapter reconnection or monitoring start).
//!   Using MAC addresses directly as map/equality keys ensures uniqueness and idempotency without
//!   requiring synthetic auto-incremented identifiers.
//! - **Design B (`remove_device` vs `force_remove_device`)**: When a paired device disconnects or moves
//!   out of radio range, BlueZ may emit `DeviceRemoved`. In accordance with Design B, `remove_device()`
//!   keeps paired devices present in the store (marking `connected = false`) so they remain visible and
//!   reconnectable in the UI. `force_remove_device()` is reserved for explicit unpairing/removal actions.

use super::{
    Adapter,
    Device,
    PairingRequest,
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

/// Events emitted by the Bluetooth store to subscribed components.
#[derive(Debug, Clone)]
pub enum BluetoothEvent {
    AdaptersLoaded(Vec<Adapter>),
    AdapterChanged(Adapter),
    AdapterSelected(String),
    DevicesCleared,
    DeviceAdded(Device),
    DeviceChanged(Device),
    DeviceRemoved(String),
    PairingRequested(PairingRequest),
    PairingCleared,
}

/// Thread-safe Bluetooth in-memory store with pub/sub event broadcasting.
#[derive(Debug)]
pub struct BluetoothStore {
    adapters: Arc<Mutex<Vec<Adapter>>>,
    selected_adapter: Arc<Mutex<Option<String>>>,
    devices: Arc<Mutex<Vec<Device>>>,
    pending_pairing_request: Arc<Mutex<Option<PairingRequest>>>,
    subscribers: Arc<Mutex<Vec<Sender<BluetoothEvent>>>>,
}

impl Clone for BluetoothStore {
    fn clone(&self) -> Self {
        Self {
            adapters: Arc::clone(&self.adapters),
            selected_adapter: Arc::clone(&self.selected_adapter),
            devices: Arc::clone(&self.devices),
            pending_pairing_request: Arc::clone(&self.pending_pairing_request),
            subscribers: Arc::clone(&self.subscribers),
        }
    }
}

impl Default for BluetoothStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothStore {
    pub fn new() -> Self {
        Self {
            adapters: Arc::new(Mutex::new(Vec::new())),
            selected_adapter: Arc::new(Mutex::new(None)),
            devices: Arc::new(Mutex::new(Vec::new())),
            pending_pairing_request: Arc::new(Mutex::new(None)),
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn subscribe(&self) -> mpsc::Receiver<BluetoothEvent> {
        let (sender, receiver) = mpsc::channel();
        self.subscribers.lock().unwrap().push(sender);
        debug!("New bluetooth subscriber registered");
        receiver
    }

    fn broadcast(&self, event: BluetoothEvent) {
        let mut subs = self.subscribers.lock().unwrap();
        subs.retain(|sender| sender.send(event.clone()).is_ok());
    }

    /// Replace the full adapter list (typically after `list_adapters()`).
    pub fn load_adapters(&self, adapters: Vec<Adapter>) {
        *self.adapters.lock().unwrap() = adapters.clone();
        debug!("Bluetooth adapters loaded: {} adapter(s)", adapters.len());
        self.broadcast(BluetoothEvent::AdaptersLoaded(adapters));
    }

    /// Update a single adapter's properties in place (from `adapter_changed` signal).
    pub fn update_adapter(&self, adapter: Adapter) {
        let mut adapters = self.adapters.lock().unwrap();
        if let Some(existing) = adapters.iter_mut().find(|a| a.name == adapter.name) {
            *existing = adapter.clone();
        } else {
            adapters.push(adapter.clone());
        }
        drop(adapters);
        debug!("Bluetooth adapter changed: {}", adapter.name);
        self.broadcast(BluetoothEvent::AdapterChanged(adapter));
    }

    /// Record which adapter is currently selected and clear the device list —
    /// callers repopulate it via `upsert_device` as `device_added` events arrive
    /// for the newly selected adapter.
    pub fn select_adapter(&self, name: String) {
        *self.selected_adapter.lock().unwrap() = Some(name.clone());
        self.devices.lock().unwrap().clear();
        debug!("Bluetooth adapter selected: {}", name);
        self.broadcast(BluetoothEvent::AdapterSelected(name));
        self.broadcast(BluetoothEvent::DevicesCleared);
    }

    /// Insert or update a device by address (see module doc: `device_added` from
    /// `bluer` is not a guaranteed-new event, it replays known devices on reconnect).
    pub fn upsert_device(&self, device: Device) {
        let mut devices = self.devices.lock().unwrap();
        if let Some(existing) = devices.iter_mut().find(|d| d.address == device.address) {
            *existing = device.clone();
            drop(devices);
            debug!("Bluetooth device updated: {}", device.address);
            self.broadcast(BluetoothEvent::DeviceChanged(device));
        } else {
            devices.push(device.clone());
            drop(devices);
            debug!("Bluetooth device added: {}", device.address);
            self.broadcast(BluetoothEvent::DeviceAdded(device));
        }
    }

    pub fn remove_device(&self, address: &str) {
        let mut devices = self.devices.lock().unwrap();
        if let Some(pos) = devices.iter().position(|d| d.address == address) {
            if devices[pos].paired {
                devices[pos].connected = false;
                let device = devices[pos].clone();
                drop(devices);
                debug!("Bluetooth paired device disconnected: {}", address);
                self.broadcast(BluetoothEvent::DeviceChanged(device));
            } else {
                devices.remove(pos);
                drop(devices);
                debug!("Bluetooth device removed: {}", address);
                self.broadcast(BluetoothEvent::DeviceRemoved(address.to_string()));
            }
        }
    }

    pub fn force_remove_device(&self, address: &str) {
        let mut devices = self.devices.lock().unwrap();
        if let Some(pos) = devices.iter().position(|d| d.address == address) {
            devices.remove(pos);
            drop(devices);
            debug!("Bluetooth device force removed: {}", address);
            self.broadcast(BluetoothEvent::DeviceRemoved(address.to_string()));
        }
    }

    pub fn get_adapters(&self) -> Vec<Adapter> {
        self.adapters.lock().unwrap().clone()
    }

    pub fn get_selected_adapter(&self) -> Option<String> {
        self.selected_adapter.lock().unwrap().clone()
    }

    /// Return devices sorted for display: paired+connected devices first, then
    /// paired+disconnected, then unpaired — named devices (`alias != address`)
    /// before unnamed ones (`alias == address`) within each of those three
    /// groups — alphabetical on `alias` within each final rank.
    pub fn get_devices(&self) -> Vec<Device> {
        let mut devices = self.devices.lock().unwrap().clone();
        devices.sort_by(|a, b| {
            Self::device_sort_rank(a)
                .cmp(&Self::device_sort_rank(b))
                .then_with(|| a.alias.cmp(&b.alias))
        });
        devices
    }

    fn device_sort_rank(device: &Device) -> u8 {
        let named = !Self::is_unnamed(&device.alias, &device.address);
        match (device.paired, device.connected, named) {
            (true, true, true) => 0,
            (true, true, false) => 1,
            (true, false, true) => 2,
            (true, false, false) => 3,
            (false, _, true) => 4,
            (false, _, false) => 5,
        }
    }

    /// Compare `alias` and `address` after stripping non-alphanumeric
    /// characters and normalizing case, since BlueZ generates the default
    /// alias of an unknown device with dashes (`48-47-CC-14-0D-69`) while
    /// `Device1.Address` uses colons (`48:47:CC:14:0D:69`) — a literal
    /// comparison would never match.
    fn is_unnamed(alias: &str, address: &str) -> bool {
        fn normalize(s: &str) -> String {
            s.chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_uppercase())
                .collect()
        }
        normalize(alias) == normalize(address)
    }

    pub fn get_device(&self, address: &str) -> Option<Device> {
        self.devices
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.address == address)
            .cloned()
    }

    pub fn set_pending_pairing_request(&self, request: PairingRequest) {
        *self.pending_pairing_request.lock().unwrap() = Some(request.clone());
        debug!("Bluetooth pairing request pending: {:?}", request);
        self.broadcast(BluetoothEvent::PairingRequested(request));
    }

    pub fn clear_pending_pairing_request(&self) {
        *self.pending_pairing_request.lock().unwrap() = None;
        debug!("Bluetooth pairing request cleared");
        self.broadcast(BluetoothEvent::PairingCleared);
    }

    pub fn get_pending_pairing_request(&self) -> Option<PairingRequest> {
        self.pending_pairing_request.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device(address: &str, connected: bool) -> Device {
        Device {
            path: format!("/org/bluez/hci0/dev_{}", address.replace(':', "_")),
            address: address.to_string(),
            name: "Test Device".to_string(),
            alias: "Test Device".to_string(),
            icon: None,
            adapter: "hci0".to_string(),
            connected,
            paired: true,
            trusted: false,
            blocked: false,
            battery_percentage: None,
            rssi: None,
            modalias: None,
        }
    }

    #[test]
    fn test_upsert_device_added() {
        let store = BluetoothStore::new();
        let receiver = store.subscribe();

        let device = sample_device("AA:BB:CC:DD:EE:FF", false);
        store.upsert_device(device.clone());

        match receiver.try_recv() {
            Ok(BluetoothEvent::DeviceAdded(d)) => assert_eq!(d.address, device.address),
            other => panic!("expected DeviceAdded, got {:?}", other),
        }
        assert_eq!(store.get_devices(), vec![device]);
    }

    #[test]
    fn test_upsert_device_changed_no_duplicate() {
        let store = BluetoothStore::new();
        let receiver = store.subscribe();

        let address = "AA:BB:CC:DD:EE:FF";
        store.upsert_device(sample_device(address, false));
        let _ = receiver.try_recv(); // DeviceAdded

        let updated = sample_device(address, true);
        store.upsert_device(updated.clone());

        match receiver.try_recv() {
            Ok(BluetoothEvent::DeviceChanged(d)) => assert!(d.connected),
            other => panic!("expected DeviceChanged, got {:?}", other),
        }

        let devices = store.get_devices();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0], updated);
    }

    #[test]
    fn test_remove_device_paired_marks_disconnected() {
        let store = BluetoothStore::new();
        let receiver = store.subscribe();

        let address = "AA:BB:CC:DD:EE:FF";
        let dev = sample_device(address, true);
        store.upsert_device(dev);
        let _ = receiver.try_recv(); // DeviceAdded

        store.remove_device(address);

        match receiver.try_recv() {
            Ok(BluetoothEvent::DeviceChanged(d)) => {
                assert_eq!(d.address, address);
                assert!(!d.connected);
                assert!(d.paired);
            },
            other => panic!("expected DeviceChanged, got {:?}", other),
        }

        let devices = store.get_devices();
        assert_eq!(devices.len(), 1);
        assert!(!devices[0].connected);
        assert!(devices[0].paired);
    }

    #[test]
    fn test_remove_device_unpaired_removes_from_store() {
        let store = BluetoothStore::new();
        let receiver = store.subscribe();

        let address = "AA:BB:CC:DD:EE:FF";
        let mut dev = sample_device(address, false);
        dev.paired = false;
        store.upsert_device(dev);
        let _ = receiver.try_recv(); // DeviceAdded

        store.remove_device(address);

        match receiver.try_recv() {
            Ok(BluetoothEvent::DeviceRemoved(addr)) => assert_eq!(addr, address),
            other => panic!("expected DeviceRemoved, got {:?}", other),
        }
        assert!(store.get_devices().is_empty());
    }

    #[test]
    fn test_force_remove_device_removes_paired_device() {
        let store = BluetoothStore::new();
        let receiver = store.subscribe();

        let address = "AA:BB:CC:DD:EE:FF";
        let dev = sample_device(address, true);
        store.upsert_device(dev);
        let _ = receiver.try_recv(); // DeviceAdded

        store.force_remove_device(address);

        match receiver.try_recv() {
            Ok(BluetoothEvent::DeviceRemoved(addr)) => assert_eq!(addr, address),
            other => panic!("expected DeviceRemoved, got {:?}", other),
        }
        assert!(store.get_devices().is_empty());
    }

    #[test]
    fn test_force_remove_device_removes_unpaired_device() {
        let store = BluetoothStore::new();
        let receiver = store.subscribe();

        let address = "AA:BB:CC:DD:EE:FF";
        let mut dev = sample_device(address, false);
        dev.paired = false;
        store.upsert_device(dev);
        let _ = receiver.try_recv(); // DeviceAdded

        store.force_remove_device(address);

        match receiver.try_recv() {
            Ok(BluetoothEvent::DeviceRemoved(addr)) => assert_eq!(addr, address),
            other => panic!("expected DeviceRemoved, got {:?}", other),
        }
        assert!(store.get_devices().is_empty());
    }

    #[test]
    fn test_remove_device_absent_no_event() {
        let store = BluetoothStore::new();
        let receiver = store.subscribe();

        store.remove_device("00:00:00:00:00:00");

        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn test_force_remove_device_absent_no_event() {
        let store = BluetoothStore::new();
        let receiver = store.subscribe();

        store.force_remove_device("00:00:00:00:00:00");

        assert!(receiver.try_recv().is_err());
    }

    fn custom_device(address: &str, alias: &str, paired: bool, connected: bool) -> Device {
        Device {
            path: format!("/org/bluez/hci0/dev_{}", address.replace(':', "_")),
            address: address.to_string(),
            name: alias.to_string(),
            alias: alias.to_string(),
            icon: None,
            adapter: "hci0".to_string(),
            connected,
            paired,
            trusted: false,
            blocked: false,
            battery_percentage: None,
            rssi: None,
            modalias: None,
        }
    }

    #[test]
    fn test_get_devices_sorted_by_rank_then_alias() {
        let store = BluetoothStore::new();

        // Rank 0: paired + connected + named.
        let paired_connected_bravo = custom_device("EE:EE:EE:EE:EE:EE", "Bravo Headset", true, true);
        let paired_connected_charlie = custom_device("FF:FF:FF:FF:FF:FF", "Charlie Keyboard", true, true);
        // Rank 1: paired + connected + unnamed.
        let paired_connected_unnamed = custom_device("GG:GG:GG:GG:GG:GG", "GG:GG:GG:GG:GG:GG", true, true);
        // Rank 2: paired + disconnected + named.
        let paired_disconnected_named = custom_device("HH:HH:HH:HH:HH:HH", "Nano X", true, false);
        // Rank 3: paired + disconnected + unnamed.
        let paired_disconnected_unnamed = custom_device("II:II:II:II:II:II", "II:II:II:II:II:II", true, false);
        // Rank 4: unpaired + named.
        let unpaired_named_zeta = custom_device("CC:CC:CC:CC:CC:CC", "Zeta Speaker", false, false);
        let unpaired_named_alpha = custom_device("DD:DD:DD:DD:DD:DD", "Alpha Mouse", false, false);
        // Rank 5: unpaired + unnamed. BlueZ's default alias for an unknown
        // device uses dashes (`BB-BB-BB-BB-BB-BB`) while `Address` uses colons
        // (`BB:BB:BB:BB:BB:BB`) — must still be detected as unnamed.
        let unpaired_unnamed_b = custom_device("BB:BB:BB:BB:BB:BB", "BB-BB-BB-BB-BB-BB", false, false);
        let unpaired_unnamed_a = custom_device("AA:AA:AA:AA:AA:AA", "AA:AA:AA:AA:AA:AA", false, false);

        for device in [
            &unpaired_unnamed_b,
            &unpaired_named_zeta,
            &paired_connected_bravo,
            &unpaired_unnamed_a,
            &paired_connected_charlie,
            &unpaired_named_alpha,
            &paired_connected_unnamed,
            &paired_disconnected_named,
            &paired_disconnected_unnamed,
        ] {
            store.upsert_device(device.clone());
        }

        let devices = store.get_devices();
        let addresses: Vec<&str> = devices.iter().map(|d| d.address.as_str()).collect();
        assert_eq!(
            addresses,
            vec![
                paired_connected_bravo.address.as_str(),
                paired_connected_charlie.address.as_str(),
                paired_connected_unnamed.address.as_str(),
                paired_disconnected_named.address.as_str(),
                paired_disconnected_unnamed.address.as_str(),
                unpaired_named_alpha.address.as_str(),
                unpaired_named_zeta.address.as_str(),
                unpaired_unnamed_a.address.as_str(),
                unpaired_unnamed_b.address.as_str(),
            ]
        );
    }

    #[test]
    fn test_is_unnamed_normalizes_separators_and_case() {
        // Cas réel observé : BlueZ génère l'alias par défaut avec des tirets
        // alors que Device1.Address utilise des deux-points.
        assert!(BluetoothStore::is_unnamed(
            "48-47-cc-14-0d-69",
            "48:47:CC:14:0D:69"
        ));
        assert!(BluetoothStore::is_unnamed(
            "48:47:CC:14:0D:69",
            "48:47:CC:14:0D:69"
        ));
        assert!(!BluetoothStore::is_unnamed(
            "Bravo Headset",
            "48:47:CC:14:0D:69"
        ));
    }

    #[test]
    fn test_select_adapter_clears_devices_and_broadcasts_order() {
        let store = BluetoothStore::new();
        let receiver = store.subscribe();

        store.upsert_device(sample_device("AA:BB:CC:DD:EE:FF", false));
        let _ = receiver.try_recv(); // DeviceAdded

        store.select_adapter("hci1".to_string());

        assert_eq!(store.get_selected_adapter(), Some("hci1".to_string()));
        assert!(store.get_devices().is_empty());

        match receiver.try_recv() {
            Ok(BluetoothEvent::AdapterSelected(name)) => assert_eq!(name, "hci1"),
            other => panic!("expected AdapterSelected, got {:?}", other),
        }
        match receiver.try_recv() {
            Ok(BluetoothEvent::DevicesCleared) => {},
            other => panic!("expected DevicesCleared, got {:?}", other),
        }
    }

    #[test]
    fn test_multiple_subscribers_receive_same_events_in_order() {
        let store = BluetoothStore::new();
        let receiver1 = store.subscribe();
        let receiver2 = store.subscribe();

        store.upsert_device(sample_device("AA:BB:CC:DD:EE:FF", false));
        store.select_adapter("hci1".to_string());

        for receiver in [&receiver1, &receiver2] {
            match receiver.try_recv() {
                Ok(BluetoothEvent::DeviceAdded(_)) => {},
                other => panic!("expected DeviceAdded, got {:?}", other),
            }
            match receiver.try_recv() {
                Ok(BluetoothEvent::AdapterSelected(_)) => {},
                other => panic!("expected AdapterSelected, got {:?}", other),
            }
            match receiver.try_recv() {
                Ok(BluetoothEvent::DevicesCleared) => {},
                other => panic!("expected DevicesCleared, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_dropped_subscriber_does_not_break_broadcast() {
        let store = BluetoothStore::new();
        let receiver = store.subscribe();
        drop(receiver);

        // Should not panic even though the receiver was dropped.
        store.upsert_device(sample_device("AA:BB:CC:DD:EE:FF", false));

        let receiver2 = store.subscribe();
        store.upsert_device(sample_device("11:22:33:44:55:66", false));

        match receiver2.try_recv() {
            Ok(BluetoothEvent::DeviceAdded(d)) => assert_eq!(d.address, "11:22:33:44:55:66"),
            other => panic!("expected DeviceAdded, got {:?}", other),
        }
    }

    #[test]
    fn test_pending_pairing_request_broadcast_and_get() {
        use agw_service::bluetooth::PairingRequestKind;

        let store = BluetoothStore::new();
        let receiver = store.subscribe();

        assert!(store.get_pending_pairing_request().is_none());

        let req = PairingRequest {
            device_address: "11:22:33:44:55:66".to_string(),
            kind: PairingRequestKind::DisplayPasskey {
                passkey: 123456,
                entered: 3,
            },
        };

        store.set_pending_pairing_request(req.clone());
        assert_eq!(
            store.get_pending_pairing_request().unwrap().device_address,
            "11:22:33:44:55:66"
        );

        match receiver.try_recv() {
            Ok(BluetoothEvent::PairingRequested(r)) => {
                assert_eq!(r.device_address, "11:22:33:44:55:66");
                match r.kind {
                    PairingRequestKind::DisplayPasskey { passkey, entered } => {
                        assert_eq!(passkey, 123456);
                        assert_eq!(entered, 3);
                    },
                    other => panic!("expected DisplayPasskey, got {:?}", other),
                }
            },
            other => panic!("expected PairingRequested, got {:?}", other),
        }

        store.clear_pending_pairing_request();
        assert!(store.get_pending_pairing_request().is_none());

        match receiver.try_recv() {
            Ok(BluetoothEvent::PairingCleared) => {},
            other => panic!("expected PairingCleared, got {:?}", other),
        }
    }
}
