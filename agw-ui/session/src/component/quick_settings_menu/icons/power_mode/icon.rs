use agw_service::power_mode::PowerProfile;
use gtk4::prelude::WidgetExt;
use relm4::{
    Component,
    ComponentParts,
    ComponentSender,
    gtk,
};

#[derive(Debug)]
pub struct PowerModeIcon {
    active_profile: PowerProfile,
}

#[derive(Debug)]
pub enum PowerModeIconInput {
    UpdateProfile(PowerProfile),
}

#[relm4::component(pub)]
impl Component for PowerModeIcon {
    type Init = PowerProfile;
    type Input = PowerModeIconInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Image {
            #[watch]
            set_icon_name: Some(Self::resolve_icon(model.active_profile.clone())),
            set_pixel_size: 16,
            #[watch]
            set_tooltip_text: Some(Self::get_tooltip(model.active_profile.clone())),
        }
    }

    fn init(active_profile: Self::Init, root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = PowerModeIcon { active_profile };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            PowerModeIconInput::UpdateProfile(profile) => {
                self.active_profile = profile;
            },
        }
    }
}

impl PowerModeIcon {
    /// Resolve icon name based on active power profile
    /// Matches the AGS TypeScript implementation
    fn resolve_icon(profile: PowerProfile) -> &'static str {
        match profile {
            PowerProfile::PowerSaver => "power-profile-power-saver-symbolic",
            PowerProfile::Performance => "power-profile-performance-symbolic",
            PowerProfile::Balanced => "power-profile-balanced-symbolic",
        }
    }

    /// Get tooltip text for the power profile
    fn get_tooltip(profile: PowerProfile) -> &'static str {
        match profile {
            PowerProfile::PowerSaver => "Power Saver",
            PowerProfile::Performance => "Performance",
            PowerProfile::Balanced => "Balanced",
        }
    }
}
