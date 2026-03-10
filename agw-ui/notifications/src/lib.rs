//! Agwaita Notifications
//!
//! Centralized notification system providing:
//! - Notification store with subscriber pattern
//! - Reusable UI components (list, popup, history)
//! - Adapter for agw-service notifications (NotificationService)
//!
//! The actual D-Bus daemon is implemented in agw-service crate.

pub mod components;
pub mod message;
pub mod model;
pub mod service;

pub use components::{
    NotificationList,
    NotificationListConfig,
    NotificationListInput,
    NotificationPopup,
    NotificationPopupConfig,
    NotificationPopupInput,
};
pub use model::{
    Notification,
    NotificationAction,
    NotificationEvent,
    NotificationUrgency,
    NotificationVisibility,
};
pub use service::{
    NotificationServiceAdapter,
    NotificationStore,
};
