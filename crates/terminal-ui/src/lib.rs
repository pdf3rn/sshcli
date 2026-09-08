//! Presentation boundary for one native terminal surface.
//!
//! This crate intentionally contains no SSH, PTY, session, clipboard, or
//! persistence code. A caller owns `TerminalState` and supplies its snapshot.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_core_grid_and_cursor_to_row_major_surface_data() {
        let mut state = TerminalState::new(4, 2, 8);
        state.process(b"ab\x1b[2;3Hc");
        let (cells, columns, rows, cursor_column, cursor_row) = surface_data(&state);

        assert_eq!((columns, rows, cursor_column, cursor_row), (4, 2, 3, 1));
        assert_eq!(cells.row_data(0).unwrap().character, "a");
        assert_eq!(cells.row_data(6).unwrap().character, "c");
    }
}
