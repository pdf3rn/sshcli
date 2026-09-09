//! Presentation boundary for one native terminal surface.
//!
//! This crate intentionally contains no SSH, PTY, session, clipboard, or
//! persistence code. A caller owns `TerminalState` and supplies its snapshot.

use std::sync::mpsc;

use slint::{ModelRc, SharedString, VecModel};
use sshcli_core::terminal::TerminalState;

slint::include_modules!();

/// Copies the verified core state into the typed Slint presentation model.
pub fn surface_data(state: &TerminalState) -> (ModelRc<TerminalCell>, i32, i32, i32, i32) {
    let cells = state
        .visible_cells()
        .into_iter()
        .map(|cell| TerminalCell {
            character: SharedString::from(cell.character.to_string()),
        })
        .collect::<Vec<_>>();
    let (column, row) = state.cursor_position();
    (
        ModelRc::new(VecModel::from(cells)),
        state.columns() as i32,
        state.rows() as i32,
        column as i32,
        row as i32,
    )
}

/// Events delivered by a native PTY adapter. The receiver is drained by the
/// UI event loop; PTY reads remain on the adapter's worker thread.
#[derive(Debug, PartialEq, Eq)]
pub enum TerminalEvent {
    Connecting,
    Connected,
    Output(Vec<u8>),
    Error(String),
    Closed,
}

/// Commands sent from Slint callbacks to a native PTY worker.
#[derive(Debug, PartialEq, Eq)]
pub enum TerminalCommand {
    Input(Vec<u8>),
    Resize(u16, u16),
    Close,
}

/// Convert Slint printable/private key text to bytes suitable for a VT PTY.
pub fn encode_key_input(input: &str) -> Vec<u8> {
    use slint::platform::Key;
    let mut encoded = Vec::with_capacity(input.len());
    for character in input.chars() {
        let sequence: &[u8] = match character {
            c if c == char::from(Key::Escape) => b"\x1b",
            c if c == char::from(Key::Tab) => b"\t",
            c if c == char::from(Key::Return) => b"\r",
            c if c == char::from(Key::Backspace) => b"\x7f",
            c if c == char::from(Key::Delete) => b"\x1b[3~",
            c if c == char::from(Key::UpArrow) => b"\x1b[A",
            c if c == char::from(Key::DownArrow) => b"\x1b[B",
            c if c == char::from(Key::RightArrow) => b"\x1b[C",
            c if c == char::from(Key::LeftArrow) => b"\x1b[D",
            c if c == char::from(Key::Home) => b"\x1b[H",
            c if c == char::from(Key::End) => b"\x1b[F",
            c if c == char::from(Key::PageUp) => b"\x1b[5~",
            c if c == char::from(Key::PageDown) => b"\x1b[6~",
            _ => {
                let mut buffer = [0; 4];
                encoded.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
                continue;
            }
        };
        encoded.extend_from_slice(sequence);
    }
    encoded
}

/// Typed controller for one local terminal surface. It deliberately owns no
/// PTY: the GUI adapter supplies channels, preserving Tauri coexistence and
/// making the UI thread only parse, model, and paint.
pub struct TerminalController {
    state: TerminalState,
    events: mpsc::Receiver<TerminalEvent>,
    commands: mpsc::Sender<TerminalCommand>,
    status: TerminalStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalStatus {
    Starting,
    Ready,
    Error(String),
    Closed,
}

impl TerminalController {
    pub fn new(
        columns: usize,
        rows: usize,
        events: mpsc::Receiver<TerminalEvent>,
        commands: mpsc::Sender<TerminalCommand>,
    ) -> Self {
        Self {
            state: TerminalState::new(columns, rows, 1_000),
            events,
            commands,
            status: TerminalStatus::Starting,
        }
    }

