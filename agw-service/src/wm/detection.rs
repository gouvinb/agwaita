//! Window Manager detection

use log::debug;

/// Enumeration of supported window managers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WMType {
    /// Niri Wayland compositor
    Niri,
    // Future WMs can be added here:
    // Sway,
    // Hyprland,
    /// Unsupported or no WM detected
    Unsupported,
}

/// Detect the currently running window manager
///
/// Detection order:
/// 1. Try to connect to Niri socket (using niri_ipc)
/// 2. Check environment variables (XDG_CURRENT_DESKTOP, etc.)
/// 3. Fallback to Unsupported
// TODO: Add other WMs
pub fn detect_wm() -> WMType {
    if let Ok(_socket) = niri_ipc::socket::Socket::connect() {
        debug!("Niri socket connection successful");
        return WMType::Niri;
    }

    if std::env::var("NIRI_SOCKET").is_ok() {
        debug!("NIRI_SOCKET environment variable found");
        return WMType::Niri;
    }

    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let desktop_lower = desktop.to_lowercase();
        if desktop_lower.contains("niri") {
            debug!("Detected Niri from XDG_CURRENT_DESKTOP: {}", desktop);
            return WMType::Niri;
        }
    }

    debug!("No supported window manager detected");
    WMType::Unsupported
}
