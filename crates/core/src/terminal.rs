//! UI-independent terminal emulation state for native frontends.
//!
//! This module deliberately stops at VT parsing and renderable grid state. It
//! does not own PTY/SSH IO, clipboard, focus, or painting. A future Slint
//! surface can consume the grid and cursor state without replacing xterm.js
//! with a plain text control.

use alacritty_terminal::{
    event::VoidListener,
    grid::{Dimensions, Grid},
    index::{Column, Line},
    term::{test::TermSize, Config, Term, TermMode},
    vte::ansi::Processor,
};

/// Native terminal emulator state backed by Alacritty's maintained VT parser
/// and terminal grid.
pub struct TerminalState {
    term: Term<VoidListener>,
    parser: Processor,
    size: TermSize,
}

/// The presentation-neutral portion of a visible terminal cell.
///
/// Keeping this DTO here prevents a Slint model from depending on Alacritty's
/// internal cell type. PTY/SSH input and output remain outside this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCell {
    pub character: char,
}

impl TerminalState {
    /// Creates a terminal with the requested visible dimensions and scrollback.
    pub fn new(columns: usize, rows: usize, scrollback: usize) -> Self {
        let size = TermSize::new(columns, rows);
        let config = Config {
            scrolling_history: scrollback,
            ..Config::default()
        };
        Self {
            term: Term::new(config, &size, VoidListener),
            parser: Processor::new(),
            size,
        }
    }

    /// Feeds raw PTY/SSH output into the VT parser.
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    /// Resizes the emulator; the PTY resize remains the caller's responsibility.
    pub fn resize(&mut self, columns: usize, rows: usize) {
        self.size = TermSize::new(columns, rows);
        self.term.resize(TermSize::new(columns, rows));
    }

    pub fn columns(&self) -> usize {
        self.size.columns()
    }

    pub fn rows(&self) -> usize {
        self.size.screen_lines()
    }

    pub fn mode(&self) -> TermMode {
        *self.term.mode()
    }

    /// Provides the complete grid for a future efficient native renderer.
    pub fn grid(&self) -> &Grid<alacritty_terminal::term::cell::Cell> {
        self.term.grid()
    }

    /// Returns a cell character for focused renderer and behavior tests.
    pub fn cell_char(&self, line: i32, column: usize) -> char {
        self.term.grid()[Line(line)][Column(column)].c
    }

    /// Returns the visible grid in row-major order for a renderer model.
    pub fn visible_cells(&self) -> Vec<TerminalCell> {
        (0..self.rows() as i32)
            .flat_map(|line| {
                (0..self.columns()).map(move |column| TerminalCell {
                    character: self.cell_char(line, column),
                })
            })
            .collect()
    }

    /// Current write cursor in visible-grid coordinates.
    pub fn cursor_position(&self) -> (usize, usize) {
        let point = self.term.grid().cursor.point;
        (point.column.0, point.line.0.max(0) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ansi_text_and_cursor_movement() {
        let mut terminal = TerminalState::new(20, 4, 16);
        terminal.process(b"hello\x1b[2;3Hworld");

        assert_eq!(terminal.cell_char(0, 0), 'h');
        assert_eq!(terminal.cell_char(1, 2), 'w');
    }

    #[test]
    fn preserves_alternate_screen_and_returns_to_primary() {
        let mut terminal = TerminalState::new(20, 4, 16);
        terminal.process(b"primary\x1b[?1049halt\x1b[?1049l");

        assert!(!terminal.mode().contains(TermMode::ALT_SCREEN));
        assert_eq!(terminal.cell_char(0, 0), 'p');
    }

    #[test]
    fn retains_unicode_wide_character_state() {
        let mut terminal = TerminalState::new(20, 4, 16);
        terminal.process("界".as_bytes());

        assert_eq!(terminal.cell_char(0, 0), '界');
    }

    #[test]
    fn resizes_visible_grid_and_keeps_scrollback() {
        let mut terminal = TerminalState::new(8, 2, 16);
        terminal.process(b"one\ntwo\nthree\nfour");
        terminal.resize(12, 3);

        assert_eq!(terminal.columns(), 12);
        assert_eq!(terminal.rows(), 3);
        assert!(terminal.grid().total_lines() >= 3);
    }

    #[test]
    fn parser_accepts_sequences_split_across_chunks() {
        let mut terminal = TerminalState::new(20, 4, 16);
        terminal.process(b"part\x1b[");
        terminal.process(b"2;2Hok");

        assert_eq!(terminal.cell_char(1, 1), 'o');
    }
}
