//! Niri window manager implementation

use super::{
    trait_impl::WMServiceTrait,
    types::{
        LaunchResult,
        WorkspaceInfo,
    },
};
use crate::{
    runtime,
    signal::Signal,
};
use log::{
    debug,
    error,
    warn,
};
use niri_ipc::{
    Action,
    Event,
    Request,
    Response,
    Workspace,
    WorkspaceReferenceArg,
    socket::Socket,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    thread,
    time::Duration,
};
use tokio::sync::RwLock;

/// Global flag to ensure only one monitoring thread is started
static NIRI_MONITORING_STARTED: AtomicBool = AtomicBool::new(false);

pub struct NiriWMService {
    /// Signal emitted when workspaces change
    pub(super) workspaces_changed: Signal<Vec<WorkspaceInfo>>,
    /// Cached workspace state
    workspaces: Arc<RwLock<Vec<WorkspaceInfo>>>,
}

impl NiriWMService {
    pub fn new() -> Self {
        Self {
            workspaces_changed: Signal::new(),
            workspaces: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Parse exec command into (command, args) for Niri spawn
    fn parse_exec(exec: &str) -> (String, Vec<String>) {
        // Remove desktop entry field codes
        let mut exec_clean = exec.to_string();

        ["%f", "%F", "%u", "%U", "%i", "%c", "%k"]
            .iter()
            .for_each(|code| {
                exec_clean = exec_clean.replace(code, "");
            });

        let mut parts = exec_clean.split_whitespace();
        let Some(command) = parts.next() else {
            return (String::new(), Vec::new());
        };

        let args = parts.map(str::to_string).collect();
        (command.to_string(), args)
    }

    /// Convert Niri workspaces to our WorkspaceInfo type
    fn convert_workspaces(niri_workspaces: Vec<niri_ipc::Workspace>, output_filter: Option<String>) -> Vec<WorkspaceInfo> {
        niri_workspaces
            .into_iter()
            .filter(|ws| {
                if let Some(ref filter) = output_filter {
                    ws.output.as_deref() == Some(filter.as_str())
                } else {
                    true
                }
            })
            .map(|ws| WorkspaceInfo {
                index: ws.idx,
                name: ws.name.unwrap_or_else(|| format!("{}", ws.idx)),
                is_active: ws.is_active,
                is_focused: ws.is_focused,
                is_urgent: false, // Niri doesn't support urgent flag yet
                output_name: ws.output,
            })
            .collect()
    }
}

impl WMServiceTrait for NiriWMService {
    fn spawn_app(&self, exec: &str, terminal: bool) -> Pin<Box<dyn Future<Output = LaunchResult> + Send + '_>> {
        let exec = exec.to_string();
        Box::pin(async move {
            // Terminal apps not supported via Niri, use fallback
            if terminal {
                return Err("Terminal launch not supported via Niri IPC".to_string());
            }

            let (command, args) = Self::parse_exec(&exec);
            if command.is_empty() {
                return Err("Empty command".to_string());
            }

            // Build spawn command with all arguments
            let mut spawn_args = vec![command.clone()];
            spawn_args.extend(args.clone());

            let action = Action::SpawnSh {
                command: spawn_args.join(" "),
            };

            // Connect to socket for this request
            let mut socket = Socket::connect().map_err(|e| format!("Niri socket not connected: {}", e))?;

            match socket.send(Request::Action(action)) {
                Ok(Ok(Response::Handled)) => {
                    debug!("App launched via Niri: {}", exec);
                    Ok(())
                },
                Ok(Ok(_)) => Err("Unexpected response from Niri".to_string()),
                Ok(Err(e)) => Err(format!("Niri rejected spawn: {}", e)),
                Err(e) => Err(format!("Failed to send spawn request: {}", e)),
            }
        })
    }

    fn get_current_workspaces(&self, output_name: Option<String>) -> Pin<Box<dyn Future<Output = Vec<WorkspaceInfo>> + Send + '_>> {
        Box::pin(async move {
            match Socket::connect() {
                Ok(mut socket) => match socket.send(Request::Workspaces) {
                    Ok(Ok(Response::Workspaces(workspaces))) => Self::convert_workspaces(workspaces, output_name),
                    Ok(Ok(_)) => {
                        warn!("Unexpected response from Niri for Workspaces request");
                        Vec::new()
                    },
                    Ok(Err(e)) => {
                        warn!("Niri rejected Workspaces request: {}", e);
                        Vec::new()
                    },
                    Err(e) => {
                        warn!("Failed to send Workspaces request: {}", e);
                        Vec::new()
                    },
                },
                Err(e) => {
                    error!("Failed to connect to Niri for workspace query: {}", e);
                    Vec::new()
                },
            }
        })
    }

