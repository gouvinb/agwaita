//! Calendar service module
//!
//! Provides integration with GNOME Calendar via Evolution Data Server D-Bus interface.
//! Supports event monitoring, recurrence expansion, and calendar colors.

pub mod service;
pub mod types;

pub use service::CalendarService;
pub use types::CalendarEvent;
