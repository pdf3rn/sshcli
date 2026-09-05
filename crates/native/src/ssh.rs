//! SSH channel transport for the native terminal.
//!
//! [`SshTransport`] wraps a `russh::Channel` + `client::Handle` behind the same
//! [`SessionTransport`] interface the local PTY implements, so an interactive
//! SSH shell renders in the very same [`crate::widget::TerminalSession`] as a
//! local shell.
//!
//! Connection is performed through [`sshcli_core::ssh::open_shell`] on a
//! dedicated tokio runtime owned by this crate. The channel is `.split()` into
//! a reader/writer pair: the reader is drained by a spawned task that converts
//! `ChannelMsg::{Data,ExtendedData}` into [`SessionEvent::Output`] and
//! `Eof`/`Close` into [`SessionEvent::Closed`], delivered through an `mpsc`
//! channel polled by [`SessionTransport::try_recv`] — the same model as
//! [`crate::session::PtySession`].
//!
//! # Error mapping
//!
//! We replicate [`sshcli_app::session::map_ssh_error`] locally (host-key
//! rejections as `sshcli:host-key:` + JSON, password-required as
//! `sshcli:password-required`) rather than depending on the `sshcli-app` crate,
//! because `sshcli-app` routes everything through a Tauri emitter that the
//! native frontend does not use. Calling `sshcli_core::ssh` + `profiles` +
//! `credentials` directly keeps this crate free of the WebView indirection.

use std::sync::{
    mpsc::{self, Receiver},
    Arc,
};

use russh::{client::Msg, ChannelMsg, ChannelWriteHalf};
use tokio::task::JoinHandle;

use sshcli_core::{
    credentials,
    ssh::{self, ConnectionOptions},
    ProfileStore,
};

use crate::{
    session::{SessionEvent, SessionState},
    transport::SessionTransport,
};

/// Replicated from `sshcli_app::session` so the native crate need not pull in
/// the Tauri emitter stack.
pub const PASSWORD_REQUIRED: &str = "sshcli:password-required";
pub const HOST_KEY_PREFIX: &str = "sshcli:host-key:";

/// Map a core SSH error to a user-facing string, preserving host-key and
/// password-required semantics (identical to `sshcli_app::session::map_ssh_error`).
pub fn map_ssh_error(error: sshcli_core::AppError) -> String {
    if let sshcli_core::AppError::HostKey {
        host,
        port,
        key,
        changed,
    } = error
    {
        let payload = serde_json::json!({
            "host": host,
            "port": port,
            "key": key,
            "changed": changed,
        });
        return format!("{HOST_KEY_PREFIX}{payload}");
    }
    error.to_string()
}

/// An ad-hoc or profile-based connection request.
#[derive(Debug, Clone)]
pub enum SshConnect {
    /// Connect using a saved profile, optionally overriding the password.
    Profile {
        name: String,
        password: Option<String>,
    },
    /// Connect using an ad-hoc `user@host[:port]` target.
    Adhoc {
        target: String,
        password: Option<String>,
    },
}

/// Parse an ad-hoc target of the form `user@host`, `user@host:port`, or
/// `user@[host]:port`. Replicated verbatim from
/// `sshcli_app::session::parse_adhoc_target`.
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
        let (host, remainder) = bracketed.split_once(']').ok_or_else(|| HINT.to_string())?;
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

/// Resolve a [`SshConnect`] request into concrete [`ConnectionOptions`].
///
/// Mirrors the credential resolution in `sshcli_app::session::ssh_connect` /
/// `ssh_connect_adhoc`: the profile store supplies the profile, a supplied
/// password wins, otherwise the keyring credential is consulted, and ad-hoc
/// targets with no password fall back to the default `~/.ssh` key (or demand a
/// password).
pub fn resolve_options(connect: &SshConnect) -> Result<ConnectionOptions, String> {
    match connect {
        SshConnect::Profile { name, password } => {
            let store = ProfileStore::new();
            let profile = store
                .load()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|p| p.name == *name)
                .ok_or_else(|| format!("profile not found: {name}"))?;

            let supplied = password.as_ref().filter(|p| !p.is_empty()).cloned();
            let secret = match &profile.authentication {
                sshcli_core::Authentication::None => None,
                _ => match supplied {
                    Some(p) => Some(p),
                    None => credentials::get_optional(name).map_err(|e| e.to_string())?,
                },
            };
            match ssh::options_for_profile(&profile, secret) {
                Ok(options) => Ok(options),
                Err(_)
                    if matches!(
                        profile.authentication,
                        sshcli_core::Authentication::Password
                    ) =>
                {
                    Err(PASSWORD_REQUIRED.into())
                }
                Err(e) => Err(e.to_string()),
            }
        }
        SshConnect::Adhoc { target, password } => {
            let (username, host, port) = parse_adhoc_target(target)?;
            let has_password = password.is_some();
            match ssh::options_adhoc(host, port, username, password.clone()) {
                Ok(options) => Ok(options),
                Err(_) if !has_password => Err(PASSWORD_REQUIRED.into()),
                Err(e) => Err(e.to_string()),
            }
        }
    }
}

