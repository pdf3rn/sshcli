//! Remote-shell session management.
//!
//! Owns the lifecycle of interactive SSH channels: connecting (from a profile
//! or ad-hoc target), writing, resizing, executing commands, listing and
//! closing sessions. Emits `ssh-data` and `ssh-status` events through the
//! host-provided [`crate::events::EventEmitter`].

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use russh::{client::Msg, ChannelMsg, ChannelWriteHalf};
use serde::Serialize;
use tokio::task::JoinHandle;

use sshcli_core::{credentials, ssh, ProfileStore};

use crate::events::{emit, SharedEmitter};

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

type ShellHandle = Arc<russh::client::Handle<sshcli_core::ssh::ClientHandler>>;

struct LiveSession {
    profile: String,
    writer: Arc<tokio::sync::Mutex<ChannelWriteHalf<Msg>>>,
    handle: ShellHandle,
    join: JoinHandle<()>,
}

pub type SessionState = Arc<Mutex<SessionManager>>;

pub fn init_state() -> SessionState {
    Arc::new(Mutex::new(SessionManager::new()))
}

#[derive(Serialize, Clone)]
pub struct DataPayload {
    pub id: String,
    pub data: String,
}

#[derive(Serialize, Clone)]
pub struct StatusPayload {
    pub id: String,
    pub profile: String,
    pub status: String,
    pub message: String,
}

pub async fn ssh_connect(
    emitter: &SharedEmitter,
    state: &SessionState,
    profile_name: String,
    password: Option<String>,
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

    let supplied_password = password.filter(|password| !password.is_empty());
    let secret = match &profile.authentication {
        sshcli_core::Authentication::None => None,
        _ => match supplied_password {
            Some(password) => Some(password),
            None => credentials::get_optional(&profile_name).map_err(|error| error.to_string())?,
        },
    };
    let options = match ssh::options_for_profile(&profile, secret) {
        Ok(options) => options,
        Err(_) if matches!(profile.authentication, sshcli_core::Authentication::Password) => {
            return Err(PASSWORD_REQUIRED.into())
        }
        Err(error) => return Err(error.to_string()),
    };
    let (channel, handle) = ssh::open_shell(options, columns, rows)
        .await
        .map_err(|error| map_ssh_error(error))?;

    let id = register_session(emitter, state, profile_name.clone(), channel, Arc::new(handle)).await?;

    let _ = ProfileStore::new().touch_last_used(&profile_name);

    Ok(id)
}

pub async fn ssh_connect_adhoc(
    emitter: &SharedEmitter,
    state: &SessionState,
    target: String,
    password: Option<String>,
    columns: u16,
    rows: u16,
) -> Result<String, String> {
    let (username, host, port) = parse_adhoc_target(&target)?;
    let display = format!("{username}@{host}");
    let has_password = password.is_some();

    let options = match ssh::options_adhoc(host, port, username, password) {
        Ok(options) => options,
        Err(_) if !has_password => return Err(PASSWORD_REQUIRED.into()),
        Err(error) => return Err(error.to_string()),
    };
    let channel = match ssh::open_shell(options, columns, rows).await {
        Ok((channel, handle)) => (channel, handle),
        Err(error)
            if !has_password
                && matches!(error, sshcli_core::error::AppError::AuthenticationFailed) =>
        {
            return Err(PASSWORD_REQUIRED.into());
        }
        Err(error) => return Err(map_ssh_error(error)),
    };

    register_session(emitter, state, display, channel.0, Arc::new(channel.1)).await
}

pub const PASSWORD_REQUIRED: &str = "sshcli:password-required";

pub const HOST_KEY_PREFIX: &str = "sshcli:host-key:";

pub fn map_ssh_error(error: sshcli_core::AppError) -> String {
    if let sshcli_core::AppError::HostKey { host, port, key, changed } = error {
        let payload = serde_json::json!({
            "host": host,
            "port": port,
            "key": key,
            "changed": changed,
        });
        return format!("{HOST_KEY_PREFIX}{}", payload);
    }
    error.to_string()
}

/// Parse an ad-hoc connection target of the form `user@host`, `user@host:port`
/// or `user@[host]:port` (the bracket form is accepted for IPv6 literals).
pub fn parse_adhoc_target(target: &str) -> Result<(String, String, u16), String> {
    const HINT: &str = "formato esperado usuario@host[:puerto]";
    let (username, hostport) = target
        .trim()
        .split_once('@')
        .ok_or_else(|| HINT.to_string())?;
    if username.is_empty() || hostport.is_empty() {
        return Err(HINT.into());
    }
    let (host, port) = if let Some(bracketed) = hostport.strip_prefix('[') {
        let (host, remainder) = bracketed
            .split_once(']')
            .ok_or_else(|| HINT.to_string())?;
        let port = match remainder {
            "" => 22,
            value => value
                .strip_prefix(':')
                .ok_or_else(|| HINT.to_string())?
                .parse::<u16>()
                .map_err(|_| format!("puerto inválido: {}", &value[1..]))?,
        };
        (host, port)
    } else {
        match hostport.rsplit_once(':') {
            Some((host, port)) => {
                let port = port
                    .parse::<u16>()
                    .map_err(|_| format!("puerto inválido: {port}"))?;
                (host, port)
            }
            None => (hostport, 22),
        }
    };
    if host.is_empty() {
        return Err(HINT.into());
    }
    Ok((username.to_string(), host.to_string(), port))
}

