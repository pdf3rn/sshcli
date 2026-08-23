use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    config,
    error::{AppError, AppResult},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Profile {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub identity_file: Option<String>,
    pub authentication: Authentication,
    pub accept_unknown_host_key: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Authentication {
    None,
    Password,
    PrivateKey,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ProfileFile {
    profiles: Vec<Profile>,
}

pub struct ProfileStore {
    path: PathBuf,
}

impl ProfileStore {
    pub fn new() -> Self {
        Self {
            path: config::config_dir().join("profiles.toml"),
        }
    }

    pub fn load(&self) -> AppResult<Vec<Profile>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.path)?;
        toml::from_str::<ProfileFile>(&content)
            .map(|file| file.profiles)
            .map_err(|error| AppError::Profile(format!("invalid profiles file: {error}")))
    }

    pub fn save(&self, profiles: &[Profile]) -> AppResult<()> {
        fs::create_dir_all(config::config_dir())?;
        let content = toml::to_string_pretty(&ProfileFile {
            profiles: profiles.to_vec(),
        })
        .map_err(|error| AppError::Profile(format!("cannot serialize profiles: {error}")))?;
        let temporary_path = self.path.with_extension("toml.tmp");
        fs::write(&temporary_path, content)?;
        fs::rename(temporary_path, &self.path)?;
        Ok(())
    }

    pub fn add(&self, profile: Profile) -> AppResult<()> {
        let mut profiles = self.load()?;
        if profiles.iter().any(|saved| saved.name == profile.name) {
            return Err(AppError::Profile(format!(
                "profile already exists: {}",
                profile.name
            )));
        }
        profiles.push(profile);
        self.save(&profiles)
    }

    pub fn remove(&self, name: &str) -> AppResult<Profile> {
        let mut profiles = self.load()?;
        let index = profiles
            .iter()
            .position(|profile| profile.name == name)
            .ok_or_else(|| AppError::Profile(format!("profile not found: {name}")))?;
        let removed = profiles.remove(index);
        self.save(&profiles)?;
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::{Authentication, Profile};

    #[test]
    fn profile_serialization_contains_no_secret_field() {
        let profile = Profile {
            name: "production".into(),
            host: "example.com".into(),
            port: 22,
            username: "deploy".into(),
            identity_file: Some("~/.ssh/id_ed25519".into()),
            authentication: Authentication::PrivateKey,
            accept_unknown_host_key: false,
        };
        let serialized = toml::to_string(&profile).unwrap();
        assert!(!serialized.contains("password"));
        assert!(!serialized.contains("secret"));
    }
}
