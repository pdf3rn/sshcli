//! UI-framework-agnostic application services for sshcli.
//!
//! This crate contains the stateful managers and business operations that were
//! previously embedded in the Tauri GUI crate. It has no dependency on Tauri or
//! any other GUI framework; instead, it emits events through an
//! [`events::EventEmitter`] supplied by the host, so both the existing Tauri
//! frontend and a future native (egui) frontend can reuse it unchanged.
//!
//! The wire contracts (event names `ssh-data`, `ssh-status`, `sftp-progress`,
//! and the field names/values of the emitted payloads) are preserved exactly.

pub mod commands;
pub mod events;
pub mod local_shell;
pub mod session;
pub mod sftp_session;
pub mod telemetry;
pub mod tunnel;

// Re-export the most common entry points so hosts can import from one place.
pub use commands::{
    create_profile, delete_profile, duplicate_profile, export_profiles, import_profiles,
    list_identity_keys, list_profiles, profile_secret, save_profile_secret, ssh_trust_host_key,
    test_profile, toggle_favorite, touch_last_used, update_profile, ProfileInput,
};
pub use session::{init_state as session_init, SessionManager, SessionState};
pub use local_shell::{init_state as local_shell_init, LocalShellManager, LocalShellState};
pub use sftp_session::{init_state as sftp_init, SftpManager, SftpState};
pub use tunnel::{init_state as tunnel_init, TunnelManager, TunnelState};
pub use telemetry::{init_state as telemetry_init, TelemetryManager, TelemetryState};