//! UI components for the Bluetooth manager.

mod adapter_settings_page;
mod blocked_devices_page;
mod device_row;
mod device_settings_page;
mod main_window;
mod pairing_dialog;
mod sidebar;

pub use main_window::{
    BluetoothManagerWindow,
    BluetoothManagerWindowConfig,
    BluetoothManagerWindowInput,
};
pub use pairing_dialog::{
    PairingDialog,
    PairingDialogConfig,
    PairingDialogInput,
};
