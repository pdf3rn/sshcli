//! # sshcli-native
//!
//! Native egui-based terminal frontend for sshcli.
//!
//! This crate renders a real, interactive local terminal using a pure native
//! GUI stack (`eframe`/`egui`), an `alacritty_terminal` emulator core, and a
//! `portable-pty`-owned shell session. It is intentionally independent of the
//! Tauri/WebView frontend.
//!
//! ## Engine choice (Option B — direct `alacritty_terminal`)
//!
//! We render the terminal directly from `alacritty_terminal` grids rather than
//! using the `egui_term` wrapper. See [`crate::term`] and `crate/README.md` for
//! the full rationale. In short:
//!
//! * `egui_term::TerminalBackend` hardcodes `alacritty_terminal::tty::new` for
//!   its PTY and owns the read loop internally, with no hook to supply an
//!   external PTY (e.g. a later SSH channel) or to observe exit/reconnect.
//! * Owning the PTY through `portable-pty` lets us surface a *closed / restart*
//!   state and later swap the transport for an SSH channel without touching the
//!   terminal widget.
//!
//! ## Module map
//!
//! * [`transport`] — the [`transport::SessionTransport`] abstraction (local PTY or SSH).
//! * [`session`] — the local PTY session (spawn, read loop, resize, closed/restart).
//! * [`ssh`] — the SSH channel transport (connect, read loop, resize, close/reconnect).
//! * [`profiles`] — pure formatting/validation helpers for the profile view.
//! * [`host_key`] — the host-key confirmation dialog.
//! * [`term`] — the emulator model wrapped around `alacritty_terminal::Term`.
//! * [`color`] — ANSI 16/256/truecolor → `egui::Color32` mapping.
//! * [`render`] — painting the grid (cells, cursor, selection, scrollback).
//! * [`widget`] — the embeddable [`widget::TerminalSession`] widget.
//! * [`app`] — the `eframe` application tying it together (with `egui_dock`).
//!
//! The binary target (`src/main.rs`) launches a dockable window with a mix of
//! local and SSH terminal tabs.

pub mod app;
pub mod color;
pub mod host_key;
pub mod panels;
pub mod profiles;
pub mod profiles_view;
pub mod render;
pub mod session;
pub mod ssh;
pub mod term;
pub mod transport;
pub mod widget;

pub use app::NativeApp;
pub use transport::{NullTransport, SessionTransport, SharedNullTransport};
pub use widget::TerminalSession;
