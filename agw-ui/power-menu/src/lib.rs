//! Agwaita Power Menu
//!
//! System power management interface providing:
//! - Lock screen (via loginctl)
//! - Log-out current user
//! - System reboot
//! - System shutdown

pub mod message;
pub mod model;
pub mod window;

pub use message::power_menu_toggle;
pub use model::PowerMenuAction;
pub use window::{
    PowerMenuWindow,
    PowerMenuWindowConfig,
    PowerMenuWindowInput,
};
