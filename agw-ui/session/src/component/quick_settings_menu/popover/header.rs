use crate::component::quick_settings_menu::icons::battery::icon::{
    BatteryIcon,
    BatteryIconInput,
};
use agw_ui_power_menu::PowerMenuAction;
use gtk4::prelude::{
    BoxExt,
    ButtonExt,
    WidgetExt,
};
use log::error;
use relm4::{
    Component,
    ComponentController,
    ComponentParts,
    ComponentSender,
    gtk,
};
use std::thread;

/// PopoverHeader - Header section with battery icon and system buttons
///
/// Layout:
/// [BatteryIcon + Label] .................... [Lock] [Logout] [Reboot] [Shutdown]
pub struct PopoverHeader {
    battery_icon: relm4::Controller<BatteryIcon>,
}

#[derive(Debug)]
pub enum PopoverHeaderInput {
    UpdateBattery(f64, bool, bool), // percentage, is_charging, is_present
    Lock,
    Suspend,
    Logout,
    Reboot,
    Shutdown,
}

#[derive(Debug)]
pub enum PopoverHeaderOutput {
    ClosePopover,
}

#[relm4::component(pub)]
impl Component for PopoverHeader {
    type Init = (f64, bool, bool); // Initial battery state
    type Input = PopoverHeaderInput;
    type Output = PopoverHeaderOutput;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::CenterBox {
            set_hexpand: true,

            #[wrap(Some)]
            set_start_widget = &gtk::Box {
                set_margin_start: 8,
                set_spacing: 8,
                set_valign: gtk::Align::Center,

                model.battery_icon.widget().clone(),
            },

            #[wrap(Some)]
            set_end_widget = &gtk::Box {
                set_spacing: 8,

                gtk::Button {
                    set_icon_name: PowerMenuAction::LOCK_SCREEN_ACTION.icon_name,
                    set_tooltip_text: Some(PowerMenuAction::LOCK_SCREEN_ACTION.title),
                    add_css_class: "circular",
                    connect_clicked[sender] => move |_| {
                        sender.input(PopoverHeaderInput::Lock);
                    },
                },

                gtk::Button {
                    set_icon_name: PowerMenuAction::SUSPEND_ACTION.icon_name,
                    set_tooltip_text: Some(PowerMenuAction::SUSPEND_ACTION.title),
                    add_css_class: "circular",
                    connect_clicked[sender] => move |_| {
                        sender.input(PopoverHeaderInput::Suspend);
                    },
                },

                gtk::Button {
                    set_icon_name: PowerMenuAction::LOGOUT_ACTION.icon_name,
                    set_tooltip_text: Some(PowerMenuAction::LOGOUT_ACTION.title),
                    add_css_class: "circular",
                    connect_clicked[sender] => move |_| {
                        sender.input(PopoverHeaderInput::Logout);
                    },
                },

                gtk::Button {
                    set_icon_name: PowerMenuAction::REBOOT_ACTION.icon_name,
                    set_tooltip_text: Some(PowerMenuAction::REBOOT_ACTION.title),
                    add_css_class: "circular",
                    connect_clicked[sender] => move |_| {
                        sender.input(PopoverHeaderInput::Reboot);
                    },
                },

                gtk::Button {
                    set_icon_name: PowerMenuAction::SHUTDOWN_ACTION.icon_name,
                    set_tooltip_text: Some(PowerMenuAction::SHUTDOWN_ACTION.title),
                    add_css_class: "circular",
                    connect_clicked[sender] => move |_| {
                        sender.input(PopoverHeaderInput::Shutdown);
                    },
                },
            },
        }
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        // Initialize battery icon with initial state
        let battery_icon = BatteryIcon::builder()
            .launch(init)
            .forward(sender.input_sender(), |_| unreachable!());

        let model = PopoverHeader { battery_icon };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, #[allow(unused_variables)] root: &Self::Root) {
        match message {
            PopoverHeaderInput::UpdateBattery(percentage, is_charging, is_present) => {
                self.battery_icon.emit(BatteryIconInput::UpdateBattery(
                    percentage,
                    is_charging,
                    is_present,
                ));
            },
            PopoverHeaderInput::Lock => {
                let _ = sender.output(PopoverHeaderOutput::ClosePopover);
                thread::spawn(|| {
                    if let Err(e) = PowerMenuAction::LOCK_SCREEN_ACTION.call() {
                        error!("Failed to lock session: {}", e);
                    }
                });
            },
            PopoverHeaderInput::Suspend => {
                let _ = sender.output(PopoverHeaderOutput::ClosePopover);
                thread::spawn(|| {
                    if let Err(e) = PowerMenuAction::LOCK_SCREEN_ACTION.call() {
                        error!("Failed to suspend session: {}", e);
                    }
                });
            },
            PopoverHeaderInput::Logout => {
                let _ = sender.output(PopoverHeaderOutput::ClosePopover);

                thread::spawn(|| {
                    if let Err(e) = PowerMenuAction::LOGOUT_ACTION.call() {
                        error!("Failed to logout: {}", e);
                    }
                });
            },
            PopoverHeaderInput::Reboot => {
                let _ = sender.output(PopoverHeaderOutput::ClosePopover);

                thread::spawn(|| {
                    if let Err(e) = PowerMenuAction::REBOOT_ACTION.call() {
                        error!("Failed to reboot: {}", e);
                    }
                });
            },
            PopoverHeaderInput::Shutdown => {
                let _ = sender.output(PopoverHeaderOutput::ClosePopover);

                thread::spawn(|| {
                    if let Err(e) = PowerMenuAction::SHUTDOWN_ACTION.call() {
                        error!("Failed to shutdown: {}", e);
                    }
                });
            },
        }
    }
}
