//! UI-independent core for sshcli: configuration, profiles, credentials,
//! SSH sessions, SFTP operations and port forwarding.
//!
//! No terminal or GUI code lives here; frontends (Tauri app) consume this API.

pub mod config;
pub mod credentials;
pub mod error;
pub mod keys;
pub mod profiles;
pub mod sftp;
pub mod shells;
pub mod ssh;

pub use error::{AppError, AppResult};
pub use profiles::{Authentication, Profile, ProfileStore};
