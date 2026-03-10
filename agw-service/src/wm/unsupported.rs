//! Fallback implementation for unsupported window managers

use super::{
    trait_impl::WMServiceTrait,
    types::{
        LaunchResult,
        WorkspaceInfo,
    },
};
use log::{
    debug,
    info,
    warn,
};
use std::{
    env::var,
    future::Future,
    path::Path,
    pin::Pin,
};

pub struct UnsupportedWMService;

impl UnsupportedWMService {
    pub fn new() -> Self {
        debug!("Using unsupported WM service (fallback mode)");
        Self
    }

    fn is_command_available(cmd: &str) -> bool {
        if let Ok(path_var) = var("PATH") {
            path_var.split(':').any(|path| {
                let full_path = Path::new(path).join(cmd);
                full_path.exists() && full_path.is_file()
            })
        } else {
            false
        }
    }

    fn detect_terminal() -> Result<String, String> {
        if let Ok(terminal) = var("TERMINAL") {
            if !terminal.is_empty() {
                let cmd = terminal.split_whitespace().next().unwrap_or(&terminal);
                if Self::is_command_available(cmd) {
                    info!(
                        "Using terminal emulator from TERMINAL environment variable: {}",
                        terminal
                    );
                    return Ok(format!("{} -e", terminal));
                }
            }
        }

        // Fallback to common terminal emulators
        let terminals = [
            // Modern terminals
            "alacritty -e",
            "kitty -e",
            "wezterm start --",
            "foot -e",
            "rio -e",
            "ghostty -e",
            // GTK-based
            "gnome-terminal --",
            "tilix -e",
            "terminix -e",
            "mate-terminal -e",
            "xfce4-terminal -e",
            "lxterminal -e",
            // KDE
            "konsole -e",
            "yakuake -e",
            // Qt-based
            "qterminal -e",
            // Lightweight
            "termite -e",
            "urxvt -e",
            "rxvt -e",
            "st -e",
            "cool-retro-term -e",
            // Wayland-native
            "weston-terminal -e",
            "havoc -e",
            // Classic
            "xterm -e",
            "uxterm -e",
            // Tiling WM favorites
            "terminator -e",
            "guake -e",
            "tilda -e",
        ];

        for terminal in &terminals {
            let cmd = terminal.split_whitespace().next().unwrap_or("");
            if Self::is_command_available(cmd) {
                return Ok(terminal.to_string());
            } else {
                warn!("Terminal emulator {} not found", terminal);
            }
        }

        Err("No terminal emulator found".to_string())
    }
}

impl WMServiceTrait for UnsupportedWMService {
    fn spawn_app(&self, exec: &str, terminal: bool) -> Pin<Box<dyn Future<Output = LaunchResult> + Send + '_>> {
        let exec = exec.to_string();
        Box::pin(async move {
            let mut exec_clean = exec;
            exec_clean = exec_clean.replace("%f", "");
            exec_clean = exec_clean.replace("%F", "");
            exec_clean = exec_clean.replace("%u", "");
            exec_clean = exec_clean.replace("%U", "");
            exec_clean = exec_clean.replace("%i", "");
            exec_clean = exec_clean.replace("%c", "");
            exec_clean = exec_clean.replace("%k", "");

            let exec_clean = exec_clean.trim();

            if terminal {
                let terminal_emulator = Self::detect_terminal().map_err(|e| {
                    log::error!("No terminal emulator found. Please install one or set the TERMINAL environment variable.");
                    e
                })?;
                tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(format!("{} {}", terminal_emulator, exec_clean))
                    .spawn()
                    .map_err(|e| format!("Failed to launch in terminal: {}", e))?;
            } else {
                // Parse command and arguments
                let parts: Vec<&str> = exec_clean.split_whitespace().collect();
                if parts.is_empty() {
                    return Err("Empty exec command".to_string());
                }

                tokio::process::Command::new(parts[0])
                    .args(&parts[1..])
                    .spawn()
                    .map_err(|e| format!("Failed to launch app: {}", e))?;
            }

            debug!("App launched via fallback: {}", exec_clean);
            Ok(())
        })
    }

    fn get_current_workspaces(&self, _output_name: Option<String>) -> Pin<Box<dyn Future<Output = Vec<WorkspaceInfo>> + Send + '_>> {
        Box::pin(async move { Vec::new() })
    }

    fn switch_to_workspace(&self, _index: u8) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move { Err("Workspace switching not supported".to_string()) })
    }

    fn start_workspace_monitoring(&self, _output_name: Option<String>) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            debug!("Workspace monitoring not available in unsupported WM mode");
        })
    }
}
