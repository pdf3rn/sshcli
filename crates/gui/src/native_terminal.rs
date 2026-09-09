use std::{cell::RefCell, sync::mpsc, thread, time::Duration};

use slint::{ComponentHandle, Timer, TimerMode};
use sshcli_core::ssh::ConnectionOptions;
use sshcli_terminal_ui::{TerminalCommand, TerminalController, TerminalEvent, TerminalSurface};

use crate::local_shell::{LocalPtyEvent, NativeLocalPtyBoundary};

/// Slint-facing local PTY adapter. It is opt-in and does not alter the Tauri
/// builder, so the legacy web UI and native surface can coexist during the
/// migration.
pub struct LocalTerminalSurface {
    pub surface: TerminalSurface,
    controller: std::rc::Rc<RefCell<TerminalController>>,
    _timer: Timer,
}

impl LocalTerminalSurface {
    /// Creates a surface and starts the PTY worker without doing PTY IO on the
    /// UI thread. The worker calls `ready` before releasing buffered output.
    pub fn start(
        columns: u16,
        rows: u16,
        shell: Option<String>,
    ) -> Result<Self, slint::PlatformError> {
        let (event_tx, event_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        thread::spawn(move || run_local_pty(columns, rows, shell.as_deref(), event_tx, command_rx));

        let surface = TerminalSurface::new()?;
        let controller = std::rc::Rc::new(RefCell::new(TerminalController::new(
            columns as usize,
            rows as usize,
            event_rx,
            command_tx,
        )));
        let weak = std::rc::Rc::downgrade(&controller);
        surface.on_input(move |input| {
            if let Some(controller) = weak.upgrade() {
                let _ = controller.borrow().input(input.as_str());
            }
        });
        let weak = std::rc::Rc::downgrade(&controller);
        surface.on_resize(move |columns, rows| {
            if let Some(controller) = weak.upgrade() {
                let _ = controller.borrow_mut().resize(columns as u16, rows as u16);
            }
        });
        let weak = std::rc::Rc::downgrade(&controller);
        surface.on_close(move || {
            if let Some(controller) = weak.upgrade() {
                let _ = controller.borrow().close();
            }
        });

        let weak = std::rc::Rc::downgrade(&controller);
        let surface_weak = surface.as_weak();
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
            let (Some(controller), Some(surface)) = (weak.upgrade(), surface_weak.upgrade()) else {
                return;
            };
            let mut controller = controller.borrow_mut();
            if controller.pump() {
                controller.refresh_surface(&surface);
            }
        });
        Ok(Self {
            surface,
            controller,
            _timer: timer,
        })
    }

    pub fn close(&self) {
        let _ = self.controller.borrow().close();
    }
}

/// Native-only launch path. The Tauri `run` entry point remains unchanged and
/// continues to be the behavioral reference during migration.
pub fn run_native() -> Result<(), Box<dyn std::error::Error>> {
    let terminal = LocalTerminalSurface::start(80, 24, None)?;
    terminal.surface.run()?;
    Ok(())
}

/// Native launch-harness entry point for an explicitly supplied SSH target.
/// The default harness above remains local and credential-free.
pub fn run_native_ssh(options: ConnectionOptions) -> Result<(), Box<dyn std::error::Error>> {
    let terminal = SshTerminalSurface::start(options, 80, 24)?;
    terminal.surface.run()?;
    Ok(())
}

/// Slint-facing SSH terminal adapter. The SSH transport stays on its worker
/// thread; the verified terminal-ui controller remains responsible for VT
/// parsing, batching, and painting. This is deliberately separate from the
/// Tauri session manager so both frontends can coexist.
pub struct SshTerminalSurface {
    pub surface: TerminalSurface,
    controller: std::rc::Rc<RefCell<TerminalController>>,
    reconnect: mpsc::Sender<ConnectionOptions>,
    _timer: Timer,
}

