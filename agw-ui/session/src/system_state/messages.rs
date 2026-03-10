use agw_service::{
    accent_color::AccentColor,
    calendar::types::CalendarEvent,
    network::NetworkType,
    power_mode::{
        PowerModeService,
        PowerProfile,
    },
};
use std::sync::Arc;

/// System state updates that are broadcast to all topbar instances
///
/// Each variant represents a change in system state that needs to be
/// reflected in all topbars across all screens.
#[derive(Clone)]
pub enum SystemStateUpdate {
    /// Screen brightness changed (0.0-1.0)
    Brightness(f64),

    /// Audio state changed (volume 0.0-1.0, muted)
    Audio(f64, bool),

    /// Bluetooth state changed (powered, connected_count)
    Bluetooth(bool, u8),

    /// Network state changed (type, connected, wifi_strength)
    Network(NetworkType, bool, u8),

    /// Do Not Disturb state changed
    Dnd(bool),

    /// Dark mode state changed
    DarkMode(bool),

    /// Airplane mode state changed
    AirplaneMode(bool),

    /// Power profile changed
    PowerProfile(PowerProfile),

    /// Battery state changed (percentage 0.0-1.0, charging, present)
    Battery(f64, bool, bool),

    /// Power mode service is ready (sent once at startup)
    PowerModeServiceReady(Arc<PowerModeService>),

    /// Accent color changed
    AccentColor(AccentColor),

    /// Privacy usage changed
    Privacy(agw_service::privacy::PrivacyUsage),

    /// Systemd user units failed
    SystemdFailed(crate::system_state::systemd_failed::SystemdFailedUnits),

    /// Calendar events updated
    CalendarEvents(Vec<CalendarEvent>),
}

impl std::fmt::Debug for SystemStateUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Brightness(v) => f.debug_tuple("Brightness").field(v).finish(),
            Self::Audio(v, m) => f.debug_tuple("Audio").field(v).field(m).finish(),
            Self::Bluetooth(p, c) => f.debug_tuple("Bluetooth").field(p).field(c).finish(),
            Self::Network(t, c, s) => f.debug_tuple("Network").field(t).field(c).field(s).finish(),
            Self::Dnd(v) => f.debug_tuple("Dnd").field(v).finish(),
            Self::DarkMode(v) => f.debug_tuple("DarkMode").field(v).finish(),
            Self::AirplaneMode(v) => f.debug_tuple("AirplaneMode").field(v).finish(),
            Self::PowerProfile(p) => f.debug_tuple("PowerProfile").field(p).finish(),
            Self::Battery(p, c, pr) => f
                .debug_tuple("Battery")
                .field(p)
                .field(c)
                .field(pr)
                .finish(),
            Self::PowerModeServiceReady(_) => write!(f, "PowerModeServiceReady(...)"),
            Self::AccentColor(c) => f.debug_tuple("AccentColor").field(c).finish(),
            Self::Privacy(u) => f.debug_tuple("Privacy").field(u).finish(),
            Self::SystemdFailed(u) => f.debug_tuple("SystemdFailed").field(u).finish(),
            Self::CalendarEvents(events) => f
                .debug_tuple("CalendarEvents")
                .field(&events.len())
                .finish(),
        }
    }
}
