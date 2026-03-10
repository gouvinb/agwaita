//! Calendar service using Evolution Data Server D-Bus interface

use super::types::CalendarEvent;
use crate::{
    runtime,
    signal::{
        Signal,
        SignalHandler,
    },
};
use calcard::icalendar::{
    ICalendar,
    ICalendarComponent,
    ICalendarComponentType,
    ICalendarFrequency,
    ICalendarProperty,
    ICalendarRecurrenceRule,
    ICalendarValue,
};
use chrono::{
    DateTime,
    Datelike,
    Duration,
    Local,
    NaiveDate,
    TimeZone,
    Utc,
};
use futures::stream::StreamExt;
use log::{
    debug,
    error,
    warn,
};
use rrule::{
    RRuleSet,
    Tz,
};
use std::{
    cmp::Ordering,
    collections::{
        HashMap,
        HashSet,
    },
    error::Error,
    sync::{
        Arc,
        Mutex,
    },
};
use zbus::{
    Connection,
    proxy,
    zvariant::{
        OwnedObjectPath,
        OwnedValue,
    },
};

/// D-Bus proxy for Evolution Data Server SourceManager
#[proxy(
    interface = "org.freedesktop.DBus.ObjectManager",
    default_service = "org.gnome.evolution.dataserver.Sources5",
    default_path = "/org/gnome/evolution/dataserver/SourceManager"
)]
trait SourceManager {
    fn get_managed_objects(&self) -> zbus::Result<HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>>;

    #[zbus(signal)]
    fn interfaces_added(&self, object_path: OwnedObjectPath, interfaces: HashMap<String, HashMap<String, OwnedValue>>) -> zbus::Result<()>;
}

/// D-Bus proxy for Evolution Data Server CalendarFactory
#[proxy(
    interface = "org.gnome.evolution.dataserver.CalendarFactory",
    default_service = "org.gnome.evolution.dataserver.Calendar8",
    default_path = "/org/gnome/evolution/dataserver/CalendarFactory"
)]
trait CalendarFactory {
    fn open_calendar(&self, uid: &str) -> zbus::Result<(String, String)>;
}

/// D-Bus proxy for Evolution Data Server Calendar
#[proxy(
    interface = "org.gnome.evolution.dataserver.Calendar",
    default_service = "org.gnome.evolution.dataserver.Calendar8"
)]
trait Calendar {
    fn open(&self) -> zbus::Result<Vec<String>>;
    fn get_view(&self, sexp: &str) -> zbus::Result<OwnedObjectPath>;
    fn get_object_list(&self, sexp: &str) -> zbus::Result<Vec<String>>;
}

/// D-Bus proxy for Evolution Data Server CalendarView
#[proxy(
    interface = "org.gnome.evolution.dataserver.CalendarView",
    default_service = "org.gnome.evolution.dataserver.Calendar8"
)]
trait CalendarView {
    fn start(&self) -> zbus::Result<()>;
    fn stop(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn objects_added(&self, objects: Vec<String>) -> zbus::Result<()>;

    #[zbus(signal)]
    fn objects_modified(&self, objects: Vec<String>) -> zbus::Result<()>;

    #[zbus(signal)]
    fn objects_removed(&self, uids: Vec<OwnedValue>) -> zbus::Result<()>;

    #[zbus(signal)]
    fn complete(&self, error_msg: String, error_code: String) -> zbus::Result<()>;
}

/// Calendar service that manages events from GNOME Calendar with lazy expansion
pub struct CalendarService {
    /// Raw iCalendar components with calendar UID: (calendar_uid, component)
    raw_components: Arc<Mutex<Vec<(String, ICalendarComponent)>>>,

    /// Cache of expanded events by month: (year, month) -> events
    expanded_cache: Arc<Mutex<HashMap<(i32, u32), Vec<CalendarEvent>>>>,

    /// Calendar colors: UID -> color (hex format like #62a0ea)
    calendar_colors: Arc<Mutex<HashMap<String, String>>>,

    /// Calendar UIDs we already monitor
    monitored_calendars: Arc<Mutex<HashSet<String>>>,

    /// Signal emitted when calendar events change
    events_changed: Signal<()>,
}

impl CalendarService {
    /// Create a new CalendarService instance
    pub fn new() -> Self {
        Self {
            raw_components: Arc::new(Mutex::new(Vec::new())),
            expanded_cache: Arc::new(Mutex::new(HashMap::new())),
            calendar_colors: Arc::new(Mutex::new(HashMap::new())),
            monitored_calendars: Arc::new(Mutex::new(HashSet::new())),
            events_changed: Signal::new(),
        }
    }

