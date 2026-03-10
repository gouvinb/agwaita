//! Message passing functions for topbar control.

use crate::{
    APP_SENDER,
    component::TopbarManagerInput,
};

#[derive(Debug)]
pub enum TopbarMsg {
    Show,
    Hide,
    Toggle,
}

pub fn show() {
    if let Some(sender) = APP_SENDER.get() {
        sender.send(TopbarManagerInput::Show).ok();
    }
}

pub fn hide() {
    if let Some(sender) = APP_SENDER.get() {
        sender.send(TopbarManagerInput::Hide).ok();
    }
}

pub fn toggle() {
    if let Some(sender) = APP_SENDER.get() {
        sender.send(TopbarManagerInput::Toggle).ok();
    }
}

pub fn dnd_enable() {
    if let Some(sender) = APP_SENDER.get() {
        sender.send(TopbarManagerInput::DndEnable).ok();
    }
}

pub fn dnd_disable() {
    if let Some(sender) = APP_SENDER.get() {
        sender.send(TopbarManagerInput::DndDisable).ok();
    }
}

pub fn dnd_toggle() {
    if let Some(sender) = APP_SENDER.get() {
        sender.send(TopbarManagerInput::DndToggle).ok();
    }
}

pub fn dnd_status() -> bool {
    if let Some(sender) = APP_SENDER.get() {
        let (tx, rx) = std::sync::mpsc::channel();
        sender.send(TopbarManagerInput::DndStatus(tx)).ok();
        rx.recv().unwrap_or(false)
    } else {
        false
    }
}

pub fn quit() {
    if let Some(sender) = APP_SENDER.get() {
        sender.send(TopbarManagerInput::Quit).ok();
    }
}
