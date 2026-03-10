//! Agwaita Application Launcher
//!
//! Fast, memory-efficient application launcher with:
//! - Desktop entries scanner from XDG_DATA_DIRS
//! - Fuzzy search
//! - GSettings favorite-apps integration
//! - Inotify watcher for live updates

pub mod desktop_entries;
pub mod desktop_entries_service;
pub mod favorites;
pub mod message;
pub mod model;
pub mod search;
pub mod window;

pub use message::app_launcher_toggle;
pub use model::DesktopEntry;
pub use window::{
    AppLauncherWindow,
    AppLauncherWindowConfig,
    AppLauncherWindowInput,
};
