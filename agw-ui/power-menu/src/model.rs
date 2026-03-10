use std::process::Command;

/// Power menu action entries
#[derive(Debug, Clone)]
pub struct PowerMenuAction {
    pub title: &'static str,
    pub icon_name: &'static str,
    pub command: fn() -> Result<(), String>,
}

impl PowerMenuAction {
    pub const LOCK_SCREEN_ACTION: PowerMenuAction = PowerMenuAction {
        title: "Lock screen",
        icon_name: "system-lock-screen-symbolic",
        command: PowerMenuAction::lock_screen,
    };
    pub const SUSPEND_ACTION: PowerMenuAction = PowerMenuAction {
        title: "Suspend",
        icon_name: "preferences-desktop-screensaver-symbolic",
        command: PowerMenuAction::suspend,
    };
    pub const LOGOUT_ACTION: PowerMenuAction = PowerMenuAction {
        title: "Log-out",
        icon_name: "system-log-out-symbolic",
        command: PowerMenuAction::logout,
    };
    pub const REBOOT_ACTION: PowerMenuAction = PowerMenuAction {
        title: "Reboot",
        icon_name: "system-reboot-symbolic",
        command: PowerMenuAction::reboot,
    };
    pub const SHUTDOWN_ACTION: PowerMenuAction = PowerMenuAction {
        title: "Shutdown",
        icon_name: "system-shutdown-symbolic",
        command: PowerMenuAction::shutdown,
    };

    pub const ALL_ACTIONS: [PowerMenuAction; 5] = [
        Self::LOCK_SCREEN_ACTION,
        Self::SUSPEND_ACTION,
        Self::LOGOUT_ACTION,
        Self::REBOOT_ACTION,
        Self::SHUTDOWN_ACTION,
    ];

    pub fn lock_screen() -> Result<(), String> {
        Command::new("loginctl")
            .arg("lock-session")
            .status()
            .map_err(|e| format!("Failed to lock session: {e}"))?;
        Ok(())
    }

    pub fn suspend() -> Result<(), String> {
        Command::new("systemctl")
            .arg("suspend")
            .status()
            .map_err(|e| format!("Failed to suspend session: {e}"))?;
        Ok(())
    }

    pub fn logout() -> Result<(), String> {
        let user = std::env::var("USER").map_err(|e| format!("Failed to get USER: {e}"))?;
        Command::new("loginctl")
            .args(["kill-user", &user])
            .status()
            .map_err(|e| format!("Failed to logout user: {e}"))?;
        Ok(())
    }

    pub fn reboot() -> Result<(), String> {
        Command::new("systemctl")
            .arg("reboot")
            .status()
            .map_err(|e| format!("Failed to reboot: {e}"))?;
        Ok(())
    }

    pub fn shutdown() -> Result<(), String> {
        Command::new("systemctl")
            .args(["-i", "poweroff"])
            .status()
            .map_err(|e| format!("Failed to shutdown: {e}"))?;
        Ok(())
    }

    // TODO: replace by https://github.com/rust-lang/rust/issues/29625 when stabilized
    pub fn call(&self) -> Result<(), String> {
        (self.command)()
    }
}
