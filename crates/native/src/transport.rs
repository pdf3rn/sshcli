//! A transport-agnostic interface between the terminal widget and a session.
//!
//! [`SessionTransport`] abstracts the I/O plumbing so that the terminal widget
//! ([`crate::widget::TerminalSession`]) can drive either a local PTY
//! ([`crate::session::PtySession`]) or a remote SSH channel
//! ([`crate::ssh::SshTransport`]) through one code path. Implementors push
//! output through the shared [`SessionEvent`] vocabulary and expose the same
//! [`SessionState`] lifecycle.
//!
//! The trait is object-safe on purpose: the widget holds a
//! `Box<dyn SessionTransport>` so tabs can mix local and SSH sessions freely.

use crate::session::{SessionEvent, SessionState};

/// A terminal session transport (local PTY or remote SSH channel).
///
/// Implementors must be `Send` so they can be moved onto worker threads and
/// stored in tabs that live across the application.
pub trait SessionTransport: Send {
    /// Write user keyboard input to the session.
    fn write(&self, bytes: &[u8]) -> Result<(), String>;

    /// Propagate a terminal resize (columns × rows, cell dimensions not needed).
    fn resize(&self, cols: u16, rows: u16) -> Result<(), String>;

    /// Poll for the next [`SessionEvent`] without blocking the UI thread.
    fn try_recv(&self) -> Option<SessionEvent>;

    /// The current lifecycle state of the session.
    fn state(&self) -> SessionState;

    /// Close the session: kill the child PTY / close the SSH channel.
    fn close(&mut self);

    /// Restart/reconnect the session after it has closed.
    ///
    /// Local PTYs respawn the shell; SSH transports re-run the connection.
    fn restart(&mut self) -> Result<(), String>;

    /// Whether this transport is a remote (SSH) session. The widget and the
    /// tab UI use this for the "(closed)" / reconnect wording.
    fn is_remote(&self) -> bool {
        false
    }
}

/// A deterministic, mockable backing store used by unit tests of the terminal
/// widget and tab lifecycle.
///
/// It records every write/resize/close/restart so tests can assert the widget
/// forwards actions correctly, and it accepts a pre-seeded output script that
/// `try_recv` replays so tests can assert output bytes reach the emulator.
///
/// It does not itself implement [`SessionTransport`]; wrap it in
/// [`SharedNullTransport`] to obtain the `Send` trait object the widget holds.
pub struct NullTransport {
    /// Output events to replay on `try_recv`.
    pub events: std::collections::VecDeque<SessionEvent>,
    /// Bytes received through `write` (concatenated).
    pub written: Vec<u8>,
    /// All `(cols, rows)` resizes received.
    pub resizes: Vec<(u16, u16)>,
    /// Number of `close` calls.
    pub close_count: usize,
    /// Number of `restart` calls.
    pub restart_count: usize,
    /// Current state, mutable so tests can force transitions.
    pub state: SessionState,
    /// `restart` result override.
    pub restart_result: Result<(), String>,
}

impl Default for NullTransport {
    fn default() -> Self {
        Self {
            events: Default::default(),
            written: Vec::new(),
            resizes: Vec::new(),
            close_count: 0,
            restart_count: 0,
            state: SessionState::Running,
            restart_result: Ok(()),
        }
    }
}

impl NullTransport {
    /// Create a transport with no pending output.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue an output event for the next `try_recv` calls.
    pub fn push_output(&mut self, bytes: &[u8]) {
        self.events.push_back(SessionEvent::Output(bytes.to_vec()));
    }
}

/// A [`SessionTransport`] wrapper around a shared `Arc<Mutex<NullTransport>>`
/// that allows the instrumented mock to be driven and inspected from tests
/// while the widget holds a `Box<dyn SessionTransport>`. `Arc` (not `Rc`) keeps
/// it `Send`, matching the real transports.
#[derive(Clone)]
pub struct SharedNullTransport(std::sync::Arc<std::sync::Mutex<NullTransport>>);

impl SharedNullTransport {
    pub fn new(inner: NullTransport) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(inner)))
    }

    /// Inspect or mutate the inner transport inside a closure.
    pub fn with<T>(&self, f: impl FnOnce(&mut NullTransport) -> T) -> T {
        f(&mut self.0.lock().expect("null transport lock"))
    }
}

impl SessionTransport for SharedNullTransport {
    fn write(&self, bytes: &[u8]) -> Result<(), String> {
        self.0
            .lock()
            .expect("lock")
            .written
            .extend_from_slice(bytes);
        Ok(())
    }

    fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        self.0.lock().expect("lock").resizes.push((cols, rows));
        Ok(())
    }

    fn try_recv(&self) -> Option<SessionEvent> {
        self.0.lock().expect("lock").events.pop_front()
    }

    fn state(&self) -> SessionState {
        self.0.lock().expect("lock").state
    }

    fn close(&mut self) {
        let mut inner = self.0.lock().expect("lock");
        inner.close_count += 1;
        inner.state = SessionState::Closed;
    }

    fn restart(&mut self) -> Result<(), String> {
        let mut inner = self.0.lock().expect("lock");
        inner.restart_count += 1;
        inner.state = SessionState::Running;
        inner.restart_result.clone()
    }
}
