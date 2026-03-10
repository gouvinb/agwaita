//! Notification actions

use crate::signal::Signal;
use std::{
    fmt::Debug,
    sync::Arc,
};

/// An action that can be invoked on a notification
#[derive(Clone)]
pub struct Action {
    inner: Arc<ActionInner>,
}

struct ActionInner {
    /// Unique identifier for the action
    pub id: String,
    /// Human-readable label for the action
    pub label: String,
    /// Emitted when this action is invoked
    pub invoked: Signal<()>,
}

impl Action {
    /// Create a new action
    pub fn new(id: String, label: String) -> Self {
        Self {
            inner: Arc::new(ActionInner {
                id,
                label,
                invoked: Signal::new(),
            }),
        }
    }

    pub fn id(&self) -> &str {
        &self.inner.id
    }

    pub fn label(&self) -> &str {
        &self.inner.label
    }

    /// Invoke this action
    pub fn invoke(&self) {
        self.inner.invoked.emit_sync(());
    }

    /// Connect to the invoked signal
    pub fn connect_invoked<F>(&self, callback: F) -> crate::signal::SignalHandler
    where
        F: Fn(()) + Send + 'static,
    {
        self.inner.invoked.connect(callback)
    }

    /// Create list of actions from string array (pairs of id, label)
    pub fn new_list(strv: Vec<String>) -> Vec<Action> {
        strv.chunks(2)
            .filter_map(|chunk| {
                if chunk.len() == 2 {
                    Some(Action::new(chunk[0].clone(), chunk[1].clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Action")
            .field("id", &self.inner.id)
            .field("label", &self.inner.label)
            .finish()
    }
}
