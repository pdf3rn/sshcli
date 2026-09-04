use tauri::{AppHandle, Emitter};

/// Adapts a Tauri [`AppHandle`] to the application layer's
/// [`sshcli_app::events::EventEmitter`] trait, forwarding every event through
/// `AppHandle::emit` so the existing frontend wire contract is unchanged.
pub struct TauriEmitter(pub AppHandle);

impl sshcli_app::events::EventEmitter for TauriEmitter {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        let _ = self.0.emit(event, payload);
    }
}