//! Notification data model.

use chrono::{
    DateTime,
    Local,
};

/// Notification urgency level (freedesktop.org spec)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationUrgency {
    Low = 0,
    Normal = 1,
    Critical = 2,
}

impl From<u8> for NotificationUrgency {
    fn from(value: u8) -> Self {
        match value {
            0 => NotificationUrgency::Low,
            2 => NotificationUrgency::Critical,
            _ => NotificationUrgency::Normal,
        }
    }
}

/// Notification action button
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
}

/// Notification visibility state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationVisibility {
    Visible,
    Hidden,
    Closed,
}

/// Notification following org.freedesktop.Notifications spec
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: Option<String>,
    pub desktop_entry: Option<String>,
    pub summary: String,
    pub body: Option<String>,
    pub image: Option<String>,
    pub time: DateTime<Local>,
    pub urgency: NotificationUrgency,
    pub actions: Vec<NotificationAction>,
    pub timeout_sec: Option<i32>,
    pub visibility: NotificationVisibility,
}

impl Notification {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u32,
        app_name: String,
        app_icon: Option<String>,
        desktop_entry: Option<String>,
        summary: String,
        body: Option<String>,
        image: Option<String>,
        time: DateTime<Local>,
        urgency: NotificationUrgency,
        actions: Vec<NotificationAction>,
        timeout_ms: Option<i32>,
    ) -> Self {
        Self {
            id,
            app_name,
            app_icon,
            desktop_entry,
            summary,
            body,
            image,
            time,
            urgency,
            actions,
            timeout_sec: timeout_ms,
            visibility: NotificationVisibility::Visible,
        }
    }

    pub fn urgency_css_class(&self) -> &str {
        match self.urgency {
            NotificationUrgency::Low => "border: 1px solid var(--border-color);",
            NotificationUrgency::Normal => "border: 1px solid var(--accent-color);",
            NotificationUrgency::Critical => "border: 1px solid var(--error-color);",
        }
    }

    pub fn has_timeout(&self) -> bool {
        self.timeout_sec.is_some() && self.timeout_sec != Some(-1)
    }

    pub fn get_timeout_duration(&self) -> Option<std::time::Duration> {
        match self.timeout_sec {
            Some(sec) if sec > 0 => Some(std::time::Duration::from_secs(sec as u64)),
            _ => None,
        }
    }
}

impl PartialOrd for Notification {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Notification {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.time.cmp(&self.time)
    }
}
