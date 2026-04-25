use crate::system_state::global_service::GlobalSystemService;
use agw_service::{
    signal::SignalHandler,
    time::TimeUnit,
};
use chrono::{
    Local,
    Locale,
};
use gtk4::prelude::{
    BoxExt,
    OrientableExt,
    PopoverExt,
    WidgetExt,
};
use relm4::{
    Component,
    ComponentController,
    ComponentParts,
    ComponentSender,
    Controller,
    SimpleComponent,
    adw,
    gtk,
};
use std::{
    str::FromStr,
    sync::Arc,
};

mod events_view;
mod notification_history;

use events_view::calendar::Calendar;
use notification_history::notification_list::Notifications;

pub struct InfoCenter {
    current_time: String,

    notifications: Controller<Notifications>,
    calendar: Controller<Calendar>,
    _time_handler: SignalHandler,
}

#[derive(Debug)]
pub enum InfoCenterInput {
    UpdateTime,
    PopoverClosed,
}

#[relm4::component(pub)]
impl SimpleComponent for InfoCenter {
    type Input = InfoCenterInput;
    type Output = ();
    type Init = Arc<GlobalSystemService>;

    view! {
        #[root]
        gtk::MenuButton {
            #[watch]
            set_label: &model.current_time,

            #[wrap(Some)]
            #[name = "popover_widget"]
            set_popover = &gtk::Popover {
                adw::Clamp {
                    set_orientation: gtk::Orientation::Vertical,
                    set_maximum_size: 640,
                    set_height_request: 640,

                    gtk::Box {
                        set_spacing: 8,

                        adw::Clamp {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_maximum_size: 320,
                            set_width_request: 320,
                            set_height_request: 640,

                            model.notifications.widget(),
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                            set_height_request: 640,
                        },

                        adw::Clamp {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_maximum_size: 320,
                            set_width_request: 320,
                            set_height_request: 640,

                            model.calendar.widget(),
                        }
                    }
                }
            }
        }
    }

    fn init(global_service: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        // Get the notification store from notification_service_adapter
        let notification_store = global_service.notification_service_adapter().store();
        let notifications = Notifications::builder().launch(notification_store).detach();
        let calendar = Calendar::builder().launch(global_service.clone()).detach();

        // Subscribe to time changes (second precision)
        let time_sender = sender.input_sender().clone();
        let time_handler = global_service
            .time_service()
            .subscribe(TimeUnit::Second, move |_| {
                // Use send() instead of input() to avoid panic when component is dropped
                let _ = time_sender.send(InfoCenterInput::UpdateTime);
            });

        let model = InfoCenter {
            current_time: Self::format_time(),
            notifications,
            calendar,
            _time_handler: time_handler,
        };
        let widgets = view_output!();

        // Connect to popover closed signal to reset calendar
        let popover_sender = sender.input_sender().clone();
        widgets.popover_widget.connect_closed(move |_| {
            popover_sender.send(InfoCenterInput::PopoverClosed).ok();
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, #[allow(unused_variables)] sender: ComponentSender<Self>) {
        match message {
            InfoCenterInput::UpdateTime => {
                self.current_time = Self::format_time();
            },
            InfoCenterInput::PopoverClosed => {
                // Reset calendar to today when popover closes
                self.calendar
                    .sender()
                    .send(events_view::calendar::CalendarInput::ResetToToday)
                    .ok();
            },
        }
    }
}

impl InfoCenter {
    fn get_system_locale() -> Locale {
        std::env::var("LANG")
            .or_else(|_| std::env::var("LC_TIME"))
            .or_else(|_| std::env::var("LC_ALL"))
            .ok()
            .and_then(|lang| {
                let locale_str = lang.split('.').next().unwrap_or("en_US");

                match Locale::from_str(locale_str) {
                    Ok(locale) => Some(locale),
                    Err(_) => Some(Locale::default()),
                }
            })
            .unwrap_or(Locale::en_US)
    }

    fn format_time() -> String {
        let now = Local::now();
        let locale = Self::get_system_locale();
        let formatted = now
            .format_localized("%a %d %b %Y %H:%M:%S", locale)
            .to_string();

        // Capitalize first character
        let mut chars = formatted.chars();
        match chars.next() {
            None => formatted,
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}
