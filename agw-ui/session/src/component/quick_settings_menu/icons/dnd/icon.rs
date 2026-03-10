use gtk4::prelude::*;
use relm4::{
    ComponentParts,
    ComponentSender,
    SimpleComponent,
    gtk,
};

/// DoNotDisturbIcon - Displays Do Not Disturb status
/// Matches the AGS TypeScript implementation using AstalNotifd
pub struct DoNotDisturbIcon {
    dont_disturb: bool,
}

#[derive(Debug)]
pub enum DoNotDisturbIconInput {
    UpdateState(bool), // dont_disturb
}

#[relm4::component(pub)]
impl SimpleComponent for DoNotDisturbIcon {
    type Init = bool;
    type Input = DoNotDisturbIconInput;
    type Output = ();

    view! {
        #[root]
        gtk::Image {
            #[watch]
            set_icon_name: Some(Self::resolve_icon(model.dont_disturb)),
            set_pixel_size: 16,
            #[watch]
            set_tooltip_text: Some(Self::get_tooltip(model.dont_disturb)),
        }
    }

    fn init(dont_disturb: Self::Init, root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = DoNotDisturbIcon { dont_disturb };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            DoNotDisturbIconInput::UpdateState(dont_disturb) => {
                self.dont_disturb = dont_disturb;
            },
        }
    }
}

impl DoNotDisturbIcon {
    /// Resolves the DND icon based on state
    /// Matches the AGS implementation
    fn resolve_icon(dont_disturb: bool) -> &'static str {
        if dont_disturb {
            "notifications-disabled-symbolic"
        } else {
            "org.gnome.Settings-notifications-symbolic"
        }
    }

    fn get_tooltip(dont_disturb: bool) -> &'static str {
        if dont_disturb {
            "Do Not Disturb: Enabled"
        } else {
            "Do Not Disturb: Disabled"
        }
    }
}
