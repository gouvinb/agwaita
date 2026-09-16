//! Bluetooth adapter UI model.

/// UI-side snapshot of a Bluetooth adapter's properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    pub name: String,
    pub address: String,
    pub alias: String,
    pub powered: bool,
    pub discoverable: bool,
    pub discoverable_timeout: u32,
    pub discovering: bool,
}
