//! Brightness Service
//!
//! Manages screen brightness with event-driven monitoring and D-Bus logind integration

mod monitor;
mod service;

pub use service::BrightnessService;
