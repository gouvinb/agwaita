//! Shared types for WM (Window Manager) service

/// Information about a workspace
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceInfo {
    /// Workspace index
    pub index: u8,
    /// Workspace name
    pub name: String,
    /// Is this workspace active on its output
    pub is_active: bool,
    /// Is this workspace currently focused
    pub is_focused: bool,
    /// Is this workspace marked as urgent
    pub is_urgent: bool,
    /// Output name this workspace belongs to (for filtering by monitor)
    pub output_name: Option<String>,
}

/// Result type for application launch operations
pub type LaunchResult = Result<(), String>;