    fn switch_to_workspace(&self, index: u8) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move {
            let mut socket = Socket::connect().map_err(|e| format!("Niri socket not connected: {}", e))?;

            let action = Action::FocusWorkspace {
                reference: WorkspaceReferenceArg::Index(index),
            };

            match socket.send(Request::Action(action)) {
                Ok(Ok(Response::Handled)) => {
                    debug!("Switched to workspace {}", index);
                    Ok(())
                },
                Ok(Ok(_)) => Err("Unexpected response from Niri".to_string()),
                Ok(Err(e)) => Err(format!("Niri rejected workspace switch: {}", e)),
                Err(e) => Err(format!("Failed to send workspace switch request: {}", e)),
            }
        })
    }

    fn start_workspace_monitoring(
        &self,
        _output_name: Option<String>, // Ignored - we emit ALL workspaces
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let signal = self.workspaces_changed.clone();
        let workspaces_cache = Arc::clone(&self.workspaces);

        Box::pin(async move {
            if NIRI_MONITORING_STARTED.swap(true, Ordering::SeqCst) {
                debug!("Workspace monitoring already started, skipping");
                return;
            }

            debug!("Starting workspace monitoring thread (singleton)");

            thread::spawn(move || {
                runtime::runtime().block_on(async move {
                    async fn emit_workspaces(
                        signal: &Signal<Vec<WorkspaceInfo>>,
                        workspaces_cache: &Arc<RwLock<Vec<WorkspaceInfo>>>,
                        workspaces: Vec<Workspace>,
                    ) {
                        let converted = NiriWMService::convert_workspaces(workspaces, None);
                        *workspaces_cache.write().await = converted.clone();
                        signal.emit_sync(converted);
                    }

                    // Outer reconnection loop: if the socket drops (sleep/wake, niri restart),
                    // reconnect automatically instead of leaving monitoring dead.
                    loop {
                        let mut event_sock = match Socket::connect() {
                            Ok(s) => s,
                            Err(e) => {
                                error!("Failed to connect to Niri socket: {}", e);
                                tokio::time::sleep(Duration::from_secs(5)).await;
                                continue;
                            },
                        };

                        debug!("Niri event socket connected for workspace monitoring");

                        match event_sock.send(Request::EventStream) {
                            Err(e) => {
                                error!("Failed to send EventStream request: {}", e);
                                tokio::time::sleep(Duration::from_secs(5)).await;
                                continue;
                            },
                            Ok(Err(e)) => {
                                error!("Niri rejected EventStream subscription: {}", e);
                                tokio::time::sleep(Duration::from_secs(5)).await;
                                continue;
                            },
                            Ok(Ok(_)) => {},
                        }

                        let mut read_event = event_sock.read_events();

                        loop {
                            match read_event() {
                                Ok(Event::WorkspacesChanged { workspaces }) => {
                                    debug!("WorkspacesChanged event received");
                                    emit_workspaces(&signal, &workspaces_cache, workspaces).await;
                                },
                                Ok(Event::WorkspaceActivated { id, focused }) => {
                                    debug!("WorkspaceActivated event: id={}, focused={}", id, focused);
                                    if let Ok(mut query_sock) = Socket::connect() {
                                        if let Ok(Ok(Response::Workspaces(workspaces))) = query_sock.send(Request::Workspaces) {
                                            emit_workspaces(&signal, &workspaces_cache, workspaces).await;
                                        }
                                    }
                                },
                                Ok(Event::WorkspaceActiveWindowChanged { workspace_id, .. }) => {
                                    debug!(
                                        "WorkspaceActiveWindowChanged event: workspace_id={}",
                                        workspace_id
                                    );
                                    if let Ok(mut query_sock) = Socket::connect() {
                                        if let Ok(Ok(Response::Workspaces(workspaces))) = query_sock.send(Request::Workspaces) {
                                            emit_workspaces(&signal, &workspaces_cache, workspaces).await;
                                        }
                                    }
                                },
                                Ok(_) => {},
                                Err(e) => {
                                    error!("Niri event stream disconnected: {}", e);
                                    break; // break inner loop → reconnect via outer loop
                                },
                            }
                        }

                        debug!("Niri event stream lost, reconnecting in 5s...");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                });
            });
        })
    }
}
