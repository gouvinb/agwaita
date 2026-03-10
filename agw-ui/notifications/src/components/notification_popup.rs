use crate::{
    components::NotificationWithContext,
    model::{
        NotificationEvent,
        NotificationVisibility,
    },
    service::NotificationStore,
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
use gtk4_layer_shell::{
    Edge,
    Layer,
    LayerShell,
};
use log::debug;
use relm4::{
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    SimpleComponent,
    adw,
    gtk,
    typed_view::list::TypedListView,
};
use std::sync::Arc;

pub struct NotificationPopup {
    store: Arc<NotificationStore>,
    list_view: TypedListView<NotificationWithContext, gtk::NoSelection>,
    visible: bool,
    dnd_enabled: bool,
    window: gtk::Window,
}

#[derive(Debug, Clone)]
pub enum NotificationPopupInput {
    NotificationEvent(Box<NotificationEvent>),
    DndChanged(bool),
}

#[derive(Debug, Clone)]
pub struct NotificationPopupConfig {
    pub store: Arc<NotificationStore>,
    pub dnd_enabled: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for NotificationPopup {
    type Input = NotificationPopupInput;
    type Output = ();
    type Init = NotificationPopupConfig;

    view! {
        #[root]
        gtk::Window {
            set_namespace: Some("agwaita-notifications"),
            set_layer: Layer::Overlay,
            set_anchor: (Edge::Top, true),
            set_anchor: (Edge::Right, true),
            set_anchor: (Edge::Bottom, true),
            inline_css: "
            |background: linear-gradient(to right, transparent, var(--shade-color));
            ".trim_margin().as_str(),
            set_margin_vertical: 0,
            set_margin_end: 0,
            set_visible: false,

            adw::Clamp {
                set_maximum_size: 320,

                gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_propagate_natural_width: true,
                    set_propagate_natural_height: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,
                        set_margin_top: 12,
                        set_margin_bottom: 12,
                        set_margin_start: 12,
                        set_margin_end: 12,

                        #[local_ref]
                        notification_list_view -> gtk::ListView {
                            inline_css: "
                            |background: transparent;
                            ".trim_margin().as_str(),
                            set_hexpand: true,
                            set_can_focus: false,
                            set_focusable: false,
                        },
                    }
                }
            }
        }
    }

    fn init(config: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        // Setup layer shell
        root.init_layer_shell();

        // Initialize global store for notification items to access
        crate::components::init_notification_store(config.store.clone());

        // Create list view (start empty, only show new notifications)
        let list_view: TypedListView<NotificationWithContext, gtk::NoSelection> = TypedListView::new();

        let notification_list_view = &list_view.view;
        let widgets = view_output!();

        let model = NotificationPopup {
            store: config.store.clone(),
            list_view,
            visible: false,
            dnd_enabled: config.dnd_enabled,
            window: root.clone(),
        };

        // Subscribe to notification events after component is built
        let store_clone = config.store.clone();
        let sender_clone = sender.input_sender().clone();
        std::thread::spawn(move || {
            debug!("Popup: Event listener started");
            let receiver = store_clone.subscribe();
            loop {
                match receiver.recv() {
                    Ok(event) => {
                        debug!("Popup: Received event from store: {:?}", event);
                        if sender_clone
                            .send(NotificationPopupInput::NotificationEvent(Box::new(event)))
                            .is_err()
                        {
                            debug!("Popup: Component disconnected, stopping event listener");
                            break;
                        }
                    },
                    Err(_) => {
                        debug!("Popup: Store disconnected, stopping event listener");
                        break;
                    },
                }
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            NotificationPopupInput::NotificationEvent(event) => match event.as_ref() {
                NotificationEvent::Added(notification) => {
                    // Only show visible notifications and only if DND is disabled
                    if notification.visibility == NotificationVisibility::Visible && !self.dnd_enabled {
                        debug!(
                            "Popup: Adding notification id={}, summary={} (DND={})",
                            notification.id, notification.summary, self.dnd_enabled
                        );

                        // Add at the beginning (most recent first)
                        self.list_view
                            .insert(0, NotificationWithContext::new(notification.clone(), true));

                        // Schedule timeout to hide the notification
                        if let Some(duration) = notification.get_timeout_duration() {
                            let store = self.store.clone();
                            let id = notification.id;
                            glib::timeout_add_local_once(duration, move || {
                                debug!("Popup: Hiding notification id={} after timeout", id);
                                store.hide(id);
                            });
                        } else {
                            // Default 5 second timeout for popup
                            let store = self.store.clone();
                            let id = notification.id;
                            glib::timeout_add_local_once(std::time::Duration::from_secs(5), move || {
                                debug!(
                                    "Popup: Hiding notification id={} after default 5s timeout",
                                    id
                                );
                                store.hide(id);
                            });
                        }
                    }
                },
                NotificationEvent::Updated(notification) => {
                    if let Some(pos) = self
                        .list_view
                        .iter()
                        .position(|n| n.borrow().notification.id == notification.id)
                    {
                        // If notification became hidden or closed, remove from popup
                        if notification.visibility != NotificationVisibility::Visible {
                            debug!(
                                "Popup: Removing notification id={} (visibility changed to {:?})",
                                notification.id, notification.visibility
                            );
                            self.list_view.remove(pos as u32);
                        } else {
                            // Update in place
                            self.list_view.remove(pos as u32);
                            self.list_view.insert(
                                pos as u32,
                                NotificationWithContext::new(notification.clone(), true),
                            );
                        }
                    }
                },
                NotificationEvent::Closed(id) => {
                    if let Some(pos) = self
                        .list_view
                        .iter()
                        .position(|n| n.borrow().notification.id == *id)
                    {
                        debug!("Popup: Removing closed notification id={}", id);
                        self.list_view.remove(pos as u32);
                    }
                },
                NotificationEvent::ActionInvoked(id, action_id) => {
                    debug!(
                        "Popup: Action invoked on notification id={}, action={}",
                        id, action_id
                    );
                    // The notification will be closed by the store, which will trigger Closed event
                },
            },
            NotificationPopupInput::DndChanged(enabled) => {
                debug!(
                    "Popup: DND changed to {} (was {})",
                    enabled, self.dnd_enabled
                );
                self.dnd_enabled = enabled;

                // If DND is being enabled, clear all notifications from popup
                if enabled {
                    debug!(
                        "Popup: Clearing {} notifications due to DND activation",
                        self.list_view.len()
                    );
                    self.list_view.clear();
                }

                self.update_visibility();
            },
        }

        // Update window visibility based on notification count
        self.update_visibility();
    }
}

impl NotificationPopup {
    fn update_visibility(&mut self) {
        let should_be_visible = !self.list_view.is_empty() && !self.dnd_enabled;

        if self.visible != should_be_visible {
            debug!(
                "Popup: Changing visibility from {} to {} (notifications={}, dnd={})",
                self.visible,
                should_be_visible,
                self.list_view.len(),
                self.dnd_enabled
            );
            self.visible = should_be_visible;
            self.window.set_visible(should_be_visible);
        }
    }
}
