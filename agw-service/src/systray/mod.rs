//! SystemTray service implementing org.kde.StatusNotifierWatcher
//!
//! This module provides systray functionality compatible with the StatusNotifier specification.

pub mod dbusmenu;
pub mod tray_item;
pub mod watcher;

// Re-export DBusMenu types
pub use dbusmenu::{
    DBusMenuProxy,
    Layout,
    LayoutProps,
};
// Re-export StatusNotifierItem types
pub use tray_item::{
    Category,
    Icon,
    Status,
    StatusNotifierItemProxy,
    Tooltip,
};
// Re-export StatusNotifierWatcher
pub use watcher::{
    StatusNotifierWatcher,
    StatusNotifierWatcherProxy,
};
