use crate::{
    runtime,
    signal::{
        Signal,
        SignalHandler,
    },
};
use chrono::{
    DateTime,
    Datelike,
    Local,
    NaiveDate,
    Timelike,
};
use std::{
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    time::Duration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

#[derive(Clone)]
struct TimeSignals {
    second_changed: Signal<DateTime<Local>>,
    minute_changed: Signal<DateTime<Local>>,
    hour_changed: Signal<DateTime<Local>>,
    day_changed: Signal<DateTime<Local>>,
    week_changed: Signal<DateTime<Local>>,
    month_changed: Signal<DateTime<Local>>,
    year_changed: Signal<DateTime<Local>>,
}

impl TimeSignals {
    fn new() -> Self {
        Self {
            second_changed: Signal::new(),
            minute_changed: Signal::new(),
            hour_changed: Signal::new(),
            day_changed: Signal::new(),
            week_changed: Signal::new(),
            month_changed: Signal::new(),
            year_changed: Signal::new(),
        }
    }
}

struct TimeState {
    second: u32,
    minute: u32,
    hour: u32,
    day: NaiveDate,
    week_year: i32,
    week: u32,
    month_year: i32,
    month: u32,
    year: i32,
}

pub struct TimeService {
    started: AtomicBool,
    tick_interval: Duration,
    signals: TimeSignals,
    state: Arc<Mutex<TimeState>>,
}

impl TimeService {
    pub fn new() -> Self {
        let now = Local::now();
        let iso_week = now.iso_week();
        Self {
            started: AtomicBool::new(false),
            tick_interval: Duration::from_millis(100),
            signals: TimeSignals::new(),
            state: Arc::new(Mutex::new(TimeState {
                second: now.second(),
                minute: now.minute(),
                hour: now.hour(),
                day: now.date_naive(),
                week_year: iso_week.year(),
                week: iso_week.week(),
                month_year: now.year(),
                month: now.month(),
                year: now.year(),
            })),
        }
    }

    pub fn start(&self) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }

        let tick_interval = self.tick_interval;
        let signals = self.signals.clone();
        let state = Arc::clone(&self.state);

        runtime::spawn(async move {
            let mut interval = tokio::time::interval(tick_interval);
            loop {
                interval.tick().await;

                let now = Local::now();
                let iso_week = now.iso_week();

                let second = now.second();
                let minute = now.minute();
                let hour = now.hour();
                let day = now.date_naive();
                let week_year = iso_week.year();
                let week = iso_week.week();
                let month_year = now.year();
                let month = now.month();
                let year = now.year();

                let (mut second_changed, mut minute_changed, mut hour_changed) = (false, false, false);
                let (mut day_changed, mut week_changed, mut month_changed, mut year_changed) = (false, false, false, false);

                {
                    let mut state = state.lock().unwrap();

                    if second != state.second {
                        state.second = second;
                        second_changed = true;
                    }
                    if minute != state.minute {
                        state.minute = minute;
                        minute_changed = true;
                    }
                    if hour != state.hour {
                        state.hour = hour;
                        hour_changed = true;
                    }
                    if day != state.day {
                        state.day = day;
                        day_changed = true;
                    }
                    if week_year != state.week_year || week != state.week {
                        state.week_year = week_year;
                        state.week = week;
                        week_changed = true;
                    }
                    if month_year != state.month_year || month != state.month {
                        state.month_year = month_year;
                        state.month = month;
                        month_changed = true;
                    }
                    if year != state.year {
                        state.year = year;
                        year_changed = true;
                    }
                }

                if second_changed && signals.second_changed.handler_count() > 0 {
                    signals.second_changed.emit_sync(now.clone());
                }
                if minute_changed && signals.minute_changed.handler_count() > 0 {
                    signals.minute_changed.emit_sync(now.clone());
                }
                if hour_changed && signals.hour_changed.handler_count() > 0 {
                    signals.hour_changed.emit_sync(now.clone());
                }
                if day_changed && signals.day_changed.handler_count() > 0 {
                    signals.day_changed.emit_sync(now.clone());
                }
                if week_changed && signals.week_changed.handler_count() > 0 {
                    signals.week_changed.emit_sync(now.clone());
                }
                if month_changed && signals.month_changed.handler_count() > 0 {
                    signals.month_changed.emit_sync(now.clone());
                }
                if year_changed && signals.year_changed.handler_count() > 0 {
                    signals.year_changed.emit_sync(now);
                }
            }
        });
    }

    pub fn subscribe<F>(&self, unit: TimeUnit, callback: F) -> SignalHandler
    where
        F: Fn(DateTime<Local>) + Send + 'static,
    {
        match unit {
            TimeUnit::Second => self.signals.second_changed.connect(callback),
            TimeUnit::Minute => self.signals.minute_changed.connect(callback),
            TimeUnit::Hour => self.signals.hour_changed.connect(callback),
            TimeUnit::Day => self.signals.day_changed.connect(callback),
            TimeUnit::Week => self.signals.week_changed.connect(callback),
            TimeUnit::Month => self.signals.month_changed.connect(callback),
            TimeUnit::Year => self.signals.year_changed.connect(callback),
        }
    }
}
