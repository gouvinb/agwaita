//! Time Service
//!
//! Centralized time tick service with unit-based subscriptions.

mod service;

pub use service::{
    TimeService,
    TimeUnit,
};
