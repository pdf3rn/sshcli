use sshcli_core::{Profile, ProfileStore};

#[tauri::command]
pub fn list_profiles() -> Result<Vec<Profile>, String> {
    ProfileStore::new()
        .load()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_identity_keys() -> Result<Vec<String>, String> {
    Ok(sshcli_core::keys::available_identity_files())
}