impl SshTerminalSurface {
    pub fn start(
        options: ConnectionOptions,
        columns: u16,
        rows: u16,
    ) -> Result<Self, slint::PlatformError> {
        let (event_tx, event_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let (reconnect_tx, reconnect_rx) = mpsc::channel();
        thread::spawn(move || {
            run_ssh_worker(options, columns, rows, event_tx, command_rx, reconnect_rx)
        });

        let surface = TerminalSurface::new()?;
        let controller = std::rc::Rc::new(RefCell::new(TerminalController::new(
            columns as usize,
            rows as usize,
            event_rx,
            command_tx,
        )));
        let weak = std::rc::Rc::downgrade(&controller);
        surface.on_input(move |input| {
            if let Some(controller) = weak.upgrade() {
                let _ = controller.borrow().input(input.as_str());
            }
        });
        let weak = std::rc::Rc::downgrade(&controller);
        surface.on_resize(move |columns, rows| {
            if let Some(controller) = weak.upgrade() {
                let _ = controller.borrow_mut().resize(columns as u16, rows as u16);
            }
        });
        let weak = std::rc::Rc::downgrade(&controller);
        surface.on_close(move || {
            if let Some(controller) = weak.upgrade() {
                let _ = controller.borrow().close();
            }
        });

        let weak = std::rc::Rc::downgrade(&controller);
        let surface_weak = surface.as_weak();
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
            let (Some(controller), Some(surface)) = (weak.upgrade(), surface_weak.upgrade()) else {
                return;
            };
            let mut controller = controller.borrow_mut();
            if controller.pump() {
                controller.refresh_surface(&surface);
            }
        });
        Ok(Self {
            surface,
            controller,
            reconnect: reconnect_tx,
            _timer: timer,
        })
    }

    /// Requests a fresh SSH connection without blocking the UI thread.
    pub fn reconnect(
        &self,
        options: ConnectionOptions,
    ) -> Result<(), mpsc::SendError<ConnectionOptions>> {
        self.reconnect.send(options)
    }

    pub fn close(&self) {
        let _ = self.controller.borrow().close();
    }
}

type SshWriter = std::sync::Arc<tokio::sync::Mutex<russh::ChannelWriteHalf<russh::client::Msg>>>;
type SshHandle = russh::client::Handle<sshcli_core::ssh::ClientHandler>;

