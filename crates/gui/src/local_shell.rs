use std::{
    collections::HashMap,
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[cfg(windows)]
use std::ffi::OsString;

use base64::Engine;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use sshcli_core::shells;

pub struct LocalShellManager {
    shells: HashMap<String, LocalShell>,
    next_id: u64,
}

impl LocalShellManager {
    fn new() -> Self {
        Self {
            shells: HashMap::new(),
            next_id: 0,
        }
    }
}

struct LocalShell {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    killer: Arc<Mutex<Box<dyn ChildKiller + Send + Sync>>>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    startup: Arc<Mutex<StartupOutput>>,
}

struct StartupOutput {
    ready: bool,
    pending: Vec<Vec<u8>>,
}

pub type LocalShellState = Arc<Mutex<LocalShellManager>>;

pub fn init_state() -> LocalShellState {
    Arc::new(Mutex::new(LocalShellManager::new()))
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

fn local_shell_cwd() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(home) = std::env::var_os("USERPROFILE").filter(|home| !home.is_empty()) {
            return PathBuf::from(home);
        }
        if let (Some(drive), Some(path)) = (
            std::env::var_os("HOMEDRIVE"),
            std::env::var_os("HOMEPATH"),
        ) {
            let mut home = OsString::from(drive);
            home.push(path);
            return PathBuf::from(home);
        }
    }

    if let Some(home) = std::env::var_os("HOME").filter(|home| !home.is_empty()) {
        return PathBuf::from(home);
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn emit_data(app: &AppHandle, id: &str, data: &[u8]) {
    let _ = app.emit(
        "ssh-data",
        DataPayload {
            id: id.to_string(),
            data: base64::engine::general_purpose::STANDARD.encode(data),
        },
    );
}

#[tauri::command]
pub fn local_shell_detect() -> Result<serde_json::Value, String> {
    let detected = shells::detect_shell().ok_or("no se encontró ningún intérprete local")?;
    Ok(serde_json::json!({
        "detected": detected.display().to_string(),
        "available": shells::list_shells(&detected),
    }))
}

#[tauri::command]
pub fn local_shell_start(
    app: AppHandle,
    state: State<'_, LocalShellState>,
    columns: u16,
    rows: u16,
    shell: Option<String>,
) -> Result<serde_json::Value, String> {
    let shell_path = match shell.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(custom) => {
            let path = PathBuf::from(custom);
            if !path.is_file() {
                return Err(format!("intérprete no encontrado: {custom}"));
            }
            path
        }
        None => shells::detect_shell()
            .ok_or_else(|| "no se encontró ningún intérprete local".to_string())?,
    };
    let shell_str = shell_path.display().to_string();

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols: columns,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| error.to_string())?;

    let mut command = CommandBuilder::new(&shell_str);
    command.cwd(local_shell_cwd());
    command.env("TERM", "xterm-256color");

    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| error.to_string())?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| error.to_string())?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| error.to_string())?;
    let killer: Box<dyn ChildKiller + Send + Sync> =
        child.clone_killer();

    let id = {
        let mut manager = state.lock().map_err(|_| "local shell state poisoned")?;
        manager.next_id += 1;
        format!("local-{}", manager.next_id)
    };

    let profile = PathBuf::from(&shell_str)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| shell_str.clone());

    let startup = Arc::new(Mutex::new(StartupOutput {
        ready: false,
        pending: Vec::new(),
    }));
    state
        .lock()
        .map_err(|_| "local shell state poisoned")?
        .shells
        .insert(
            id.clone(),
            LocalShell {
                writer: Arc::new(Mutex::new(writer)),
                killer: Arc::new(Mutex::new(killer)),
                master: pair.master,
                startup: startup.clone(),
            },
        );

    let read_app = app.clone();
    let read_id = id.clone();
    let read_startup = startup;
    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let data = buffer[..n].to_vec();
                    let emit_now = {
                        let mut startup = match read_startup.lock() {
                            Ok(startup) => startup,
                            Err(_) => break,
                        };
                        if startup.ready {
                            true
                        } else {
                            startup.pending.push(data.clone());
                            false
                        }
                    };
                    if emit_now {
                        emit_data(&read_app, &read_id, &data);
                    }
                }
            }
        }
    });

    let manager = state.inner().clone();
    let wait_app = app.clone();
    let wait_id = id.clone();
    let wait_profile = profile.clone();
    std::thread::spawn(move || {
        let _ = child.wait();
        let _ = wait_app.emit(
            "ssh-status",
            StatusPayload {
                id: wait_id.clone(),
                profile: wait_profile,
                status: "closed".into(),
                message: "shell finalizado".into(),
            },
        );
        let _ = manager
            .lock()
            .map(|mut guard| guard.shells.remove(&wait_id));
    });

    Ok(serde_json::json!({ "id": id, "shell": shell_str, "profile": profile }))
}

