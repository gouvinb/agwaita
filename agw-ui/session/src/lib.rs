//! Top bar UI component for Agwaita.
//!
//! This module provides the main top bar interface with support for multiple monitors,
//! system tray icons, workspaces, quick settings, and system state monitoring.

use crate::component::TopbarManagerInput;
use std::sync::OnceLock;

pub mod component;
pub mod daemon;
pub mod message;
pub mod system_state;

pub static APP_SENDER: OnceLock<relm4::Sender<TopbarManagerInput>> = OnceLock::new();