    /// Connect a callback to the events_changed signal
    pub fn connect_events_changed<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn(()) + Send + 'static,
    {
        self.events_changed.connect(callback)
    }

    /// Disconnect a signal handler
    pub fn disconnect(&self, handler: SignalHandler) {
        self.events_changed.disconnect(handler);
    }

    /// Initialize the calendar service and start listening for events
    pub async fn start_monitoring(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Starting calendar monitoring with Evolution Data Server");

        let conn = Connection::session().await?;
        let source_manager = SourceManagerProxy::new(&conn).await?;
        let sources = source_manager.get_managed_objects().await?;

        let mut calendar_uids = Vec::new();
        sources
            .iter()
            .filter_map(|(path, interfaces)| {
                // find Source interfaces
                interfaces
                    .get("org.gnome.evolution.dataserver.Source")
                    .map(|source| (path, source))
            })
            .filter_map(|(path, source)| {
                // "Data" string
                source
                    .get("Data")
                    .and_then(|v| String::try_from(v.clone()).ok())
                    .map(|data| (path, source, data))
            })
            .filter_map(|(path, source, data)| {
                // "UID" value
                source
                    .get("UID")
                    .and_then(|v| String::try_from(v.clone()).ok())
                    .map(|uid| (path, uid, data))
            })
            .filter(|(_, _, data)| Self::is_calendar_source(data))
            .for_each(|(path, uid, data)| {
                // side effect
                debug!("Found calendar source: {} at {:?}", uid, path);

                if self.register_calendar_uid(&uid, &data) {
                    calendar_uids.push(uid);
                }
            });

        if calendar_uids.is_empty() {
            warn!("No calendar sources found in Evolution Data Server");
        }

        debug!("Found {} calendar source(s)", calendar_uids.len());

        for uid in calendar_uids {
            let service = self.clone();
            let conn_clone = conn.clone();
            let uid_clone = uid.clone();

            let _monitor_handle = runtime::spawn(async move {
                if let Err(e) = service.monitor_calendar(conn_clone, uid_clone).await {
                    error!("Error monitoring calendar {}: {}", uid, e);
                }
            });
        }

        let service = self.clone();
        let conn_clone = conn.clone();
        let mut added_stream = source_manager.receive_interfaces_added().await?;
        let _added_handle = runtime::spawn(async move {
            while let Some(signal) = added_stream.next().await {
                let Ok(args) = signal.args() else {
                    continue;
                };
                let Some(source) = args.interfaces.get("org.gnome.evolution.dataserver.Source") else {
                    continue;
                };

                let Some(data) = source
                    .get("Data")
                    .and_then(|v| String::try_from(v.clone()).ok())
                else {
                    continue;
                };

                if !Self::is_calendar_source(&data) {
                    continue;
                }

                let Some(uid) = source
                    .get("UID")
                    .and_then(|v| String::try_from(v.clone()).ok())
                else {
                    continue;
                };

                debug!("Calendar source added: {} at {:?}", uid, args.object_path);

                if !service.register_calendar_uid(&uid, &data) {
                    continue;
                }

                let conn = conn_clone.clone();
                let uid_clone = uid.clone();
                let service_clone = service.clone();
                let _monitor_handle = runtime::spawn(async move {
                    if let Err(e) = service_clone.monitor_calendar(conn, uid_clone).await {
                        error!("Error monitoring calendar {}: {}", uid, e);
                    }
                });
            }
        });

        Ok(())
    }

