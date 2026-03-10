use crate::window::AppLauncherWindowInput;
use relm4::Sender;
use std::sync::OnceLock;

/// Global sender for app launcher window
pub static APP_LAUNCHER_SENDER: OnceLock<Sender<AppLauncherWindowInput>> = OnceLock::new();

/// Toggle app launcher visibility
pub fn app_launcher_toggle() {
    if let Some(sender) = APP_LAUNCHER_SENDER.get() {
        sender.send(AppLauncherWindowInput::Toggle).ok();
    }
}
