//! The `eframe` application: a window with dockable terminal tabs.
//!
//! `NativeApp` drives an `egui_dock::DockState` of [`Tab`]s. Each session tab
//! owns a [`crate::widget::TerminalSession`] (backed by a local PTY or an SSH
//! channel behind the same [`crate::transport::SessionTransport`]). Tabs can be
//! split, reordered, closed (with confirmation), and — once closed — reconnected.
//!
//! A top panel offers "New local terminal" and a "New SSH connection" form.

use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex, TabIndex};

use crate::{
    session::SessionState,
    ssh::{SshConnect, SshTransport},
    widget::TerminalSession,
};

/// A single dockable tab: a named terminal session (local or SSH), or the
/// About page.
pub enum Tab {
    Session {
        id: usize,
        label: String,
        session: TerminalSession,
    },
    About,
}

impl Tab {
    fn new_local(id: usize, label: &str) -> Self {
        Tab::Session {
            id,
            label: label.to_string(),
            session: TerminalSession::new_local(),
        }
    }

    fn new_ssh(id: usize, transport: SshTransport) -> Self {
        let label = transport.label().to_string();
        Tab::Session {
            id,
            label,
            session: TerminalSession::with_transport(Box::new(transport)),
        }
    }

    /// The tab id (only session tabs have one).
    fn id(&self) -> Option<usize> {
        match self {
            Tab::Session { id, .. } => Some(*id),
            Tab::About => None,
        }
    }

    fn title(&self) -> String {
        match self {
            Tab::Session { label, session, .. } => {
                if session.state() == SessionState::Closed {
                    format!("{label} (closed)")
                } else {
                    label.clone()
                }
            }
            Tab::About => "About".to_string(),
        }
    }
}

/// Pending SSH connection form state.
#[derive(Default)]
struct SshForm {
    /// `user@host[:port]` ad-hoc target, or a saved profile name.
    target: String,
    /// Optional password override / ad-hoc password.
    password: String,
    /// `true` to treat `target` as an ad-hoc target, `false` as a profile name.
    adhoc: bool,
    /// Last connection error, shown inline.
    error: Option<String>,
}

/// The top-level application state.
pub struct NativeApp {
    dock: DockState<Tab>,
    next_id: usize,
    ssh_form: SshForm,
    /// Tab id awaiting close confirmation.
    pending_close: Option<usize>,
}

impl NativeApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        env_logger::init();
        let dock = DockState::new(vec![
            Tab::new_local(0, "Terminal"),
            Tab::new_local(1, "Terminal 2"),
            Tab::About,
        ]);
        Self {
            dock,
            next_id: 2,
            ssh_form: SshForm::default(),
            pending_close: None,
        }
    }

    fn add_local_tab(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        let label = format!("Terminal {}", id + 1);
        self.dock.push_to_focused_leaf(Tab::new_local(id, &label));
    }

    fn add_ssh_tab(&mut self) {
        let target = self.ssh_form.target.trim().to_string();
        if target.is_empty() {
            self.ssh_form.error = Some("enter a host or profile name".to_string());
            return;
        }
        let password = {
            let p = self.ssh_form.password.trim().to_string();
            if p.is_empty() {
                None
            } else {
                Some(p)
            }
        };
        let connect = if self.ssh_form.adhoc {
            SshConnect::Adhoc { target, password }
        } else {
            SshConnect::Profile {
                name: target,
                password,
            }
        };
        match SshTransport::connect(&connect, 80, 24) {
            Ok(transport) => {
                let id = self.next_id;
                self.next_id += 1;
                self.dock.push_to_focused_leaf(Tab::new_ssh(id, transport));
                self.ssh_form.error = None;
                self.ssh_form.target.clear();
                self.ssh_form.password.clear();
            }
            Err(e) => self.ssh_form.error = Some(e),
        }
    }

    /// Remove a tab by id, closing its transport first (the drop runs its
    /// `Drop` impl: PTY kill / SSH channel close).
    fn remove_tab(&mut self, id: usize) {
        if let Some((surface, node, tab_index)) = self.find_tab(id) {
            if let Some(tab) = self.dock.remove_tab((surface, node, tab_index)) {
                drop(tab);
            }
        }
    }

    fn find_tab(&self, id: usize) -> Option<(SurfaceIndex, NodeIndex, TabIndex)> {
        for ((surface, node), tab) in self.dock.iter_all_tabs() {
            if tab.id() == Some(id) {
                if let Some(tree) = self.dock.get_surface(surface).and_then(|s| s.node_tree()) {
                    if let Some(tabs) = tree[node].tabs() {
                        for (index, t) in tabs.iter().enumerate() {
                            if t.id() == Some(id) {
                                return Some((surface, node, TabIndex(index)));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Show the close-confirmation dialog if one is pending.
    fn close_confirm(&mut self, ctx: &egui::Context) {
        let Some(id) = self.pending_close else {
            return;
        };
        let mut confirm = false;
        let mut cancel = false;
        let mut open = true;
        egui::Window::new("Close session?")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("The session is still running. Close it anyway?");
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        confirm = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if confirm {
            self.pending_close = None;
            self.remove_tab(id);
        } else if cancel || !open {
            self.pending_close = None;
        }
    }
}

struct AppTabViewer<'a> {
    pending_close: &'a mut Option<usize>,
}

impl egui_dock::TabViewer for AppTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        match tab {
            Tab::Session { id, .. } => egui::Id::new(("session", *id)),
            Tab::About => egui::Id::new("about"),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Session { session, .. } => session.show(ui),
            Tab::About => {
                ui.heading("sshcli-native");
                ui.label(
                    "Native terminal frontend (eframe + alacritty_terminal + portable-pty/russh).",
                );
                ui.separator();
                ui.label("Keyboard shortcuts:");
                for (keys, desc) in crate::app::shortcuts() {
                    ui.label(format!("{keys:<22} {desc}"));
                }
                ui.separator();
                ui.label(
                    "Terminal tabs spawn your local shell in a PTY, or an SSH shell over russh.",
                );
                ui.label("Drag a tab to split the view; close a tab with its ✕ button.");
            }
        }
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        match tab {
            Tab::Session { id, session, .. } => {
                if session.state() == SessionState::Closed {
                    true
                } else {
                    *self.pending_close = Some(*id);
                    false
                }
            }
            Tab::About => true,
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

        egui::TopBottomPanel::top("sshcli-native-top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New local terminal").clicked() {
                    self.add_local_tab();
                }
            });
        });

        egui::TopBottomPanel::top("sshcli-native-ssh-form").show(ctx, |ui| {
            egui::CollapsingHeader::new("New SSH connection")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Host:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ssh_form.target)
                                .hint_text("user@host[:port] or profile name")
                                .desired_width(240.0),
                        );
                        ui.label("Password:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ssh_form.password)
                                .password(true)
                                .hint_text("(optional)")
                                .desired_width(160.0),
                        );
                        ui.checkbox(&mut self.ssh_form.adhoc, "adhoc");
                        if ui.button("Connect").clicked() {
                            self.add_ssh_tab();
                        }
                    });
                    if let Some(err) = &self.ssh_form.error {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut viewer = AppTabViewer {
                pending_close: &mut self.pending_close,
            };
            DockArea::new(&mut self.dock)
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut viewer);
        });

        self.close_confirm(ctx);
    }
}