    /// Monitor a single calendar for events
    async fn monitor_calendar(&self, conn: Connection, uid: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Opening calendar: {}", uid);

        let factory = CalendarFactoryProxy::new(&conn).await?;
        let (calendar_path, _bus_name) = factory.open_calendar(&uid).await?;

        let calendar = CalendarProxy::builder(&conn)
            .path(calendar_path.clone())?
            .build()
            .await?;

        calendar.open().await?;

        let now_timestamps = Utc::now().timestamp();
        let one_year = 365 * 24 * 60 * 60i64;
        let start = now_timestamps - one_year;
        let end = now_timestamps + one_year;

        let sexp = format!(
            "(occur-in-time-range? (make-time \"{}\") (make-time \"{}\"))",
            DateTime::from_timestamp(start, 0)
                .unwrap()
                .format("%Y%m%dT%H%M%SZ"),
            DateTime::from_timestamp(end, 0)
                .unwrap()
                .format("%Y%m%dT%H%M%SZ")
        );

        let view_path = calendar.get_view(&sexp).await?;

        let view = CalendarViewProxy::builder(&conn)
            .path(view_path.into_inner())?
            .build()
            .await?;

        let initial_objects = calendar.get_object_list(&sexp).await?;
        debug!(
            "Initial fetch returned {} calendar objects for {}",
            initial_objects.len(),
            uid
        );
        let initial_components = Self::parse_raw_components(&initial_objects, &uid);
        if initial_components.is_empty() {
            debug!("No initial components to add for {}", uid);
        } else {
            let mut store = self.raw_components.lock().unwrap();
            store.extend(initial_components);
            let total_components = store.len();
            drop(store);

            self.expanded_cache.lock().unwrap().clear();
            self.events_changed.emit_sync(());
            debug!(
                "Initial components stored, total components: {}",
                total_components
            );
        }

        let service_clone = self.clone();
        let uid_clone = uid.clone();
        let mut added_stream = view.receive_objects_added().await?;

        let _added_handle = runtime::spawn(async move {
            while let Some(signal) = added_stream.next().await {
                if let Ok(args) = signal.args() {
                    let ical_objects = args.objects;
                    debug!("Received {} calendar objects from EDS", ical_objects.len());

                    let components = Self::parse_raw_components(&ical_objects, &uid_clone);

                    if components.is_empty() {
                        debug!("No new components to add, skipping");
                        continue;
                    }

                    let mut store = service_clone.raw_components.lock().unwrap();
                    store.extend(components);
                    let total_components = store.len();
                    drop(store);

                    debug!("Total components in store: {}", total_components);

                    let mut cache = service_clone.expanded_cache.lock().unwrap();
                    let cleared_months: Vec<_> = cache.keys().cloned().collect();
                    cache.clear();
                    drop(cache);
                    debug!("Cleared {} cached months", cleared_months.len());

                    service_clone.events_changed.emit_sync(());
                }
            }
        });

        let service_clone2 = self.clone();
        let uid_clone2 = uid.clone();
        let mut modified_stream = view.receive_objects_modified().await?;
        let _modified_handle = runtime::spawn(async move {
            while let Some(signal) = modified_stream.next().await {
                if let Ok(args) = signal.args() {
                    let ical_objects = args.objects;
                    debug!("Modified {} calendar objects", ical_objects.len());

                    let components = Self::parse_raw_components(&ical_objects, &uid_clone2);

                    let mut store = service_clone2.raw_components.lock().unwrap();
                    for (_calendar_uid, component) in &components {
                        if let Some(event_uid) = Self::get_component_uid(component) {
                            store.retain(|(_cal_uid, c)| Self::get_component_uid(c) != Some(event_uid.clone()));
                        }
                    }
                    store.extend(components);
                    drop(store);

                    service_clone2.expanded_cache.lock().unwrap().clear();

                    service_clone2.events_changed.emit_sync(());
                    debug!("Emitted events_changed signal");
                }
            }
        });

        let service_clone3 = self.clone();
        let mut removed_stream = view.receive_objects_removed().await?;
        let _removed_handle = runtime::spawn(async move {
            while let Some(signal) = removed_stream.next().await {
                if let Ok(args) = signal.args() {
                    debug!("Removed {} calendar objects", args.uids.len());

                    service_clone3.expanded_cache.lock().unwrap().clear();

                    service_clone3.events_changed.emit_sync(());
                    debug!("Emitted events_changed signal");
                }
            }
        });

        view.start().await?;
        debug!("Calendar view started for {}", uid);

        Ok(())
    }

    /// Parse raw iCalendar strings into ICalendarComponents
    fn parse_raw_components(ical_strings: &[String], calendar_uid: &str) -> Vec<(String, ICalendarComponent)> {
        let mut components = Vec::new();

        ical_strings
            .iter()
            .for_each(|ical_str| match ICalendar::parse(ical_str) {
                Ok(calendar) => {
                    calendar
                        .components
                        .iter()
                        .filter(|component| component.component_type == ICalendarComponentType::VEvent)
                        .for_each(|component| components.push((calendar_uid.to_string(), component.clone())));
                },
                Err(e) => {
                    error!("Failed to parse iCalendar: {:?}", e);
                },
            });

        components
    }

    /// Extract calendar color from EDS source data (Color=#xxxxxx in [Calendar] section)
    fn extract_calendar_color(data: &str) -> Option<String> {
        if let Some(calendar_section_start) = data.find("[Calendar]") {
            let after_calendar = &data[calendar_section_start..];

            for line in after_calendar.lines() {
                if line.starts_with("Color=") {
                    return Some(line[6..].trim().to_string());
                }
                if line.starts_with('[') && line != "[Calendar]" {
                    break;
                }
            }
        }
        None
    }

