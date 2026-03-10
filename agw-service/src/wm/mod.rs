//! Window Manager service - Generic abstraction over WM-specific implementations
//!
//! This module provides a unified API for interacting with different window managers
//! using an event-driven architecture with signals. It automatically detects the
//! current window manager and uses the appropriate backend.

mod detection;
mod niri;
pub mod trait_impl;
pub mod types;
pub mod unsupported;

use crate::signal::{
    Signal,
    SignalHandler,
};
use detection::{
    WMType,
    detect_wm,
};
use log::info;
use std::sync::Arc;
use trait_impl::WMServiceTrait;
pub use types::{
    LaunchResult,
    WorkspaceInfo,
};

/// Generic window manager service
///
/// This service abstracts over different window managers (Niri, Sway, Hyprland, etc.)
/// and provides a unified API for workspace management and application launching.
///
/// The service automatically detects the current window manager at initialization
/// and uses the appropriate backend. If no supported WM is detected, it falls back
/// to a basic implementation.
///
/// # Event-driven architecture
///
/// The service uses signals to notify about workspace changes:
/// - `connect_workspaces_changed()` - Connect to workspace change events
/// - `observe_workspaces()` - High-level wrapper for observing workspace state
///
/// # Example
/// ```ignore
/// use agw_service::wm::WMService;
///
/// #[tokio::main]
/// async fn main() {
///     let wm = WMService::init().await;
///
///     // Connect to workspace changes
///     let _handler = wm.connect_workspaces_changed(|workspaces| {
///         println!("Workspaces changed: {:?}", workspaces);
///     });
///
///     // Start monitoring (required for signals to work)
///     wm.start_monitoring(None).await;
///
///     // Get current state
///     let workspaces = wm.get_workspaces(None).await;
///     println!("Current workspaces: {:?}", workspaces);
/// }
/// ```
pub struct WMService {
    inner: Arc<Box<dyn WMServiceTrait>>,
    wm_type: WMType,
    workspaces_changed: Signal<Vec<WorkspaceInfo>>,
}

impl WMService {
    /// Initialize the WM service with automatic detection
    ///
    /// This will detect the current window manager and initialize the appropriate backend.
    /// If no supported WM is detected, falls back to basic functionality.
    pub async fn init() -> Arc<Self> {
        let wm_type = detect_wm();

        info!("Detected window manager: {:?}", wm_type);

        let (inner, signal): (Box<dyn WMServiceTrait>, Signal<Vec<WorkspaceInfo>>) = match wm_type {
            WMType::Niri => {
                let niri_service = niri::NiriWMService::new();
                let signal = niri_service.workspaces_changed.clone();
                (Box::new(niri_service), signal)
            },
            WMType::Unsupported => {
                let unsupported_service = unsupported::UnsupportedWMService::new();
                (Box::new(unsupported_service), Signal::new())
            },
        };

        Arc::new(Self {
            inner: Arc::new(inner),
            wm_type,
            workspaces_changed: signal,
        })
    }

    /// Launch an application
    ///
    /// # Arguments
    /// * `exec` - The exec command string (may contain field codes like %f, %u, etc.)
    /// * `terminal` - Whether the application requires a terminal
    ///
    /// # Returns
    /// `Ok(())` if the application was launched successfully, `Err(message)` otherwise
    ///
    /// # Example
    /// ```ignore
    /// let wm_service = WMService::init().await;
    /// wm_service.spawn_app("firefox", false).await?;
    /// ```
    pub async fn spawn_app(&self, exec: &str, terminal: bool) -> LaunchResult {
        self.inner.spawn_app(exec, terminal).await
    }

    /// Get current workspace state
    ///
    /// # Arguments
    /// * `output_name` - Filter workspaces by output name (None = all outputs)
    ///
    /// # Returns
    /// List of current workspaces with their state
    pub async fn get_workspaces(&self, output_name: Option<String>) -> Vec<WorkspaceInfo> {
        self.inner.get_current_workspaces(output_name).await
    }

    /// Switch to a workspace by index
    ///
    /// # Arguments
    /// * `index` - The workspace index to switch to
    ///
    /// # Returns
    /// `Ok(())` if successful, `Err(message)` otherwise
    pub async fn switch_to_workspace(&self, index: u8) -> Result<(), String> {
        self.inner.switch_to_workspace(index).await
    }

    /// Start monitoring workspace changes
    ///
    /// This method starts a background task that listens for workspace changes
    /// and emits signals through `workspaces_changed`.
    ///
    /// Must be called for signals to work!
    ///
    /// # Arguments
    /// * `output_name` - Filter workspaces by output name (None = all outputs)
    pub async fn start_monitoring(&self, output_name: Option<String>) {
        self.inner.start_workspace_monitoring(output_name).await;
    }

    /// Connect to workspace change events
    ///
    /// The callback will be invoked whenever workspaces change (added, removed, switched, etc.)
    ///
    /// # Arguments
    /// * `callback` - Called whenever workspaces change with the new list
    ///
    /// # Returns
    /// A `SignalHandler` that can be used to disconnect the callback
    ///
    /// # Example
    /// ```ignore
    /// let handler = wm.connect_workspaces_changed(|workspaces| {
    ///     println!("Workspaces: {:?}", workspaces);
    /// });
    ///
    /// // Later, disconnect:
    /// wm.disconnect_workspaces_changed(handler);
    /// ```
    pub fn connect_workspaces_changed<F>(&self, callback: F) -> SignalHandler
    where
        F: Fn(Vec<WorkspaceInfo>) + Send + 'static,
    {
        self.workspaces_changed.connect(callback)
    }

    /// Disconnect a workspace change handler
    ///
    /// # Arguments
    /// * `handler` - The handler returned from `connect_workspaces_changed()`
    pub fn disconnect_workspaces_changed(&self, handler: SignalHandler) {
        self.workspaces_changed.disconnect(handler);
    }

    /// Observe workspaces (high-level wrapper)
    ///
    /// Combines monitoring and signal connection in one call.
    /// The callback will be invoked with the current workspace state whenever it changes.
    ///
    /// # Arguments
    /// * `output_name` - Filter workspaces by output name (None = all outputs)
    /// * `callback` - Called whenever workspaces change with the new list
    ///
    /// # Returns
    /// A `SignalHandler` that can be used to disconnect
    ///
    /// # Example
    /// ```ignore
    /// let handler = wm.observe_workspaces(None, |workspaces| {
    ///     for ws in workspaces {
    ///         println!("Workspace {}: {}", ws.index, ws.name);
    ///     }
    /// }).await;
    /// ```
    pub async fn observe_workspaces<F>(&self, output_name: Option<String>, callback: F) -> SignalHandler
    where
        F: Fn(Vec<WorkspaceInfo>) + Send + 'static,
    {
        self.start_monitoring(output_name.clone()).await;
        self.connect_workspaces_changed(callback)
    }

    /// Get the detected window manager type
    ///
    /// Useful for debugging or showing WM-specific UI
    pub fn wm_type(&self) -> WMType {
        self.wm_type
    }
}

// Make WMService cloneable via Arc
impl Clone for WMService {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            wm_type: self.wm_type,
            workspaces_changed: self.workspaces_changed.clone(),
        }
    }
}