fn run_ssh_worker(
    options: ConnectionOptions,
    columns: u16,
    rows: u16,
    event_tx: mpsc::Sender<TerminalEvent>,
    command_rx: mpsc::Receiver<TerminalCommand>,
    reconnect_rx: mpsc::Receiver<ConnectionOptions>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
            return;
        }
    };
    let _ = event_tx.send(TerminalEvent::Connecting);
    let mut connection = match runtime.block_on(connect_ssh(options, columns, rows, &event_tx)) {
        Some(connection) => Some(connection),
        None => None,
    };

    loop {
        if let Ok(options) = reconnect_rx.try_recv() {
            if let Some(old) = connection.take() {
                old.reader.abort();
                let _ = runtime.block_on(old.handle.disconnect(
                    russh::Disconnect::ByApplication,
                    "reconnecting",
                    "",
                ));
            }
            let _ = event_tx.send(TerminalEvent::Connecting);
            connection = runtime.block_on(connect_ssh(options, columns, rows, &event_tx));
        }

        match command_rx.recv_timeout(Duration::from_millis(16)) {
            Ok(TerminalCommand::Input(bytes)) => {
                if let Some(current) = connection.as_ref() {
                    if let Err(error) = runtime.block_on(async {
                        current.writer.lock().await.data(bytes.as_slice()).await
                    }) {
                        let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
                    }
                }
            }
            Ok(TerminalCommand::Resize(columns, rows)) => {
                if let Some(current) = connection.as_ref() {
                    if let Err(error) = runtime.block_on(async {
                        current
                            .writer
                            .lock()
                            .await
                            .window_change(u32::from(columns), u32::from(rows), 0, 0)
                            .await
                    }) {
                        let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
                    }
                }
            }
            Ok(TerminalCommand::Close) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(current) = connection.take() {
                    current.reader.abort();
                    let _ = runtime.block_on(current.handle.disconnect(
                        russh::Disconnect::ByApplication,
                        "closed",
                        "",
                    ));
                }
                let _ = event_tx.send(TerminalEvent::Closed);
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

struct SshConnection {
    writer: SshWriter,
    handle: SshHandle,
    reader: tokio::task::JoinHandle<()>,
}

async fn connect_ssh(
    options: ConnectionOptions,
    columns: u16,
    rows: u16,
    event_tx: &mpsc::Sender<TerminalEvent>,
) -> Option<SshConnection> {
    let (channel, handle) = match sshcli_core::ssh::open_shell(options, columns, rows).await {
        Ok(connection) => connection,
        Err(error) => {
            let _ = event_tx.send(TerminalEvent::Error(crate::session::map_ssh_error(error)));
            return None;
        }
    };
    let (mut reader, writer) = channel.split();
    let writer = std::sync::Arc::new(tokio::sync::Mutex::new(writer));
    let output_tx = event_tx.clone();
    let reader = tokio::spawn(async move {
        while let Some(message) = reader.wait().await {
            match message {
                russh::ChannelMsg::Data { data } | russh::ChannelMsg::ExtendedData { data, .. } => {
                    if output_tx
                        .send(TerminalEvent::Output(data.to_vec()))
                        .is_err()
                    {
                        return;
                    }
                }
                russh::ChannelMsg::Eof | russh::ChannelMsg::Close => break,
                _ => {}
            }
        }
        let _ = output_tx.send(TerminalEvent::Closed);
    });
    let _ = event_tx.send(TerminalEvent::Connected);
    Some(SshConnection {
        writer,
        handle,
        reader,
    })
}

fn run_local_pty(
    columns: u16,
    rows: u16,
    shell: Option<&str>,
    event_tx: mpsc::Sender<TerminalEvent>,
    command_rx: mpsc::Receiver<TerminalCommand>,
) {
    let mut boundary = NativeLocalPtyBoundary::new();
    let session = match boundary.start(columns, rows, shell) {
        Ok(session) => session,
        Err(error) => {
            let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
            return;
        }
    };
    if let Err(error) = session.ready() {
        let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
        return;
    }
    let _ = event_tx.send(TerminalEvent::Output(Vec::new()));
    loop {
        while let Ok(command) = command_rx.try_recv() {
            let result = match command {
                TerminalCommand::Input(bytes) => session.input(&bytes),
                TerminalCommand::Resize(columns, rows) => session.resize(columns, rows),
                TerminalCommand::Close => {
                    let _ = session.close();
                    let _ = event_tx.send(TerminalEvent::Closed);
                    return;
                }
            };
            if let Err(error) = result {
                let _ = event_tx.send(TerminalEvent::Error(error.to_string()));
            }
        }
        match session.events.recv_timeout(Duration::from_millis(16)) {
            Ok(LocalPtyEvent::Output(bytes)) => {
                let _ = event_tx.send(TerminalEvent::Output(bytes));
            }
            Ok(LocalPtyEvent::Error(error)) => {
                let _ = event_tx.send(TerminalEvent::Error(error));
            }
            Ok(LocalPtyEvent::Closed) => {
                let _ = event_tx.send(TerminalEvent::Closed);
                return;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        net::TcpListener,
        path::PathBuf,
        process::{Child, Command, Stdio},
        time::Instant,
    };
    use sshcli_terminal_ui::TerminalStatus;

    struct LocalSshFixture {
        _dir: PathBuf,
        child: Child,
        stderr_path: PathBuf,
        options: ConnectionOptions,
    }

    impl LocalSshFixture {
        fn start() -> Self {
            let dir = std::env::temp_dir().join(format!("sshcli-native-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            let run = |args: &[&str]| {
                let status = Command::new(args[0]).args(&args[1..]).status().unwrap();
                assert!(status.success(), "command failed: {args:?}");
            };
            let host_key = dir.join("host_key");
            let user_key = dir.join("user_key");
            run(&["ssh-keygen", "-q", "-N", "", "-t", "ed25519", "-f", host_key.to_str().unwrap()]);
            run(&["ssh-keygen", "-q", "-N", "", "-t", "ed25519", "-f", user_key.to_str().unwrap()]);
            fs::write(dir.join("authorized_keys"), fs::read_to_string(user_key.with_extension("pub")).unwrap()).unwrap();
            let command = dir.join("command.sh");
            fs::write(&command, "#!/bin/sh\nprintf 'READY\\n'\nwhile IFS= read -r line; do\n  case \"$line\" in\n    resize) printf 'SIZE:%s\\n' \"$(stty size)\";;\n    close) exit 0;;\n    *) printf 'ECHO:%s\\n' \"$line\";;\n  esac\ndone\n").unwrap();
            run(&["chmod", "+x", command.to_str().unwrap()]);
            let port = TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
            let config = dir.join("sshd_config");
            let user = std::env::var("USER").unwrap();
            fs::write(&config, format!(
                "Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nAuthorizedKeysFile {}\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nPermitRootLogin no\nStrictModes no\nForceCommand {}\nLogLevel VERBOSE\n",
                host_key.display(), dir.join("authorized_keys").display(), command.display()
            )).unwrap();
            let config_check = Command::new("/usr/sbin/sshd")
                .args(["-t", "-f", config.to_str().unwrap()])
                .output()
                .unwrap();
            assert!(
                config_check.status.success(),
                "localhost sshd config validation failed: {}",
                String::from_utf8_lossy(&config_check.stderr)
            );
            let stderr_path = dir.join("sshd.stderr.log");
            let stderr = fs::File::create(&stderr_path).unwrap();
            let child = Command::new("/usr/sbin/sshd")
                .args(["-D", "-e", "-f", config.to_str().unwrap()])
                .stderr(Stdio::from(stderr))
                .spawn()
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
            let options = ConnectionOptions {
                host: "127.0.0.1".into(), port, username: user,
                identity_file: Some(user_key.to_string_lossy().into_owned()),
                accept_unknown_host_key: true,
                authentication: sshcli_core::ssh::Authentication::PrivateKey(None),
            };
            Self { _dir: dir, child, stderr_path, options }
        }
    }

    impl Drop for LocalSshFixture {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            if let Ok(diagnostics) = fs::read_to_string(&self.stderr_path) {
                if !diagnostics.is_empty() {
                    eprintln!("localhost sshd stderr:\n{diagnostics}");
                }
            }
            let _ = fs::remove_dir_all(&self._dir);
        }
    }

    fn wait_for<F: FnMut(&TerminalEvent) -> bool>(rx: &mpsc::Receiver<TerminalEvent>, mut predicate: F) -> Vec<TerminalEvent> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut seen = Vec::new();
        while Instant::now() < deadline {
            let event = match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => event,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("SSH fixture event channel disconnected; saw {seen:?}")
                }
            };
            let done = predicate(&event);
            seen.push(event);
            if done { return seen; }
        }
        panic!("timed out waiting for SSH event; saw {seen:?}");
    }

    #[test]
    fn ssh_worker_covers_lifecycle_io_resize_reconnect_and_remote_close() {
        let fixture = LocalSshFixture::start();
        let (events, event_rx) = mpsc::channel();
        let (commands, command_rx) = mpsc::channel();
        let (reconnect, reconnect_rx) = mpsc::channel();
        let options = ConnectionOptions {
            host: fixture.options.host.clone(),
            port: fixture.options.port,
            username: fixture.options.username.clone(),
            identity_file: fixture.options.identity_file.clone(),
            accept_unknown_host_key: fixture.options.accept_unknown_host_key,
            authentication: sshcli_core::ssh::Authentication::PrivateKey(None),
        };
        thread::spawn(move || run_ssh_worker(options, 80, 24, events, command_rx, reconnect_rx));

        let startup = wait_for(&event_rx, |event| matches!(event, TerminalEvent::Connected));
        assert!(startup.iter().any(|event| matches!(event, TerminalEvent::Connecting)));
        assert!(startup.iter().any(|event| matches!(event, TerminalEvent::Output(bytes) if bytes.windows(5).any(|w| w == b"READY"))));
        commands.send(TerminalCommand::Input(b"hello\n".to_vec())).unwrap();
        assert!(wait_for(&event_rx, |event| matches!(event, TerminalEvent::Output(bytes) if bytes.windows(10).any(|w| w == b"ECHO:hello"))).iter().any(|event| matches!(event, TerminalEvent::Output(_))));
        commands.send(TerminalCommand::Resize(120, 40)).unwrap();
        commands.send(TerminalCommand::Input(b"resize\n".to_vec())).unwrap();
        assert!(wait_for(&event_rx, |event| matches!(event, TerminalEvent::Output(bytes) if bytes.windows(8).any(|w| w == b"SIZE:40 "))).iter().any(|event| matches!(event, TerminalEvent::Output(_))));

        reconnect.send(ConnectionOptions {
            host: fixture.options.host.clone(), port: fixture.options.port, username: fixture.options.username.clone(),
            identity_file: fixture.options.identity_file.clone(), accept_unknown_host_key: true,
            authentication: sshcli_core::ssh::Authentication::PrivateKey(None),
        }).unwrap();
        let reconnected = wait_for(&event_rx, |event| matches!(event, TerminalEvent::Connected));
        assert!(reconnected.iter().any(|event| matches!(event, TerminalEvent::Connecting)));
        commands.send(TerminalCommand::Input(b"close\n".to_vec())).unwrap();
        assert!(wait_for(&event_rx, |event| matches!(event, TerminalEvent::Closed)).iter().any(|event| matches!(event, TerminalEvent::Closed)));
        commands.send(TerminalCommand::Close).unwrap();
    }

    #[test]
    fn ssh_worker_reports_connect_error_to_terminal_controller() {
        let (events, event_rx) = mpsc::channel();
        let (commands, command_rx) = mpsc::channel();
        let (_reconnect, reconnect_rx) = mpsc::channel();
        let options = ConnectionOptions { host: "127.0.0.1".into(), port: 1, username: "fixture".into(), identity_file: None, accept_unknown_host_key: true, authentication: sshcli_core::ssh::Authentication::None };
        thread::spawn(move || run_ssh_worker(options, 80, 24, events, command_rx, reconnect_rx));
        let mut controller = TerminalController::new(80, 24, event_rx, commands);
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && !matches!(controller.status(), TerminalStatus::Error(_)) {
            controller.pump();
            thread::sleep(Duration::from_millis(10));
        }
        assert!(matches!(controller.status(), TerminalStatus::Error(_)));
    }
}