    fn get_component_uid(component: &ICalendarComponent) -> Option<String> {
        component
            .property(&ICalendarProperty::Uid)
            .and_then(|entry| entry.values.first())
            .and_then(|val| match val {
                ICalendarValue::Text(s) => Some(s.clone()),
                _ => None,
            })
    }

    /// Get events for a specific date (lazy expansion)
    pub fn get_events_for_date(&self, date: NaiveDate) -> Vec<CalendarEvent> {
        let year = date.year();
        let month = date.month();

        debug!("Get events for {}", date);

        self.ensure_month_expanded(year, month);

        let mut events: Vec<CalendarEvent> = self
            .expanded_cache
            .lock()
            .unwrap()
            .get(&(year, month))
            .map(|events| {
                events
                    .iter()
                    .filter(|e| e.start.date_naive() == date)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        events.sort_by(|a, b| match (a.is_all_day, b.is_all_day) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.start.cmp(&b.start),
        });

        if !events.is_empty() {
            debug!("Found {} events for {}", events.len(), date);
        }
        events
    }

    pub fn get_days_with_events(&self, year: i32, month: u32) -> HashSet<u32> {
        self.ensure_month_expanded(year, month);

        self.expanded_cache
            .lock()
            .unwrap()
            .get(&(year, month))
            .map(|events| events.iter().map(|e| e.start.day()).collect())
            .unwrap_or_default()
    }

    pub fn get_today_events(&self) -> Vec<CalendarEvent> {
        let today = Local::now().date_naive();
        self.get_events_for_date(today)
    }

    /// Ensure a specific month is expanded in cache
    fn ensure_month_expanded(&self, year: i32, month: u32) {
        let mut cache = self.expanded_cache.lock().unwrap();

        if cache.contains_key(&(year, month)) {
            debug!("Month {}-{:02} already in cache", year, month);
            return;
        }

        let raw_count = self.raw_components.lock().unwrap().len();
        debug!("Have {} raw components to process", raw_count);

        if raw_count == 0 {
            debug!(
                "No raw components yet, skipping cache for {}-{:02}",
                year, month
            );
            return;
        }

        let events = self.expand_month(year, month);
        debug!(
            "Inserting {} events into cache for {}-{:02}",
            events.len(),
            year,
            month
        );
        cache.insert((year, month), events);
    }

    /// Expand all raw components for a specific month
    fn expand_month(&self, year: i32, month: u32) -> Vec<CalendarEvent> {
        debug!("Expanding events for {}-{:02}", year, month);

        let raw_components = self.raw_components.lock().unwrap();
        let mut events = Vec::new();

        // Deduplicate by UID (EDS sometimes sends duplicates)
        let mut unique_components = HashMap::new();

        raw_components
            .iter()
            .filter_map(|(calendar_uid, component)| Self::get_component_uid(component).map(|uid| (uid, calendar_uid.clone(), component.clone())))
            .for_each(|(uid, calendar_uid, component)| {
                unique_components.insert(uid, (calendar_uid, component));
            });

        if unique_components.len() < raw_components.len() {
            debug!(
                "Deduplicated {} components to {} unique UIDs",
                raw_components.len(),
                unique_components.len()
            );
        }

        let components: Vec<_> = unique_components.into_values().collect();

        let month_start = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let month_end = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
        };

        let mut rrule_count = 0;
        let mut expanded_event_count = 0;
        components
            .iter()
            .filter(|(_, component)| !Self::has_recurrence_id(component))
            .for_each(|(calendar_uid, component)| {
                if Self::has_rrule(component) {
                    rrule_count += 1;
                    if let Some(recurring_events) = Self::expand_recurring_event(
                        component,
                        month_start,
                        month_end,
                        calendar_uid,
                        &self.calendar_colors,
                    ) {
                        expanded_event_count += recurring_events.len();
                        events.extend(recurring_events);
                    }
                } else if let Some(event) = Self::parse_component_to_event(
                    component,
                    month_start,
                    month_end,
                    calendar_uid,
                    &self.calendar_colors,
                ) {
                    events.push(event);
                }
            });

        debug!(
            "For {}, found {} component(s) with RRULE",
            components.len(),
            rrule_count
        );
        debug!("Expanded {} recurring events", expanded_event_count);

        // Second pass: handle RECURRENCE-ID exceptions that override recurring occurrences
        let mut exceptions_processed = 0;
        components
            .iter()
            .filter(|(_, component)| Self::has_recurrence_id(component))
            .for_each(|(calendar_uid, component)| {
                exceptions_processed += 1;

                let uid = Self::get_component_uid(component);
                let recurrence_id_entry = component.property(&ICalendarProperty::RecurrenceId);

                debug!(
                    "Processing exception {} with UID: {:?}",
                    exceptions_processed, uid
                );

                if let (Some(uid), Some(recurrence_id_entry)) = (uid, recurrence_id_entry) {
                    let exception_event = Self::parse_component_to_event(
                        component,
                        month_start,
                        month_end,
                        calendar_uid,
                        &self.calendar_colors,
                    );

                    if exception_event.is_none() {
                        debug!("  parse_component_to_event returned None for exception");
                        return;
                    }
                    let exception_event = exception_event.unwrap();

                    debug!(
                        "  Exception parsed: {} at {}",
                        exception_event.summary, exception_event.start
                    );

                    if let Some(recurrence_id_value) = recurrence_id_entry.values.first() {
                        if let Some((original_dt, _)) = Self::parse_datetime_value(recurrence_id_value) {
                            let before_count = events.len();
                            let matching_events: Vec<_> = events
                                .iter()
                                .filter(|e| e.uid.starts_with(&uid) && e.start == original_dt)
                                .map(|e| (e.uid.clone(), e.start))
                                .collect();

                            debug!(
                                "Looking for occurrence to replace: UID starts with '{}', start = {}",
                                uid, original_dt
                            );
                            debug!(
                                "  Found {} matching events to remove",
                                matching_events.len()
                            );
                            for (event_uid, event_start) in &matching_events {
                                debug!("    Match: UID='{}', start={}", event_uid, event_start);
                            }

                            events.retain(|e| !(e.uid.starts_with(&uid) && e.start == original_dt));

                            let after_count = events.len();
                            debug!(
                                "Replaced {} occurrence(s) at {} with exception for UID {}",
                                before_count - after_count,
                                original_dt,
                                uid
                            );
                        }
                    }

                    events.push(exception_event);
                }
            });

        debug!("Expanded {} events for {}-{:02}", events.len(), year, month);
        if events.is_empty() && !components.is_empty() {
            let sample_count = components.len().min(5);
            debug!(
                "No events expanded for {}-{:02}. Sample of {} component(s):",
                year, month, sample_count
            );
            for (calendar_uid, component) in components.iter().take(sample_count) {
                let uid = Self::get_component_uid(component).unwrap_or_else(|| "<none>".to_string());
                let summary = component
                    .property(&ICalendarProperty::Summary)
                    .and_then(|e| e.values.first())
                    .and_then(|v| match v {
                        ICalendarValue::Text(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "<no summary>".to_string());
                let dtstart = component
                    .property(&ICalendarProperty::Dtstart)
                    .and_then(|e| e.values.first())
                    .and_then(|v| Self::parse_datetime_value(v))
                    .map(|(dt, _)| dt.to_string())
                    .unwrap_or_else(|| "<no dtstart>".to_string());

                debug!(
                    "  component uid={} calendar_uid={} summary='{}' dtstart={}",
                    uid, calendar_uid, summary, dtstart
                );
            }
        }
        events
    }

    fn has_rrule(component: &ICalendarComponent) -> bool {
        component.property(&ICalendarProperty::Rrule).is_some()
    }

    fn has_recurrence_id(component: &ICalendarComponent) -> bool {
        component
            .property(&ICalendarProperty::RecurrenceId)
            .is_some()
    }

    /// Convert ICalendarRecurrenceRule to RRULE string format
    fn recurrence_rule_to_string(rrule: &ICalendarRecurrenceRule) -> String {
        let mut parts = Vec::new();

        let freq = match rrule.freq {
            ICalendarFrequency::Secondly => "SECONDLY",
            ICalendarFrequency::Minutely => "MINUTELY",
            ICalendarFrequency::Hourly => "HOURLY",
            ICalendarFrequency::Daily => "DAILY",
            ICalendarFrequency::Weekly => "WEEKLY",
            ICalendarFrequency::Monthly => "MONTHLY",
            ICalendarFrequency::Yearly => "YEARLY",
        };
        parts.push(format!("FREQ={}", freq));

        if let Some(until) = rrule.until.as_ref().and_then(|u| {
            Some((
                u.year?,
                u.month?,
                u.day?,
                u.hour.unwrap_or(0),
                u.minute.unwrap_or(0),
                u.second.unwrap_or(0),
            ))
        }) {
            parts.push(format!(
                "UNTIL={:04}{:02}{:02}T{:02}{:02}{:02}Z",
                until.0, until.1, until.2, until.3, until.4, until.5
            ));
        }

        if let Some(count) = rrule.count {
            parts.push(format!("COUNT={}", count));
        }

        if let Some(interval) = rrule.interval {
            parts.push(format!("INTERVAL={}", interval));
        }

        if !rrule.byday.is_empty() {
            let days: Vec<String> = rrule
                .byday
                .iter()
                .map(|day| {
                    use calcard::icalendar::ICalendarWeekday;
                    let wd = match day.weekday {
                        ICalendarWeekday::Sunday => "SU",
                        ICalendarWeekday::Monday => "MO",
                        ICalendarWeekday::Tuesday => "TU",
                        ICalendarWeekday::Wednesday => "WE",
                        ICalendarWeekday::Thursday => "TH",
                        ICalendarWeekday::Friday => "FR",
                        ICalendarWeekday::Saturday => "SA",
                    };
                    if let Some(ordwk) = day.ordwk {
                        format!("{}{}", ordwk, wd)
                    } else {
                        wd.to_string()
                    }
                })
                .collect();
            parts.push(format!("BYDAY={}", days.join(",")));
        }

        if !rrule.bymonthday.is_empty() {
            let days: Vec<String> = rrule.bymonthday.iter().map(|d| d.to_string()).collect();
            parts.push(format!("BYMONTHDAY={}", days.join(",")));
        }

        if !rrule.bymonth.is_empty() {
            let months: Vec<String> = rrule.bymonth.iter().map(|m| m.to_string()).collect();
            parts.push(format!("BYMONTH={}", months.join(",")));
        }

        parts.join(";")
    }

    /// Expand a recurring event for a given time range
    fn expand_recurring_event(
        component: &ICalendarComponent,
        month_start: NaiveDate,
        month_end: NaiveDate,
        calendar_uid: &str,
        calendar_colors: &Arc<Mutex<HashMap<String, String>>>,
    ) -> Option<Vec<CalendarEvent>> {
        let uid = Self::get_component_uid(component);
        if uid.is_none() {
            warn!("expand_recurring_event: No UID found");
            return None;
        }
        let uid = uid.unwrap();

        let summary = component
            .property(&ICalendarProperty::Summary)
            .and_then(|e| e.values.first())
            .and_then(|v| match v {
                ICalendarValue::Text(s) => Some(s.clone()),
                _ => None,
            })?;

        let dtstart_entry = component.property(&ICalendarProperty::Dtstart);
        if dtstart_entry.is_none() {
            warn!("expand_recurring_event: No DTSTART found for '{}'", summary);
            return None;
        }
        let dtstart_value = dtstart_entry.unwrap().values.first();
        if dtstart_value.is_none() {
            warn!(
                "expand_recurring_event: DTSTART has no value for '{}'",
                summary
            );
            return None;
        }
        let dtstart_value = dtstart_value.unwrap();

        let parsed_start = Self::parse_datetime_value(dtstart_value);
        if parsed_start.is_none() {
            warn!(
                "expand_recurring_event: Failed to parse DTSTART value for '{}'",
                summary
            );
            warn!("  DTSTART value type: {:?}", dtstart_value);
            return None;
        }
        let (start, is_all_day) = parsed_start.unwrap();

        let duration = component
            .property(&ICalendarProperty::Dtend)
            .and_then(|e| e.values.first())
            .and_then(|v| Self::parse_datetime_value(v))
            .map(|(end_dt, _)| end_dt.signed_duration_since(start))
            .unwrap_or_else(|| Duration::zero());

        let rrule_value = component
            .property(&ICalendarProperty::Rrule)
            .and_then(|e| e.values.first());

        if rrule_value.is_none() {
            warn!("expand_recurring_event: No RRULE found for '{}'", summary);
            return None;
        }

        let rrule_str = match rrule_value.unwrap() {
            ICalendarValue::Text(s) => s.clone(),
            ICalendarValue::RecurrenceRule(rrule) => Self::recurrence_rule_to_string(rrule),
            other => {
                warn!(
                    "expand_recurring_event: RRULE value is unexpected type for '{}'",
                    summary
                );
                warn!("  Got: {:?}", other);
                return None;
            },
        };

        let mut rrule_string = format!(
            "DTSTART:{}\nRRULE:{}",
            start.format("%Y%m%dT%H%M%S"),
            rrule_str
        );

        // trace!("Expanding RRULE for '{}' (UID: {})", summary, uid);
        // trace!("  DTSTART: {}", start);
        // trace!("  RRULE: {}", rrule_str);

        // let mut exdate_count = 0;
        if let Some(exdate_entry) = component.property(&ICalendarProperty::Exdate) {
            // debug!("  Found EXDATE property with {} values", exdate_entry.values.len());
            exdate_entry
                .values
                .iter()
                .filter_map(|exdate_value| Self::parse_datetime_value(exdate_value))
                .for_each(|(exdate_dt, _)| {
                    rrule_string.push_str(&format!("\nEXDATE:{}", exdate_dt.format("%Y%m%dT%H%M%S")));
                    // exdate_count += 1;
                    // debug!("    EXDATE: {}", exdate_dt);
                });
        }
        // debug!("  Total EXDATE parsed: {}", exdate_count);

        let rrule_set: RRuleSet = match rrule_string.parse() {
            Ok(set) => set,
            Err(e) => {
                warn!("Failed to parse RRULE/EXDATE for '{}': {:?}", summary, e);
                debug!("RRULE string was: {}", rrule_string);
                return None;
            },
        };

        // debug!("  RRuleSet parsed successfully");

        let month_start_dt = Local
            .from_local_datetime(&month_start.and_hms_opt(0, 0, 0).unwrap())
            .single()?;

        let month_end_dt = Local
            .from_local_datetime(&month_end.and_hms_opt(0, 0, 0).unwrap())
            .single()?;

        // trace!("  Generating occurrences from {} to {}", month_start_dt, month_end_dt);

        let occurrences: Vec<DateTime<Tz>> = rrule_set
            .into_iter()
            .skip_while(|dt| dt.with_timezone(&Local) < month_start_dt)
            .take_while(|dt| dt.with_timezone(&Local) < month_end_dt)
            .collect();

        // trace!("  Generated {} occurrences for '{}'", occurrences.len(), summary);
        // if occurrences.len() <= 10 {
        //     for (i, occ) in occurrences.iter().enumerate() {
        //         trace!("    Occurrence {}: {}", i + 1, occ.with_timezone(&Local));
        //     }
        // }

        // Extract optional properties
        let description = component
            .property(&ICalendarProperty::Description)
            .and_then(|e| e.values.first())
            .and_then(|v| match v {
                ICalendarValue::Text(s) => Some(s.clone()),
                _ => None,
            });

        let location = component
            .property(&ICalendarProperty::Location)
            .and_then(|e| e.values.first())
            .and_then(|v| match v {
                ICalendarValue::Text(s) => Some(s.clone()),
                _ => None,
            });

        let color = component
            .property(&ICalendarProperty::Color)
            .and_then(|e| e.values.first())
            .and_then(|v| match v {
                ICalendarValue::Text(s) => Some(s.clone()),
                _ => None,
            });

        // Color priority: event color > calendar color > @accent_color
        let final_color = if let Some(event_color) = color {
            debug!(
                "Recurring event '{}' has explicit COLOR: {}",
                summary, event_color
            );
            Some(event_color)
        } else {
            let colors = calendar_colors.lock().unwrap();
            if let Some(cal_color) = colors.get(calendar_uid) {
                debug!(
                    "Recurring event '{}' using calendar color: {}",
                    summary, cal_color
                );
                Some(cal_color.clone())
            } else {
                debug!(
                    "Recurring event '{}' using default color @accent_color",
                    summary
                );
                Some("@accent_color".to_string())
            }
        };

        let mut events = Vec::new();
        for (idx, occurrence) in occurrences.iter().enumerate() {
            let occurrence_local = occurrence.with_timezone(&Local);
            let occurrence_end = occurrence_local + duration;
            let occurrence_uid = format!("{}-occurrence-{}", uid, idx);

            let mut builder = CalendarEvent::builder()
                .uid(occurrence_uid)
                .summary(summary.clone())
                .description(description.clone().unwrap_or_default())
                .all_day(is_all_day)
                .start(occurrence_local)
                .end(occurrence_end)
                .location(location.clone().unwrap_or_default())
                .calendar_name(String::new());

            if let Some(ref color) = final_color {
                builder = builder.color(color.clone());
            }

            if let Some(event) = builder.try_build() {
                events.push(event);
            }
        }

        debug!(
            "Expanded {} occurrences for recurring event '{}'",
            events.len(),
            summary
        );
        Some(events)
    }

    /// Parse a single ICalendarComponent into CalendarEvent(s) for a given time range
    fn parse_component_to_event(
        component: &ICalendarComponent,
        month_start: NaiveDate,
        month_end: NaiveDate,
        calendar_uid: &str,
        calendar_colors: &Arc<Mutex<HashMap<String, String>>>,
    ) -> Option<CalendarEvent> {
        use calcard::icalendar::{
            ICalendarProperty,
            ICalendarValue,
        };

        let uid = Self::get_component_uid(component)?;

        let summary = component
            .property(&ICalendarProperty::Summary)
            .and_then(|e| e.values.first())
            .and_then(|v| match v {
                ICalendarValue::Text(s) => Some(s.clone()),
                _ => None,
            })?;

        let dtstart_value = component
            .property(&ICalendarProperty::Dtstart)
            .and_then(|e| e.values.first())?;

        let (start, is_all_day) = Self::parse_datetime_value(dtstart_value)?;

        let end = component
            .property(&ICalendarProperty::Dtend)
            .and_then(|e| e.values.first())
            .and_then(|v| Self::parse_datetime_value(v))
            .map(|(dt, _)| dt)
            .unwrap_or(start);

        let start_naive = start.date_naive();
        if start_naive < month_start || start_naive >= month_end {
            return None;
        }

        let description = component
            .property(&ICalendarProperty::Description)
            .and_then(|e| e.values.first())
            .and_then(|v| match v {
                ICalendarValue::Text(s) => Some(s.clone()),
                _ => None,
            });

        let location = component
            .property(&ICalendarProperty::Location)
            .and_then(|e| e.values.first())
            .and_then(|v| match v {
                ICalendarValue::Text(s) => Some(s.clone()),
                _ => None,
            });

        let color = component
            .property(&ICalendarProperty::Color)
            .and_then(|e| e.values.first())
            .and_then(|v| match v {
                ICalendarValue::Text(s) => Some(s.clone()),
                _ => None,
            });

        // Color priority: event color > calendar color > @accent_color
        let final_color = if let Some(event_color) = color {
            debug!("Event '{}' has explicit COLOR: {}", summary, event_color);
            Some(event_color)
        } else {
            let colors = calendar_colors.lock().unwrap();
            if let Some(cal_color) = colors.get(calendar_uid) {
                debug!("Event '{}' using calendar color: {}", summary, cal_color);
                Some(cal_color.clone())
            } else {
                debug!("Event '{}' using default color @accent_color", summary);
                Some("@accent_color".to_string())
            }
        };

        let mut builder = CalendarEvent::builder()
            .uid(uid)
            .summary(summary)
            .description(description.unwrap_or_default())
            .all_day(is_all_day)
            .start(start)
            .end(end)
            .location(location.unwrap_or_default())
            .calendar_name(String::new());

        if let Some(color) = final_color {
            builder = builder.color(color);
        }

        builder.try_build()
    }

    /// Parse ICalendarValue to DateTime<Local> and determine if all-day
    fn parse_datetime_value(value: &ICalendarValue) -> Option<(DateTime<Local>, bool)> {
        match value {
            ICalendarValue::PartialDateTime(partial) => {
                let partial = partial.as_ref();

                let year = partial.year? as i32;
                let month = partial.month? as u32;
                let day = partial.day? as u32;

                let has_time = partial.hour.is_some() || partial.minute.is_some() || partial.second.is_some();

                if has_time {
                    let hour = partial.hour.unwrap_or(0) as u32;
                    let minute = partial.minute.unwrap_or(0) as u32;
                    let second = partial.second.unwrap_or(0) as u32;

                    let naive = NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(hour, minute, second)?;

                    let is_utc = partial.tz_hour.is_some() && partial.tz_hour == Some(0) && partial.tz_minute.unwrap_or(0) == 0;

                    let dt = if is_utc {
                        naive.and_utc().with_timezone(&Local)
                    } else {
                        naive.and_local_timezone(Local).single()?
                    };

                    Some((dt, false))
                } else {
                    let naive = NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(0, 0, 0)?;
                    let dt = naive.and_local_timezone(Local).single()?;
                    Some((dt, true))
                }
            },
            _ => None,
        }
    }

    pub fn clear_cache(&self) {
        self.expanded_cache.lock().unwrap().clear();
    }

    fn is_calendar_source(data: &str) -> bool {
        data.contains("[Calendar]") && (data.contains("BackendName=caldav") || data.contains("BackendName=local"))
    }

    fn register_calendar_uid(&self, uid: &str, data: &str) -> bool {
        let mut monitored = self.monitored_calendars.lock().unwrap();
        if monitored.contains(uid) {
            return false;
        }
        monitored.insert(uid.to_string());

        if let Some(color) = Self::extract_calendar_color(data) {
            self.calendar_colors
                .lock()
                .unwrap()
                .insert(uid.to_string(), color);
        }
        true
    }
}

impl Default for CalendarService {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CalendarService {
    fn clone(&self) -> Self {
        Self {
            raw_components: Arc::clone(&self.raw_components),
            expanded_cache: Arc::clone(&self.expanded_cache),
            calendar_colors: Arc::clone(&self.calendar_colors),
            monitored_calendars: Arc::clone(&self.monitored_calendars),
            events_changed: self.events_changed.clone(),
        }
    }
}
