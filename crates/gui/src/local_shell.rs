use std::{
    collections::HashMap,
    io::{Read, Write},
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
};

#[cfg(windows)]
use std::ffi::OsString;

use base64::Engine;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use sshcli_core::shells;

/// Events emitted by the native PTY boundary. Bytes are deliberately kept as
/// bytes: decoding, VT parsing, and painting belong to the terminal surface.
#[derive(Debug, PartialEq, Eq)]
pub enum LocalPtyEvent {
    Output(Vec<u8>),
    Error(String),
    Closed,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LocalPtyError {
    StatePoisoned,
    InvalidShell(String),
    Pty(String),
    UnknownSession(String),
    Writer(String),
    Killer(String),
}

impl std::fmt::Display for LocalPtyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StatePoisoned => formatter.write_str("local PTY state poisoned"),
            Self::InvalidShell(shell) => write!(formatter, "interpreter not found: {shell}"),
            Self::Pty(error) => formatter.write_str(error),
            Self::UnknownSession(id) => write!(formatter, "unknown session: {id}"),
            Self::Writer(error) => formatter.write_str(error),
            Self::Killer(error) => formatter.write_str(error),
        }
    }
}

impl std::error::Error for LocalPtyError {}

/// Native local-shell boundary for a future Slint adapter.
///
/// The receiver is intentionally separate from the handle so a UI event loop
/// can drain and coalesce output without putting PTY IO on the UI thread.
pub struct LocalPtySession {
    pub id: String,
    pub shell: String,
    pub profile: String,
    pub events: mpsc::Receiver<LocalPtyEvent>,
    handle: LocalPtyHandle,
}

impl LocalPtySession {
    pub fn ready(&self) -> Result<(), LocalPtyError> {
        self.handle.ready()
    }

    pub fn input(&self, data: &[u8]) -> Result<(), LocalPtyError> {
        self.handle.input(data)
    }

    pub fn resize(&self, columns: u16, rows: u16) -> Result<(), LocalPtyError> {
        self.handle.resize(columns, rows)
    }

    pub fn close(&self) -> Result<(), LocalPtyError> {
        self.handle.close()
    }
}

#[derive(Clone)]
pub struct LocalPtyHandle {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    killer: Arc<Mutex<Box<dyn ChildKiller + Send + Sync>>>,
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    startup: Arc<Mutex<StartupOutput>>,
}

/// Starts local PTYs without depending on Tauri's `AppHandle` or event bus.
pub struct NativeLocalPtyBoundary {
    next_id: u64,
}

impl NativeLocalPtyBoundary {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    pub fn start(
        &mut self,
        columns: u16,
        rows: u16,
        shell: Option<&str>,
    ) -> Result<LocalPtySession, LocalPtyError> {
        let shell_path = match shell.map(str::trim).filter(|shell| !shell.is_empty()) {
            Some(custom) => {
                let path = PathBuf::from(custom);
                if !path.is_file() {
                    return Err(LocalPtyError::InvalidShell(custom.to_string()));
                }
                path
            }
            None => shells::detect_shell().ok_or_else(|| {
                LocalPtyError::InvalidShell("no local interpreter detected".into())
            })?,
        };
        let shell_string = shell_path.display().to_string();
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols: columns,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| LocalPtyError::Pty(error.to_string()))?;
        let mut command = CommandBuilder::new(&shell_string);
        command.cwd(local_shell_cwd());
        command.env("TERM", "xterm-256color");
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| LocalPtyError::Pty(error.to_string()))?;
        drop(pair.slave);
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| LocalPtyError::Pty(error.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| LocalPtyError::Pty(error.to_string()))?;
        let (sender, events) = mpsc::channel();
        let startup = Arc::new(Mutex::new(StartupOutput {
            ready: false,
            pending: Vec::new(),
            sender: Some(sender.clone()),
        }));
        let handle = LocalPtyHandle {
            writer: Arc::new(Mutex::new(writer)),
            killer: Arc::new(Mutex::new(child.clone_killer())),
            master: Arc::new(Mutex::new(pair.master)),
            startup: startup.clone(),
        };

        let read_sender = sender.clone();
        std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(size) => {
                        let output = buffer[..size].to_vec();
                        let emit_now = match startup.lock() {
                            Ok(mut startup) if startup.ready => true,
                            Ok(mut startup) => {
                                startup.pending.push(output.clone());
                                false
                            }
                            Err(_) => {
                                let _ = read_sender.send(LocalPtyEvent::Error(
                                    "local PTY startup state poisoned".into(),
                                ));
                                break;
                            }
                        };
                        if emit_now && read_sender.send(LocalPtyEvent::Output(output)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = read_sender.send(LocalPtyEvent::Error(error.to_string()));
                        break;
                    }
                }
            }
        });
        let wait_sender = sender;
        std::thread::spawn(move || {
            let _ = child.wait();
            let _ = wait_sender.send(LocalPtyEvent::Closed);
        });

        self.next_id += 1;
        let id = format!("native-local-{}", self.next_id);
        let profile = shell_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| shell_string.clone());
        Ok(LocalPtySession {
            id,
            shell: shell_string,
            profile,
            events,
            handle,
        })
    }
}

