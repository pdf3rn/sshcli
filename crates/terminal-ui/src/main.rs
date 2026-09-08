use slint::ComponentHandle;
use sshcli_core::terminal::TerminalState;
use sshcli_terminal_ui::{surface_data, TerminalSurface};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = TerminalState::new(80, 24, 1_000);
    state.process(b"sshcli native terminal surface\r\n");

    let surface = TerminalSurface::new()?;
    let (cells, columns, rows, cursor_column, cursor_row) = surface_data(&state);
    surface.set_cells(cells);
    surface.set_columns(columns);
    surface.set_rows(rows);
    surface.set_cursor_column(cursor_column);
    surface.set_cursor_row(cursor_row);
    surface.on_input(|input| eprintln!("terminal input boundary: {input}"));
    surface.on_resize(|columns, rows| eprintln!("terminal resize boundary: {columns}x{rows}"));
    surface.run()?;
    Ok(())
}
