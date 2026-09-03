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
    #[serde(default)]
    pub original_name: Option<String>,
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
    #[serde(default)]
    pub favorite: bool,
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
                favorite: self.favorite,
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
        credentials::set_verified(&profile.name, &secret).map_err(|error| error.to_string())?;
    }
    store.add(profile.clone()).map_err(|error| {
        let _ = credentials::delete(&profile.name);
        error.to_string()
    })
}

#[tauri::command]
pub fn update_profile(input: ProfileInput) -> Result<(), String> {
    let store = ProfileStore::new();
    let original_name = input
        .original_name
        .clone()
        .unwrap_or_else(|| input.name.clone());
    let previous = store
        .load()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|profile| profile.name == original_name)
        .ok_or_else(|| format!("profile not found: {original_name}"))?;
    let (mut profile, secret) = input.into_profile()?;
    profile.last_used = previous.last_used;
    profile.favorite = previous.favorite;

    let replacement_secret = profile_secret(&profile, secret);
    let preserve_secret = replacement_secret.is_none()
        && previous.authentication == profile.authentication
        && matches!(profile.authentication, Authentication::Password | Authentication::PrivateKey);
    if let Some(secret) = replacement_secret.as_deref() {
        credentials::set_verified(&profile.name, secret).map_err(|error| error.to_string())?;
    } else if preserve_secret && original_name != profile.name {
        if let Some(secret) = credentials::get_optional(&original_name).map_err(|error| error.to_string())? {
            credentials::set_verified(&profile.name, &secret).map_err(|error| error.to_string())?;
        }
    }

    store
        .replace(&original_name, profile.clone())
        .map_err(|error| error.to_string())?;

    if original_name != profile.name {
        credentials::delete(&original_name).map_err(|error| error.to_string())?;
    }
    if !preserve_secret && replacement_secret.is_none() {
        credentials::delete(&profile.name).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn duplicate_profile(source_name: String, name: String) -> Result<(), String> {
    let store = ProfileStore::new();
    let profiles = store.load().map_err(|error| error.to_string())?;
    if profiles.iter().any(|profile| profile.name == name) {
        return Err(format!("profile already exists: {name}"));
    }
    let source = profiles
        .iter()
        .find(|profile| profile.name == source_name)
        .cloned()
        .ok_or_else(|| format!("profile not found: {source_name}"))?;
    let mut duplicate = source.clone();
    duplicate.name = name.clone();
    duplicate.last_used = None;
    if let Some(secret) = credentials::get_optional(&source_name).map_err(|error| error.to_string())? {
        credentials::set_verified(&name, &secret).map_err(|error| error.to_string())?;
    }
    store.add(duplicate).map_err(|error| {
        let _ = credentials::delete(&name);
        error.to_string()
    })
}

#[tauri::command]
pub fn delete_profile(name: String) -> Result<(), String> {
    let store = ProfileStore::new();
    store.remove(&name).map_err(|error| error.to_string())?;
    let _ = credentials::delete(&name);
    Ok(())
}

#[tauri::command]
pub fn ssh_trust_host_key(host: String, port: u16, key: String) -> Result<(), String> {
    sshcli_core::host_keys::add(&host, port, &key).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_profile_secret(name: String, secret: String) -> Result<(), String> {
    if secret.is_empty() {
        return Err("credential cannot be empty".into());
    }
    let exists = ProfileStore::new()
        .load()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|profile| profile.name == name);
    if !exists {
        return Err(format!("profile not found: {name}"));
    }
    credentials::set_verified(&name, &secret).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_profiles() -> Result<String, String> {
    ProfileStore::new()
        .export_toml()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn toggle_favorite(name: String) -> Result<bool, String> {
    let store = ProfileStore::new();
    let mut profiles = store.load().map_err(|error| error.to_string())?;
    let profile = profiles
        .iter_mut()
        .find(|profile| profile.name == name)
        .ok_or_else(|| format!("profile not found: {name}"))?;
    profile.favorite = !profile.favorite;
    let favorite = profile.favorite;
    store.save(&profiles).map_err(|error| error.to_string())?;
    Ok(favorite)
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
