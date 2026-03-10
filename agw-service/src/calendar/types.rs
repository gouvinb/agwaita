//! Calendar event types and structures

use chrono::{
    DateTime,
    Local,
};

/// Represents a calendar event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    /// Unique identifier (iCalendar UID)
    pub uid: String,

    /// Event summary/title
    pub summary: String,

    /// Event description
    pub description: Option<String>,

    /// Event color (CSS color string)
    pub color: Option<String>,

    /// Whether this is an all-day event
    pub is_all_day: bool,

    /// Event start time
    pub start: DateTime<Local>,

    /// Event end time
    pub end: DateTime<Local>,

    /// Event location
    pub location: Option<String>,

    /// Calendar source name
    pub calendar_name: Option<String>,
}

impl CalendarEvent {
    pub fn builder() -> CalendarEventBuilder {
        CalendarEventBuilder::default()
    }

    /// Check if this event is happening on a specific date
    pub fn is_on_date(&self, date: &DateTime<Local>) -> bool {
        let event_date = self.start.date_naive();
        let check_date = date.date_naive();
        event_date == check_date
    }

    /// Check if this event is currently happening
    pub fn is_happening_now(&self) -> bool {
        let now = Local::now();
        self.start <= now && now <= self.end
    }

    /// Check if this event is in the future
    pub fn is_upcoming(&self) -> bool {
        self.start > Local::now()
    }
}

impl Ord for CalendarEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start.cmp(&other.start)
    }
}

impl PartialOrd for CalendarEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEventBuilder {
    /// Unique identifier (iCalendar UID)
    pub uid: Option<String>, // mandatory

    /// Event summary/title
    pub summary: Option<String>, // mandatory

    /// Event description
    pub description: Option<String>,

    /// Event color (CSS color string)
    pub color: Option<String>,

    /// Whether this is an all-day event
    pub is_all_day: Option<bool>, // mandatory

    /// Event start time
    pub start: Option<DateTime<Local>>, // mandatory

    /// Event end time
    pub end: Option<DateTime<Local>>, // mandatory

    /// Event location
    pub location: Option<String>,

    /// Calendar source name
    pub calendar_name: Option<String>,
}

impl Default for CalendarEventBuilder {
    fn default() -> Self {
        CalendarEventBuilder {
            uid: None,
            summary: None,
            description: None,
            color: None,
            is_all_day: None,
            start: None,
            end: None,
            location: None,
            calendar_name: None,
        }
    }
}

impl Ord for CalendarEventBuilder {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start.cmp(&other.start)
    }
}

impl PartialOrd for CalendarEventBuilder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl CalendarEventBuilder {
    /// Build CalendarEvent, panics if mandatory fields are missing
    pub fn build(&self) -> CalendarEvent {
        CalendarEvent {
            uid: self.uid.clone().unwrap(),
            summary: self.summary.clone().unwrap(),
            description: self.description.clone(),
            color: self.color.clone(),
            is_all_day: self.is_all_day.clone().unwrap(),
            start: self.start.clone().unwrap(),
            end: self.end.clone().unwrap(),
            location: self.location.clone(),
            calendar_name: self.calendar_name.clone(),
        }
    }

    /// Try to build CalendarEvent, returns None if mandatory fields are missing
    pub fn try_build(&self) -> Option<CalendarEvent> {
        Some(CalendarEvent {
            uid: self.uid.clone()?,
            summary: self.summary.clone()?,
            description: self.description.clone(),
            color: self.color.clone(),
            is_all_day: self.is_all_day?,
            start: self.start?,
            end: self.end?,
            location: self.location.clone(),
            calendar_name: self.calendar_name.clone(),
        })
    }

    pub fn uid(mut self, uid: String) -> Self {
        self.uid = Some(uid);
        self
    }

    pub fn summary(mut self, summary: String) -> Self {
        self.summary = Some(summary);
        self
    }

    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn color(mut self, color: String) -> Self {
        self.color = Some(color);
        self
    }

    pub fn all_day(mut self, is_all_day: bool) -> Self {
        self.is_all_day = Some(is_all_day);
        self
    }

    pub fn start(mut self, start: DateTime<Local>) -> Self {
        self.start = Some(start);
        self
    }

    pub fn end(mut self, end: DateTime<Local>) -> Self {
        self.end = Some(end);
        self
    }

    pub fn location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }

    pub fn calendar_name(mut self, calendar_name: String) -> Self {
        self.calendar_name = Some(calendar_name);
        self
    }
}
