//! Agwaita daemon service.
//!
//! This module provides the daemon that manages Agwaita's background operations.
//! The daemon listens on a Unix socket for commands from the CLI and other components,
//! routes messages to appropriate UI components, and handles graceful shutdown via SIGINT.

use agw_lib_outcome::{
    error::AgwError,
    exit_code::{
        SERVICE_ERROR,
        SIGINT,
    },
};
use log::info;
use signal_hook::iterator::Signals;
use std::{
    io::{
        Read,
        Write,
    },
    os::unix::net::{
        UnixListener,
        UnixStream,
    },
    path::PathBuf,
    sync::{
        Arc,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    thread,
    time::Duration,
};

/// Agwaita daemon that manages background operations via Unix socket.
///
/// The daemon listens for commands on a Unix socket and routes them to
/// appropriate components. It handles SIGINT for graceful shutdown.
pub struct Daemon {
    socket_path: PathBuf,
}

impl Daemon {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Start the daemon and listen for incoming connections.
    ///
    /// # Errors
    /// Returns an error if socket binding fails, signal handler setup fails,
    /// another instance is already running, or SIGINT is received.
    pub fn start(&self) -> Result<(), AgwError> {
        if self.socket_path.exists() {
            match UnixStream::connect(&self.socket_path) {
                Ok(_) => {
                    return Err(AgwError::with_help(
                        SERVICE_ERROR,
                        "Another instance of Agwaita is already running".to_string(),
                        format!(
                            "A running Agwaita daemon is listening on `{}`. Stop it first or use the existing instance.",
                            self.socket_path.display()
                        ),
                    ));
                },
                Err(_) => {
                    info!("Removing stale socket at {:?}", self.socket_path);
                    if let Err(e) = std::fs::remove_file(&self.socket_path) {
                        return Err(AgwError::with_source(
                            SERVICE_ERROR,
                            format!("Failed to remove stale socket: {}", e),
                            Box::new(e),
                        ));
                    }
                },
            }
        }

        let listener = UnixListener::bind(&self.socket_path).map_err(|e| {
            AgwError::with_source(
                SERVICE_ERROR,
                format!("Failed to bind socket: {}", e),
                Box::new(e),
            )
        })?;

        listener.set_nonblocking(true).map_err(|e| {
            let _ = self.remove_socket();
            AgwError::with_source(
                SERVICE_ERROR,
                format!("Failed to set socket nonblocking: {}", e),
                Box::new(e),
            )
        })?;
        info!("Daemon listening on {:?}", self.socket_path);

        let should_quit = Arc::new(AtomicBool::new(false));
        let should_quit_clone = Arc::clone(&should_quit);

        let mut signals = Signals::new(&[signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM]).map_err(|e| {
            AgwError::with_source(
                SERVICE_ERROR,
                format!("Failed to setup signal handler: {}", e),
                Box::new(e),
            )
        })?;

        thread::spawn(move || {
            for sig in signals.forever() {
                match sig {
                    signal_hook::consts::SIGINT | signal_hook::consts::SIGTERM => {
                        info!("Received shutdown signal, exiting...");
                        should_quit_clone.store(true, Ordering::Relaxed);
                        crate::message::quit();
                        break;
                    },
                    _ => {},
                }
            }
        });

        while !should_quit.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if let Ok(true) = self.handle_client(stream) {
                        info!("Received quit command");
                        break;
                    }
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                },
                Err(e) => {
                    AgwError::with_source(
                        SERVICE_ERROR,
                        format!("Failed to accept client: {}", e),
                        Box::new(e),
                    )
                    .print_and_exit();
                },
            }
        }

        self.remove_socket()?;

        if should_quit.load(Ordering::Relaxed) {
            return Err(AgwError::new(
                SIGINT,
                "Shutdown signal received".to_string(),
            ));
        }

        Ok(())
    }

    fn remove_socket(&self) -> Result<(), AgwError> {
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path).map_err(|e| {
                AgwError::with_source(
                    SERVICE_ERROR,
                    format!("Failed to remove socket: {}", e),
                    Box::new(e),
                )
            })?;
            info!("Socket removed");
        }
        Ok(())
    }

    fn handle_client(&self, mut stream: UnixStream) -> std::io::Result<bool> {
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer)?;
        let message = String::from_utf8_lossy(&buffer[..n]);

        info!("Received: {}", message.trim());

        if message.trim() == "quit" {
            let response = "OK: Shutting down daemon";
            stream.write_all(response.as_bytes())?;
            return Ok(true);
        }

        let response = self.route_message(&message);
        stream.write_all(response.as_bytes())?;

        Ok(false)
    }

    fn route_message(&self, message: &str) -> String {
        let parts: Vec<&str> = message.trim().split_whitespace().collect();

        if parts.is_empty() {
            return "ERROR: Empty message".to_string();
        }

        match parts[0] {
            "topbar" => {
                if parts.len() < 2 {
                    return "ERROR: topbar command requires action (show/hide/toggle)".to_string();
                }
                match parts[1] {
                    "show" => {
                        crate::message::show();
                        "OK: topbar shown".to_string()
                    },
                    "hide" => {
                        crate::message::hide();
                        "OK: topbar hidden".to_string()
                    },
                    "toggle" => {
                        crate::message::toggle();
                        "OK: topbar toggled".to_string()
                    },
                    _ => format!("ERROR: Unknown topbar action '{}'", parts[1]),
                }
            },
            "notification" => {
                if parts.len() < 2 {
                    return "ERROR: notification command requires action (close-last)".to_string();
                }
                match parts[1] {
                    "close-last" => {
                        agw_ui_notifications::message::close_last();
                        "OK: last notification closed".to_string()
                    },
                    _ => format!("ERROR: Unknown notification action '{}'", parts[1]),
                }
            },
            "dnd" => {
                if parts.len() < 2 {
                    return "ERROR: dnd command requires action (enable/disable/toggle/status)".to_string();
                }
                match parts[1] {
                    "enable" => {
                        crate::message::dnd_enable();
                        "OK: DND enabled".to_string()
                    },
                    "disable" => {
                        crate::message::dnd_disable();
                        "OK: DND disabled".to_string()
                    },
                    "toggle" => {
                        crate::message::dnd_toggle();
                        "OK: DND toggled".to_string()
                    },
                    "status" => {
                        let enabled = crate::message::dnd_status();
                        format!("DND: {}", if enabled { "enabled" } else { "disabled" })
                    },
                    _ => format!("ERROR: Unknown dnd action '{}'", parts[1]),
                }
            },
            "power-menu" => {
                if parts.len() < 2 {
                    return "ERROR: power-menu command requires action (toggle)".to_string();
                }
                match parts[1] {
                    "toggle" => {
                        agw_ui_power_menu::message::power_menu_toggle();
                        "OK: power-menu toggled".to_string()
                    },
                    _ => format!("ERROR: Unknown power-menu action '{}'", parts[1]),
                }
            },
            "app-launcher" => {
                if parts.len() < 2 {
                    return "ERROR: app-launcher command requires action (toggle)".to_string();
                }
                match parts[1] {
                    "toggle" => {
                        agw_ui_app_launcher::message::app_launcher_toggle();
                        "OK: app-launcher toggled".to_string()
                    },
                    _ => format!("ERROR: Unknown app-launcher action '{}'", parts[1]),
                }
            },
            "wallpaper" => "OK: wallpaper".to_string(),
            _ => format!("ERROR: Unknown component '{}'", parts[0]),
        }
    }
}

/// Get the default Unix socket path for the daemon.
///
/// Priority: `$XDG_RUNTIME_DIR/agwaita.sock`, `$TMPDIR/agwaita.sock`, `/tmp/agwaita.sock`
pub fn get_default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join("agwaita.sock");
    }

    if let Ok(tmpdir) = std::env::var("TMPDIR") {
        return PathBuf::from(tmpdir).join("agwaita.sock");
    }

    PathBuf::from("/tmp/agwaita.sock")
}

/// Initialize and start the Agwaita service.
///
/// Starts the daemon in a background thread and runs the GTK application
/// on the main thread.
///
/// # Errors
/// Returns an error if the GTK application fails to initialize.
pub fn init() -> Result<(), AgwError> {
    info!("Starting Agwaita daemon");

    let socket_path = get_default_socket_path();
    let daemon_thread = thread::spawn(move || {
        if let Err(e) = Daemon::new(socket_path).start() {
            if e.code != SIGINT {
                eprintln!("Daemon error: {}", e);
            }
        }
    });

    crate::component::run_app()?;

    let _ = daemon_thread.join();

    Ok(())
}
