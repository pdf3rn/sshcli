//! The terminal emulation model.
//!
//! [`TermModel`] owns the `alacritty_terminal` emulator state ([`Term`]) plus a
//! synchronous VTE [`Processor`] used to feed raw PTY bytes. It exposes the
//! deterministic operations needed by the renderer: feed, resize, scroll,
//! selection, and text extraction, along with cursor position and grid
//! dimensions.

use std::ops::RangeInclusive;

use alacritty_terminal::{
    event::VoidListener,
    grid::Dimensions,
    index::{Column, Direction, Line, Point, Side},
    selection::{Selection, SelectionRange, SelectionType},
    term::{cell::Cell, search::RegexSearch, viewport_to_point, Config, Term},
    vte::ansi::Processor,
    Grid,
};

/// Terminal dimensions for a given columns/rows pair.
#[derive(Debug, Clone, Copy)]
pub struct TermDimensions {
    columns: usize,
    screen_lines: usize,
}

impl TermDimensions {
    pub fn new(columns: usize, screen_lines: usize) -> Self {
        Self {
            columns,
            screen_lines,
        }
    }
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.screen_lines
    }

    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

/// Owned terminal emulation model.
pub struct TermModel {
    term: Term<VoidListener>,
    processor: Processor<alacritty_terminal::vte::ansi::StdSyncHandler>,
    dimensions: TermDimensions,
    /// Active selection, if any.
    selection: Option<Selection>,
    /// Currently compiled search query (for find-next).
    search: Option<RegexSearch>,
    /// Last search match, used as the origin for the next find-next.
    last_match: Option<RangeInclusive<Point>>,
}

impl TermModel {
    /// Create a new model with the given columns/rows and a default scrollback
    /// buffer of 10_000 lines.
    pub fn new(columns: usize, screen_lines: usize) -> Self {
        let dimensions = TermDimensions::new(columns, screen_lines);
        let term = Term::new(Config::default(), &dimensions, VoidListener);
        let processor = Processor::<alacritty_terminal::vte::ansi::StdSyncHandler>::new();
        Self {
            term,
            processor,
            dimensions,
            selection: None,
            search: None,
            last_match: None,
        }
    }

    /// Feed raw PTY bytes into the emulator.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.term, bytes);
    }

    /// Resize the emulator grid.
    pub fn resize(&mut self, columns: usize, screen_lines: usize) {
        if columns == self.dimensions.columns && screen_lines == self.dimensions.screen_lines {
            return;
        }
        self.dimensions = TermDimensions::new(columns, screen_lines);
        self.term.resize(self.dimensions);
    }

    /// Scroll the viewport backwards (positive) or forwards (negative).
    pub fn scroll(&mut self, lines: i32) {
        // Scroll::Delta(i32): positive scrolls up (older lines), negative down.
        self.term
            .scroll_display(alacritty_terminal::grid::Scroll::Delta(lines));
    }

    /// Scroll the viewport back to the bottom (newest output).
    pub fn scroll_to_bottom(&mut self) {
        self.term
            .scroll_display(alacritty_terminal::grid::Scroll::Bottom);
    }

    /// Number of columns in the viewport.
    pub fn columns(&self) -> usize {
        self.dimensions.columns
    }

    /// Number of screen (visible) lines.
    pub fn screen_lines(&self) -> usize {
        self.dimensions.screen_lines
    }

    /// Total lines (visible + scrollback).
    pub fn total_lines(&self) -> usize {
        self.term.total_lines()
    }

    /// Current scrollback offset (0 == at bottom).
    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    /// A read-only view of the grid (for the renderer).
    pub fn grid(&self) -> &Grid<Cell> {
        self.term.grid()
    }

    /// Cursor position within the grid.
    pub fn cursor_position(&self) -> Point {
        self.term.grid().cursor.point
    }

    /// Start a new selection at a screen-space (x, y), in cell coordinates.
    pub fn start_selection(&mut self, selection_type: SelectionType, column: usize, line: usize) {
        let point = Point::new(Line(line as i32), Column(column));
        self.selection = Some(Selection::new(selection_type, point, Side::Left));
    }

    /// Update the selection end point at a screen-space (x, y).
    pub fn update_selection(&mut self, column: usize, line: usize) {
        if let Some(selection) = self.selection.as_mut() {
            let point = Point::new(Line(line as i32), Column(column));
            selection.update(point, Side::Right);
        }
    }

    /// End the current selection (leaves it in place so it can be copied).
    pub fn end_selection(&mut self) {
        // No-op; the selection stays until cleared.
    }

    /// Clear any active selection.
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// The current selection range (grid coordinates), if any.
    #[allow(dead_code)]
    pub fn selection_range(&self) -> Option<SelectionRange> {
        self.selection.as_ref().and_then(|s| s.to_range(&self.term))
    }

    /// Extract the text of the current selection.
    pub fn selected_text(&mut self) -> Option<String> {
        let selection = self.selection.as_ref()?;
        let range = selection.to_range(&self.term)?;
        Some(extract_text(&self.term, &range))
    }

    /// Convert screen-space (column, line) into a grid point, accounting for
    /// the current scroll offset.
    pub fn viewport_to_point(&self, column: usize, line: usize) -> Point {
        viewport_to_point(self.display_offset(), Point::new(line, Column(column)))
    }

    /// Set the search query, compiling a new `RegexSearch`.
    ///
    /// An empty query clears any previous search.
    pub fn set_search(&mut self, query: &str) -> Result<(), String> {
        if query.is_empty() {
            self.search = None;
            self.last_match = None;
            return Ok(());
        }
        let regex = RegexSearch::new(query).map_err(|e| format!("search: {e}"))?;
        self.search = Some(regex);
        self.last_match = None;
        Ok(())
    }

    /// Find the next match (wrapping forward). Returns the match range and a
    /// (column, line) screen-space location suitable for scrolling to.
    pub fn find_next(&mut self) -> Option<(RangeInclusive<Point>, Point)> {
        let display_offset = self.display_offset() as i32;
        let regex = self.search.as_mut()?;
        let origin = match self.last_match.as_ref() {
            Some(m) => *m.end(),
            None => Point::new(Line(-display_offset), Column(0)),
        };
        let m = self
            .term
            .search_next(regex, origin, Direction::Right, Side::Left, None)?;
        self.last_match = Some(m.clone());
        let start = *m.start();
        Some((m, start))
    }

    /// Clear the scrollback history (keeps the visible screen intact).
    pub fn clear_scrollback(&mut self) {
        self.term.grid_mut().clear_history();
    }

    /// Feed a raw ANSI escape; used for clear-screen (Ctrl+L).
    pub fn send_ansi(&mut self, seq: &[u8]) {
        self.feed(seq);
    }
}

