use agw_service::{
    signal::SignalHandler,
    wm::{
        WMService,
        WorkspaceInfo,
    },
};
use catalyser::stdx::extension::str_extension::MultilineStr;
use gtk4::prelude::*;
use log::{
    debug,
    error,
};
use relm4::{
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    SimpleComponent,
    gtk,
};
use std::sync::Arc;

pub struct Workspaces {
    #[allow(dead_code)] // public api
    output_name: Option<String>,
    workspaces: Vec<WorkspaceInfo>,
    wm_service: Arc<WMService>,
    container: gtk::Box,
    _wm_handler: SignalHandler,
}

#[derive(Debug)]
pub enum WorkspacesInput {
    UpdateWorkspaces(Vec<WorkspaceInfo>),
    // RefreshWorkspaces,
    //SwitchToWorkspace(u8),
}

#[relm4::component(pub)]
impl SimpleComponent for Workspaces {
    type Init = (Option<String>, Arc<WMService>);
    type Input = WorkspacesInput;
    type Output = ();

    view! {
        #[root]
        #[name = "container"]
        gtk::Box {
            set_spacing: 4,
            set_halign: gtk::Align::Start,
        }
    }

    fn init((output_name, wm_service): Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        debug!(
            "Initializing Workspaces component for output: {:?}",
            output_name
        );

        // Connect to workspace changes signal before starting monitoring.
        // The handler must be stored in the model — if left as a local in an async block,
        // the compiler may drop it before the first await, disconnecting the signal immediately.
        let sender_clone = sender.clone();
        let output_filter = output_name.clone();
        let wm_handler = wm_service.connect_workspaces_changed(move |workspaces| {
            debug!(
                "Workspaces updated: {} workspaces (before filter)",
                workspaces.len()
            );

            let filtered: Vec<_> = workspaces
                .into_iter()
                .filter(|ws| {
                    if let Some(ref filter) = output_filter {
                        ws.output_name.as_deref() == Some(filter.as_str())
                    } else {
                        true
                    }
                })
                .collect();

            debug!(
                "Workspaces after filter for {:?}: {} workspaces",
                output_filter,
                filtered.len()
            );

            let sender_for_idle = sender_clone.clone();
            gtk4::glib::idle_add_once(move || {
                if sender_for_idle
                    .input_sender()
                    .send(WorkspacesInput::UpdateWorkspaces(filtered))
                    .is_err()
                {
                    debug!("Component was dropped, ignoring workspace update");
                }
            });
        });

        let model = Workspaces {
            output_name: output_name.clone(),
            workspaces: Vec::new(),
            wm_service: wm_service.clone(),
            container: root.clone(),
            _wm_handler: wm_handler,
        };

        let widgets = view_output!();

        // Get initial workspace state (async)
        let wm_clone2 = wm_service.clone();
        let output_clone2 = output_name.clone();
        let sender_clone2 = sender.clone();
        gtk4::glib::spawn_future_local(async move {
            let initial_workspaces = wm_clone2.get_workspaces(output_clone2).await;
            if !initial_workspaces.is_empty() {
                sender_clone2.input(WorkspacesInput::UpdateWorkspaces(initial_workspaces));
            }
        });

        // Start monitoring in the background (connect_workspaces_changed already done above)
        let wm_clone = wm_service.clone();
        let output_clone = output_name.clone();
        gtk4::glib::spawn_future_local(async move {
            wm_clone.start_monitoring(output_clone).await;
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            WorkspacesInput::UpdateWorkspaces(workspaces) => {
                let mut sorted_workspaces = workspaces;
                sorted_workspaces.sort_by_key(|ws| ws.index);

                // Only rebuild if workspaces have changed
                if self.workspaces != sorted_workspaces {
                    debug!("Workspaces changed, rebuilding UI");
                    self.workspaces = sorted_workspaces;

                    // Rebuild the buttons
                    while let Some(child) = self.container.first_child() {
                        self.container.remove(&child);
                    }

                    for workspace in &self.workspaces {
                        let button = gtk::Button::builder().label(&workspace.name).build();

                        if let Some(css) = Self::resolve_workspace_classes(workspace) {
                            button.inline_css(css.as_str());
                        }

                        let index = workspace.index;
                        let wm_service = self.wm_service.clone();
                        button.connect_clicked(move |_| {
                            // NEW: Async switch with glib spawn
                            let wm_clone = wm_service.clone();
                            gtk4::glib::spawn_future_local(async move {
                                if let Err(e) = wm_clone.switch_to_workspace(index).await {
                                    error!("Failed to switch to workspace {}: {}", index, e);
                                }
                            });
                        });

                        self.container.append(&button);
                    }
                }
            },
        }
    }
}

impl Workspaces {
    fn resolve_workspace_classes(workspace: &WorkspaceInfo) -> Option<String> {
        let mut css = "padding: 0 10px;".to_string();

        if workspace.is_urgent {
            css.push_str(
                "
                |background: var(--warning-bg-color);
                |color: var(--warning-fg-color);
                "
                .trim_margin()
                .as_str(),
            );
        } else if workspace.is_focused {
            css.push_str(
                "
                |background: var(--accent-bg-color);
                |color: var(--accent-fg-color);
                "
                .trim_margin()
                .as_str(),
            );
        } else if workspace.is_active {
            css.push_str(
                "
                |background: var(--accent-bg-color);
                "
                .trim_margin()
                .as_str(),
            );
        }

        Some(css)
    }
}
