//! Bluetooth manager UI.

use agw_lib_outcome::error::AgwError;
use agw_service::bluetooth::BluetoothService;
use log::info;
use relm4::{
    RelmApp,
    adw,
    gtk,
};
use std::sync::Arc;

pub mod components;
pub mod model;
pub mod service;

use components::{
    BluetoothManagerWindow,
    BluetoothManagerWindowConfig,
};
use model::BluetoothStore;
use service::BluetoothServiceAdapter;

/// Custom CSS mapping `gtk::LevelBar` custom offset classes (`block.low`, `block.high`)
/// to libadwaita theme color variables (`--error-color`, `--success-color`).
///
/// GTK 4 `LevelBar` assigns `.low` and `.high` CSS classes to sub-blocks when custom offsets
/// are defined, but neither GTK nor Adwaita provide default styling for these offset names.
const BATTERY_LEVELBAR_CSS: &str = "
levelbar block.low { background-color: var(--error-color); }
levelbar block.high { background-color: var(--success-color); }
";

/// Initialize and display the Bluetooth manager.
///
/// `scan_override`, when set, forces the discovery state for this session
/// (see `agwaita bluetooth-manager --scan`).
///
/// # Errors
/// Returns an error if initialization fails.
pub fn init(scan_override: Option<bool>) -> Result<(), AgwError> {
    info!("Initializing Bluetooth Manager");

    adw::init().map_err(|err| AgwError::new(1, err.to_string()))?;

    let store = Arc::new(BluetoothStore::new());
    let bluetooth_service = Arc::new(BluetoothService::new());
    bluetooth_service.set_scan_override(scan_override);
    let service_adapter = Arc::new(BluetoothServiceAdapter::new(
        bluetooth_service,
        Arc::clone(&store),
    ));

    load_battery_levelbar_css();

    let app = RelmApp::new("com.agwaita.bluetooth-manager").with_args(vec![]);
    app.run::<BluetoothManagerWindow>(BluetoothManagerWindowConfig { service_adapter });

    Ok(())
}

fn load_battery_levelbar_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(BATTERY_LEVELBAR_CSS);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("no default display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
