use crate::model::DesktopEntry;
use freedesktop_desktop_entry::{
    DesktopEntry as FreedesktopEntry,
    get_languages_from_env,
};
use inotify::{
    Inotify,
    WatchMask,
};
use log::{
    debug,
    warn,
};
use std::{
    collections::HashMap,
    path::{
        Path,
        PathBuf,
    },
    sync::{
        Arc,
        Mutex,
        RwLock,
    },
};

/// Scanner for desktop entries with live reload support
#[derive(Debug)]
pub struct DesktopEntryScanner {
    entries: Arc<RwLock<HashMap<String, DesktopEntry>>>,
    favorites: Arc<RwLock<Vec<String>>>,
    pub(crate) inotify: Arc<Mutex<Option<Inotify>>>,
    watched_paths: Arc<RwLock<Vec<PathBuf>>>,
}

impl DesktopEntryScanner {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            favorites: Arc::new(RwLock::new(Vec::new())),
            inotify: Arc::new(Mutex::new(None)),
            watched_paths: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn get_xdg_data_dirs() -> Vec<PathBuf> {
        let xdg_data_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());

        xdg_data_dirs
            .split(':')
            .map(|p| PathBuf::from(p).join("applications"))
            .filter(|p| p.exists())
            .collect()
    }

    fn scan_directory(&self, dir: &Path, subdirs: &mut Vec<PathBuf>) -> Vec<DesktopEntry> {
        let mut entries = Vec::new();

        let read_dir = match std::fs::read_dir(dir) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to read directory {:?}: {}", dir, e);
                return entries;
            },
        };

        for entry in read_dir.flatten() {
            let path = entry.path();

            if path.is_dir() {
                subdirs.push(path.clone());
                entries.extend(self.scan_directory(&path, subdirs));
            } else if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                match self.parse_desktop_entry(&path) {
                    Ok(Some(de)) => entries.push(de),
                    Ok(None) => {},
                    Err(e) => {
                        debug!("Failed to parse {:?}: {}", path, e);
                    },
                }
            }
        }

        entries
    }

    fn parse_desktop_entry(&self, path: &Path) -> Result<Option<DesktopEntry>, String> {
        let locales = get_languages_from_env();
        let desktop = FreedesktopEntry::from_path(path, Some(&locales)).map_err(|e| format!("Failed to parse desktop entry: {e}"))?;

        if desktop.no_display() {
            return Ok(None);
        }

        let name = desktop.name(&locales).ok_or("No name field")?.to_string();

        let generic_name = desktop.generic_name(&locales).map(|s| s.to_string());

        let comment = desktop.comment(&locales).map(|s| s.to_string());

        let icon = desktop.icon().map(|s| s.to_string());

        let exec = desktop.exec().ok_or("No exec field")?.to_string();

        let categories = desktop
            .categories()
            .unwrap_or_default()
            .iter()
            .map(|str| str.to_string())
            .collect::<Vec<_>>();

        let terminal = desktop.terminal();

        let id = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid filename")?
            .to_string();

        let favorites = self.favorites.read().unwrap();
        let is_favorite = favorites.contains(&id);

        Ok(Some(DesktopEntry {
            id,
            name,
            generic_name,
            comment,
            icon,
            exec,
            categories,
            path: path.to_path_buf(),
            is_favorite,
            no_display: false,
            terminal,
        }))
    }

    pub fn scan(&self) -> Result<(), String> {
        debug!("Starting desktop entries scan");

        let paths = Self::get_xdg_data_dirs();
        debug!("Scanning paths: {:?}", paths);

        let mut all_entries = Vec::new();
        let mut all_subdirs = Vec::new();
        let mut watched = self.watched_paths.write().unwrap();

        for path in &paths {
            debug!("Scanning directory: {:?}", path);
            let mut subdirs = Vec::new();
            let entries = self.scan_directory(path, &mut subdirs);
            debug!("Found {} entries in {:?}", entries.len(), path);
            all_entries.extend(entries);

            if !watched.contains(path) {
                watched.push(path.clone());
            }

            all_subdirs.extend(subdirs);
        }

        for subdir in all_subdirs {
            if !watched.contains(&subdir) {
                watched.push(subdir);
            }
        }

        drop(watched);

        let mut entries_map = self.entries.write().unwrap();
        entries_map.clear();
        for entry in all_entries {
            entries_map.insert(entry.id.clone(), entry);
        }

        debug!("Total desktop entries loaded: {}", entries_map.len());

        Ok(())
    }

    pub fn setup_inotify(&self) -> Result<(), String> {
        let inotify_inst = Inotify::init().map_err(|e| format!("Failed to init inotify: {e}"))?;

        let watched = self.watched_paths.read().unwrap();
        for path in watched.iter() {
            match inotify_inst.watches().add(
                path,
                WatchMask::CREATE | WatchMask::DELETE | WatchMask::MODIFY | WatchMask::MOVED_TO | WatchMask::MOVED_FROM,
            ) {
                Ok(_) => debug!("Watching {:?}", path),
                Err(e) => warn!("Failed to watch {:?}: {}", path, e),
            }
        }
        drop(watched);

        *self.inotify.lock().unwrap() = Some(inotify_inst);

        Ok(())
    }

    pub fn set_favorites(&self, favorites: Vec<String>) {
        debug!("Updating favorites: {:?}", favorites);
        *self.favorites.write().unwrap() = favorites;

        let mut entries = self.entries.write().unwrap();
        let favorites_set = self.favorites.read().unwrap();

        for entry in entries.values_mut() {
            entry.is_favorite = favorites_set.contains(&entry.id);
        }
    }

    pub fn get_entries(&self) -> Vec<DesktopEntry> {
        let entries = self.entries.read().unwrap();
        let mut list: Vec<_> = entries.values().cloned().collect();
        list.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        list
    }

    pub fn entries_arc(&self) -> Arc<RwLock<HashMap<String, DesktopEntry>>> {
        self.entries.clone()
    }
}

impl Default for DesktopEntryScanner {
    fn default() -> Self {
        Self::new()
    }
}
