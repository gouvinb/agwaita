//! Privacy indicator component displaying active resource usage.

use crate::system_state::messages::SystemStateUpdate;
use agw_service::privacy::PrivacyUsage;
use catalyser::stdx::extension::str_extension::MultilineStr;
use gtk4::prelude::*;
use relm4::{
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    SimpleComponent,
    gtk,
};

#[derive(Debug, Clone)]
pub enum PrivacyIndicatorInput {
    SystemStateUpdate(SystemStateUpdate),
}

pub struct PrivacyIndicator {
    usage: PrivacyUsage,
}

#[relm4::component(pub)]
impl SimpleComponent for PrivacyIndicator {
    type Init = std::sync::mpsc::Receiver<SystemStateUpdate>;
    type Input = PrivacyIndicatorInput;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            add_css_class: "card",
            inline_css: "
            |color: var(--accent-color);
            |padding: 0 10px;
            ".trim_margin().as_str(),
            set_spacing: 8,

            #[watch]
            set_tooltip_markup: Some(&model.usage_to_tooltip_markup()),
            #[watch]
            set_visible: !model.usage_is_empty(),

            gtk::Image {
                set_icon_name: Some("camera-web-symbolic"),

                #[watch]
                set_visible: !model.usage.camera.is_empty(),
            },

            gtk::Image {
                set_icon_name: Some("audio-input-microphone-symbolic"),

                #[watch]
                set_visible: !model.usage.microphone.is_empty(),
            },

            gtk::Image {
                set_icon_name: Some("location-services-active-symbolic"),

                #[watch]
                set_visible: !model.usage.location.is_empty(),
            },

            gtk::Image {
                set_icon_name: Some("screen-shared-symbolic"),

                #[watch]
                set_visible: !model.usage.screencast.is_empty(),
            },
        }
    }

    fn init(system_state_receiver: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = PrivacyIndicator {
            usage: PrivacyUsage::default(),
        };

        let input_sender = sender.input_sender().clone();
        std::thread::spawn(move || {
            while let Ok(update) = system_state_receiver.recv() {
                input_sender
                    .send(PrivacyIndicatorInput::SystemStateUpdate(update))
                    .ok();
            }
        });

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            PrivacyIndicatorInput::SystemStateUpdate(SystemStateUpdate::Privacy(usage)) => {
                self.usage = usage;
            },
            _ => {},
        }
    }
}

impl PrivacyIndicator {
    fn usage_is_empty(&self) -> bool {
        self.usage.camera.is_empty() && self.usage.microphone.is_empty() && self.usage.location.is_empty() && self.usage.screencast.is_empty()
    }

    fn usage_to_tooltip_markup(&self) -> String {
        let mut lines = Vec::new();

        if !self.usage.microphone.is_empty() {
            let mut apps: Vec<_> = self.usage.microphone.iter().cloned().collect();
            apps.sort_unstable();
            lines.push(format!("<b>Microphone:</b>\n{}", apps.join("\n")));
        }
        if !self.usage.camera.is_empty() {
            let mut apps: Vec<_> = self.usage.camera.iter().cloned().collect();
            apps.sort_unstable();
            lines.push(format!("<b>Camera:</b>\n{}", apps.join("\n")));
        }
        if !self.usage.location.is_empty() {
            let mut apps: Vec<_> = self.usage.location.iter().cloned().collect();
            apps.sort_unstable();
            lines.push(format!("<b>Location:</b>\n{}", apps.join("\n")));
        }
        if !self.usage.screencast.is_empty() {
            let mut apps: Vec<_> = self.usage.screencast.iter().cloned().collect();
            apps.sort_unstable();
            lines.push(format!("<b>Screen sharing:</b>\n{}", apps.join("\n")));
        }

        lines.join("\n\n")
    }
}
