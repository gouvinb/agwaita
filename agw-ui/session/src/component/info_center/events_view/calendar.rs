use super::event_item::{
    EventData,
    EventItem,
};
use crate::system_state::global_service::GlobalSystemService;
use agw_service::{
    calendar::types::CalendarEvent,
    signal::SignalHandler,
    time::TimeUnit,
};
use catalyser::stdx::extension::str_extension::MultilineStr;
use chrono::{
    Datelike,
    Local,
    Locale,
    Timelike,
};
use gtk4::prelude::*;
use relm4::{
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    SimpleComponent,
    factory::FactoryVecDeque,
    gtk,
};
use std::{
    str::FromStr,
    sync::Arc,
};

pub struct Calendar {
    day_week: String,
    date: String,
    selected_date: String,
    selected_naive_date: Option<chrono::NaiveDate>,
    is_selected_today: bool,
    today_events: FactoryVecDeque<EventItem>,
    selected_day_events: FactoryVecDeque<EventItem>,
    locale: Locale,
    global_service: Arc<GlobalSystemService>,
    calendar_widget: gtk::Calendar, // Store widget reference to mark days
    _day_handler: Option<SignalHandler>,
}

#[derive(Debug)]
pub enum CalendarInput {
    DaySelected(gtk::glib::DateTime),
    MonthChanged,
    UpdateEvents(Vec<CalendarEvent>),
    ResetToToday,
    DayChanged,
}

#[relm4::component(pub)]
impl SimpleComponent for Calendar {
    type Init = Arc<GlobalSystemService>;
    type Input = CalendarInput;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,

