//! GeoClue2 D-Bus interface for location service monitoring.

use zbus::proxy;

/// D-Bus proxy for org.freedesktop.GeoClue2.Manager interface.
#[proxy(
    interface = "org.freedesktop.GeoClue2.Manager",
    default_service = "org.freedesktop.GeoClue2",
    default_path = "/org/freedesktop/GeoClue2/Manager"
)]
pub trait GeoclueManager {
    #[zbus(property)]
    fn in_use(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn available_accuracy_level(&self) -> zbus::Result<u32>;
}
