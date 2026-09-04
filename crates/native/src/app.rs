//! The `eframe` application: a window with a dockable terminal tab.
//!
//! This is the native app seed. The terminal is rendered via
//! [`crate::widget::TerminalSession`], which owns the local PTY and emulator.
//! `egui_dock` provides the tab/docking chrome so that later steps can add
//! more terminal tabs (and, eventually, SSH sessions) without UI restructuring.

use eframe::egui;
use egui_dock::{DockArea, DockState};

use crate::widget::TerminalSession;

/// Tab identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tab {
    Terminal,
    About,
}

impl Tab {
    fn title(&self) -> &'static str {
        match self {
            Tab::Terminal => "Terminal",
            Tab::About => "About",
        }
    }
}

/// The top-level application state.
pub struct NativeApp {
    terminal: TerminalSession,
    dock: DockState<Tab>,
}

impl NativeApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        env_logger::init();
        Self {
            terminal: TerminalSession::new(),
            dock: DockState::new(vec![Tab::Terminal, Tab::About]),
        }
    }
}

struct AppTabViewer<'a> {
    terminal: &'a mut TerminalSession,
}

impl egui_dock::TabViewer for AppTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Terminal => {
                self.terminal.show(ui);
            }
            Tab::About => {
                ui.heading("sshcli-native");
                ui.label("Native terminal frontend (eframe + alacritty_terminal + portable-pty).");
                ui.separator();
                ui.label("Keyboard shortcuts:");
                for (keys, desc) in crate::app::shortcuts() {
                    ui.label(format!("{keys:<22} {desc}"));
                }
                ui.separator();
                ui.label("The Terminal tab spawns your local shell in a PTY.");
                ui.label("Resize the window to resize the terminal; scroll to view scrollback.");
            }
        }
    }
}

/// Documented keyboard shortcuts (reused by the About tab and tests).
pub fn shortcuts() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Enter", "submit input (CR)"),
        ("Backspace", "delete previous char"),
        (
            "Arrow keys / Home / End",
            "cursor movement (VT100 sequences)",
        ),
        ("Ctrl+C", "SIGINT (interrupt)"),
        ("Ctrl+L", "clear screen"),
        ("Ctrl+Shift+C", "copy selection to clipboard"),
        ("Ctrl+Shift+V", "paste from clipboard"),
        ("Ctrl+Shift+F", "search within scrollback (find next)"),
        ("PageUp / PageDown", "scroll scrollback"),
        ("Mouse wheel", "scroll scrollback"),
        ("Mouse drag", "select text"),
        ("R (when closed)", "restart / reconnect session"),
    ]
}

impl eframe::App for NativeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll terminal output even when no other input occurs.
        ctx.request_repaint_after(std::time::Duration::from_millis(50));

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut viewer = AppTabViewer {
                terminal: &mut self.terminal,
            };
            DockArea::new(&mut self.dock)
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut viewer);
        });
    }
}
