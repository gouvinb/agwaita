mod bar_action;
mod notification_action;

use crate::{
    CommandRun,
    action::{
        bar_action::BarAction,
        notification_action::{
            DndAction,
            NotificationAction,
        },
    },
};
use agw_lib_outcome::error::AgwError;
use log::debug;
use std::{
    io::{
        Read,
        Write,
    },
    os::unix::net::UnixStream,
};

#[derive(clap::Subcommand, Debug)]
pub enum CliAction {
    /// Start the Agwaita daemon
    Session,

    /// Launch the application launcher
    #[command(name = "app-launcher")]
    AppLauncher,

    /// Launch the power menu
    #[command(name = "power-menu")]
    PowerMenu,

    /// Interacts with the bar
    #[command(name = "bar")]
    Bar {
        #[command(subcommand)]
        command: BarAction,
    },

    /// Interacts with notifications
    #[command(name = "notification")]
    Notification {
        #[command(subcommand)]
        command: NotificationAction,
    },

    /// Manage audio settings
    #[command(name = "audio-manager")]
    AudioManager,

    /// Open the Bluetooth device manager
    #[command(name = "bluetooth-manager", alias = "bluetooth")]
    BluetoothManager,

    /// Manage network connections
    #[command(name = "networkd-manager")]
    NetworkdManager,

    /// Manage wallpaper
    #[command(name = "wallpaper-manager")]
    WallpaperManager,
}

impl CommandRun for CliAction {
    fn run(&self) -> Result<(), AgwError> {
        debug!("Running subcommand: {:?}", self);
        match self {
            CliAction::Session => agw_ui_session::daemon::init(),
            CliAction::AppLauncher => send_daemon_command("app-launcher toggle"),
            CliAction::PowerMenu => send_daemon_command("power-menu toggle"),
            CliAction::Bar { command } => {
                let message = match command {
                    BarAction::Toggle => "topbar toggle",
                    BarAction::Show => "topbar show",
                    BarAction::Hide => "topbar hide",
                };
                send_daemon_command(message)
            },
            CliAction::Notification { command } => {
                let message = match command {
                    NotificationAction::CloseLast => "notification close-last",
                    NotificationAction::Dnd { command: dnd_cmd } => match dnd_cmd {
                        DndAction::Enable => "dnd enable",
                        DndAction::Disable => "dnd disable",
                        DndAction::Toggle => "dnd toggle",
                        DndAction::Status => "dnd status",
                    },
                };
                send_daemon_command(message)
            },
            CliAction::AudioManager => agw_ui_audio_manager::init(),
            CliAction::BluetoothManager => agw_ui_bluetooth_manager::init(),
            CliAction::NetworkdManager => agw_ui_networkd_manager::init(),
            CliAction::WallpaperManager => agw_ui_wallpaper::init(),
        }
    }
}

/// Sends a command to the Agwaita daemon via Unix socket and prints the response.
fn send_daemon_command(command: &str) -> Result<(), AgwError> {
    let socket_path = agw_ui_session::daemon::get_default_socket_path();
    let mut stream = UnixStream::connect(&socket_path).map_err(|e| {
        AgwError::with_source(
            1,
            format!("Failed to connect to daemon at {:?}", socket_path),
            Box::new(e),
        )
    })?;

    stream.write_all(command.as_bytes()).map_err(|e| {
        AgwError::with_source(
            1,
            "Failed to send command to daemon".to_string(),
            Box::new(e),
        )
    })?;

    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|e| {
        AgwError::with_source(
            1,
            "Failed to read response from daemon".to_string(),
            Box::new(e),
        )
    })?;

    println!("{}", response);
    Ok(())
}
