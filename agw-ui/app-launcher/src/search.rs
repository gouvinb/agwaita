use crate::model::DesktopEntry;
use nucleo_matcher::chars::normalize;
use std::collections::HashSet;

/// Searcher for desktop entries with prioritized filtering
pub struct AppSearcher {
    all_entries: Vec<DesktopEntry>,
}

impl AppSearcher {
    pub fn new() -> Self {
        Self {
            all_entries: Vec::new(),
        }
    }

    pub fn set_entries(&mut self, entries: Vec<DesktopEntry>) {
        self.all_entries = entries;
    }

    /// Search with priority: name > generic_name > categories > comment
    pub fn search(&mut self, query: &str) -> Vec<DesktopEntry> {
        if query.is_empty() {
            return self.all_entries.clone();
        }

        let query_normalized = self.normalize_str(query);

        let mut list_name: Vec<DesktopEntry> = self
            .all_entries
            .iter()
            .filter(|entry| self.normalize_str(&entry.name).contains(&query_normalized))
            .cloned()
            .collect();

        let mut list_generic: Vec<DesktopEntry> = self
            .all_entries
            .iter()
            .filter(|entry| {
                entry
                    .generic_name
                    .as_ref()
                    .map(|s| self.normalize_str(&s).contains(&query_normalized))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        let mut list_categories: Vec<DesktopEntry> = self
            .all_entries
            .iter()
            .filter(|entry| {
                entry
                    .categories
                    .iter()
                    .any(|c| self.normalize_str(&c).contains(&query_normalized))
            })
            .cloned()
            .collect();

        let mut list_comment: Vec<DesktopEntry> = self
            .all_entries
            .iter()
            .filter(|entry| {
                entry
                    .comment
                    .as_ref()
                    .map(|s| self.normalize_str(s).contains(&query_normalized))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        list_name.sort_by(|a, b| {
            self.normalize_str(&a.name)
                .cmp(&self.normalize_str(&b.name))
        });

        list_generic.sort_by(|a, b| {
            self.normalize_str(&a.name)
                .cmp(&self.normalize_str(&b.name))
        });
        let seen_ids: HashSet<String> = list_name.iter().map(|e| e.id.clone()).collect();
        list_generic.retain(|e| !seen_ids.contains(&e.id));

        list_categories.sort_by(|a, b| {
            self.normalize_str(&a.name)
                .cmp(&self.normalize_str(&b.name))
        });
        let seen_ids: HashSet<String> = list_name
            .iter()
            .chain(list_generic.iter())
            .map(|e| e.id.clone())
            .collect();
        list_categories.retain(|e| !seen_ids.contains(&e.id));

        list_comment.sort_by(|a, b| {
            self.normalize_str(&a.name)
                .cmp(&self.normalize_str(&b.name))
        });
        let seen_ids: HashSet<String> = list_name
            .iter()
            .chain(list_generic.iter())
            .chain(list_categories.iter())
            .map(|e| e.id.clone())
            .collect();
        list_comment.retain(|e| !seen_ids.contains(&e.id));

        let mut results = Vec::new();
        results.extend(list_name);
        results.extend(list_generic);
        results.extend(list_categories);
        results.extend(list_comment);

        results.sort_by(|a, b| match (a.is_favorite, b.is_favorite) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });

        results
    }

    fn normalize_str(&self, string: &str) -> String {
        string
            .to_lowercase()
            .chars()
            .map(|c| normalize(c))
            .collect::<String>()
    }
}

impl Default for AppSearcher {
    fn default() -> Self {
        Self::new()
    }
}
