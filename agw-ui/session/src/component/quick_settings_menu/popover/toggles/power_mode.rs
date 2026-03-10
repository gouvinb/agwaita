pub(crate) use agw_service::power_mode::{
    PowerModeService,
    PowerProfile,
};
use gtk4::prelude::*;
use relm4::{
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    SimpleComponent,
    gtk,
};
use std::sync::Arc;

/// PowerModeRevealer - Radio buttons for power profile selection
///
/// Displays three mutually exclusive options:
/// - Power Saver
/// - Balanced
/// - Performance
#[allow(dead_code)] // public api
pub struct PowerModeRevealer {
    service: Option<Arc<PowerModeService>>,
    active_profile: PowerProfile,
}

#[derive(Debug)]
#[allow(dead_code)] // public api
pub enum PowerModeRevealerInput {
    SelectProfile(PowerProfile),
    UpdateProfile(PowerProfile), // From external change (monitor)
}

#[relm4::component(pub)]
impl SimpleComponent for PowerModeRevealer {
    type Init = Option<Arc<PowerModeService>>;
    type Input = PowerModeRevealerInput;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 8,
            set_margin_vertical: 4,
            set_homogeneous: true,

            #[name = "power_saver_button"]
            gtk::ToggleButton {
                set_label: "Power Saver",
                #[watch]
                set_active: model.active_profile == PowerProfile::PowerSaver,
            },

            #[name = "balanced_button"]
            gtk::ToggleButton {
                set_label: "Balanced",
                #[watch]
                set_active: model.active_profile == PowerProfile::Balanced,
            },

            #[name = "performance_button"]
            gtk::ToggleButton {
                set_label: "Performance",
                #[watch]
                set_active: model.active_profile == PowerProfile::Performance,
            },
        }
    }

    fn init(service: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        // Get initial profile from service or default to Balanced
        let active_profile = service
            .as_ref()
            .map(|s| s.get_active_profile())
            .unwrap_or(PowerProfile::Balanced);

        let model = PowerModeRevealer {
            service,
            active_profile,
        };

        let widgets = view_output!();

        // Connect button signals - implement radio button behavior
        let sender_clone = sender.input_sender().clone();
        widgets.power_saver_button.connect_clicked(move |button| {
            if button.is_active() {
                sender_clone
                    .send(PowerModeRevealerInput::SelectProfile(
                        PowerProfile::PowerSaver,
                    ))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.balanced_button.connect_clicked(move |button| {
            if button.is_active() {
                sender_clone
                    .send(PowerModeRevealerInput::SelectProfile(
                        PowerProfile::Balanced,
                    ))
                    .ok();
            }
        });

        let sender_clone = sender.input_sender().clone();
        widgets.performance_button.connect_clicked(move |button| {
            if button.is_active() {
                sender_clone
                    .send(PowerModeRevealerInput::SelectProfile(
                        PowerProfile::Performance,
                    ))
                    .ok();
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            PowerModeRevealerInput::SelectProfile(profile) => {
                // User clicked a button - change the profile
                if let Some(ref service) = self.service {
                    let service_clone = Arc::clone(service);
                    let profile_clone = profile.clone();
                    gtk4::glib::MainContext::default().spawn_local(async move {
                        if let Err(e) = service_clone.set_active_profile(profile_clone).await {
                            log::error!("Failed to set power profile: {}", e);
                        }
                    });
                }
                // Update local state immediately for responsiveness
                self.active_profile = profile;
            },
            PowerModeRevealerInput::UpdateProfile(profile) => {
                // External change via monitor - just update the UI
                self.active_profile = profile;
            },
        }
    }
}
