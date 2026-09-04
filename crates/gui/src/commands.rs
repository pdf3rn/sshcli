use sshcli_app::ProfileInput;
use sshcli_core::Profile as CoreProfile;

#[tauri::command]
pub fn list_profiles() -> Result<Vec<CoreProfile>, String> {
    sshcli_app::list_profiles()
}

#[tauri::command]
pub fn list_identity_keys() -> Result<Vec<String>, String> {
    sshcli_app::list_identity_keys()
}

#[tauri::command]
pub async fn test_profile(input: ProfileInput) -> Result<(), String> {
    sshcli_app::test_profile(input).await
}

#[tauri::command]
pub fn create_profile(input: ProfileInput) -> Result<(), String> {
    sshcli_app::create_profile(input)
}

#[tauri::command]
pub fn update_profile(input: ProfileInput) -> Result<(), String> {
    sshcli_app::update_profile(input)
}

#[tauri::command]
pub fn duplicate_profile(source_name: String, name: String) -> Result<(), String> {
    sshcli_app::duplicate_profile(source_name, name)
}

#[tauri::command]
pub fn delete_profile(name: String) -> Result<(), String> {
    sshcli_app::delete_profile(name)
}

#[tauri::command]
pub fn ssh_trust_host_key(host: String, port: u16, key: String) -> Result<(), String> {
    sshcli_app::ssh_trust_host_key(host, port, key)
}

#[tauri::command]
pub fn save_profile_secret(name: String, secret: String) -> Result<(), String> {
    sshcli_app::save_profile_secret(name, secret)
}

#[tauri::command]
pub fn export_profiles() -> Result<String, String> {
    sshcli_app::export_profiles()
}

#[tauri::command]
pub fn toggle_favorite(name: String) -> Result<bool, String> {
    sshcli_app::toggle_favorite(name)
}

#[tauri::command]
pub fn touch_last_used(name: String) -> Result<(), String> {
    sshcli_app::touch_last_used(name)
}

#[tauri::command]
pub fn import_profiles(content: String) -> Result<usize, String> {
    sshcli_app::import_profiles(content)
}