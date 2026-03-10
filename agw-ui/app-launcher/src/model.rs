use agw_service::wm::WMService;
use std::{
    path::PathBuf,
    sync::Arc,
};

/// Desktop application entry
#[derive(Debug, Clone)]
pub struct DesktopEntry {
    /// Desktop entry ID (e.g., "firefox.desktop")
    pub id: String,
    /// Application name (localized)
    pub name: String,
    /// Generic name (localized)
    pub generic_name: Option<String>,
    /// Comment/description (localized)
    pub comment: Option<String>,
    /// Icon name or path
    pub icon: Option<String>,
    /// Exec command
    pub exec: String,
    /// Categories
    pub categories: Vec<String>,
    /// Path to .desktop file
    pub path: PathBuf,
    /// Is this app in favorites
    pub is_favorite: bool,
    /// Should be shown in launcher
    pub no_display: bool,
    /// Terminal required
    pub terminal: bool,
}

impl DesktopEntry {
    pub fn display_name(&self) -> &str {
        &self.name
    }

    pub fn search_text(&self) -> String {
        let mut text = self.name.clone();

        if let Some(ref generic) = self.generic_name {
            text.push(' ');
            text.push_str(generic);
        }

        if let Some(ref comment) = self.comment {
            text.push(' ');
            text.push_str(comment);
        }

        for category in &self.categories {
            text.push(' ');
            text.push_str(category);
        }

        text
    }

    pub fn sort_key(&self) -> (bool, String) {
        (!self.is_favorite, self.name.to_lowercase())
    }

    pub async fn launch(&self, wm_service: &Arc<WMService>) -> Result<(), String> {
        let mut exec = self.exec.clone();

        // Remove Desktop Entry field codes
        exec = exec.replace("%f", "");
        exec = exec.replace("%F", "");
        exec = exec.replace("%u", "");
        exec = exec.replace("%U", "");
        exec = exec.replace("%i", "");
        exec = exec.replace("%c", "");
        exec = exec.replace("%k", "");

        let exec = exec.trim();

        // Use WMService's spawn_app which handles WM-specific launching
        wm_service.spawn_app(exec, self.terminal).await
    }
}
