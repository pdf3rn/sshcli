//! Local PTY session, owned by `sshcli-native`.
//!
//! [`PtySession`] spawns a real shell through `portable-pty`, owns the read
//! loop, forwards bytes to the terminal renderer, writes user input back to
//! the PTY, propagates resizes as `PtySize`, and surfaces a `Closed` state (with
//! a way to restart) when the child exits.
//!
//! This is deliberately decoupled from `alacritty_terminal`'s own `tty`
//! module: the session is owned by `sshcli-native` so that later steps can
//! swap the local PTY for an SSH channel without changing the terminal widget.

use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};

use sshcli_core::shells;

/// The default terminal environment variables advertised to the shell.
pub const DEFAULT_TERM: &str = "xterm-256color";

/// Messages emitted by the session's read loop and delivered to the UI thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    /// A chunk of output bytes from the PTY, to be fed to the terminal parser.
    Output(Vec<u8>),
    /// The child process exited. The terminal is now closed.
    Closed,
    /// A spawn-level failure occurred while starting the child.
    SpawnFailed(String),
}

/// The current lifecycle state of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// The PTY is running and interactive.
    Running,
    /// The child process has exited. Call [`PtySession::restart`] to respawn.
    Closed,
}

/// Configuration for spawning a local shell.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Path to the shell binary. Detected automatically when `None`.
    pub shell: Option<PathBuf>,
    /// Working directory. Defaults to the user's home directory.
    pub working_directory: Option<PathBuf>,
    /// Initial terminal size in columns/rows.
    pub size: PtySize,
    /// The `TERM` value advertised to the shell.
    pub term: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            shell: None,
            working_directory: None,
            size: PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
            term: DEFAULT_TERM.to_string(),
        }
    }
}

/// Determine the user's home directory (portable across unix + windows).
pub fn home_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf())
}

/// Resolve the shell to spawn: the configured path, or auto-detected.
pub fn resolve_shell(config: &SessionConfig) -> Option<PathBuf> {
    config.shell.clone().or_else(shells::detect_shell)
}

/// A running (or closed) local PTY session.
///
/// The heavy lifting (blocking PTY I/O) happens on background threads; the
/// UI thread polls [`PtySession::try_recv`] for [`SessionEvent`]s each frame
/// and calls [`PtySession::write`] / [`PtySession::resize`] synchronously.
pub struct PtySession {
    config: SessionConfig,
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    killer: Option<Arc<Mutex<Box<dyn ChildKiller + Send + Sync>>>>,
    master: Option<Box<dyn MasterPty + Send>>,
    events: Receiver<SessionEvent>,
    state: SessionState,
}

