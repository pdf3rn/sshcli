use std::{fs, path::PathBuf};

/// Discover private SSH keys in `~/.ssh` (files named `id_*`, excluding `.pub`).
pub fn available_identity_files() -> Vec<String> {
    let Some(home) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) else {
        return Vec::new();
    };
    let ssh_dir = home.join(".ssh");
    let mut keys = fs::read_dir(ssh_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            let is_candidate = path.is_file() && name.starts_with("id_") && !name.ends_with(".pub");
            is_candidate.then(|| path.to_string_lossy().into_owned())
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

pub fn expand_home(path: &str) -> PathBuf {
    let home = || directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    if path == "~" {
        if let Some(home) = home() {
            return home;
        }
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = home() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}