/// Extract the string content of the grid within `range`.
fn extract_text(term: &Term<VoidListener>, range: &SelectionRange) -> String {
    let mut out = String::new();
    for indexed in term.grid().display_iter() {
        if range.contains(indexed.point) {
            out.push(indexed.c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_and_parse_basic_output() {
        let mut model = TermModel::new(80, 24);
        model.feed(b"hello");
        // The first visible row should now contain "hello".
        let grid = model.grid();
        let line = &grid[Line(0)];
        let mut rendered = String::new();
        for col in 0..model.columns() {
            rendered.push(line[Column(col)].c);
        }
        assert!(rendered.starts_with("hello"), "rendered: {rendered:?}");
    }

    #[test]
    fn ansi_colors_render_to_cells() {
        let mut model = TermModel::new(80, 24);
        model.feed(b"\x1b[31mred\x1b[0m");
        let grid = model.grid();
        let cell = &grid[Line(0)][Column(0)];
        assert_eq!(cell.c, 'r');
        // Foreground should be Named Red.
        assert_eq!(
            cell.fg,
            alacritty_terminal::vte::ansi::Color::Named(
                alacritty_terminal::vte::ansi::NamedColor::Red
            )
        );
    }

    #[test]
    fn cursor_position_moves() {
        let mut model = TermModel::new(80, 24);
        model.feed(b"abc");
        let cursor = model.cursor_position();
        assert_eq!(cursor.column, Column(3));
        assert_eq!(cursor.line.0, 0);
    }

    #[test]
    fn resize_updates_dimensions() {
        let mut model = TermModel::new(80, 24);
        assert_eq!(model.columns(), 80);
        assert_eq!(model.screen_lines(), 24);
        model.resize(120, 40);
        assert_eq!(model.columns(), 120);
        assert_eq!(model.screen_lines(), 40);
    }

    #[test]
    fn selection_extracts_text() {
        let mut model = TermModel::new(80, 24);
        model.feed(b"select me");
        model.start_selection(SelectionType::Simple, 0, 0);
        model.update_selection(8, 0);
        let text = model.selected_text().expect("selection text");
        assert_eq!(text, "select me");
    }

    #[test]
    fn scrollback_buffers_and_can_clear() {
        let mut model = TermModel::new(80, 2);
        // Emit more lines than the screen so history accumulates.
        for i in 0..10 {
            let line = format!("line {i}\r\n");
            model.feed(line.as_bytes());
        }
        model.scroll_to_bottom();
        assert_eq!(model.display_offset(), 0);
        // Scrolling up reveals older output.
        model.scroll(1);
        assert!(model.display_offset() > 0);
        // Clearing history resets offset.
        model.clear_scrollback();
        assert_eq!(model.display_offset(), 0);
    }

    #[test]
    fn search_find_next_finds_match() {
        let mut model = TermModel::new(80, 24);
        model.feed(b"alpha\r\nbeta\r\ngamma\r\n");
        model.set_search("beta").expect("compile regex");
        let (range, _point) = model.find_next().expect("a match");
        let start = range.start();
        assert_eq!(start.line.0, 1); // "beta" is on line index 1.
    }

    #[test]
    fn scrollback_line_count_helper() {
        // total_lines grows as output exceeds the screen.
        let mut model = TermModel::new(80, 2);
        assert_eq!(model.total_lines(), 2);
        for i in 0..5 {
            model.feed(format!("row {i}\r\n").as_bytes());
        }
        assert!(model.total_lines() > 2, "history should accumulate");
    }
}
