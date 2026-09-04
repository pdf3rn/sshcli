use std::sync::Arc;

use tauri::{AppHandle, State};

use sshcli_app::events::SharedEmitter;
use sshcli_app::session::{self, SessionState};

use crate::events::TauriEmitter;

fn emitter(app: &AppHandle) -> SharedEmitter {
    Arc::new(TauriEmitter(app.clone()))
}

#[tauri::command]
pub async fn ssh_connect(
    app: AppHandle,
    state: State<'_, SessionState>,
    profile_name: String,
    password: Option<String>,
    columns: u16,
    rows: u16,
) -> Result<String, String> {
    session::ssh_connect(&emitter(&app), &state, profile_name, password, columns, rows).await
}

#[tauri::command]
pub async fn ssh_connect_adhoc(
    app: AppHandle,
    state: State<'_, SessionState>,
    target: String,
    password: Option<String>,
    columns: u16,
    rows: u16,
) -> Result<String, String> {
    session::ssh_connect_adhoc(&emitter(&app), &state, target, password, columns, rows).await
}

#[tauri::command]
pub async fn ssh_exec(
    state: State<'_, SessionState>,
    id: String,
    command: String,
) -> Result<String, String> {
    session::ssh_exec(&state, id, command).await
}

#[tauri::command]
pub async fn ssh_write(
    state: State<'_, SessionState>,
    id: String,
    data: String,
) -> Result<(), String> {
    session::ssh_write(&state, id, data).await
}

#[tauri::command]
pub async fn ssh_resize(
    state: State<'_, SessionState>,
    id: String,
    columns: u16,
    rows: u16,
) -> Result<(), String> {
    session::ssh_resize(&state, id, columns, rows).await
}

#[tauri::command]
pub fn ssh_list(state: State<'_, SessionState>) -> Result<Vec<serde_json::Value>, String> {
    session::ssh_list(&state)
}

#[tauri::command]
pub fn ssh_close(state: State<'_, SessionState>, id: String) -> Result<(), String> {
    session::ssh_close(&state, id)
}