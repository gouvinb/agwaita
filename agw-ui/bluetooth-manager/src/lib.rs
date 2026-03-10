use agw_lib_outcome::error::AgwError;
use log::info;

/// Initialize and display the Bluetooth manager.
///
/// # Errors
/// Returns an error if initialization fails.
pub fn init() -> Result<(), AgwError> {
    info!("Initializing Bluetooth Manager");

    Ok(())
}