impl PtySession {
    /// Spawn a new session using `config` and return it in the `Running` state.
    pub fn start(config: SessionConfig) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let mut session = Self {
            config,
            writer: None,
            killer: None,
            master: None,
            events: rx,
            state: SessionState::Closed,
        };
        session.spawn(&tx)?;
        session.state = SessionState::Running;
        Ok(session)
    }

    /// Spawn the child PTY and start the read loop; `tx` delivers events.
    fn spawn(&mut self, tx: &mpsc::Sender<SessionEvent>) -> Result<(), String> {
        let shell = resolve_shell(&self.config)
            .ok_or_else(|| "no se encontró ningún intérprete local".to_string())?;
        let shell_str = shell.display().to_string();

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(self.config.size)
            .map_err(|e| e.to_string())?;

        let mut command = CommandBuilder::new(&shell_str);
        if let Some(cwd) = self.config.working_directory.clone().or_else(home_dir) {
            command.cwd(cwd);
        }
        command.env("TERM", self.config.term.clone());

        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| e.to_string())?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let killer: Box<dyn ChildKiller + Send + Sync> = child.clone_killer();

        self.writer = Some(Arc::new(Mutex::new(writer)));
        self.killer = Some(Arc::new(Mutex::new(killer)));
        self.master = Some(pair.master);
        self.state = SessionState::Running;

        // Read loop: forward output bytes, signal closure on EOF/error.
        let read_tx = tx.clone();
        thread::Builder::new()
            .name("sshcli-native-pty-reader".into())
            .spawn(move || {
                let mut buffer = [0u8; 8192];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            if read_tx
                                .send(SessionEvent::Output(buffer[..n].to_vec()))
                                .is_err()
                            {
                                break;
                            }
                        }
                        // A transient EIO typically means the child exited.
                        Err(_) => break,
                    }
                }
                let _ = read_tx.send(SessionEvent::Closed);
            })
            .map_err(|e| e.to_string())?;

        // Wait loop: reap the child and mark the session closed.
        let wait_tx = tx.clone();
        thread::Builder::new()
            .name("sshcli-native-pty-waiter".into())
            .spawn(move || {
                let _ = child.wait();
                let _ = wait_tx.send(SessionEvent::Closed);
            })
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Poll for the next [`SessionEvent`], without blocking the UI.
    pub fn try_recv(&self) -> Option<SessionEvent> {
        self.events.try_recv().ok()
    }

    /// Wait up to `dur` for the next event (used by tests).
    pub fn wait_event(&self, dur: Duration) -> Option<SessionEvent> {
        match self.events.recv_timeout(dur) {
            Ok(event) => Some(event),
            Err(RecvTimeoutError::Timeout) => Some(SessionEvent::Output(Vec::new())),
            Err(RecvTimeoutError::Disconnected) => None,
        }
    }

    /// Write bytes to the PTY (user keyboard input).
    pub fn write(&self, bytes: &[u8]) -> Result<(), String> {
        let writer = self
            .writer
            .as_ref()
            .ok_or_else(|| "no active session".to_string())?;
        let mut writer = writer.lock().map_err(|_| "writer poisoned".to_string())?;
        writer.write_all(bytes).map_err(|e| e.to_string())
    }

    /// Propagate a terminal resize to the PTY.
    pub fn resize(&self, size: PtySize) -> Result<(), String> {
        let master = self
            .master
            .as_ref()
            .ok_or_else(|| "no active session".to_string())?;
        master.resize(size).map_err(|e| e.to_string())
    }

    /// Kill the current child and respawn it with the same configuration.
    ///
    /// This is the "reconnect/restart" action used when the session is closed.
    pub fn restart(&mut self) -> Result<(), String> {
        self.kill();
        let (tx, rx) = mpsc::channel();
        self.events = rx;
        self.writer = None;
        self.killer = None;
        self.master = None;
        self.spawn(&tx)?;
        self.state = SessionState::Running;
        Ok(())
    }

    /// Terminate the child (if any) and free PTY handles.
    pub fn kill(&mut self) {
        if let Some(killer) = self.killer.take() {
            if let Ok(mut killer) = killer.lock() {
                let _ = killer.kill();
            }
        }
        self.writer = None;
        self.master = None;
        self.state = SessionState::Closed;
    }

    /// The current lifecycle state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// The configuration this session was created with.
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// Consume all pending events, applying a state transition for `Closed`.
    pub fn drain(&mut self, mut on_output: impl FnMut(&[u8]), on_closed: impl FnOnce()) {
        let mut closed = false;
        while let Some(event) = self.try_recv() {
            match event {
                SessionEvent::Output(bytes) => on_output(&bytes),
                SessionEvent::Closed => {
                    if !closed {
                        closed = true;
                    }
                }
                SessionEvent::SpawnFailed(_) => {}
            }
        }
        if closed {
            self.state = SessionState::Closed;
            on_closed();
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirror of `crates/app/src/local_shell.rs::spawns_local_shell_in_pty`:
    /// spawn a real shell and assert it emits output at startup.
    #[test]
    fn spawns_local_shell_in_pty_roundtrip() {
        let config = SessionConfig {
            shell: None,
            working_directory: None,
            size: PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
            term: DEFAULT_TERM.to_string(),
        };
        let session = PtySession::start(config).expect("debe iniciar una sesión local");
        assert_eq!(session.state(), SessionState::Running);

        // Accumulate output until the first non-empty chunk or timeout.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut total = 0usize;
        while std::time::Instant::now() < deadline {
            match session.wait_event(Duration::from_secs(1)) {
                Some(SessionEvent::Output(bytes)) => {
                    total += bytes.len();
                    if total > 0 {
                        break;
                    }
                }
                Some(SessionEvent::Closed) => panic!("el shell se cerró antes de emitir output"),
                _ => continue,
            }
        }
        assert!(
            total > 0,
            "el shell debería emitir prompt/banner al arrancar"
        );
    }

    /// Writing should round-trip: send a command and observe echoed output.
    #[test]
    fn write_reaches_the_shell() {
        let config = SessionConfig {
            shell: None,
            working_directory: None,
            size: PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
            term: DEFAULT_TERM.to_string(),
        };
        let session = PtySession::start(config).expect("debe iniciar una sesión local");

        // Drain startup output first.
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if let Some(SessionEvent::Output(_)) = session.wait_event(Duration::from_millis(200)) {
                // keep draining
            } else {
                break;
            }
        }

        // Send an echo command and expect the distinctive marker to come back.
        session.write(b"echo SSHCLI_NATIVE_PTY_OK\r\n").unwrap();
        let marker = b"SSHCLI_NATIVE_PTY_OK";
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut saw_output = false;
        while std::time::Instant::now() < deadline {
            match session.wait_event(Duration::from_secs(1)) {
                Some(SessionEvent::Output(bytes)) => {
                    if bytes.windows(marker.len()).any(|w| w == marker) {
                        saw_output = true;
                        break;
                    }
                }
                Some(SessionEvent::Closed) => break,
                _ => continue,
            }
        }
        assert!(
            saw_output,
            "el shell debería hacer eco del comando con el marker"
        );
    }

    #[test]
    fn restart_after_close_respawns_a_running_session() {
        let config = SessionConfig {
            shell: None,
            working_directory: None,
            size: PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
            term: DEFAULT_TERM.to_string(),
        };
        let mut session = PtySession::start(config).expect("start");

        // Terminate it explicitly.
        session.kill();
        assert_eq!(session.state(), SessionState::Closed);

        // Restart should bring it back to Running and emit output again.
        session.restart().expect("restart");
        assert_eq!(session.state(), SessionState::Running);

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut total = 0usize;
        while std::time::Instant::now() < deadline {
            match session.wait_event(Duration::from_secs(1)) {
                Some(SessionEvent::Output(bytes)) => {
                    total += bytes.len();
                    if total > 0 {
                        break;
                    }
                }
                Some(SessionEvent::Closed) => break,
                _ => continue,
            }
        }
        assert!(total > 0, "la sesión reiniciada debería emitir output");
    }
}
