use crate::window::PowerMenuWindowInput;
use relm4::Sender;
use std::sync::OnceLock;

pub static POWER_MENU_SENDER: OnceLock<Sender<PowerMenuWindowInput>> = OnceLock::new();

pub fn power_menu_toggle() {
    if let Some(sender) = POWER_MENU_SENDER.get() {
        sender.send(PowerMenuWindowInput::Toggle).ok();
    }
}
