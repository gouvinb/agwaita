//! Do Not Disturb management via GSettings.
//!
//! Uses `org.gnome.desktop.notifications` and the `show-banners` key.

use gtk4::{
    gio,
    glib,
    prelude::SettingsExt,
};
use log::{
    debug,
    error,
    info,
};
use std::sync::{
    Arc,
    Mutex,
};

/// DND service backed by GSettings.
pub struct DndService {
    dont_disturb: Arc<Mutex<bool>>,
}

impl DndService {
    const SCHEMA: &'static str = "org.gnome.desktop.notifications";
    const KEY: &'static str = "show-banners";

    pub fn new() -> Self {
        let show_banners = match Self::get_show_banners() {
            Ok(value) => value,
            Err(e) => {
                error!("Failed to read DND state: {}", e);
                true
            },
        };
        let dont_disturb = !show_banners;

        debug!(
            "DND service initialized: show-banners={}, dont_disturb={}",
            show_banners, dont_disturb
        );

        Self {
            dont_disturb: Arc::new(Mutex::new(dont_disturb)),
        }
    }

    /// Get the cached DND state.
    pub fn get_dont_disturb(&self) -> bool {
        *self.dont_disturb.lock().unwrap()
    }

    /// Set DND state.
    pub fn set_dont_disturb(&self, enabled: bool) -> Result<(), String> {
        let show_banners = !enabled;

        *self.dont_disturb.lock().unwrap() = enabled;

        let settings = gio::Settings::new(Self::SCHEMA);
        if let Some(schema) = settings.settings_schema() {
            if schema.has_key(Self::KEY) {
                settings
                    .set_boolean(Self::KEY, show_banners)
                    .map_err(|e| e.to_string())?;
            } else {
                return Err(format!(
                    "{} key not found in schema {}",
                    Self::KEY,
                    Self::SCHEMA
                ));
            }
        } else {
            return Err(format!("{} schema not found in settings", Self::SCHEMA));
        }

        info!(
            "DND state set to: {} (show-banners={})",
            enabled, show_banners
        );

        Ok(())
    }

    /// Toggle DND state.
    pub fn toggle_dont_disturb(&self) {
        let current = self.get_dont_disturb();
        if let Err(e) = self.set_dont_disturb(!current) {
            error!("Failed to toggle DND: {}", e);
        }
    }

    /// Start monitoring DND changes via GSettings notifications.
    pub fn monitor_dnd<F>(&self, callback: F) -> DndMonitor
    where
        F: Fn(bool) + Send + 'static,
    {
        let settings = gio::Settings::new(Self::SCHEMA);
        if let Some(schema) = settings.settings_schema() {
            if !schema.has_key(Self::KEY) {
                error!("{} key not found in schema {}", Self::KEY, Self::SCHEMA);
                return DndMonitor::empty();
            }
        } else {
            error!("{} schema not found in settings", Self::SCHEMA);
            return DndMonitor::empty();
        }

        let dont_disturb = Arc::clone(&self.dont_disturb);

        let handler_id = settings.connect_changed(Some(Self::KEY), move |settings, _| {
            let show_banners = settings.boolean(Self::KEY);
            let enabled = !show_banners;

            let mut state = dont_disturb.lock().unwrap();
            let changed = *state != enabled;
            *state = enabled;
            drop(state);

            if changed {
                debug!(
                    "DND state changed via monitor: {} (show-banners={})",
                    enabled, show_banners
                );
            }

            callback(enabled);
        });

        DndMonitor::new(handler_id, settings)
    }

    fn get_show_banners() -> Result<bool, String> {
        let settings = gio::Settings::new(Self::SCHEMA);
        if let Some(schema) = settings.settings_schema() {
            if schema.has_key(Self::KEY) {
                Ok(settings.boolean(Self::KEY))
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

/// Monitor handle for DND changes.
pub struct DndMonitor {
    _handler_id: Option<glib::SignalHandlerId>,
    settings: Option<gio::Settings>,
}
impl DndMonitor {
    fn new(handler_id: glib::SignalHandlerId, settings: gio::Settings) -> Self {
        Self {
            _handler_id: Some(handler_id),
            settings: Some(settings),
        }
    }

    fn empty() -> Self {
        Self {
            _handler_id: None,
            settings: None,
        }
    }
}

impl Default for DndService {
    fn default() -> Self {
        Self::new()
    }
}
