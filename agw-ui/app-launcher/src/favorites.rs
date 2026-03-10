use gtk4::{
    gio,
    prelude::*,
};
use std::collections::HashSet;

/// Service for managing favorite applications via GSettings
pub struct FavoritesService {
    settings: gio::Settings,
}

impl FavoritesService {
    pub fn new() -> Result<Self, String> {
        let settings = gio::Settings::new("org.gnome.shell");
        Ok(Self { settings })
    }

    pub fn get_favorites(&self) -> HashSet<String> {
        let value = self.settings.value("favorite-apps");

        if let Ok(array) = value.array_iter_str() {
            array.map(|s| s.to_string()).collect()
        } else {
            log::warn!("Failed to read favorite-apps as string array");
            HashSet::new()
        }
    }

    pub fn is_favorite(&self, app_id: &str) -> bool {
        self.get_favorites().contains(app_id)
    }

    pub fn set_favorites(&self, favorites: HashSet<String>) -> Result<(), String> {
        let favorites_vec: Vec<String> = favorites.into_iter().collect();
        let favorites_strs: Vec<&str> = favorites_vec.iter().map(|s| s.as_str()).collect();

        self.settings
            .set_strv("favorite-apps", favorites_strs.as_slice())
            .map_err(|e| format!("Failed to save favorites to GSettings: {}", e))
    }

    pub fn toggle_favorite(&self, app_id: &str) -> Result<(), String> {
        let mut favorites = self.get_favorites();

        if favorites.contains(app_id) {
            favorites.remove(app_id);
        } else {
            favorites.insert(app_id.to_string());
        }

        self.set_favorites(favorites)
    }
}

impl Default for FavoritesService {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            log::warn!("Failed to initialize FavoritesService: {}", e);
            log::warn!("Favorites will not be available");
            panic!("Cannot create FavoritesService without valid GSettings")
        })
    }
}
