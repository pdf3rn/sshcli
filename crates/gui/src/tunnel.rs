use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::Serialize;
use tauri::State;

use sshcli_core::{credentials, ssh, ssh::LocalForward, ProfileStore};

pub struct TunnelManager {
    tunnels: HashMap<String, LiveTunnel>,
    next_id: u64,
}

impl TunnelManager {
    fn new() -> Self {
        Self {
            tunnels: HashMap::new(),
            next_id: 0,
        }
    }
}

struct LiveTunnel {
    profile: String,
    local: String,
    target: String,
    forward: Option<LocalForward>,
}

pub type TunnelState = Arc<Mutex<TunnelManager>>;

pub fn init_state() -> TunnelState {
    Arc::new(Mutex::new(TunnelManager::new()))
}

#[derive(Serialize)]
pub struct TunnelInfo {
    pub id: String,
    pub profile: String,
    pub local: String,
    pub target: String,
}

#[tauri::command]
pub async fn tunnel_start(
    state: State<'_, TunnelState>,
    profile_name: String,
    bind_host: String,
    bind_port: u16,
    target_host: String,
    target_port: u16,
) -> Result<TunnelInfo, String> {
    let store = ProfileStore::new();
    let profile = store
        .load()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|profile| profile.name == profile_name)
        .ok_or_else(|| format!("profile not found: {profile_name}"))?;

    let secret = match &profile.authentication {
        sshcli_core::Authentication::None => None,
        _ => credentials::get(&profile_name).ok(),
    };
    let options = ssh::options_for_profile(&profile, secret).map_err(|error| error.to_string())?;
    let forward = LocalForward::start(
        options,
        bind_host.clone(),
        bind_port,
        target_host.clone(),
        target_port,
    )
    .await
    .map_err(|error| error.to_string())?;

    let local = forward.bind_addr.to_string();
    let target = format!("{}:{}", forward.target_host, forward.target_port);

    let info = {
        let mut manager = state.lock().map_err(|_| "tunnel state poisoned")?;
        manager.next_id += 1;
        let id = format!("t{}-{}", profile_name, manager.next_id);
        let info = TunnelInfo {
            id: id.clone(),
            profile: profile_name.clone(),
            local: local.clone(),
            target: target.clone(),
        };
        manager.tunnels.insert(
            id,
            LiveTunnel {
                profile: profile_name,
                local,
                target,
                forward: Some(forward),
            },
        );
        info
    };

    Ok(info)
}

#[tauri::command]
pub fn tunnel_list(state: State<'_, TunnelState>) -> Result<Vec<TunnelInfo>, String> {
    let manager = state.lock().map_err(|_| "tunnel state poisoned")?;
    Ok(manager
        .tunnels
        .iter()
        .map(|(id, tunnel)| TunnelInfo {
            id: id.clone(),
            profile: tunnel.profile.clone(),
            local: tunnel.local.clone(),
            target: tunnel.target.clone(),
        })
        .collect())
}

#[tauri::command]
pub async fn tunnel_stop(state: State<'_, TunnelState>, id: String) -> Result<(), String> {
    let removed = {
        let mut manager = state.lock().map_err(|_| "tunnel state poisoned")?;
        manager.tunnels.remove(&id)
    };
    if let Some(tunnel) = removed {
        if let Some(forward) = tunnel.forward {
            forward.stop().await;
        }
    }
    Ok(())
}