#[tauri::command]
pub fn local_shell_ready(
    app: AppHandle,
    state: State<'_, LocalShellState>,
    id: String,
) -> Result<(), String> {
    let startup = state
        .lock()
        .map_err(|_| "local shell state poisoned")?
        .shells
        .get(&id)
        .map(|shell| shell.startup.clone())
        .ok_or_else(|| format!("unknown session: {id}"))?;
    let pending = {
        let mut startup = startup.lock().map_err(|_| "startup output poisoned")?;
        startup.ready = true;
        std::mem::take(&mut startup.pending)
    };
    for data in pending {
        emit_data(&app, &id, &data);
    }
    Ok(())
}

fn shell_for(state: &State<'_, LocalShellState>, id: &str) -> Result<Arc<Mutex<Box<dyn Write + Send>>>, String> {
    state
        .lock()
        .map_err(|_| "local shell state poisoned")?
        .shells
        .get(id)
        .map(|shell| shell.writer.clone())
        .ok_or_else(|| format!("unknown session: {id}"))
}

fn killer_for(
    state: &State<'_, LocalShellState>,
    id: &str,
) -> Result<Arc<Mutex<Box<dyn ChildKiller + Send + Sync>>>, String> {
    state
        .lock()
        .map_err(|_| "local shell state poisoned")?
        .shells
        .get(id)
        .map(|shell| shell.killer.clone())
        .ok_or_else(|| format!("unknown session: {id}"))
}

#[tauri::command]
pub fn local_write(state: State<'_, LocalShellState>, id: String, data: String) -> Result<(), String> {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &data)
        .map_err(|error| error.to_string())?;
    let writer = shell_for(&state, &id)?;
    let mut writer = writer.lock().map_err(|_| "writer poisoned")?;
    writer.write_all(&bytes).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn local_resize(
    state: State<'_, LocalShellState>,
    id: String,
    columns: u16,
    rows: u16,
) -> Result<(), String> {
    let mut manager = state.lock().map_err(|_| "local shell state poisoned")?;
    let shell = manager
        .shells
        .get_mut(&id)
        .ok_or_else(|| format!("unknown session: {id}"))?;
    shell
        .master
        .resize(PtySize {
            rows,
            cols: columns,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn local_close(state: State<'_, LocalShellState>, id: String) -> Result<(), String> {
    let killer = killer_for(&state, &id)?;
    let _ = killer.lock().map_err(|_| "killer poisoned")?.kill();
    state
        .lock()
        .map_err(|_| "local shell state poisoned")?
        .shells
        .remove(&id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn spawns_local_shell_in_pty() {
        let shell = shells::detect_shell().expect("debe detectar un shell local");
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("openpty");
        let path = shell.display().to_string();
        let command = CommandBuilder::new(&path);
        let mut child = pair.slave.spawn_command(command).expect("spawn del shell");
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().expect("reader");

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            let n = reader.read(&mut buffer).unwrap_or(0);
            let _ = tx.send(n);
        });
        let n = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap_or(0);
        assert!(n > 0, "el shell debería emitir prompt/banner al arrancar");
        let _ = child.kill();
    }
}
