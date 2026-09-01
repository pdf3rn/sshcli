use std::fs;

use serde::{Deserialize, Serialize};

use crate::{config, AppError, AppResult};

#[derive(Default, Deserialize, Serialize)]
struct HostKeyFile {
    entries: Vec<HostKeyEntry>,
}

#[derive(Deserialize, Serialize)]
struct HostKeyEntry {
    host: String,
    port: u16,
    key: String,
}

fn path() -> std::path::PathBuf {
    config::config_dir().join("known_hosts.toml")
}

pub fn verify_or_add(host: &str, port: u16, key: &str, accept_unknown: bool) -> AppResult<bool> {
    let path = path();
    let mut known = if path.exists() {
        let content = fs::read_to_string(&path)?;
        toml::from_str::<HostKeyFile>(&content)
            .map_err(|error| AppError::Profile(format!("invalid known hosts file: {error}")))?
    } else {
        HostKeyFile::default()
    };

    if let Some(entry) = known
        .entries
        .iter()
        .find(|entry| entry.host == host && entry.port == port)
    {
        return Ok(entry.key == key);
    }
    if !accept_unknown {
        return Ok(false);
    }

    known.entries.push(HostKeyEntry {
        host: host.to_string(),
        port,
        key: key.to_string(),
    });
    fs::create_dir_all(config::config_dir())?;
    let temporary = path.with_extension("toml.tmp");
    let content = toml::to_string_pretty(&known)
        .map_err(|error| AppError::Profile(format!("cannot serialize known hosts: {error}")))?;
    fs::write(&temporary, content)?;
    fs::rename(temporary, path)?;
    Ok(true)
}
