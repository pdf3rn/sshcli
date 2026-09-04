//! egui rendering of the terminal grid.
//!
//! Paints each visible cell (background rectangle + glyph) using the resolved
//! foreground/background colors, plus the cursor, selection highlight, and
//! text adornments (bold, dim, inverse, underline). Scrollback is automatic:
//! [`Grid::display_iter`] only yields cells in the visible viewport, already
//! offset by the scroll position.

use alacritty_terminal::{selection::SelectionRange, term::cell};

use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use crate::{color::ansi_color_to_color32, term::TermModel};

/// Background and default foreground colors for the terminal surface.
const BACKGROUND: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
const FOREGROUND: Color32 = Color32::from_rgb(0xd8, 0xd8, 0xd8);
const SELECTION: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x6a);

/// Render the terminal into `painter` at `origin` using `font`.
///
/// Returns the cell dimensions (width, height) in pixels, which the widget
/// uses to compute grid coordinates from pointer position.
pub fn render(
    model: &TermModel,
    selection: Option<SelectionRange>,
    painter: &Painter,
    origin: Pos2,
    font: FontId,
    max_size: Vec2,
) -> Vec2 {
    let fonts = painter.fonts(|f| f.clone());
    let (cell_width, cell_height) =
        painter.fonts(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));

    // Background fill.
    painter.rect_filled(Rect::from_min_size(origin, max_size), 0.0, BACKGROUND);

    let grid = model.grid();
    let cursor_point = model.cursor_position();
    let display_offset = model.display_offset();

    let mut shapes: Vec<Shape> = Vec::new();

    for indexed in grid.display_iter() {
        let flags = indexed.cell.flags;
        if flags.contains(cell::Flags::WIDE_CHAR_SPACER)
            || flags.contains(cell::Flags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }

        let point = indexed.point;
        let is_cursor = point == cursor_point;
        let is_selected = selection
            .as_ref()
            .is_some_and(|range| range.contains(point));
        let is_inverse = flags.contains(cell::Flags::INVERSE);
        let is_bold = flags.contains(cell::Flags::BOLD) || flags.contains(cell::Flags::DIM_BOLD);
        let is_dim = flags.intersects(cell::Flags::DIM | cell::Flags::DIM_BOLD);

        let mut fg = ansi_color_to_color32(indexed.fg);
        let mut bg = ansi_color_to_color32(indexed.bg);

        if is_bold {
            fg = brighten(fg, FOREGROUND);
        }
        if is_dim {
            fg = dim(fg);
        }

        // Inverse video swaps fg/bg.
        if is_inverse {
            std::mem::swap(&mut fg, &mut bg);
        }
        // Selection overrides the cell background.
        if is_selected {
            bg = SELECTION;
            fg = Color32::WHITE;
        }

        let x = origin.x + (point.column.0 as f32 * cell_width);
        let line_num = point.line.0 + display_offset as i32;
        let y = origin.y + (line_num as f32 * cell_height);
        let cell_min = pos2(x, y);

        // Cell background (only when it differs from the global bg).
        if bg != BACKGROUND {
            shapes.push(Shape::rect_filled(
                Rect::from_min_size(cell_min, vec2(cell_width, cell_height)),
                0.0,
                bg,
            ));
        }

        // Cursor block.
        if is_cursor {
            shapes.push(Shape::rect_filled(
                Rect::from_min_size(cell_min, vec2(cell_width, cell_height)),
                0.0,
                FOREGROUND,
            ));
        }

        // Underline.
        if flags.intersects(cell::Flags::ALL_UNDERLINES) {
            shapes.push(Shape::line_segment(
                [
                    pos2(x, y + cell_height - 1.0),
                    pos2(x + cell_width, y + cell_height - 1.0),
                ],
                Stroke::new(1.0_f32, fg),
            ));
        }

        // Glyph.
        if indexed.c != ' ' && indexed.c != '\t' && indexed.c != '\0' && !is_cursor {
            shapes.push(Shape::text(
                &fonts,
                pos2(x + cell_width / 2.0, y),
                Align2::CENTER_TOP,
                indexed.c,
                font.clone(),
                fg,
            ));
        }
    }

    painter.extend(shapes);

    vec2(cell_width, cell_height)
}

/// Brighten a foreground color toward white for bold text.
fn brighten(color: Color32, base: Color32) -> Color32 {
    if color == base {
        Color32::WHITE
    } else {
        color
    }
}

/// Reduce intensity of a color for the "dim" SGR attribute.
fn dim(color: Color32) -> Color32 {
    Color32::from_rgb(color.r() / 2, color.g() / 2, color.b() / 2)
}

/// Convert a pointer position (relative to the widget origin) into a grid cell
/// coordinate, used by input handling and selection.
pub fn cell_at_position(
    origin: Pos2,
    cell_size: Vec2,
    position: Pos2,
    columns: usize,
    lines: usize,
) -> (usize, usize) {
    let rel = position - origin;
    let col = ((rel.x / cell_size.x).floor() as usize).min(columns.saturating_sub(1));
    let line = ((rel.y / cell_size.y).floor() as usize).min(lines.saturating_sub(1));
    (col, line)
}
