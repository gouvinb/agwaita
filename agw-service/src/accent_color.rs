//! Accent color management via GSettings.
//!
//! This module provides functionality to get, set, and monitor the system accent color
//! using the `org.gnome.desktop.interface` GSettings schema.

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

/// Available accent colors in GNOME.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentColor {
    Blue,
    Teal,
    Green,
    Yellow,
    Orange,
    Red,
    Pink,
    Purple,
    Slate,
}

impl AccentColor {
    /// Parse an accent color from a string.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "blue" => Some(AccentColor::Blue),
            "teal" => Some(AccentColor::Teal),
            "green" => Some(AccentColor::Green),
            "yellow" => Some(AccentColor::Yellow),
            "orange" => Some(AccentColor::Orange),
            "red" => Some(AccentColor::Red),
            "pink" => Some(AccentColor::Pink),
            "purple" => Some(AccentColor::Purple),
            "slate" => Some(AccentColor::Slate),
            _ => None,
        }
    }

    /// Convert the accent color to its GSettings string value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AccentColor::Blue => "blue",
            AccentColor::Teal => "teal",
            AccentColor::Green => "green",
            AccentColor::Yellow => "yellow",
            AccentColor::Orange => "orange",
            AccentColor::Red => "red",
            AccentColor::Pink => "pink",
            AccentColor::Purple => "purple",
            AccentColor::Slate => "slate",
        }
    }

    /// List all available accent colors.
    pub fn all() -> &'static [AccentColor] {
        &[
            AccentColor::Blue,
            AccentColor::Teal,
            AccentColor::Green,
            AccentColor::Yellow,
            AccentColor::Orange,
            AccentColor::Red,
            AccentColor::Pink,
            AccentColor::Purple,
            AccentColor::Slate,
        ]
    }
}

/// Service to manage the GNOME accent color via GSettings.
pub struct AccentColorService;

pub struct AccentColorMonitor {
    _handler_id: glib::SignalHandlerId,
    settings: gio::Settings,
}

impl AccentColorService {
    const SCHEMA: &'static str = "org.gnome.desktop.interface";
    const KEY: &'static str = "accent-color";

    /// Get the current accent color.
    pub fn get_accent_color() -> AccentColor {
        match Self::get_accent_color_string() {
            Ok(color) => AccentColor::from_str(&color).unwrap_or(AccentColor::Blue),
            Err(e) => {
                error!("Failed to get accent color: {}", e);
                AccentColor::Blue
            },
        }
    }

    /// Set the accent color.
    pub fn set_accent_color(color: AccentColor) -> Result<(), String> {
        debug!("Setting accent color to {:?}", color);

        let settings = gio::Settings::new(Self::SCHEMA);
        if let Some(_) = settings.settings_schema() {
            settings
                .set_string(Self::KEY, color.as_str())
                .map_err(|e| e.to_string())
        } else {
            Err(format!("{} schema not found in settings", Self::SCHEMA))
        }
    }

    /// Monitor accent color changes.
    ///
    /// Returns a handle that must be kept alive.
    pub fn monitor_accent_color<F>(callback: F) -> Option<AccentColorMonitor>
    where
        F: Fn(AccentColor) + Send + 'static,
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
            let color_str = settings.string(Self::KEY);
            match AccentColor::from_str(color_str.as_str()) {
                Some(color) => callback(color),
                None => {
                    warn!("Failed to set accent color: {}", color_str);
                    callback(AccentColor::Blue);
                },
            }
        });

        Some(AccentColorMonitor {
            _handler_id: handler_id,
            settings,
        })
    }

    fn get_accent_color_string() -> Result<String, String> {
        let settings = gio::Settings::new(Self::SCHEMA);
        if let Some(schema) = settings.settings_schema() {
            if schema.has_key(Self::KEY) {
                let color = settings
                    .string(Self::KEY)
                    .trim()
                    .trim_matches('\'')
                    .to_string();

                Ok(color)
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