async fn register_session(
    emitter: &SharedEmitter,
    state: &SessionState,
    display_name: String,
    channel: russh::Channel<russh::client::Msg>,
    handle: ShellHandle,
) -> Result<String, String> {
    let (mut reader, writer) = channel.split();

    let id = {
        let mut manager = state.lock().map_err(|_| "session state poisoned")?;
        manager.next_id += 1;
        format!("{}-{}", display_name, manager.next_id)
    };

    let emit_emitter = emitter.clone();
    let emit_id = id.clone();
    let emit_profile = display_name.clone();
    let manager = state.clone();
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
                    emit(&emit_emitter, "ssh-data", &payload);
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }
        emit(
            &emit_emitter,
            "ssh-status",
            &StatusPayload {
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
                handle,
                join,
            },
        );
    }

    emit(
        emitter,
        "ssh-status",
        &StatusPayload {
            id: id.clone(),
            profile: display_name,
            status: "connected".into(),
            message: "connected".into(),
        },
    );

    Ok(id)
}

async fn writer_for(
    state: &SessionState,
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

fn handle_for(state: &SessionState, id: &str) -> Result<ShellHandle, String> {
    state
        .lock()
        .map_err(|_| "session state poisoned")?
        .sessions
        .get(id)
        .map(|session| session.handle.clone())
        .ok_or_else(|| format!("unknown session: {id}"))
}

pub async fn ssh_exec(state: &SessionState, id: String, command: String) -> Result<String, String> {
    let handle = handle_for(state, &id)?;
    ssh::collect_exec(&handle, &command)
        .await
        .map_err(|error| error.to_string())
}

pub async fn ssh_write(state: &SessionState, id: String, data: String) -> Result<(), String> {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &data)
        .map_err(|error| error.to_string())?;
    let writer = writer_for(state, &id).await?;
    let writer = writer.lock().await;
    writer
        .data(bytes.as_slice())
        .await
        .map_err(|error| error.to_string())
}

pub async fn ssh_resize(
    state: &SessionState,
    id: String,
    columns: u16,
    rows: u16,
) -> Result<(), String> {
    let writer = writer_for(state, &id).await?;
    let writer = writer.lock().await;
    writer
        .window_change(u32::from(columns), u32::from(rows), 0, 0)
        .await
        .map_err(|error| error.to_string())
}

pub fn ssh_list(state: &SessionState) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.lock().map_err(|_| "session state poisoned")?;
    Ok(manager
        .sessions
        .iter()
        .map(|(id, session)| serde_json::json!({ "id": id, "profile": session.profile }))
        .collect())
}

pub fn ssh_close(state: &SessionState, id: String) -> Result<(), String> {
    let mut manager = state.lock().map_err(|_| "session state poisoned")?;
    if let Some(session) = manager.sessions.remove(&id) {
        session.join.abort();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_user_host() {
        assert_eq!(
            parse_adhoc_target("alice@example.com").unwrap(),
            ("alice".to_string(), "example.com".to_string(), 22)
        );
    }

    #[test]
    fn parses_user_host_port() {
        assert_eq!(
            parse_adhoc_target("alice@example.com:2222").unwrap(),
            ("alice".to_string(), "example.com".to_string(), 2222)
        );
    }

    #[test]
    fn parses_bracketed_ipv6() {
        assert_eq!(
            parse_adhoc_target("alice@[::1]").unwrap(),
            ("alice".to_string(), "::1".to_string(), 22)
        );
    }

    #[test]
    fn parses_bracketed_ipv6_with_port() {
        assert_eq!(
            parse_adhoc_target("alice@[::1]:2222").unwrap(),
            ("alice".to_string(), "::1".to_string(), 2222)
        );
    }

    #[test]
    fn rejects_missing_at() {
        assert!(parse_adhoc_target("aliceexample.com").is_err());
    }

    #[test]
    fn rejects_empty_username() {
        assert!(parse_adhoc_target("@example.com").is_err());
    }

    #[test]
    fn rejects_empty_host() {
        assert!(parse_adhoc_target("alice@").is_err());
    }

    #[test]
    fn rejects_invalid_port() {
        assert!(parse_adhoc_target("alice@example.com:notaport").is_err());
    }

    #[test]
    fn rejects_unclosed_bracket() {
        assert!(parse_adhoc_target("alice@[::1").is_err());
    }
}