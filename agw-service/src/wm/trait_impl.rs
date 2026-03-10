//! Internal trait for WM service implementations

use super::types::{
    LaunchResult,
    WorkspaceInfo,
};
use std::{
    future::Future,
    pin::Pin,
};

/// Internal trait implemented by each WM backend
///
/// All methods return boxed futures for async operations.
pub(super) trait WMServiceTrait: Send + Sync {
    /// Launch an application
    ///
    /// # Arguments
    /// * `exec` - The exec command string (may contain field codes like %f, %u, etc.)
    /// * `terminal` - Whether the application requires a terminal
    ///
    /// # Returns
    /// `Ok(())` if the application was launched successfully, `Err(message)` otherwise
    fn spawn_app(&self, exec: &str, terminal: bool) -> Pin<Box<dyn Future<Output = LaunchResult> + Send + '_>>;

    /// Get current workspace state
    ///
    /// # Arguments
    /// * `output_name` - Filter workspaces by output name (None = all outputs)
    ///
    /// # Returns
    /// List of current workspaces
    fn get_current_workspaces(&self, output_name: Option<String>) -> Pin<Box<dyn Future<Output = Vec<WorkspaceInfo>> + Send + '_>>;

    /// Switch to a workspace by index
    ///
    /// # Arguments
    /// * `index` - The workspace index to switch to
    ///
    /// # Returns
    /// `Ok(())` if successful, `Err(message)` otherwise
    fn switch_to_workspace(&self, index: u8) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;

    /// Start listening for workspace changes
    ///
    /// This method should start a background task that monitors workspace changes
    /// and emits signals through the service's signal system.
    ///
    /// # Arguments
    /// * `output_name` - Filter workspaces by output name (None = all outputs)
    fn start_workspace_monitoring(&self, output_name: Option<String>) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}
