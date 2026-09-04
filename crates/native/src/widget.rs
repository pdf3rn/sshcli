//! The reusable terminal widget + PTY-backed session.
//!
//! [`TerminalSession`] ties together a [`PtySession`] (owns the real local PTY
//! and its read loop) and a [`TermModel`] (the alacritty emulator state). It is
//! the unit later steps will embed per-tab and, eventually, rewire to an SSH
//! channel instead of a local PTY.
//!
//! `show()` renders the grid and handles all input: keyboard, special keys,
//! selection + clipboard, scrolling, search, clear actions, and resize. The
//! PTY write path and the emulator grid are fully owned here (no magic).

use egui::{text::LayoutJob, Align2, Color32, FontId, Key, Modifiers, Pos2, Sense, Vec2};

use crate::{
    render::{cell_at_position, render},
    session::{PtySession, SessionConfig, SessionState},
    term::TermModel,
};

/// Default terminal font size in points.
const DEFAULT_POINT_SIZE: f32 = 14.0;

/// A single terminal: a local PTY session plus its emulator grid and UI state.
pub struct TerminalSession {
    session: PtySession,
    model: TermModel,
    font: FontId,
    // Search UI state.
    search_open: bool,
    search_query: String,
    // Selection state.
    dragging: bool,
    have_selection: bool,
    // Scroll pixel accumulator for wheel smoothness.
    scroll_pixels: f32,
    // Cached cell size, for pointer→cell mapping.
    cell_size: Vec2,
}

impl TerminalSession {
    /// Create a new session using the detected local shell and a default size.
    pub fn new() -> Self {
        let config = SessionConfig::default();
        let session = PtySession::start(config).unwrap_or_else(|e| {
            log::error!("failed to start local shell: {e}");
            PtySession::start(SessionConfig::default()).expect("retry shell spawn")
        });
        let model = TermModel::new(80, 24);
        Self {
            session,
            model,
            font: FontId::monospace(DEFAULT_POINT_SIZE),
            search_open: false,
            search_query: String::new(),
            dragging: false,
            have_selection: false,
            scroll_pixels: 0.0,
            cell_size: Vec2::new(8.0, 16.0),
        }
    }

    /// Drain pending PTY output into the emulator, and update the closed state.
    /// Returns `true` if the session transitioned to `Closed` this frame.
    pub fn pump(&mut self) -> bool {
        let mut closed = false;
        while let Some(event) = self.session.try_recv() {
            match event {
                crate::session::SessionEvent::Output(bytes) => self.model.feed(&bytes),
                crate::session::SessionEvent::Closed => closed = true,
                crate::session::SessionEvent::SpawnFailed(e) => log::error!("spawn failed: {e}"),
            }
        }
        closed
    }

    /// Render and handle input for this terminal within `ui`.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // 1. Drain output (fast path for repaint).
        if self.pump() {
            ui.ctx().request_repaint();
        }

        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
        let origin = response.rect.min;