    /// Non-blocking UI tick. Call from a Slint timer; never from PTY IO.
    pub fn pump(&mut self) -> bool {
        let mut changed = false;
        while let Ok(event) = self.events.try_recv() {
            match event {
                TerminalEvent::Connecting => {
                    self.status = TerminalStatus::Starting;
                    changed = true;
                }
                TerminalEvent::Connected => {
                    self.status = TerminalStatus::Ready;
                    changed = true;
                }
                TerminalEvent::Output(bytes) => {
                    self.state.process(&bytes);
                    self.status = TerminalStatus::Ready;
                    changed = true;
                }
                TerminalEvent::Error(error) => {
                    self.status = TerminalStatus::Error(error);
                    changed = true;
                }
                TerminalEvent::Closed => {
                    self.status = TerminalStatus::Closed;
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn refresh_surface(&self, surface: &TerminalSurface) {
        let (cells, columns, rows, cursor_column, cursor_row) = surface_data(&self.state);
        surface.set_cells(cells);
        surface.set_columns(columns);
        surface.set_rows(rows);
        surface.set_cursor_column(cursor_column);
        surface.set_cursor_row(cursor_row);
        surface.set_status(self.status_text());
    }

    pub fn input(&self, input: &str) -> Result<(), mpsc::SendError<TerminalCommand>> {
        self.commands
            .send(TerminalCommand::Input(encode_key_input(input)))
    }

    pub fn resize(
        &mut self,
        columns: u16,
        rows: u16,
    ) -> Result<(), mpsc::SendError<TerminalCommand>> {
        self.state.resize(columns as usize, rows as usize);
        self.commands.send(TerminalCommand::Resize(columns, rows))
    }

    pub fn close(&self) -> Result<(), mpsc::SendError<TerminalCommand>> {
        self.commands.send(TerminalCommand::Close)
    }

    pub fn status(&self) -> &TerminalStatus {
        &self.status
    }

    pub fn status_text(&self) -> SharedString {
        match &self.status {
            TerminalStatus::Starting => "Starting".into(),
            TerminalStatus::Ready => "Ready".into(),
            TerminalStatus::Error(error) => format!("Error: {error}").into(),
            TerminalStatus::Closed => "Closed".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::Model;
    use std::sync::mpsc;

    #[test]
    fn maps_core_grid_and_cursor_to_row_major_surface_data() {
        let mut state = TerminalState::new(4, 2, 8);
        state.process(b"ab\x1b[2;3Hc");
        let (cells, columns, rows, cursor_column, cursor_row) = surface_data(&state);

        assert_eq!((columns, rows, cursor_column, cursor_row), (4, 2, 3, 1));
        assert_eq!(cells.row_data(0).unwrap().character, "a");
        assert_eq!(cells.row_data(6).unwrap().character, "c");
    }

    #[test]
    fn controller_releases_startup_output_in_order_and_updates_surface_model() {
        let (event_tx, event_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();
        let mut controller = TerminalController::new(4, 2, event_rx, command_tx);
        event_tx
            .send(TerminalEvent::Output(b"ready".to_vec()))
            .unwrap();
        event_tx.send(TerminalEvent::Output(b"!".to_vec())).unwrap();

        assert!(controller.pump());
        let surface = TerminalSurface::new().unwrap();
        controller.refresh_surface(&surface);
        assert_eq!(surface.get_cells().row_data(0).unwrap().character, "r");
        assert_eq!(surface.get_cells().row_data(5).unwrap().character, "!");
        assert_eq!(controller.status(), &TerminalStatus::Ready);
    }

    #[test]
    fn controller_forwards_input_and_resize_without_blocking() {
        let (_event_tx, event_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let mut controller = TerminalController::new(4, 2, event_rx, command_tx);
        controller.input("x").unwrap();
        controller.resize(80, 24).unwrap();
        assert_eq!(
            command_rx.recv().unwrap(),
            TerminalCommand::Input(b"x".to_vec())
        );
        assert_eq!(command_rx.recv().unwrap(), TerminalCommand::Resize(80, 24));
    }

    #[test]
    fn controller_exposes_error_and_close_state() {
        let (event_tx, event_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();
        let mut controller = TerminalController::new(4, 2, event_rx, command_tx);
        event_tx
            .send(TerminalEvent::Error("pty failed".into()))
            .unwrap();
        controller.pump();
        assert_eq!(
            controller.status(),
            &TerminalStatus::Error("pty failed".into())
        );
        event_tx.send(TerminalEvent::Closed).unwrap();
        controller.pump();
        assert_eq!(controller.status(), &TerminalStatus::Closed);
        assert_eq!(controller.status_text(), "Closed");
    }

    #[test]
    fn controller_tracks_connecting_and_connected_lifecycle() {
        let (event_tx, event_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();
        let mut controller = TerminalController::new(4, 2, event_rx, command_tx);
        event_tx.send(TerminalEvent::Connecting).unwrap();
        event_tx.send(TerminalEvent::Connected).unwrap();

        assert!(controller.pump());
        assert_eq!(controller.status(), &TerminalStatus::Ready);
    }

    #[test]
    fn controller_close_forwards_explicit_close_command() {
        let (_event_tx, event_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let controller = TerminalController::new(4, 2, event_rx, command_tx);
        controller.close().unwrap();
        assert_eq!(command_rx.recv().unwrap(), TerminalCommand::Close);
    }

    #[test]
    fn encodes_special_keys_for_pty_input() {
        use slint::platform::Key;
        let input = format!(
            "a{}{}{}{}",
            char::from(Key::UpArrow),
            char::from(Key::LeftArrow),
            char::from(Key::Return),
            char::from(Key::Backspace)
        );
        assert_eq!(encode_key_input(&input), b"a\x1b[A\x1b[D\r\x7f");
    }

    #[test]
    fn large_output_is_drained_as_one_nonblocking_pump() {
        let (event_tx, event_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();
        let mut controller = TerminalController::new(80, 24, event_rx, command_tx);
        event_tx
            .send(TerminalEvent::Output(vec![b'x'; 64 * 1024]))
            .unwrap();
        assert!(controller.pump());
        assert_eq!(controller.status(), &TerminalStatus::Ready);
    }
}