impl Default for NativeLocalPtyBoundary {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalPtyHandle {
    pub fn ready(&self) -> Result<(), LocalPtyError> {
        let (pending, sender) = self
            .startup
            .lock()
            .map_err(|_| LocalPtyError::StatePoisoned)
            .map(|mut startup| {
                startup.ready = true;
                (std::mem::take(&mut startup.pending), startup.sender.clone())
            })?;
        if let Some(sender) = sender {
            for output in pending {
                let _ = sender.send(LocalPtyEvent::Output(output));
            }
        }
        Ok(())
    }

    pub fn input(&self, data: &[u8]) -> Result<(), LocalPtyError> {
        self.writer
            .lock()
            .map_err(|_| LocalPtyError::StatePoisoned)?
            .write_all(data)
            .map_err(|error| LocalPtyError::Writer(error.to_string()))
    }

    pub fn resize(&self, columns: u16, rows: u16) -> Result<(), LocalPtyError> {
        self.master
            .lock()
            .map_err(|_| LocalPtyError::StatePoisoned)?
            .resize(PtySize {
                rows,
                cols: columns,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| LocalPtyError::Pty(error.to_string()))
    }

    pub fn close(&self) -> Result<(), LocalPtyError> {
        self.killer
            .lock()
            .map_err(|_| LocalPtyError::StatePoisoned)?
            .kill()
            .map_err(|error| LocalPtyError::Killer(error.to_string()))
    }
}

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
    sender: Option<mpsc::Sender<LocalPtyEvent>>,
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
        if let (Some(drive), Some(path)) =
            (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH"))
        {
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
    let killer: Box<dyn ChildKiller + Send + Sync> = child.clone_killer();

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
        sender: None,
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

fn shell_for(
    state: &State<'_, LocalShellState>,
    id: &str,
) -> Result<Arc<Mutex<Box<dyn Write + Send>>>, String> {
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
pub fn local_write(
    state: State<'_, LocalShellState>,
    id: String,
    data: String,
) -> Result<(), String> {
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

    #[test]
    fn native_boundary_forwards_input_output_and_close() {
        let mut boundary = NativeLocalPtyBoundary::new();
        let session = boundary.start(80, 24, None).expect("start native PTY");
        session.ready().expect("ready");
        session.resize(100, 30).expect("resize");
        session
            .input(b"printf native-boundary; exit\n")
            .expect("input");

        let mut output = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let event = session
                .events
                .recv_timeout(remaining)
                .expect("native PTY event");
            match event {
                LocalPtyEvent::Output(bytes) => output.extend(bytes),
                LocalPtyEvent::Closed => break,
                LocalPtyEvent::Error(error) => panic!("native PTY error: {error}"),
            }
        }
        assert!(
            output
                .windows(b"native-boundary".len())
                .any(|window| window == b"native-boundary"),
            "PTY output should contain command output"
        );
    }

    #[test]
    fn native_boundary_rejects_missing_shell() {
        let mut boundary = NativeLocalPtyBoundary::new();
        let result = boundary.start(80, 24, Some("/definitely/not/a/shell"));
        assert!(matches!(
            result,
            Err(LocalPtyError::InvalidShell(ref shell)) if shell == "/definitely/not/a/shell"
        ));
    }
}