            gtk::Label {
                add_css_class: "title-5",
                inline_css: "
                |font-weight: bold;
                ".trim_margin().as_str(),

                #[watch]
                set_label: &model.day_week,
            },
            gtk::Label {
                add_css_class: "title-4",

                #[watch]
                set_label: &model.date,
            },
            #[name = "calendar_widget"]
            gtk::Calendar {
                inline_css: "
                |background: transparent;
                |box-shadow: none;
                |border: none;
                |outline: none;
                ".trim_margin().as_str(),

                connect_day_selected[sender] => move |cal| {
                    sender.input(CalendarInput::DaySelected(cal.date()));
                },

                connect_next_month[sender] => move |_| {
                    sender.input(CalendarInput::MonthChanged);
                },

                connect_prev_month[sender] => move |_| {
                    sender.input(CalendarInput::MonthChanged);
                },

                connect_next_year[sender] => move |_| {
                    sender.input(CalendarInput::MonthChanged);
                },

                connect_prev_year[sender] => move |_| {
                    sender.input(CalendarInput::MonthChanged);
                },
            },
            gtk::Separator {},
            gtk::ScrolledWindow {
                set_propagate_natural_width: true,
                set_propagate_natural_height: true,
                set_hexpand: true,
                set_vexpand: true,
                set_width_request: 320,
                set_max_content_height: 320,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    // Today events
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,

                        gtk::Label {
                            add_css_class: "title-5",
                            inline_css: "
                            |font-weight: bold;
                            ".trim_margin().as_str(),
                            set_halign: gtk::Align::Start,

                            set_label: "Today",
                        },
                        #[local_ref]
                        today_events_box -> gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                        },
                    },
                    gtk::Box {
                        set_margin_top: 8,
                    },
                    // Selected day events
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,

                        #[watch]
                        set_visible: !model.is_selected_today,

                        gtk::Label {
                            add_css_class: "title-5",
                            inline_css: "
                            |font-weight: bold;
                            ".trim_margin().as_str(),
                            set_halign: gtk::Align::Start,

                            #[watch]
                            set_label: &model.selected_date,
                        },
                        #[local_ref]
                        selected_events_box -> gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_width_request: 320 - 16,
                            set_can_focus: false,
                            set_focusable: false,
                            set_spacing: 8,
                        },

                    },
                },
            },
        }
    }

    fn init(global_service: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let now = Local::now();

        let today_events = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .detach();

        let selected_day_events = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .detach();

        let locale = Self::get_system_locale();

        let model = Calendar {
            day_week: Self::capitalize_first(now.format_localized("%A", locale).to_string()),
            date: Self::capitalize_first(now.format_localized("%d %B %Y", locale).to_string()),
            selected_date: Self::capitalize_first(now.format_localized("%d %B %Y", locale).to_string()),
            selected_naive_date: None,
            is_selected_today: true,
            today_events,
            selected_day_events,
            locale,
            global_service,
            calendar_widget: gtk::Calendar::new(), // Temporary, will be replaced
            _day_handler: None,
        };

        let today_events_box = model.today_events.widget();
        let selected_events_box = model.selected_day_events.widget();

        let widgets = view_output!();

        // Replace with actual widget reference
        let mut model = Calendar {
            calendar_widget: widgets.calendar_widget.clone(),
            ..model
        };

        // Load initial events and mark days
        sender.input(CalendarInput::UpdateEvents(Vec::new()));

        // Mark days with events for current month
        sender.input(CalendarInput::MonthChanged);

        // Subscribe to calendar events updates
        let receiver = model.global_service.subscribe();
        let sender_clone = sender.clone();
        std::thread::spawn(move || {
            while let Ok(update) = receiver.recv() {
                if let crate::system_state::messages::SystemStateUpdate::CalendarEvents(events) = update {
                    sender_clone.input(CalendarInput::UpdateEvents(events));
                }
            }
        });

        // Subscribe to day changes (midnight rollover).
        // Store the handler in the model so it's disconnected when this component is dropped.
        let day_sender = sender.clone();
        model._day_handler = Some(
            model
                .global_service
                .time_service()
                .subscribe(TimeUnit::Day, move |_| {
                    day_sender.input(CalendarInput::DayChanged);
                }),
        );

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, #[allow(unused_variables)] sender: ComponentSender<Self>) {
        match message {
            CalendarInput::DaySelected(date) => {
                let year = date.year();
                let month = date.month() as u32;
                let day = date.day_of_month() as u32;

                if let Some(naive_date) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
                    self.selected_date = Self::capitalize_first(
                        naive_date
                            .format_localized("%A %d %B %Y", self.locale)
                            .to_string(),
                    );

                    let today = Local::now().date_naive();
                    self.is_selected_today = naive_date == today;
                    self.selected_naive_date = Some(naive_date);

                    // Update selected day events
                    self.update_selected_day_events();
                }
            },
            CalendarInput::MonthChanged => {
                // Mark days with events for the visible month
                self.mark_days_with_events();

                // Update events for the selected day (GTK Calendar keeps day selection when changing months)
                // GTK Calendar keeps the same day number, so we need to read the current selection
                let date = self.calendar_widget.date();
                let year = date.year();
                let month = date.month() as u32;
                let day = date.day_of_month() as u32;

                if let Some(naive_date) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
                    log::debug!("MonthChanged: updating selected date to {}", naive_date);
                    self.selected_naive_date = Some(naive_date);
                    self.selected_date = Self::capitalize_first(
                        naive_date
                            .format_localized("%A %d %B %Y", self.locale)
                            .to_string(),
                    );
                    let today = Local::now().date_naive();
                    self.is_selected_today = naive_date == today;
                }

                self.update_selected_day_events();
            },
            CalendarInput::UpdateEvents(_events) => {
                // Calendar data changed - fetch events using lazy API
                self.update_today_events_from_service();
                self.update_selected_day_events();

                // Refresh day markers
                self.mark_days_with_events();
            },
            CalendarInput::ResetToToday => {
                // Reset calendar to today's date
                let today = Local::now();
                let today_date = today.date_naive();
                let today_time = today.time();
                let glib_date = gtk::glib::DateTime::from_local(
                    today_date.year(),
                    today_date.month() as i32,
                    today_date.day() as i32,
                    today_time.hour() as i32,
                    today_time.minute() as i32,
                    today_time.second() as f64,
                )
                .unwrap();

                // Select today's date in the calendar widget
                self.calendar_widget.set_date(&glib_date);

                // Update the view
                self.day_week = Self::capitalize_first(today.format_localized("%A", self.locale).to_string());
                self.date = Self::capitalize_first(today.format_localized("%d %B %Y", self.locale).to_string());
                self.selected_date = Self::capitalize_first(today.format_localized("%d %B %Y", self.locale).to_string());
                self.selected_naive_date = Some(today.date_naive());
                self.is_selected_today = true;

                // Update events
                self.update_today_events_from_service();
                self.update_selected_day_events();
                self.mark_days_with_events();
            },
            CalendarInput::DayChanged => {
                let today = Local::now();
                let today_date = today.date_naive();

                self.day_week = Self::capitalize_first(today.format_localized("%A", self.locale).to_string());
                self.date = Self::capitalize_first(today.format_localized("%d %B %Y", self.locale).to_string());

                if self.is_selected_today {
                    let today_time = today.time();
                    let glib_date = gtk::glib::DateTime::from_local(
                        today_date.year(),
                        today_date.month() as i32,
                        today_date.day() as i32,
                        today_time.hour() as i32,
                        today_time.minute() as i32,
                        today_time.second() as f64,
                    )
                    .unwrap();

                    self.calendar_widget.set_date(&glib_date);
                    self.selected_naive_date = Some(today_date);
                    self.selected_date = Self::capitalize_first(
                        today_date
                            .format_localized("%A %d %B %Y", self.locale)
                            .to_string(),
                    );
                    self.is_selected_today = true;
                } else {
                    self.is_selected_today = self.selected_naive_date == Some(today_date);
                }

                self.update_today_events_from_service();
                self.update_selected_day_events();
                self.mark_days_with_events();
            },
        }
    }
}