        // 2. Resize detection: propagate to PTY + emulator when the cell grid
        //    dimensions actually change.
        self.cell_size =
            ui.fonts(|f| Vec2::new(f.glyph_width(&self.font, 'M'), f.row_height(&self.font)));
        let cols = (available.x / self.cell_size.x).floor().max(1.0) as usize;
        let rows = (available.y / self.cell_size.y).floor().max(1.0) as usize;
        if cols != self.model.columns() || rows != self.model.screen_lines() {
            self.model.resize(cols, rows);
            let _ = self.session.resize(portable_pty::PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            });
        }

        // 3. Handle input events.
        self.handle_input(ui, &response, origin);

        // 4. Render the grid.
        let selection = self.model.selection_range();
        render(
            &self.model,
            selection,
            &painter,
            origin,
            self.font.clone(),
            available,
        );

        // 5. Overlay: search bar and closed/reconnect banner.
        if self.search_open {
            self.show_search_bar(ui);
        }
        self.overlay(ui, &response, origin, available);

        // Keep repainting while running (needed to poll PTY when idle).
        if self.session.state() == SessionState::Running {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    /// Handle keyboard, mouse, and clipboard events for the terminal.
    fn handle_input(&mut self, ui: &egui::Ui, response: &egui::Response, origin: Pos2) {
        let focused = response.has_focus() || response.clicked();
        let events = ui.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Text(text) => {
                    if focused {
                        let _ = self.session.write(text.as_bytes());
                    }
                }
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers,
                    ..
                } => {
                    if pressed && focused {
                        self.handle_key(key, modifiers, ui);
                    }
                }
                egui::Event::Copy => {
                    // Ctrl+C with a selection → copy; without → SIGINT to PTY.
                    if self.have_selection {
                        self.copy_selection(ui);
                    } else if focused {
                        let _ = self.session.write(&[0x03]);
                    }
                }
                egui::Event::Paste(text) => {
                    if focused {
                        let _ = self.session.write(text.as_bytes());
                    }
                }
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed,
                    pos,
                    ..
                } => {
                    self.handle_pointer(ui, pressed, pos, origin);
                }
                egui::Event::PointerMoved(pos) => {
                    if self.dragging {
                        let (col, line) = cell_at_position(
                            origin,
                            self.cell_size,
                            pos,
                            self.model.columns(),
                            self.model.screen_lines(),
                        );
                        self.model.update_selection(col, line);
                    }
                }
                egui::Event::MouseWheel {
                    unit,
                    delta,
                    modifiers: _,
                } => {
                    let lines = match unit {
                        egui::MouseWheelUnit::Line => delta.y as i32,
                        egui::MouseWheelUnit::Point => {
                            self.scroll_pixels += delta.y;
                            let n = (self.scroll_pixels / self.cell_size.y).trunc();
                            self.scroll_pixels -= n * self.cell_size.y;
                            n as i32
                        }
                        _ => 0,
                    };
                    if lines != 0 {
                        self.model.scroll(lines);
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_key(&mut self, key: Key, modifiers: Modifiers, ui: &egui::Ui) {
        use egui::Key as K;
        let ctrl = modifiers.ctrl;
        let shift = modifiers.shift;

        // Global shortcuts (documented in README).
        if ctrl && shift {
            match key {
                K::C => {
                    self.copy_selection(ui);
                    return;
                }
                K::V => {
                    self.paste(ui);
                    return;
                }
                K::F => {
                    self.search_open = true;
                    self.search_query.clear();
                    return;
                }
                _ => {}
            }
        }
        if ctrl && !shift {
            match key {
                K::L => {
                    // Clear screen: ED(2) + cursor home.
                    let _ = self.session.write(b"\x1b[2J\x1b[H");
                    self.model.send_ansi(b"\x1b[2J\x1b[H");
                    return;
                }
                K::C => {
                    // SIGINT (already handled by Copy event fallback too).
                    let _ = self.session.write(&[0x03]);
                    return;
                }
                _ => {}
            }
        }

        // Character-producing keys (A..Z, digits, punctuation) are delivered
        // via Text events, so only non-char keys are mapped here.
        let seq: Option<&[u8]> = match key {
            K::Enter => Some(b"\r"),
            K::Backspace => Some(b"\x7f"),
            K::Tab => Some(if shift {
                b"\x1b[Z".as_ref()
            } else {
                b"\t".as_ref()
            }),
            K::Escape => Some(b"\x1b"),
            K::ArrowUp => Some(b"\x1b[A"),
            K::ArrowDown => Some(b"\x1b[B"),
            K::ArrowRight => Some(b"\x1b[C"),
            K::ArrowLeft => Some(b"\x1b[D"),
            K::Home => Some(b"\x1b[H"),
            K::End => Some(b"\x1b[F"),
            K::Delete => Some(b"\x1b[3~"),
            K::Insert => Some(b"\x1b[2~"),
            K::PageUp => {
                self.model.scroll(self.model.screen_lines() as i32);
                None
            }
            K::PageDown => {
                self.model.scroll(-(self.model.screen_lines() as i32));
                None
            }
            _ => None,
        };

        if let Some(seq) = seq {
            let _ = self.session.write(seq);
        }
    }

    fn handle_pointer(&mut self, ui: &egui::Ui, pressed: bool, pos: Pos2, origin: Pos2) {
        let (col, line) = cell_at_position(
            origin,
            self.cell_size,
            pos,
            self.model.columns(),
            self.model.screen_lines(),
        );
        if pressed {
            self.dragging = true;
            self.have_selection = false;
            self.model.start_selection(
                alacritty_terminal::selection::SelectionType::Simple,
                col,
                line,
            );
            let _ = ui;
        } else {
            self.dragging = false;
            self.have_selection = true;
        }
    }

    fn copy_selection(&mut self, ui: &egui::Ui) {
        if let Some(text) = self.model.selected_text() {
            ui.ctx().copy_text(text);
        }
    }

    fn paste(&mut self, ui: &egui::Ui) {
        // Clipboard paste is delivered via egui's Event::Paste and handled in
        // handle_input(); this is a no-op kept for shortcut completeness.
        let _ = ui;
    }

    fn scroll_to_point(&mut self, point: alacritty_terminal::index::Point) {
        // When a match is found above the viewport, scroll up to reveal it.
        let viewport_top = -(self.model.display_offset() as i32);
        if point.line.0 < viewport_top {
            self.model.scroll(viewport_top - point.line.0 + 1);
        }
    }

    fn overlay(&mut self, ui: &egui::Ui, response: &egui::Response, origin: Pos2, available: Vec2) {
        let _ = (origin, available);
        if self.session.state() == SessionState::Closed {
            let painter = ui.painter();
            let rect = response.rect;
            painter.rect_filled(rect, 0.0, Color32::from_black_alpha(180));
            painter.text(
                rect.center() - Vec2::new(0.0, 12.0),
                Align2::CENTER_CENTER,
                "Session closed",
                self.font.clone(),
                Color32::WHITE,
            );
            painter.text(
                rect.center() + Vec2::new(0.0, 12.0),
                Align2::CENTER_CENTER,
                "Press R to reconnect, or click Restart",
                FontId::proportional(12.0),
                Color32::LIGHT_GRAY,
            );
            if ui.input(|i| i.key_pressed(Key::R)) {
                let _ = self.session.restart();
            }
        }
    }

    /// Draw the search bar overlay (called during `show` when open).
    fn show_search_bar(&mut self, ui: &mut egui::Ui) {
        let mut close = false;
        let mut find = false;
        egui::Area::new(egui::Id::new("sshcli-search-bar"))
            .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-8.0, 8.0))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        let edit = egui::TextEdit::singleline(&mut self.search_query)
                            .desired_width(220.0)
                            .font(self.font.clone());
                        let response = ui.add(edit);
                        response.request_focus();
                        if ui.button("Find next").clicked() {
                            find = true;
                        }
                        if ui.button("Close").clicked() || ui.input(|i| i.key_pressed(Key::Escape))
                        {
                            close = true;
                        }
                    });
                });
            });

        if ui.input(|i| i.key_pressed(Key::Enter)) {
            find = true;
        }

        if find {
            let query = self.search_query.trim().to_string();
            if !query.is_empty() && self.model.set_search(&query).is_ok() {
                if let Some((_range, point)) = self.model.find_next() {
                    self.scroll_to_point(point);
                }
            }
        }
        if close {
            self.search_open = false;
            self.search_query.clear();
        }
    }
}

impl Default for TerminalSession {
    fn default() -> Self {
        Self::new()
    }
}

/// A reusable per-cell color renderer for arbitrary text (kept as a public
/// utility for tests and future SSH-channel renderers).
#[allow(dead_code)]
pub fn cell_layout_job(line: &str, color: Color32, font: FontId) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        line,
        0.0,
        egui::text::TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    job
}
