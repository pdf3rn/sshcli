use std::sync::Arc;

use tauri::{AppHandle, State};

use sshcli_app::events::SharedEmitter;
use sshcli_app::local_shell::{self, LocalShellState};

use crate::events::TauriEmitter;

fn emitter(app: &AppHandle) -> SharedEmitter {
    Arc::new(TauriEmitter(app.clone()))
}

#[tauri::command]
pub fn local_shell_detect() -> Result<serde_json::Value, String> {
    local_shell::local_shell_detect()
}

#[tauri::command]
pub fn local_shell_start(
    app: AppHandle,
    state: State<'_, LocalShellState>,
    columns: u16,
    rows: u16,
    shell: Option<String>,
) -> Result<serde_json::Value, String> {
    local_shell::local_shell_start(&emitter(&app), &state, columns, rows, shell)
}

#[tauri::command]
pub fn local_shell_ready(
    app: AppHandle,
    state: State<'_, LocalShellState>,
    id: String,
) -> Result<(), String> {
    local_shell::local_shell_ready(&emitter(&app), &state, id)
}

#[tauri::command]
pub fn local_write(state: State<'_, LocalShellState>, id: String, data: String) -> Result<(), String> {
    local_shell::local_write(&state, id, data)
}

#[tauri::command]
pub fn local_resize(
    state: State<'_, LocalShellState>,
    id: String,
    columns: u16,
    rows: u16,
) -> Result<(), String> {
    local_shell::local_resize(&state, id, columns, rows)
}

#[tauri::command]
pub fn local_close(state: State<'_, LocalShellState>, id: String) -> Result<(), String> {
    local_shell::local_close(&state, id)
}