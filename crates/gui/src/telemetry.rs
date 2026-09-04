use tauri::State;

use sshcli_app::telemetry::{self, TelemetrySample, TelemetryState};

#[tauri::command]
pub async fn telemetry_sample(
    state: State<'_, TelemetryState>,
    profile_name: String,
    password: Option<String>,
) -> Result<TelemetrySample, String> {
    telemetry::telemetry_sample(&state, profile_name, password).await
}

#[tauri::command]
pub async fn telemetry_disconnect(
    state: State<'_, TelemetryState>,
    profile_name: String,
) -> Result<(), String> {
    telemetry::telemetry_disconnect(&state, profile_name).await
}