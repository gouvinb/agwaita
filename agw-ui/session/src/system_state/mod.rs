//! System State Management

pub mod brightness_adapter;
pub mod calendar_adapter;
pub mod global_service;
pub mod messages;
pub mod privacy_adapter;
pub mod systemd_failed;

pub use global_service::GlobalSystemService;
pub use messages::SystemStateUpdate;
