//! Dark mode management via GSettings.
//!
//! Uses the `org.gnome.desktop.interface` schema and `color-scheme` key.

use gtk4::{
    gio,
    glib,
    prelude::SettingsExt,
};
use log::{
    debug,
    error,
    warn,
};

pub struct DarkModeService;

pub struct DarkModeMonitor {
    _handler_id: glib::SignalHandlerId,
    settings: gio::Settings,
}

impl DarkModeService {
    const SCHEMA: &'static str = "org.gnome.desktop.interface";
    const KEY: &'static str = "color-scheme";
    const DARK_VALUE: &'static str = "prefer-dark";
    const LIGHT_VALUE: &'static str = "prefer-light";

    /// Check if dark mode is enabled.
    pub fn is_enabled() -> bool {
        match Self::get_color_scheme() {
            Ok(scheme) => scheme == Self::DARK_VALUE,
            Err(e) => {
                error!("Failed to get color scheme: {}", e);
                false
            },
        }
    }

    /// Enable dark mode.
    pub fn enable() -> Result<(), String> {
        debug!("Enabling dark mode");
        Self::set_color_scheme(Self::DARK_VALUE)
    }

    /// Disable dark mode.
    pub fn disable() -> Result<(), String> {
        debug!("Disabling dark mode");
        Self::set_color_scheme(Self::LIGHT_VALUE)
    }

    /// Toggle dark mode.
    pub fn toggle() -> Result<(), String> {
        if Self::is_enabled() {
            Self::disable()
        } else {
            Self::enable()
        }
    }

    /// Monitor dark mode changes.
    ///
    /// Returns a handle that must be kept alive.
    pub fn monitor_dark_mode<F>(callback: F) -> Option<DarkModeMonitor>
    where
        F: Fn(bool) + Send + 'static,
    {
        let settings = gio::Settings::new(Self::SCHEMA);
        if let Some(schema) = settings.settings_schema() {
            if !schema.has_key(Self::KEY) {
                error!("{} key not found in schema {}", Self::KEY, Self::SCHEMA);
                return None;
            }
        } else {
            error!("{} schema not found in settings", Self::SCHEMA);
            return None;
        }

        let handler_id = settings.connect_changed(Some(Self::KEY), move |settings, _| {
            let scheme = settings.string(Self::KEY);
            if scheme == Self::DARK_VALUE {
                callback(true);
            } else if scheme == Self::LIGHT_VALUE {
                callback(false);
            } else {
                warn!("Unexpected color scheme: {}", scheme);
            }
        });

        Some(DarkModeMonitor {
            _handler_id: handler_id,
            settings,
        })
    }

    /// Get the current color scheme.
    pub fn get_color_scheme() -> Result<String, String> {
        let settings = gio::Settings::new(Self::SCHEMA);
        if let Some(schema) = settings.settings_schema() {
            if schema.has_key(Self::KEY) {
                Ok(settings.string(Self::KEY).to_string())
            } else {
                Err(format!(
                    "{} key not found in schema {}",
                    Self::KEY,
                    Self::SCHEMA
                ))
            }
        } else {
            Err(format!("{} schema not found in settings", Self::SCHEMA))
        }
    }

    fn set_color_scheme(scheme: &str) -> Result<(), String> {
        let settings = gio::Settings::new(Self::SCHEMA);
        if let Some(schema) = settings.settings_schema() {
            if schema.has_key(Self::KEY) {
                settings
                    .set_string(Self::KEY, scheme)
                    .map_err(|e| e.to_string())
            } else {
                Err(format!(
                    "{} key not found in schema {}",
                    Self::KEY,
                    Self::SCHEMA
                ))
            }
        } else {
            Err(format!("{} schema not found in settings", Self::SCHEMA))
        }
    }
}
