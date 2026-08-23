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
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub last_used: Option<u64>,
    #[serde(default)]
    pub favorite: bool,
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

    pub fn touch_last_used(&self, name: &str) -> AppResult<()> {
        let mut profiles = self.load()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        if let Some(profile) = profiles.iter_mut().find(|profile| profile.name == name) {
            profile.last_used = Some(now);
        }
        self.save(&profiles)
    }

    pub fn import_toml(&self, content: &str) -> AppResult<usize> {
        let incoming = toml::from_str::<ProfileFile>(content)
            .map_err(|error| AppError::Profile(format!("invalid profiles file: {error}")))?
            .profiles;
        let mut current = self.load()?;
        let mut imported = 0;
        for profile in incoming {
            if current.iter().any(|existing| existing.name == profile.name) {
                continue;
            }
            current.push(profile);
            imported += 1;
        }
        if imported > 0 {
            self.save(&current)?;
        }
        Ok(imported)
    }

    pub fn export_toml(&self) -> AppResult<String> {
        let file = ProfileFile {
            profiles: self.load()?,
        };
        toml::to_string(&file).map_err(|error| AppError::Profile(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::{Authentication, Profile};
    use serde::Deserialize;

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
            group: Some("Production".into()),
            tags: vec!["web".into()],
            last_used: Some(1_700_000_000),
            favorite: true,
        };
        let serialized = toml::to_string(&profile).unwrap();
        assert!(!serialized.contains("password"));
        assert!(!serialized.contains("secret"));
    }

    #[test]
    fn profile_deserializes_without_optional_fields() {
        #[derive(Deserialize)]
        struct Wrapper {
            profiles: Vec<Profile>,
        }
        let legacy = r#"
            [[profiles]]
            name = "legacy"
            host = "example.com"
            port = 22
            username = "deploy"
            authentication = "None"
            accept_unknown_host_key = false
        "#;
        let parsed = toml::from_str::<Wrapper>(legacy).unwrap();
        assert_eq!(parsed.profiles.len(), 1);
        assert_eq!(parsed.profiles[0].group, None);
        assert!(parsed.profiles[0].tags.is_empty());
        assert_eq!(parsed.profiles[0].last_used, None);
        assert!(!parsed.profiles[0].favorite);
    }
}
