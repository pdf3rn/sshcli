use tauri::State;

use sshcli_app::tunnel::{self, TunnelInfo, TunnelState};

#[tauri::command]
pub async fn tunnel_start(
    state: State<'_, TunnelState>,
    profile_name: String,
    password: Option<String>,
    bind_host: String,
    bind_port: u16,
    target_host: String,
    target_port: u16,
) -> Result<TunnelInfo, String> {
    tunnel::tunnel_start(
        &state,
        profile_name,
        password,
        bind_host,
        bind_port,
        target_host,
        target_port,
    )
    .await
}

#[tauri::command]
pub fn tunnel_list(state: State<'_, TunnelState>) -> Result<Vec<TunnelInfo>, String> {
    tunnel::tunnel_list(&state)
}

#[tauri::command]
pub async fn tunnel_stop(state: State<'_, TunnelState>, id: String) -> Result<(), String> {
    tunnel::tunnel_stop(&state, id).await
}