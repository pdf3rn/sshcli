use serde::Deserialize;
use sshcli_core::{
    credentials,
    profiles::{Authentication, Profile, ProfileStore},
    Profile as CoreProfile,
};

#[tauri::command]
pub fn list_profiles() -> Result<Vec<CoreProfile>, String> {
    ProfileStore::new()
        .load()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_identity_keys() -> Result<Vec<String>, String> {
    Ok(sshcli_core::keys::available_identity_files())
}

#[derive(Deserialize)]
pub struct ProfileInput {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub identity_file: Option<String>,
    pub authentication: String,
    pub accept_unknown_host_key: bool,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    pub secret: Option<String>,
}

impl ProfileInput {
    fn authentication(&self) -> Result<Authentication, String> {
        match self.authentication.as_str() {
            "none" => Ok(Authentication::None),
            "password" => Ok(Authentication::Password),
            "private-key" => Ok(Authentication::PrivateKey),
            other => Err(format!("unknown authentication method: {other}")),
        }
    }

    fn into_profile(self) -> Result<(CoreProfile, Option<String>), String> {
        let authentication = self.authentication()?;
        let identity_file = self.identity_file;
        if matches!(authentication, Authentication::PrivateKey) && identity_file.is_none() {
            return Err("private-key authentication requires an identity file".into());
        }
        Ok((
            Profile {
                name: self.name,
                host: self.host,
                port: self.port,
                username: self.username,
                identity_file,
                authentication,
                accept_unknown_host_key: self.accept_unknown_host_key,
                group: self.group.filter(|group| !group.is_empty()),
                tags: self.tags.unwrap_or_default(),
                last_used: None,
            },
            self.secret,
        ))
    }
}

#[tauri::command]
pub fn create_profile(input: ProfileInput) -> Result<(), String> {
    let store = ProfileStore::new();
    if store
        .load()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|profile| profile.name == input.name)
    {
        return Err(format!("profile already exists: {}", input.name));
    }
    let (profile, secret) = input.into_profile()?;
    if let Some(secret) = profile_secret(&profile, secret) {
        credentials::set(&profile.name, &secret).map_err(|error| error.to_string())?;
    }
    store.add(profile.clone()).map_err(|error| {
        let _ = credentials::delete(&profile.name);
        error.to_string()
    })
}

#[tauri::command]
pub fn update_profile(input: ProfileInput) -> Result<(), String> {
    let store = ProfileStore::new();
    let previous_last_used = store
        .load()
        .ok()
        .and_then(|profiles| profiles.into_iter().find(|p| p.name == input.name))
        .and_then(|profile| profile.last_used);
    let (mut profile, secret) = input.into_profile()?;
    profile.last_used = previous_last_used;
    let _ = credentials::delete(&profile.name);
    if let Some(secret) = profile_secret(&profile, secret) {
        credentials::set(&profile.name, &secret).map_err(|error| error.to_string())?;
    }
    if let Err(error) = store.remove(&profile.name).and_then(|_| store.add(profile)) {
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn delete_profile(name: String) -> Result<(), String> {
    let store = ProfileStore::new();
    store.remove(&name).map_err(|error| error.to_string())?;
    let _ = credentials::delete(&name);
    Ok(())
}

#[tauri::command]
pub fn touch_last_used(name: String) -> Result<(), String> {
    ProfileStore::new()
        .touch_last_used(&name)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_profiles(content: String) -> Result<usize, String> {
    ProfileStore::new()
        .import_toml(&content)
        .map_err(|error| error.to_string())
}

fn profile_secret(profile: &Profile, supplied: Option<String>) -> Option<String> {
    match profile.authentication {
        Authentication::Password => supplied.filter(|secret| !secret.is_empty()),
        Authentication::PrivateKey => supplied.filter(|secret| !secret.is_empty()),
        Authentication::None => None,
    }
}
