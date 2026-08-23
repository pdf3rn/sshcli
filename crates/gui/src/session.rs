use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use russh::{client::Msg, ChannelMsg, ChannelWriteHalf};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::task::JoinHandle;

use sshcli_core::{credentials, ssh, ProfileStore};

pub struct SessionManager {
    sessions: HashMap<String, LiveSession>,
    next_id: u64,
}

impl SessionManager {
    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 0,
        }
    }
}

struct LiveSession {
    profile: String,
    writer: Arc<tokio::sync::Mutex<ChannelWriteHalf<Msg>>>,
    join: JoinHandle<()>,
}

pub type SessionState = Arc<Mutex<SessionManager>>;

pub fn init_state() -> SessionState {
    Arc::new(Mutex::new(SessionManager::new()))
}

#[derive(Serialize, Clone)]
struct DataPayload {
    id: String,
    data: String,
}

#[derive(Serialize, Clone)]
struct StatusPayload {
    id: String,
    profile: String,
    status: String,
    message: String,
}

#[tauri::command]
pub async fn ssh_connect(
    app: AppHandle,
    state: State<'_, SessionState>,
    profile_name: String,
    columns: u16,
    rows: u16,
) -> Result<String, String> {
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
    let channel = ssh::open_shell(options, columns, rows)
        .await
        .map_err(|error| error.to_string())?;

    let id = register_session(&app, &state, profile_name.clone(), channel).await?;

    let _ = ProfileStore::new().touch_last_used(&profile_name);

    Ok(id)
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
    let (username, host, port) = parse_adhoc_target(&target)?;
    let display = format!("{username}@{host}");
    let options =
        ssh::options_adhoc(host, port, username, password).map_err(|error| error.to_string())?;
    let channel = ssh::open_shell(options, columns, rows)
        .await
        .map_err(|error| error.to_string())?;

    register_session(&app, &state, display, channel).await
}

fn parse_adhoc_target(target: &str) -> Result<(String, String, u16), String> {
    const HINT: &str = "formato esperado usuario@host[:puerto]";
    let (username, hostport) = target
        .trim()
        .split_once('@')
        .ok_or_else(|| HINT.to_string())?;
    if username.is_empty() || hostport.is_empty() {
        return Err(HINT.into());
    }
    let (host, port) = match hostport.rsplit_once(':') {
        Some((host, port)) => {
            let port = port
                .parse::<u16>()
                .map_err(|_| format!("puerto inválido: {port}"))?;
            (host, port)
        }
        None => (hostport, 22),
    };
    if host.is_empty() {
        return Err(HINT.into());
    }
    Ok((username.to_string(), host.to_string(), port))
}

async fn register_session(
    app: &AppHandle,
    state: &State<'_, SessionState>,
    display_name: String,
    channel: russh::Channel<russh::client::Msg>,
) -> Result<String, String> {
    let (mut reader, writer) = channel.split();

    let id = {
        let mut manager = state.lock().map_err(|_| "session state poisoned")?;
        manager.next_id += 1;
        format!("{}-{}", display_name, manager.next_id)
    };

    let emit_app = app.clone();
    let emit_id = id.clone();
    let emit_profile = display_name.clone();
    let manager = state.inner().clone();
    let join = tokio::spawn(async move {
        while let Some(message) = reader.wait().await {
            match message {
                ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                    let payload = DataPayload {
                        id: emit_id.clone(),
                        data: base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &data,
                        ),
                    };
                    let _ = emit_app.emit("ssh-data", payload);
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }
        let _ = emit_app.emit(
            "ssh-status",
            StatusPayload {
                id: emit_id.clone(),
                profile: emit_profile.clone(),
                status: "closed".into(),
                message: "connection closed".into(),
            },
        );
        let _ = manager
            .lock()
            .map(|mut guard| guard.sessions.remove(&emit_id));
    });

    {
        let mut manager = state.lock().map_err(|_| "session state poisoned")?;
        manager.sessions.insert(
            id.clone(),
            LiveSession {
                profile: display_name.clone(),
                writer: Arc::new(tokio::sync::Mutex::new(writer)),
                join,
            },
        );
    }

    let _ = app.emit(
        "ssh-status",
        StatusPayload {
            id: id.clone(),
            profile: display_name,
            status: "connected".into(),
            message: "connected".into(),
        },
    );

    Ok(id)
}

async fn writer_for(
    state: &State<'_, SessionState>,
    id: &str,
) -> Result<Arc<tokio::sync::Mutex<ChannelWriteHalf<Msg>>>, String> {
    state
        .lock()
        .map_err(|_| "session state poisoned")?
        .sessions
        .get(id)
        .map(|session| session.writer.clone())
        .ok_or_else(|| format!("unknown session: {id}"))
}

#[tauri::command]
pub async fn ssh_write(
    state: State<'_, SessionState>,
    id: String,
    data: String,
) -> Result<(), String> {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &data)
        .map_err(|error| error.to_string())?;
    let writer = writer_for(&state, &id).await?;
    let writer = writer.lock().await;
    writer
        .data(bytes.as_slice())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn ssh_resize(
    state: State<'_, SessionState>,
    id: String,
    columns: u16,
    rows: u16,
) -> Result<(), String> {
    let writer = writer_for(&state, &id).await?;
    let writer = writer.lock().await;
    writer
        .window_change(u32::from(columns), u32::from(rows), 0, 0)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn ssh_list(state: State<'_, SessionState>) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.lock().map_err(|_| "session state poisoned")?;
    Ok(manager
        .sessions
        .iter()
        .map(|(id, session)| serde_json::json!({ "id": id, "profile": session.profile }))
        .collect())
}

#[tauri::command]
pub fn ssh_close(state: State<'_, SessionState>, id: String) -> Result<(), String> {
    let mut manager = state.lock().map_err(|_| "session state poisoned")?;
    if let Some(session) = manager.sessions.remove(&id) {
        session.join.abort();
    }
    Ok(())
}
