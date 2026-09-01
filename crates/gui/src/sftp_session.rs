use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use russh_sftp::client::SftpSession;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use std::sync::atomic::{AtomicU64, Ordering};

use sshcli_core::{credentials, sftp, ssh, ProfileStore};

pub struct SftpManager {
    sessions: HashMap<String, Arc<tokio::sync::Mutex<SftpSession>>>,
    next_id: u64,
}

impl SftpManager {
    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 0,
        }
    }
}

pub type SftpState = Arc<Mutex<SftpManager>>;

pub fn init_state() -> SftpState {
    Arc::new(Mutex::new(SftpManager::new()))
}

#[derive(Serialize, Clone)]
pub struct SftpEntry {
    pub name: String,
    pub kind: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    id: String,
    name: String,
    direction: &'static str,
    transferred: u64,
    total: u64,
}

fn session_id(manager: &SftpManager, profile: &str) -> String {
    format!("{}-{}", profile, manager.next_id)
}

fn fetch_session(
    state: &State<'_, SftpState>,
    id: &str,
) -> Result<Arc<tokio::sync::Mutex<SftpSession>>, String> {
    state
        .lock()
        .map_err(|_| "sftp state poisoned")?
        .sessions
        .get(id)
        .cloned()
        .ok_or_else(|| format!("unknown sftp session: {id}"))
}

#[tauri::command]
pub async fn sftp_connect(
    state: State<'_, SftpState>,
    profile_name: String,
    password: Option<String>,
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
            return Err(crate::session::PASSWORD_REQUIRED.into())
        }
        Err(error) => return Err(error.to_string()),
    };
    let session = ssh::open_sftp(options)
        .await
        .map_err(|error| error.to_string())?;

    let id = {
        let mut manager = state.lock().map_err(|_| "sftp state poisoned")?;
        manager.next_id += 1;
        session_id(&manager, &profile_name)
    };

    {
        let mut manager = state.lock().map_err(|_| "sftp state poisoned")?;
        manager
            .sessions
            .insert(id.clone(), Arc::new(tokio::sync::Mutex::new(session)));
    }

    Ok(id)
}

#[tauri::command]
pub async fn sftp_close(state: State<'_, SftpState>, id: String) -> Result<(), String> {
    let mut manager = state.lock().map_err(|_| "sftp state poisoned")?;
    manager.sessions.remove(&id);
    Ok(())
}

#[tauri::command]
pub async fn sftp_list_dir(
    state: State<'_, SftpState>,
    id: String,
    path: String,
) -> Result<Vec<SftpEntry>, String> {
    let session = fetch_session(&state, &id)?;
    let session = session.lock().await;
    sftp::list_dir(&session, &path)
        .await
        .map(|entries| {
            entries
                .into_iter()
                .map(|entry| {
                    let kind = format!("{:?}", entry.kind).to_lowercase();
                    let is_dir = kind == "dir";
                    SftpEntry {
                        name: entry.name,
                        kind,
                        is_dir,
                        size: entry.size,
                    }
                })
                .collect()
        })
        .map_err(|error| error.to_string())
}

fn progress_closure(
    app: &AppHandle,
    id: &str,
    name: String,
    direction: &'static str,
) -> impl Fn(u64, u64) + 'static {
    let app = app.clone();
    let id = id.to_string();
    let last = Arc::new(AtomicU64::new(0));
    move |transferred: u64, total: u64| {
        let previous = last.load(Ordering::Relaxed);
        let step = transferred.saturating_sub(previous);
        if transferred == 0 || transferred >= total || step >= 256 * 1024 {
            last.store(transferred, Ordering::Relaxed);
            let _ = app.emit(
                "sftp-progress",
                ProgressPayload {
                    id: id.clone(),
                    name: name.clone(),
                    direction,
                    transferred,
                    total,
                },
            );
        }
    }
}

#[tauri::command]
pub async fn sftp_pwd(state: State<'_, SftpState>, id: String) -> Result<String, String> {
    let session = fetch_session(&state, &id)?;
    let session = session.lock().await;
    sftp::canonicalize(&session, ".")
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sftp_download(
    app: AppHandle,
    state: State<'_, SftpState>,
    id: String,
    remote: String,
    local: String,
) -> Result<(), String> {
    let session = fetch_session(&state, &id)?;
    let session = session.lock().await;
    let name = remote.rsplit('/').next().unwrap_or(&remote).to_string();
    sftp::download(
        &session,
        &remote,
        &std::path::PathBuf::from(local),
        progress_closure(&app, &id, name, "download"),
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sftp_upload(
    app: AppHandle,
    state: State<'_, SftpState>,
    id: String,
    local: String,
    remote: String,
) -> Result<(), String> {
    let session = fetch_session(&state, &id)?;
    let session = session.lock().await;
    let name = local.rsplit('/').next().unwrap_or(&local).to_string();
    sftp::upload(
        &session,
        &std::path::PathBuf::from(local),
        &remote,
        progress_closure(&app, &id, name, "upload"),
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sftp_file_exists(
    state: State<'_, SftpState>,
    id: String,
    path: String,
) -> Result<bool, String> {
    let session = fetch_session(&state, &id)?;
    let session = session.lock().await;
    Ok(session.metadata(&path).await.is_ok())
}

#[tauri::command]
pub fn local_file_exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).is_file())
}

#[tauri::command]
pub async fn sftp_mkdir(
    state: State<'_, SftpState>,
    id: String,
    path: String,
) -> Result<(), String> {
    let session = fetch_session(&state, &id)?;
    let session = session.lock().await;
    sftp::create_dir(&session, &path)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sftp_rm_file(
    state: State<'_, SftpState>,
    id: String,
    path: String,
) -> Result<(), String> {
    let session = fetch_session(&state, &id)?;
    let session = session.lock().await;
    sftp::remove_file(&session, &path)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sftp_rm_dir(
    state: State<'_, SftpState>,
    id: String,
    path: String,
) -> Result<(), String> {
    let session = fetch_session(&state, &id)?;
    let session = session.lock().await;
    sftp::remove_dir(&session, &path)
        .await
        .map_err(|error| error.to_string())
}

#[derive(Serialize)]
pub struct LocalEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[tauri::command]
pub fn list_local_dir(path: String) -> Result<Vec<LocalEntry>, String> {
    let entries = std::fs::read_dir(path).map_err(|error| error.to_string())?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let metadata = entry.metadata().map_err(|error| error.to_string())?;
        result.push(LocalEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }
    result.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    Ok(result)
}

#[tauri::command]
pub fn local_home() -> Result<String, String> {
    sshcli_core::keys::expand_home("~")
        .to_str()
        .map(|path| path.to_string())
        .ok_or_else(|| "cannot resolve home directory".into())
}
