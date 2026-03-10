//! Notification item widget for displaying notifications in lists.

use crate::{
    model::Notification,
    service::NotificationStore,
};
use catalyser::stdx::extension::str_extension::MultilineStr;
use gtk4::prelude::*;
use relm4::{
    RelmWidgetExt,
    gtk,
    typed_view::list::RelmListItem,
};
use std::sync::{
    Arc,
    OnceLock,
};

/// Creates a standalone notification widget (not for ListView)
pub fn create_notification_widget(notification: &Notification, store: &Arc<NotificationStore>, is_popup: bool) -> gtk::Box {
    relm4::view! {
        root_box = gtk::Box {
            add_css_class: "card",
            set_orientation: gtk::Orientation::Vertical,
            set_width_request: 320 - 16,
            set_can_focus: false,
            set_focusable: false,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 4,

                // Header
                gtk::Box {
                    set_spacing: 8,
                    set_margin_all: 8,
                    set_margin_top: 12,
                    set_margin_start: 16,
                    set_margin_end: 12,

                    #[name = "app_icon"]
                    gtk::Image {
                        set_icon_size: gtk::IconSize::Normal,
                        set_margin_end: 16,
                    },

                    #[name = "app_name_label"]
                    gtk::Label {
                        inline_css: "
                        |font-style: italic;
                        ".trim_margin().as_str(),
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Fill,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                    },

                    #[name = "time_label"]
                    gtk::Label {
                        add_css_class: "caption",
                        set_hexpand: true,
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::Fill,
                    },

                    #[name = "close_button"]
                    gtk::Button {
                        set_icon_name: "window-close-symbolic",
                        set_margin_start: 16,
                    },
                },

                gtk::Separator {},

                // Body
                gtk::Box {
                    set_spacing: 8,
                    set_margin_all: 8,
                    set_margin_top: 6,
                    set_margin_horizontal: 12,

                    #[name = "notification_image"]
                    gtk::Image {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Start,
                        set_icon_size: gtk::IconSize::Large,
                        set_margin_top: 4,
                        set_margin_end: 6,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_spacing: 8,

                        #[name = "summary_label"]
                        gtk::Label {
                            add_css_class: "title-5",
                            inline_css: "font-weight: bold;".trim_margin().as_str(),
                            set_halign: gtk::Align::Start,
                            set_hexpand: true,
                            set_wrap: true,
                            set_wrap_mode: gtk::pango::WrapMode::WordChar,
                            set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                            set_width_chars: 1,
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                            set_lines: 2,
                        },

                        #[name = "body_label"]
                        gtk::Label {
                            add_css_class: "caption",
                            set_halign: gtk::Align::Start,
                            set_hexpand: true,
                            set_wrap: true,
                            set_wrap_mode: gtk::pango::WrapMode::WordChar,
                            set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                            set_max_width_chars: 1,
                            set_use_markup: true,
                        },
                    },
                },

                // Actions
                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hexpand: true,

                    #[name = "actions_box"]
                    gtk::Box {
                        set_spacing: 8,
                        set_margin_horizontal: 12,
                        set_margin_bottom: 12,
                    },
                },
            },
        }
    }

    // Apply styling based on context
    if is_popup {
        root_box.inline_css(&format!(
            "{}background: var(--popover-bg-color);",
            notification.urgency_css_class()
        ));
    } else {
        root_box.inline_css(notification.urgency_css_class());
    }

    // App icon
    if let Some(ref icon) = notification.app_icon {
        app_icon.set_icon_name(Some(icon));
        app_icon.set_visible(true);
    } else if let Some(ref desktop_entry) = notification.desktop_entry {
        app_icon.set_icon_name(Some(desktop_entry));
        app_icon.set_visible(true);
    } else {
        app_icon.set_visible(false);
    }

    // App name
    app_name_label.set_label(&notification.app_name);

    // Time
    let time_str = notification.time.format("%H:%M").to_string();
    time_label.set_label(&time_str);

    // Summary
    summary_label.set_label(&notification.summary);

    // Body
    if let Some(ref body) = notification.body {
        let escaped_body = gtk::glib::markup_escape_text(body);
        body_label.set_markup(&escaped_body);
        body_label.set_visible(true);
    } else {
        body_label.set_visible(false);
    }

    // Image
    if let Some(ref image) = notification.image {
        notification_image.set_from_file(Some(image));
        notification_image.set_visible(true);
    } else {
        notification_image.set_visible(false);
    }

    // Connect close button
    let store_clone = store.clone();
    let notification_id = notification.id;
    close_button.connect_clicked(move |_| {
        store_clone.close(notification_id);
    });

    // Add action buttons
    if !notification.actions.is_empty() {
        for action in &notification.actions {
            let button = gtk::Button::builder()
                .label(&action.label)
                .hexpand(true)
                .halign(gtk::Align::Center)
                .build();

            // Connect action button to invoke via DBus
            let store_clone = store.clone();
            let notification_id = notification.id;
            let action_id = action.id.clone();
            let button_clone = button.clone();
            button.connect_clicked(move |_| {
                // Disable button to prevent multiple invocations
                button_clone.set_sensitive(false);
                store_clone.invoke_action(notification_id, &action_id);

                // Delay closing to let the client process the ActionInvoked signal
                let store_delayed = store_clone.clone();
                gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                    store_delayed.close(notification_id);
                });
            });

            actions_box.append(&button);
        }
        actions_box.set_visible(true);
    } else {
        actions_box.set_visible(false);
    }

    root_box
}

