use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

use sshcli_core::{credentials, ssh, Authentication, ProfileStore};

const SAMPLE_COMMAND: &str = "cat /proc/stat /proc/meminfo /proc/net/dev 2>/dev/null && df -Pk /";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySample {
    pub cpu_percent: f64,
    pub mem_used_mb: f64,
    pub mem_total_mb: f64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub rx_kbps: f64,
    pub tx_kbps: f64,
}

struct CpuTotals {
    idle: u64,
    total: u64,
}

struct ConnState {
    exec: ssh::ExecSession,
    prev_cpu: Option<CpuTotals>,
    prev_net: Option<(Instant, u64, u64)>,
}

impl ConnState {
    fn new(exec: ssh::ExecSession) -> Self {
        Self {
            exec,
            prev_cpu: None,
            prev_net: None,
        }
    }
}

#[derive(Default)]
pub struct TelemetryManager {
    conns: Mutex<HashMap<String, ConnState>>,
}

pub type TelemetryState = Arc<TelemetryManager>;

pub fn init_state() -> TelemetryState {
    Arc::new(TelemetryManager::default())
}

async fn connect_exec(profile_name: &str) -> Result<ssh::ExecSession, String> {
    let store = ProfileStore::new();
    let profile = store
        .load()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|profile| profile.name == profile_name)
        .ok_or_else(|| format!("profile not found: {profile_name}"))?;

    let secret = match &profile.authentication {
        Authentication::None => None,
        _ => credentials::get(profile_name).ok(),
    };
    let options = ssh::options_for_profile(&profile, secret).map_err(|error| error.to_string())?;
    ssh::ExecSession::connect(options)
        .await
        .map_err(|error| error.to_string())
}

fn parse_u64(field: &str) -> Option<u64> {
    field.parse().ok()
}

fn parse_sample(output: &str, state: &mut ConnState) -> Result<TelemetrySample, String> {
    let mut cpu: Option<(u64, u64)> = None;
    let mut mem_total_kb = 0u64;
    let mut mem_available_kb = 0u64;
    let mut rx_bytes = 0u64;
    let mut tx_bytes = 0u64;
    let mut disk_total_kb = 0u64;
    let mut disk_used_kb = 0u64;

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("cpu ") {
            let fields: Vec<u64> = rest
                .split_whitespace()
                .filter_map(parse_u64)
                .collect();
            if fields.len() >= 5 {
                let idle = fields[3] + fields[4];
                let total: u64 = fields.iter().sum();
                cpu = Some((idle, total));
            }
        } else if let Some(rest) = line.strip_prefix("MemTotal:") {
            mem_total_kb = rest.split_whitespace().next().and_then(parse_u64).unwrap_or(0);
        } else if line.starts_with("MemAvailable:") || line.starts_with("MemFree:") {
            if mem_available_kb == 0 {
                mem_available_kb = line
                    .split(':')
                    .nth(1)
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(parse_u64)
                    .unwrap_or(0);
            }
        } else if let Some((interface, stats)) = line.split_once(':') {
            let interface = interface.trim();
            if interface != "lo" && !interface.contains(' ') {
                let fields: Vec<u64> = stats.split_whitespace().filter_map(parse_u64).collect();
                if fields.len() >= 16 {
                    rx_bytes += fields[0];
                    tx_bytes += fields[8];
                }
            }
        } else {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 6 && parts[5] == "/" && parts[0] != "Filesystem" {
                disk_total_kb = parts[1].parse().unwrap_or(disk_total_kb);
                disk_used_kb = parts[2].parse().unwrap_or(disk_used_kb);
            }
        }
    }

    let (cpu_percent, next_cpu) = match (state.prev_cpu.as_ref(), cpu) {
        (Some(prev), Some((idle, total))) => {
            let d_total = total.saturating_sub(prev.total);
            let d_idle = idle.saturating_sub(prev.idle);
            let percent = if d_total > 0 {
                ((d_total - d_idle) as f64 / d_total as f64) * 100.0
            } else {
                0.0
            };
            (
                percent.clamp(0.0, 100.0),
                Some(CpuTotals { idle, total }),
            )
        }
        (_, Some((idle, total))) => (0.0, Some(CpuTotals { idle, total })),
        _ => (0.0, None),
    };

    let now = Instant::now();
    let ((rx_kbps, tx_kbps), next_net) = match state.prev_net {
        Some((start, prev_rx, prev_tx)) => {
            let elapsed = now.duration_since(start).as_secs_f64();
            let rate = |current: u64, previous: u64| {
                if elapsed > 0.0 {
                    current.saturating_sub(previous) as f64 / 1024.0 / elapsed
                } else {
                    0.0
                }
            };
            ((rate(rx_bytes, prev_rx), rate(tx_bytes, prev_tx)), Some((now, rx_bytes, tx_bytes)))
        }
        None => ((0.0, 0.0), Some((now, rx_bytes, tx_bytes))),
    };

    state.prev_cpu = next_cpu;
    state.prev_net = next_net;

    Ok(TelemetrySample {
        cpu_percent,
        mem_total_mb: mem_total_kb as f64 / 1024.0,
        mem_used_mb: mem_total_kb.saturating_sub(mem_available_kb) as f64 / 1024.0,
        disk_total_gb: disk_total_kb as f64 / 1024.0 / 1024.0,
        disk_used_gb: disk_used_kb as f64 / 1024.0 / 1024.0,
        rx_kbps,
        tx_kbps,
    })
}

async fn sample_once(
    state: &TelemetryState,
    profile_name: &str,
) -> Result<TelemetrySample, String> {
    let mut conns = state.conns.lock().await;
    let healthy = matches!(conns.get(profile_name), Some(conn) if !conn.exec.is_closed());
    if !healthy {
        conns.remove(profile_name);
        let mut conn = ConnState::new(connect_exec(profile_name).await?);
        let warmup = conn
            .exec
            .run(SAMPLE_COMMAND)
            .await
            .map_err(|error| error.to_string())?;
        let _ = parse_sample(&warmup, &mut conn);
        conns.insert(profile_name.to_string(), conn);
    }

    let conn = conns.get_mut(profile_name).expect("connection inserted");
    match conn.exec.run(SAMPLE_COMMAND).await {
        Ok(output) => parse_sample(&output, conn),
        Err(error) => {
            conns.remove(profile_name);
            Err(error.to_string())
        }
    }
}

async fn sample_with_manager(
    state: &TelemetryState,
    profile_name: &str,
) -> Result<TelemetrySample, String> {
    match sample_once(state, profile_name).await {
        Ok(sample) => Ok(sample),
        Err(_) => sample_once(state, profile_name)
            .await
            .map_err(|error| format!("telemetry unavailable: {error}")),
    }
}

#[tauri::command]
pub async fn telemetry_sample(
    state: State<'_, TelemetryState>,
    profile_name: String,
) -> Result<TelemetrySample, String> {
    sample_with_manager(&state, &profile_name).await
}

#[tauri::command]
pub async fn telemetry_disconnect(
    state: State<'_, TelemetryState>,
    profile_name: String,
) -> Result<(), String> {
    state.conns.lock().await.remove(&profile_name);
    Ok(())
}
