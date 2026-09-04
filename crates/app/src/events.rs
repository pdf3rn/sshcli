//! UI-framework-agnostic event emission.
//!
//! Managers in this crate never talk to Tauri (or any other GUI framework)
//! directly. Instead, the host provides an [`EventEmitter`] implementation
//! that knows how to deliver events to its frontend. The emitted payloads are
//! plain [`serde_json::Value`]s built from the typed payload structs in this
//! crate, so the frontend wire contract (event name + field names/values) is
//! preserved regardless of which UI is consuming it.

use std::sync::Arc;

use serde::Serialize;

/// Sink for application events (e.g. `ssh-data`, `ssh-status`, `sftp-progress`).
///
/// Implementors translate the `(event, payload)` pair into their own
/// event-delivery mechanism. The Tauri frontend implements this by calling
/// `AppHandle::emit`; a future native (egui) frontend can implement it with an
/// in-process channel instead.
pub trait EventEmitter: Send + Sync {
    /// Emit `event` carrying `payload`.
    fn emit(&self, event: &str, payload: serde_json::Value);
}

/// A shared, cloneable handle to an [`EventEmitter`].
pub type SharedEmitter = Arc<dyn EventEmitter>;

/// Serialize `payload` into a JSON value and emit it. Failures to serialize or
/// emit are ignored, matching the previous best-effort `let _ = app.emit(...)`
/// semantics.
pub fn emit<E: Serialize>(emitter: &SharedEmitter, event: &str, payload: &E) {
    let Ok(value) = serde_json::to_value(payload) else {
        return;
    };
    emitter.emit(event, value);
}

/// A no-op emitter used by pure helpers and tests that do not need to deliver
/// events anywhere.
#[derive(Default, Clone)]
pub struct NullEmitter;

impl EventEmitter for NullEmitter {
    fn emit(&self, _event: &str, _payload: serde_json::Value) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Serialize)]
    struct Payload {
        id: String,
    }

    #[test]
    fn null_emitter_ignores_events() {
        let emitter: SharedEmitter = Arc::new(NullEmitter);
        emit(&emitter, "ssh-data", &Payload { id: "x".into() });
    }

    #[test]
    fn emit_serializes_to_value_shape() {
        struct Recorder(Arc<Mutex<Vec<(String, serde_json::Value)>>>);
        impl EventEmitter for Recorder {
            fn emit(&self, event: &str, payload: serde_json::Value) {
                self.0
                    .lock()
                    .unwrap()
                    .push((event.to_string(), payload));
            }
        }
        let inner = Arc::new(Mutex::new(Vec::new()));
        let recorder: SharedEmitter = Arc::new(Recorder(inner.clone()));
        emit(&recorder, "ssh-data", &Payload { id: "s1".into() });
        let events = inner.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "ssh-data");
        assert_eq!(events[0].1["id"], "s1");
    }
}