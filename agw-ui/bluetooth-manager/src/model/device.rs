//! Bluetooth device UI model.

/// UI-side snapshot of a Bluetooth device's properties.
#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub path: String,
    pub address: String,
    pub name: String,
    pub alias: String,
    pub icon: Option<String>,
    pub adapter: String,
    pub connected: bool,
    pub paired: bool,
    pub trusted: bool,
    pub blocked: bool,
    pub battery_percentage: Option<u8>,
    pub rssi: Option<i16>,
    pub modalias: Option<String>,
}