/// A live or closed SSH shell transport.
pub struct SshTransport {
    /// Dedicated tokio runtime for this session's connection + read task.
    runtime: Arc<tokio::runtime::Runtime>,
    /// Writer half of the split channel (empty when closed/not yet connected).
    writer: Option<Arc<tokio::sync::Mutex<ChannelWriteHalf<Msg>>>>,
    /// Connection handle, kept for `disconnect` on close.
    handle: Option<russh::client::Handle<ssh::ClientHandler>>,
    /// Read-loop task handle, aborted on close.
    read_task: Option<JoinHandle<()>>,
    /// Output event receiver polled by `try_recv`.
    events: Receiver<SessionEvent>,
    /// The last successful connection options, used by `restart`.
    options: Option<ConnectionOptions>,
    /// Display label for the tab (profile name or `user@host`).
    label: String,
    /// Last requested terminal size, reused on reconnect.
    cols: std::sync::atomic::AtomicU16,
    rows: std::sync::atomic::AtomicU16,
    state: SessionState,
}

impl SshTransport {
    /// Connect and return a live transport, or a user-facing error string.
    ///
    /// This is synchronous from the caller's perspective: it blocks on the
    /// dedicated runtime until the shell channel is open. Connection is fast
    /// enough for interactive use, and matches how a local PTY is spawned
    /// synchronously in `PtySession::start`.
    pub fn connect(connect: &SshConnect, columns: u16, rows: u16) -> Result<Self, String> {
        let options = resolve_options(connect)?;
        let label = label_for(connect, &options);
        Self::connect_with_options(options, label, columns, rows)
    }

    fn connect_with_options(
        options: ConnectionOptions,
        label: String,
        columns: u16,
        rows: u16,
    ) -> Result<Self, String> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?,
        );

        let (tx, rx) = mpsc::channel::<SessionEvent>();

        // Block the caller while the connection is established.
        let opened = runtime
            .block_on(ssh::open_shell(options.clone(), columns, rows))
            .map_err(map_ssh_error)?;

        let (channel, handle) = opened;
        let (mut reader, writer) = channel.split();

        let writer = Arc::new(tokio::sync::Mutex::new(writer));
        let writer_for_task = writer.clone();
        let read_task = runtime.spawn(async move {
            while let Some(message) = reader.wait().await {
                match message {
                    ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                        if tx.send(SessionEvent::Output(data.to_vec())).is_err() {
                            break;
                        }
                    }
                    ChannelMsg::Eof | ChannelMsg::Close => break,
                    _ => {}
                }
            }
            let _ = tx.send(SessionEvent::Closed);
            let _ = writer_for_task;
        });

        Ok(Self {
            runtime,
            writer: Some(writer),
            handle: Some(handle),
            read_task: Some(read_task),
            events: rx,
            options: Some(options),
            label,
            cols: std::sync::atomic::AtomicU16::new(columns),
            rows: std::sync::atomic::AtomicU16::new(rows),
            state: SessionState::Running,
        })
    }

    /// Human-readable label for the tab.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Re-run the connection using the previously successful options.
    fn reconnect(&mut self) -> Result<(), String> {
        let options = self
            .options
            .clone()
            .ok_or_else(|| "cannot reconnect: no previous connection".to_string())?;
        let label = self.label.clone();
        let (cols, rows) = (
            self.cols.load(std::sync::atomic::Ordering::Relaxed),
            self.rows.load(std::sync::atomic::Ordering::Relaxed),
        );
        let fresh = Self::connect_with_options(options, label, cols, rows)?;
        *self = fresh;
        Ok(())
    }
}

