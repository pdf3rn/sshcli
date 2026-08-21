use std::path::PathBuf;

/// Resolves platform-specific application paths without creating files yet.
pub fn config_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "sshcli", "sshcli")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}
