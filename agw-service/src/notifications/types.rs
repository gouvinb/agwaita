//! Types and enums for notification service

use serde::{
    Deserialize,
    Serialize,
};

/// Urgency level of a notification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Urgency {
    /// Low urgency
    Low = 0,
    /// Normal urgency
    Normal = 1,
    /// Critical urgency
    Critical = 2,
}

impl From<u8> for Urgency {
    fn from(value: u8) -> Self {
        match value {
            0 => Urgency::Low,
            2 => Urgency::Critical,
            _ => Urgency::Normal,
        }
    }
}

impl From<i64> for Urgency {
    fn from(value: i64) -> Self {
        match value {
            0 => Urgency::Low,
            2 => Urgency::Critical,
            _ => Urgency::Normal,
        }
    }
}

impl Default for Urgency {
    fn default() -> Self {
        Urgency::Normal
    }
}

/// Reason why a notification was closed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum ClosedReason {
    /// The notification expired (timeout)
    Expired = 1,
    /// The notification was dismissed by the user
    DismissedByUser = 2,
    /// The notification was closed by a call to CloseNotification
    Closed = 3,
    /// Undefined/reserved reason
    Undefined = 4,
}

impl From<u32> for ClosedReason {
    fn from(value: u32) -> Self {
        match value {
            1 => ClosedReason::Expired,
            2 => ClosedReason::DismissedByUser,
            3 => ClosedReason::Closed,
            _ => ClosedReason::Undefined,
        }
    }
}

/// State of a notification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    /// Notification is being created
    Draft,
    /// Notification was sent to the daemon
    Sent,
    /// Notification was received by the daemon
    Received,
}