impl Calendar {
    fn update_today_events_from_service(&mut self) {
        let today = Local::now().date_naive();
        let events = self
            .global_service
            .calendar_service()
            .get_events_for_date(today);

        let mut today_events_guard = self.today_events.guard();
        today_events_guard.clear();

        for event in &events {
            today_events_guard.push_back(Self::convert_event(event));
        }
    }

    fn update_selected_day_events(&mut self) {
        // Get events for selected day using new lazy API
        if let Some(naive_date) = self.selected_naive_date {
            let events = self
                .global_service
                .calendar_service()
                .get_events_for_date(naive_date);
            self.update_selected_day_events_from_list(&events);
        }
    }

    fn update_selected_day_events_from_list(&mut self, events: &[CalendarEvent]) {
        let Some(selected_date) = self.selected_naive_date else {
            return;
        };

        let mut events_guard = self.selected_day_events.guard();
        events_guard.clear();

        for event in events {
            if event.start.date_naive() == selected_date {
                events_guard.push_back(Self::convert_event(event));
            }
        }
    }

    fn convert_event(event: &CalendarEvent) -> EventData {
        EventData {
            title: event.summary.clone(),
            description: event.description.clone(),
            start: event.start,
            end: event.end,
            color: event
                .color
                .clone()
                .unwrap_or_else(|| "@accent_color".to_string()),
            is_all_day: event.is_all_day,
        }
    }

    /// Mark days with events on the gtk::Calendar widget
    fn mark_days_with_events(&self) {
        // Get current displayed month/year from calendar widget
        let date = self.calendar_widget.date();
        let year = date.year();
        let month = date.month() as u32;

        // Clear all marks
        self.calendar_widget.clear_marks();

        // Get days with events for this month
        let days = self
            .global_service
            .calendar_service()
            .get_days_with_events(year, month);

        // Mark each day that has events
        for day in days {
            self.calendar_widget.mark_day(day);
        }
    }
}

impl Calendar {
    fn get_system_locale() -> Locale {
        std::env::var("LANG")
            .or_else(|_| std::env::var("LC_TIME"))
            .or_else(|_| std::env::var("LC_ALL"))
            .ok()
            .and_then(|lang| Locale::from_str(&lang).ok())
            .unwrap_or(Locale::default())
    }

    fn capitalize_first(s: String) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => s,
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}
