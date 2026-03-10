//! Systemd unit failed indicator component.

use crate::system_state::{
    messages::SystemStateUpdate,
    systemd_failed::SystemdFailedUnits,
};
use gtk4::prelude::*;
use log::{
    debug,
    warn,
};
use relm4::{
    ComponentParts,
    ComponentSender,
    SimpleComponent,
    gtk,
};
use std::process::Command;

#[derive(Debug, Clone)]
pub enum SystemdUnitFailedIndicatorInput {
    SystemStateUpdate(SystemStateUpdate),
    ResetFailed,
}

pub struct SystemdUnitFailedIndicator {
    failed_units: SystemdFailedUnits,
}

#[relm4::component(pub)]
impl SimpleComponent for SystemdUnitFailedIndicator {
    type Init = std::sync::mpsc::Receiver<SystemStateUpdate>;
    type Input = SystemdUnitFailedIndicatorInput;
    type Output = ();

    view! {
        #[root]
        gtk::Button {
            #[watch]
            set_tooltip_markup: Some(&model.units_to_tooltip_markup()),
            #[watch]
            set_visible: model.failed_units.count > 0,

            connect_clicked => SystemdUnitFailedIndicatorInput::ResetFailed,

            gtk::Box {
                gtk::Image {
                    #[watch]
                    set_icon_name: Some(&model.get_icon()),
                },

                gtk::Label {
                    set_use_markup: true,

                    #[watch]
                    set_markup: &model.get_count_markup(),
                },
            }
        }
    }

    fn init(system_state_receiver: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = SystemdUnitFailedIndicator {
            failed_units: SystemdFailedUnits::default(),
        };

        let input_sender = sender.input_sender().clone();
        std::thread::spawn(move || {
            while let Ok(update) = system_state_receiver.recv() {
                input_sender
                    .send(SystemdUnitFailedIndicatorInput::SystemStateUpdate(update))
                    .ok();
            }
        });

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            SystemdUnitFailedIndicatorInput::SystemStateUpdate(SystemStateUpdate::SystemdFailed(units)) => {
                self.failed_units = units;
            },
            SystemdUnitFailedIndicatorInput::ResetFailed => {
                debug!("Resetting failed systemd units");
                match Command::new("systemctl")
                    .args(&["--user", "reset-failed"])
                    .output()
                {
                    Ok(_) => {
                        debug!("Successfully reset failed units");
                    },
                    Err(e) => {
                        warn!("Failed to reset systemd units: {}", e);
                    },
                }
            },
            _ => {},
        }
    }
}

impl SystemdUnitFailedIndicator {
    fn get_icon(&self) -> String {
        if self.failed_units.count > 0 {
            "software-update-urgent-symbolic".to_string()
        } else {
            "".to_string()
        }
    }

    fn get_count_markup(&self) -> String {
        format!(
            " <span baseline_shift=\"superscript\" font_scale=\"superscript\">{}</span>",
            self.failed_units.count
        )
    }

    fn units_to_tooltip_markup(&self) -> String {
        if self.failed_units.units.is_empty() {
            return String::new();
        }

        let units_list: Vec<String> = self
            .failed_units
            .units
            .iter()
            .enumerate()
            .map(|(idx, unit)| {
                let cleaned = unit.replace('●', "").trim().to_string();
                format!("{}. {}", idx + 1, cleaned)
            })
            .collect();

        format!("<b>Failed Units:</b>\n{}", units_list.join("\n"))
    }
}
