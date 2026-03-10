//! Reusable notification list component with configurable behavior.

use crate::{
    components::notification_item::create_notification_widget,
    model::{
        Notification,
        NotificationEvent,
        NotificationStore,
        NotificationVisibility,
    },
};
use catalyser::stdx::extension::str_extension::MultilineStr;
use gtk4::{
    glib,
    prelude::{
        BoxExt,
        OrientableExt,
        WidgetExt,
    },
};
use log::debug;
use relm4::{
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    SimpleComponent,
    gtk,
};
use std::{
    collections::HashMap,
    sync::Arc,
};

/// Configuration for NotificationList behavior.
#[derive(Debug, Clone)]
pub struct NotificationListConfig {
    pub store: Arc<NotificationStore>,
    pub show_all: bool,
    pub enable_popup_timeout: bool,
}

/// Reusable notification list component.
pub struct NotificationList {
    list_box: gtk::Box,
    notification_widgets: HashMap<u32, gtk::Box>,
    config: NotificationListConfig,
    stack: gtk::Stack,
}

#[derive(Debug, Clone)]
pub enum NotificationListInput {
    NotificationEvent(Box<NotificationEvent>),
    CloseNotification(u32),
}

#[relm4::component(pub)]
impl SimpleComponent for NotificationList {
    type Input = NotificationListInput;
    type Output = ();
    type Init = NotificationListConfig;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_propagate_natural_width: true,

            #[local_ref]
            stack_widget -> gtk::Stack {
                set_transition_type: gtk::StackTransitionType::Crossfade,

                add_named[Some("list")] = &gtk::Box {
                    #[local_ref]
                    notification_list_box -> gtk::Box {
                        inline_css: "
                        |background: transparent;
                        ".trim_margin().as_str(),
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,
                        set_hexpand: true,
                        set_valign: gtk::Align::Start,
                    },
                },

                add_named[Some("empty")] = &gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_spacing: 12,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,

                        gtk::Image {
                            set_icon_name: Some("notifications-disabled-symbolic"),
                            set_pixel_size: 64,
                        },
                        gtk::Label {
                            add_css_class: "title-1",
                            set_label: "No notifications",
                        }
                    }
                }
            },
        },
    }

    fn init(config: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let list_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .hexpand(true)
            .valign(gtk::Align::Start)
            .build();

        let mut notification_widgets = HashMap::new();

        let mut notifications = if config.show_all {
            config.store.get_all()
        } else {
            config.store.get_visible()
        };

        notifications.sort();

        for notification in notifications {
            let widget = create_notification_widget(&notification, &config.store, false);
            list_box.append(&widget);
            notification_widgets.insert(notification.id, widget);
        }

        debug!(
            "NotificationList initialized with {} notifications",
            notification_widgets.len()
        );

        let store = config.store.clone();
        let input_sender = sender.input_sender().clone();

        std::thread::spawn(move || {
            let receiver = store.subscribe();
            while let Ok(event) = receiver.recv() {
                input_sender
                    .send(NotificationListInput::NotificationEvent(Box::new(event)))
                    .ok();
            }
        });

        let has_notifications = !notification_widgets.is_empty();

        // Create stack widget
        let stack_widget = gtk::Stack::new();

        let notification_list_box = &list_box;
        let stack_widget = &stack_widget;
        let widgets = view_output!();

        // Set initial stack page
        if has_notifications {
            widgets.stack_widget.set_visible_child_name("list");
        } else {
            widgets.stack_widget.set_visible_child_name("empty");
        }

        let model = NotificationList {
            list_box,
            notification_widgets,
            config,
            stack: widgets.stack_widget.clone(),
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            NotificationListInput::NotificationEvent(event) => match event.as_ref() {
                NotificationEvent::Added(notification) => {
                    if self.should_show_notification(notification) {
                        debug!("Adding notification {} to list", notification.id);
                        let widget = create_notification_widget(notification, &self.config.store, false);
                        self.list_box.prepend(&widget);
                        self.notification_widgets.insert(notification.id, widget);
                        self.update_stack_visibility();

                        // Schedule timeout to close notification if one is specified
                        if notification.has_timeout() {
                            self.schedule_timeout_close(notification.clone());
                        }

                        if self.config.enable_popup_timeout && notification.has_timeout() {
                            self.schedule_popup_hide(notification.clone());
                        }
                    }
                },
                NotificationEvent::Updated(notification) => {
                    // InfoCenter (show_all=true) ignores visibility updates - only popup handles them
                    if self.config.show_all {
                        debug!(
                            "InfoCenter: Ignoring visibility update for notification {}",
                            notification.id
                        );
                        return;
                    }

                    // NotificationPopup (show_all=false) removes hidden notifications
                    if let Some(widget) = self.notification_widgets.remove(&notification.id) {
                        if self.should_show_notification(notification) {
                            debug!("Popup: Updating notification {} in list", notification.id);
                            self.list_box.remove(&widget);
                            let new_widget = create_notification_widget(notification, &self.config.store, false);
                            self.list_box.prepend(&new_widget);
                            self.notification_widgets
                                .insert(notification.id, new_widget);
                        } else {
                            debug!(
                                "Popup: Removing hidden notification {} from list",
                                notification.id
                            );
                            self.list_box.remove(&widget);
                        }
                        self.update_stack_visibility();
                    }
                },
                NotificationEvent::Closed(id) => {
                    if let Some(widget) = self.notification_widgets.remove(id) {
                        debug!("Removing closed notification {} from list", id);
                        self.list_box.remove(&widget);
                        self.update_stack_visibility();
                    }
                },
                NotificationEvent::ActionInvoked(_id, _action_id) => {
                    // Action is logged by the store and daemon, no need to log here
                },
            },
            NotificationListInput::CloseNotification(id) => {
                debug!("Close button clicked for notification {}", id);
                self.config.store.close(id);
            },
        }
    }
}

impl NotificationList {
    fn should_show_notification(&self, notification: &Notification) -> bool {
        if self.config.show_all {
            true
        } else {
            notification.visibility == NotificationVisibility::Visible
        }
    }

    fn update_stack_visibility(&self) {
        if self.notification_widgets.is_empty() {
            self.stack.set_visible_child_name("empty");
        } else {
            self.stack.set_visible_child_name("list");
        }
    }

    fn schedule_timeout_close(&self, notification: Notification) {
        if let Some(duration) = notification.get_timeout_duration() {
            let store = self.config.store.clone();
            let id = notification.id;

            glib::timeout_add_local_once(duration, move || {
                debug!(
                    "InfoCenter: Timeout expired ({:?}) for notification {}, closing via DBus",
                    duration, id
                );
                store.close(id);
            });
        }
    }

    fn schedule_popup_hide(&self, notification: Notification) {
        if let Some(duration) = notification.get_timeout_duration() {
            let store = self.config.store.clone();
            let id = notification.id;

            std::thread::spawn(move || {
                std::thread::sleep(duration);
                debug!("Popup timeout expired for notification {}, hiding", id);
                store.hide(id);
            });
        } else if self.config.enable_popup_timeout {
            let store = self.config.store.clone();
            let id = notification.id;

            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(5));
                debug!(
                    "Default popup timeout (5s) expired for notification {}, hiding",
                    id
                );
                store.hide(id);
            });
        }
    }
}
