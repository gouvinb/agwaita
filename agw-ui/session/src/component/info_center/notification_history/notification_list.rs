use agw_ui_notifications::{
    NotificationList,
    NotificationListConfig,
    NotificationStore,
};
use relm4::{
    ComponentController,
    ComponentParts,
    ComponentSender,
    Controller,
    SimpleComponent,
    gtk,
};
use std::sync::Arc;

pub struct Notifications {
    notification_list: Controller<NotificationList>,
}

#[derive(Debug)]
pub enum NotificationsInput {}

#[relm4::component(pub)]
impl SimpleComponent for Notifications {
    type Input = NotificationsInput;
    type Output = ();
    type Init = Arc<NotificationStore>;

    view! {
        #[root]
        gtk::Box {
            model.notification_list.widget() -> &gtk::ScrolledWindow {},
        }
    }

    fn init(store: Self::Init, _root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let config = NotificationListConfig {
            store,
            show_all: true,
            enable_popup_timeout: false,
        };

        let notification_list = relm4::ComponentBuilder::<NotificationList>::default()
            .launch(config)
            .detach();

        let model = Notifications { notification_list };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _message: Self::Input, _sender: ComponentSender<Self>) {}
}