static NOTIFICATION_STORE: OnceLock<Arc<NotificationStore>> = OnceLock::new();

/// Initialize the global notification store for notification items
pub fn init_notification_store(store: Arc<NotificationStore>) {
    let _ = NOTIFICATION_STORE.set(store);
}

/// Wrapper for notifications with context about where they're displayed
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotificationWithContext {
    pub notification: Notification,
    pub is_popup: bool,
}

impl NotificationWithContext {
    pub fn new(notification: Notification, is_popup: bool) -> Self {
        Self {
            notification,
            is_popup,
        }
    }
}

pub struct NotificationItemWidget {
    app_icon: gtk::Image,
    app_name_label: gtk::Label,
    time_label: gtk::Label,
    close_button: gtk::Button,
    notification_image: gtk::Image,
    summary_label: gtk::Label,
    body_label: gtk::Label,
    actions_box: gtk::Box,
    store: Option<Arc<NotificationStore>>,
}

impl Drop for NotificationItemWidget {
    fn drop(&mut self) {
        // Cleanup if needed
    }
}

impl RelmListItem for NotificationWithContext {
    type Root = gtk::Box;
    type Widgets = NotificationItemWidget;

    fn setup(list_item: &gtk::ListItem) -> (gtk::Box, NotificationItemWidget) {
        list_item.set_activatable(false);
        list_item.set_selectable(false);
        list_item.set_focusable(false);

        relm4::view! {
            root_box = gtk::Box {
                add_css_class: "card",
                set_orientation: gtk::Orientation::Vertical,
                set_width_request: 320 - 16,
                set_can_focus: false,
                set_focusable: false,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,

                    // Header
                    gtk::Box {
                        set_spacing: 8,
                        set_margin_all: 8,
                        set_margin_top: 12,
                        set_margin_start: 16,
                        set_margin_end: 12,

                        #[name = "app_icon"]
                        gtk::Image {
                            set_icon_size: gtk::IconSize::Normal,
                            set_margin_end: 16,
                        },

                        #[name = "app_name_label"]
                        gtk::Label {
                            inline_css: "
                            |font-style: italic;
                            ".trim_margin().as_str(),
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Fill,
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                        },

                        #[name = "time_label"]
                        gtk::Label {
                            add_css_class: "caption",
                            set_hexpand: true,
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::Fill,
                        },

                        #[name = "close_button"]
                        gtk::Button {
                            set_icon_name: "window-close-symbolic",
                            set_margin_start: 16,
                        },
                    },

                    gtk::Separator {},

                    // Body
                    gtk::Box {
                        set_spacing: 8,
                        set_margin_all: 8,
                        set_margin_top: 6,
                        set_margin_horizontal: 12,

                        #[name = "notification_image"]
                        gtk::Image {
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Start,
                            set_icon_size: gtk::IconSize::Large,
                            set_margin_top: 4,
                            set_margin_end: 6,
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_hexpand: true,
                            set_spacing: 8,

                            #[name = "summary_label"]
                            gtk::Label {
                                add_css_class: "title-5",
                                inline_css: "font-weight: bold;".trim_margin().as_str(),
                                set_halign: gtk::Align::Start,
                                set_hexpand: true,
                                set_wrap: true,
                                set_wrap_mode: gtk::pango::WrapMode::WordChar,
                                set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                                set_width_chars: 1,
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                set_lines: 2,
                            },

                            #[name = "body_label"]
                            gtk::Label {
                                add_css_class: "caption",
                                set_halign: gtk::Align::Start,
                                set_hexpand: true,
                                set_wrap: true,
                                set_wrap_mode: gtk::pango::WrapMode::WordChar,
                                set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                                set_max_width_chars: 1,
                                set_use_markup: true,
                            },
                        },
                    },

                    // Actions
                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hexpand: true,

                        #[name = "actions_box"]
                        gtk::Box {
                            set_spacing: 8,
                            set_margin_horizontal: 12,
                            set_margin_bottom: 12,
                        },
                    },
                },
            }
        }

        let widget = NotificationItemWidget {
            app_icon,
            app_name_label,
            time_label,
            close_button,
            notification_image,
            summary_label,
            body_label,
            actions_box,
            store: None,
        };

        (root_box, widget)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, root: &mut Self::Root) {
        // Store the store reference if not already done
        if widgets.store.is_none() {
            if let Some(store) = NOTIFICATION_STORE.get() {
                widgets.store = Some(store.clone());

                // Connect close button
                let store_clone = store.clone();
                let notification_id = self.notification.id;
                widgets.close_button.connect_clicked(move |_| {
                    store_clone.close(notification_id);
                });
            }
        }

        // Apply styling based on context
        if self.is_popup {
            root.inline_css(&format!(
                "{}background: var(--popover-bg-color);",
                self.notification.urgency_css_class()
            ));
        } else {
            root.inline_css(self.notification.urgency_css_class());
        }

        // App icon
        if let Some(ref icon) = self.notification.app_icon {
            widgets.app_icon.set_icon_name(Some(icon));
            widgets.app_icon.set_visible(true);
        } else if let Some(ref desktop_entry) = self.notification.desktop_entry {
            widgets.app_icon.set_icon_name(Some(desktop_entry));
            widgets.app_icon.set_visible(true);
        } else {
            widgets.app_icon.set_visible(false);
        }

        // App name
        widgets
            .app_name_label
            .set_label(&self.notification.app_name);

        // Time
        let time_str = self.notification.time.format("%H:%M").to_string();
        widgets.time_label.set_label(&time_str);

        // Summary
        widgets.summary_label.set_label(&self.notification.summary);

        // Body
        if let Some(ref body) = self.notification.body {
            let escaped_body = gtk::glib::markup_escape_text(body);
            widgets.body_label.set_markup(&escaped_body);
            widgets.body_label.set_visible(true);
        } else {
            widgets.body_label.set_visible(false);
        }

        // Image
        if let Some(ref image) = self.notification.image {
            widgets.notification_image.set_from_file(Some(image));
            widgets.notification_image.set_visible(true);
        } else {
            widgets.notification_image.set_visible(false);
        }

        // Clear previous actions
        while let Some(child) = widgets.actions_box.first_child() {
            widgets.actions_box.remove(&child);
        }

        // Add action buttons
        if !self.notification.actions.is_empty() {
            if let Some(store) = &widgets.store {
                for action in &self.notification.actions {
                    let button = gtk::Button::builder()
                        .label(&action.label)
                        .hexpand(true)
                        .halign(gtk::Align::Center)
                        .build();

                    // Connect action button to invoke via DBus
                    let store_clone = store.clone();
                    let notification_id = self.notification.id;
                    let action_id = action.id.clone();
                    button.connect_clicked(move |btn| {
                        btn.set_sensitive(false);
                        store_clone.invoke_action(notification_id, &action_id);

                        // Delay closing to let the client process the ActionInvoked signal
                        // let store_delayed = store_clone.clone();
                        // gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                        //     store_delayed.close(notification_id);
                        // });
                    });

                    widgets.actions_box.append(&button);
                }
            }
            widgets.actions_box.set_visible(true);
        } else {
            widgets.actions_box.set_visible(false);
        }
    }
}
