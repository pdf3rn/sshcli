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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKeyStatus {
    Known,
    Unknown,
    Changed,
}

fn load(known: &mut HostKeyFile) -> AppResult<()> {
    let path = path();
    if let Some(content) = content(path)? {
        *known = toml::from_str::<HostKeyFile>(&content)
            .map_err(|error| AppError::Profile(format!("invalid known hosts file: {error}")))?;
    }
    Ok(())
}

fn content(path: std::path::PathBuf) -> AppResult<Option<String>> {
    if path.exists() {
        Ok(Some(fs::read_to_string(&path)?))
    } else {
        Ok(None)
    }
}

pub fn verify(host: &str, port: u16, key: &str) -> AppResult<HostKeyStatus> {
    let mut known = HostKeyFile::default();
    load(&mut known)?;
    if let Some(entry) = known
        .entries
        .iter()
        .find(|entry| entry.host == host && entry.port == port)
    {
        if entry.key == key {
            return Ok(HostKeyStatus::Known);
        }
        return Ok(HostKeyStatus::Changed);
    }
    Ok(HostKeyStatus::Unknown)
}

pub fn add(host: &str, port: u16, key: &str) -> AppResult<()> {
    let path = path();
    let mut known = HostKeyFile::default();
    load(&mut known)?;
    if let Some(entry) = known
        .entries
        .iter_mut()
        .find(|entry| entry.host == host && entry.port == port)
    {
        entry.key = key.to_string();
    } else {
        known.entries.push(HostKeyEntry {
            host: host.to_string(),
            port,
            key: key.to_string(),
        });
    }
    fs::create_dir_all(config::config_dir())?;
    let temporary = path.with_extension("toml.tmp");
    let content = toml::to_string_pretty(&known)
        .map_err(|error| AppError::Profile(format!("cannot serialize known hosts: {error}")))?;
    fs::write(&temporary, content)?;
    fs::rename(temporary, path)?;
    Ok(())
}
