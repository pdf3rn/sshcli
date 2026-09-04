use std::sync::Arc;

use tauri::{AppHandle, State};

use sshcli_app::events::SharedEmitter;
use sshcli_app::sftp_session::{self, LocalEntry, SftpEntry, SftpState};

use crate::events::TauriEmitter;

fn emitter(app: &AppHandle) -> SharedEmitter {
    Arc::new(TauriEmitter(app.clone()))
}

#[tauri::command]
pub async fn sftp_connect(
    state: State<'_, SftpState>,
    profile_name: String,
    password: Option<String>,
) -> Result<String, String> {
    sftp_session::sftp_connect(&state, profile_name, password).await
}

#[tauri::command]
pub async fn sftp_close(state: State<'_, SftpState>, id: String) -> Result<(), String> {
    sftp_session::sftp_close(&state, id).await
}

#[tauri::command]
pub async fn sftp_list_dir(
    state: State<'_, SftpState>,
    id: String,
    path: String,
) -> Result<Vec<SftpEntry>, String> {
    sftp_session::sftp_list_dir(&state, id, path).await
}

#[tauri::command]
pub async fn sftp_pwd(state: State<'_, SftpState>, id: String) -> Result<String, String> {
    sftp_session::sftp_pwd(&state, id).await
}

#[tauri::command]
pub async fn sftp_download(
    app: AppHandle,
    state: State<'_, SftpState>,
    id: String,
    remote: String,
    local: String,
) -> Result<(), String> {
    sftp_session::sftp_download(&emitter(&app), &state, id, remote, local).await
}

#[tauri::command]
pub async fn sftp_upload(
    app: AppHandle,
    state: State<'_, SftpState>,
    id: String,
    local: String,
    remote: String,
) -> Result<(), String> {
    sftp_session::sftp_upload(&emitter(&app), &state, id, local, remote).await
}

#[tauri::command]
pub async fn sftp_file_exists(
    state: State<'_, SftpState>,
    id: String,
    path: String,
) -> Result<bool, String> {
    sftp_session::sftp_file_exists(&state, id, path).await
}

#[tauri::command]
pub fn local_file_exists(path: String) -> Result<bool, String> {
    sftp_session::local_file_exists(path)
}

#[tauri::command]
pub async fn sftp_mkdir(state: State<'_, SftpState>, id: String, path: String) -> Result<(), String> {
    sftp_session::sftp_mkdir(&state, id, path).await
}

#[tauri::command]
pub async fn sftp_rename(
    state: State<'_, SftpState>,
    id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    sftp_session::sftp_rename(&state, id, old_path, new_path).await
}

#[tauri::command]
pub async fn sftp_rm_file(state: State<'_, SftpState>, id: String, path: String) -> Result<(), String> {
    sftp_session::sftp_rm_file(&state, id, path).await
}

#[tauri::command]
pub async fn sftp_rm_dir(state: State<'_, SftpState>, id: String, path: String) -> Result<(), String> {
    sftp_session::sftp_rm_dir(&state, id, path).await
}

#[tauri::command]
pub fn list_local_dir(path: String) -> Result<Vec<LocalEntry>, String> {
    sftp_session::list_local_dir(path)
}

#[tauri::command]
pub fn local_home() -> Result<String, String> {
    sftp_session::local_home()
}