use agw_service::dnd::DndService;

/// Simple DND toggle service
/// Creates its own DndService instance (shares GSettings with global monitor)
pub struct DndToggleService {
    service: DndService,
}

impl DndToggleService {
    pub fn new() -> Self {
        Self {
            service: DndService::new(),
        }
    }

    /// Check if DND is enabled
    pub fn is_enabled(&self) -> bool {
        self.service.get_dont_disturb()
    }

    /// Toggle DND state
    pub fn toggle(&self) {
        self.service.toggle_dont_disturb();
    }
}