/// Derive a display label from a connect request + resolved options.
fn label_for(connect: &SshConnect, options: &ConnectionOptions) -> String {
    match connect {
        SshConnect::Profile { name, .. } => name.clone(),
        SshConnect::Adhoc { target, .. } => {
            let _ = target;
            format!("{}@{}", options.username, options.host)
        }
    }
}

impl SessionTransport for SshTransport {
    fn write(&self, bytes: &[u8]) -> Result<(), String> {
        let writer = self
            .writer
            .as_ref()
            .ok_or_else(|| "no active ssh session".to_string())?;
        self.runtime
            .block_on(async {
                let writer = writer.lock().await;
                writer.data(bytes.as_ref()).await
            })
            .map_err(|e| e.to_string())
    }

    fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        self.cols.store(cols, std::sync::atomic::Ordering::Relaxed);
        self.rows.store(rows, std::sync::atomic::Ordering::Relaxed);
        let writer = self
            .writer
            .as_ref()
            .ok_or_else(|| "no active ssh session".to_string())?;
        self.runtime
            .block_on(async {
                let writer = writer.lock().await;
                writer
                    .window_change(u32::from(cols), u32::from(rows), 0, 0)
                    .await
            })
            .map_err(|e| e.to_string())
    }

    fn try_recv(&self) -> Option<SessionEvent> {
        self.events.try_recv().ok()
    }

    fn state(&self) -> SessionState {
        self.state
    }

    fn close(&mut self) {
        if let Some(task) = self.read_task.take() {
            task.abort();
        }
        if let Some(writer) = self.writer.take() {
            let writer = writer.clone();
            let _ = self.runtime.block_on(async move {
                let writer = writer.lock().await;
                let _ = writer.close().await;
            });
        }
        if let Some(handle) = self.handle.take() {
            let _ = self.runtime.block_on(async move {
                let _ = handle
                    .disconnect(russh::Disconnect::ByApplication, "closed by user", "en")
                    .await;
            });
        }
        self.state = SessionState::Closed;
    }

    fn restart(&mut self) -> Result<(), String> {
        self.reconnect()
    }

    fn is_remote(&self) -> bool {
        true
    }
}

impl Drop for SshTransport {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_adhoc_parses_simple_target() {
        assert_eq!(
            parse_adhoc_target("alice@example.com").unwrap(),
            ("alice".to_string(), "example.com".to_string(), 22)
        );
    }

    #[test]
    fn parse_adhoc_parses_port() {
        assert_eq!(
            parse_adhoc_target("alice@example.com:2222").unwrap(),
            ("alice".to_string(), "example.com".to_string(), 2222)
        );
    }

    #[test]
    fn parse_adhoc_parses_bracketed_ipv6() {
        assert_eq!(
            parse_adhoc_target("alice@[::1]").unwrap(),
            ("alice".to_string(), "::1".to_string(), 22)
        );
    }

    #[test]
    fn parse_adhoc_rejects_missing_at() {
        assert!(parse_adhoc_target("aliceexample.com").is_err());
    }

    #[test]
    fn parse_adhoc_rejects_empty_host() {
        assert!(parse_adhoc_target("alice@").is_err());
    }

    #[test]
    fn parse_adhoc_rejects_invalid_port() {
        assert!(parse_adhoc_target("alice@example.com:notaport").is_err());
    }

    #[test]
    fn map_ssh_error_wraps_host_key_as_json() {
        let err = sshcli_core::AppError::HostKey {
            host: "example.com".into(),
            port: 22,
            key: "ssh-ed25519 AAA".into(),
            changed: false,
        };
        let mapped = map_ssh_error(err);
        assert!(mapped.starts_with(HOST_KEY_PREFIX), "mapped: {mapped}");
        let json = &mapped[HOST_KEY_PREFIX.len()..];
        let parsed: serde_json::Value = serde_json::from_str(json).expect("valid json");
        assert_eq!(parsed["host"], "example.com");
        assert_eq!(parsed["port"], 22);
        assert_eq!(parsed["changed"], false);
    }

    #[test]
    fn map_ssh_error_passes_through_other_errors() {
        let err = sshcli_core::AppError::AuthenticationFailed;
        assert_eq!(map_ssh_error(err), "ssh authentication failed");
    }
}
