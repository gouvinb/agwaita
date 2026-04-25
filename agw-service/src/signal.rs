//! Generic signal system for event-driven service notifications
//!
//! This module provides a lightweight signal/slot mechanism inspired by GLib
//! but without heavy dependencies. It enables async, thread-safe event handling
//! across all services in agw-service.

use std::{
    collections::HashMap,
    fmt,
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicU64,
            Ordering,
        },
    },
};

/// Handle returned when connecting to a signal
///
/// Automatically disconnects from the signal when dropped (RAII).
pub struct SignalHandler {
    pub(crate) id: u64,
    disconnect: Arc<dyn Fn(u64) + Send + Sync>,
}

impl fmt::Debug for SignalHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignalHandler")
            .field("id", &self.id)
            .finish()
    }
}

impl Drop for SignalHandler {
    fn drop(&mut self) {
        (self.disconnect)(self.id);
    }
}

/// Generic signal that can emit events to multiple connected handlers
///
/// Thread-safe and async-compatible signal implementation.
/// Handlers are called asynchronously via tokio::spawn when a signal is emitted.
///
/// # Type Parameters
/// * `T` - The type of value emitted by this signal (must be Clone + Send)
///
/// # Example
/// ```ignore
/// let signal = Signal::<String>::new();
///
/// let handler = signal.connect(|value| {
///     println!("Received: {}", value);
/// });
///
/// signal.emit("Hello".to_string()).await;
///
/// signal.disconnect(handler);
/// ```
pub struct Signal<T: Clone + Send> {
    handlers: Arc<Mutex<HashMap<u64, Box<dyn Fn(T) + Send + 'static>>>>,
    next_id: Arc<AtomicU64>,
}

impl<T: Clone + Send> Signal<T> {
    /// Create a new signal
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Connect a callback to this signal
    ///
    /// The callback will be invoked whenever the signal is emitted.
    ///
    /// # Arguments
    /// * `callback` - Function to call when signal is emitted
    ///
    /// # Returns
    /// A `SignalHandler` that can be used to disconnect the callback
    pub fn connect<F>(&self, callback: F) -> SignalHandler
    where
        T: 'static,
        F: Fn(T) + Send + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.handlers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, Box::new(callback));
        let handlers = Arc::clone(&self.handlers);
        SignalHandler {
            id,
            disconnect: Arc::new(move |id| {
                if let Ok(mut h) = handlers.lock() {
                    h.remove(&id);
                }
            }),
        }
    }

    /// Emit the signal with a value
    ///
    /// All connected callbacks will be invoked asynchronously with the provided value.
    /// Each callback is spawned in its own tokio task.
    ///
    /// # Arguments
    /// * `value` - The value to pass to all connected callbacks
    pub async fn emit(&self, value: T) {
        // Call callbacks synchronously - if async needed, the callback can spawn its own task
        let handlers_lock = self.handlers.lock().unwrap_or_else(|e| e.into_inner());
        for callback in handlers_lock.values() {
            let v = value.clone();
            callback(v);
        }
    }

    /// Emit the signal synchronously
    ///
    /// All connected callbacks will be invoked synchronously in the current thread.
    ///
    /// # Arguments
    /// * `value` - The value to pass to all connected callbacks
    pub fn emit_sync(&self, value: T) {
        let handlers_lock = self.handlers.lock().unwrap_or_else(|e| e.into_inner());
        for callback in handlers_lock.values() {
            let v = value.clone();
            callback(v);
        }
    }

    /// Disconnect a callback from this signal
    ///
    /// # Arguments
    /// * `handler` - The handler returned from `connect()`
    pub fn disconnect(&self, handler: SignalHandler) {
        if let Ok(mut h) = self.handlers.lock() {
            h.remove(&handler.id);
        }
    }

    /// Get the number of connected handlers
    pub fn handler_count(&self) -> usize {
        self.handlers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }
}

impl<T: Clone + Send> Default for Signal<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Send> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
            next_id: Arc::clone(&self.next_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[test]
    fn test_signal_connect_and_emit() {
        let signal = Signal::<u32>::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let _handler = signal.connect(move |value| {
            counter_clone.fetch_add(value, Ordering::SeqCst);
        });

        signal.emit_sync(5);
        assert_eq!(counter.load(Ordering::SeqCst), 5);

        signal.emit_sync(10);
        assert_eq!(counter.load(Ordering::SeqCst), 15);
    }

    #[test]
    fn test_signal_disconnect() {
        let signal = Signal::<u32>::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let handler = signal.connect(move |value| {
            counter_clone.fetch_add(value, Ordering::SeqCst);
        });

        signal.emit_sync(5);
        assert_eq!(counter.load(Ordering::SeqCst), 5);

        signal.disconnect(handler);

        signal.emit_sync(10);
        assert_eq!(counter.load(Ordering::SeqCst), 5); // Should not change
    }

    #[test]
    fn test_multiple_handlers() {
        let signal = Signal::<String>::new();
        let messages = Arc::new(Mutex::new(Vec::new()));

        let msg1 = Arc::clone(&messages);
        let _h1 = signal.connect(move |s| {
            msg1.lock().unwrap().push(format!("Handler1: {}", s));
        });

        let msg2 = Arc::clone(&messages);
        let _h2 = signal.connect(move |s| {
            msg2.lock().unwrap().push(format!("Handler2: {}", s));
        });

        signal.emit_sync("test".to_string());

        let msgs = messages.lock().unwrap();
        assert_eq!(msgs.len(), 2);
        assert!(msgs.contains(&"Handler1: test".to_string()));
        assert!(msgs.contains(&"Handler2: test".to_string()));
    }
}
