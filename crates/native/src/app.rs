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

use sshcli_app::{commands as app_commands, sftp_init, telemetry_init, tunnel_init};

use crate::{
    host_key::{self, HostKeyRequest},
    panels::{SftpView, TelemetryView, TunnelView},
    profiles_view::{ProfilesAction, ProfilesView},
    session::SessionState,
    ssh::{SshConnect, SshTransport},
    widget::TerminalSession,
};

/// A single dockable tab: a named terminal session (local or SSH), the
/// profiles view, an SFTP browser, a tunnel panel, a telemetry panel, or the
/// About page.
pub enum Tab {
    Session {
        id: usize,
        label: String,
        session: TerminalSession,
    },
    Profiles,
    Sftp(SftpView),
    Tunnels(TunnelView),
    Telemetry(TelemetryView),
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
            Tab::Profiles | Tab::Sftp(_) | Tab::Tunnels(_) | Tab::Telemetry(_) | Tab::About => None,
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
            Tab::Profiles => "Conexiones".to_string(),
            Tab::Sftp(view) => format!("SFTP · {}", view.profile()),
            Tab::Tunnels(view) => format!("Túneles · {}", view.profile()),
            Tab::Telemetry(view) => format!("Telemetría · {}", view.profile()),
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

/// A pending host-key confirmation, together with the connect request to retry
/// once the user trusts the key.
struct PendingHostKey {
    connect: SshConnect,
    request: HostKeyRequest,
}

/// The top-level application state.
pub struct NativeApp {
    dock: DockState<Tab>,
    next_id: usize,
    ssh_form: SshForm,
    /// Tab id awaiting close confirmation.
    pending_close: Option<usize>,
    profiles: ProfilesView,
    pending_host_key: Option<PendingHostKey>,
    /// Shared tokio runtime for async SFTP/tunnel/telemetry operations.
    runtime: std::sync::Arc<tokio::runtime::Runtime>,
    sftp_state: sshcli_app::SftpState,
    tunnel_state: sshcli_app::TunnelState,
    telemetry_state: sshcli_app::TelemetryState,
}

impl NativeApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        env_logger::init();
        let runtime = std::sync::Arc::new(tokio::runtime::Runtime::new().expect("tokio runtime"));
        let dock = DockState::new(vec![
            Tab::new_local(0, "Terminal"),
            Tab::new_local(1, "Terminal 2"),
            Tab::Profiles,
            Tab::About,
        ]);
        Self {
            dock,
            next_id: 2,
            ssh_form: SshForm::default(),
            pending_close: None,
            profiles: ProfilesView::default(),
            pending_host_key: None,
            runtime,
            sftp_state: sftp_init(),
            tunnel_state: tunnel_init(),
            telemetry_state: telemetry_init(),
        }
    }

    fn add_sftp_tab(&mut self, profile: String) {
        self.dock.push_to_focused_leaf(Tab::Sftp(SftpView::new(
            profile,
            self.runtime.clone(),
            self.sftp_state.clone(),
        )));
    }

    fn add_tunnels_tab(&mut self, profile: String) {
        self.dock.push_to_focused_leaf(Tab::Tunnels(TunnelView::new(
            profile,
            self.runtime.clone(),
            self.tunnel_state.clone(),
        )));
    }

    fn add_telemetry_tab(&mut self, profile: String) {
        self.dock
            .push_to_focused_leaf(Tab::Telemetry(TelemetryView::new(
                profile,
                self.runtime.clone(),
                self.telemetry_state.clone(),
            )));
    }

    fn add_local_tab(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        let label = format!("Terminal {}", id + 1);
        self.dock.push_to_focused_leaf(Tab::new_local(id, &label));
    }

    /// Attempt to connect an SSH transport. On a host-key rejection, stash the
    /// pending request (to be confirmed by the user) and return `Ok(None)`.
    fn connect_ssh(&mut self, connect: SshConnect) -> Result<Option<SshTransport>, String> {
        match SshTransport::connect(&connect, 80, 24) {
            Ok(transport) => Ok(Some(transport)),
            Err(e) => match HostKeyRequest::from_error(&e) {
                Some(request) => {
                    self.pending_host_key = Some(PendingHostKey { connect, request });
                    Ok(None)
                }
                None => Err(e),
            },
        }
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
        match self.connect_ssh(connect) {
            Ok(Some(transport)) => {
                let id = self.next_id;
                self.next_id += 1;
                self.dock.push_to_focused_leaf(Tab::new_ssh(id, transport));
                self.ssh_form.error = None;
                self.ssh_form.target.clear();
                self.ssh_form.password.clear();
            }
            Ok(None) => {
                // Host-key confirmation pending; form kept for retry on trust.
                self.ssh_form.error = None;
            }
            Err(e) => self.ssh_form.error = Some(e),
        }
    }

    /// Connect to a profile by name (from the profiles view).
    fn connect_profile(&mut self, name: String) {
        let connect = SshConnect::Profile {
            name,
            password: None,
        };
        match self.connect_ssh(connect) {
            Ok(Some(transport)) => {
                let id = self.next_id;
                self.next_id += 1;
                self.dock.push_to_focused_leaf(Tab::new_ssh(id, transport));
            }
            Ok(None) => {}
            Err(_e) => {
                // Connection error is surfaced in the profiles view via its
                // own error path; store nothing here.
            }
        }
    }

    /// Show and handle the host-key confirmation dialog, if pending.
    fn host_key_dialog(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_host_key.take() else {
            return;
        };
        match host_key::show(ctx, &pending.request) {
            host_key::HostKeyDecision::Trust => {
                if let Err(_) = app_commands::ssh_trust_host_key(
                    pending.request.host.clone(),
                    pending.request.port,
                    pending.request.key.clone(),
                ) {
                    // If trusting failed, surface a generic error and abandon.
                    self.pending_host_key = None;
                    return;
                }
                // Retry the connection now that the key is trusted.
                match SshTransport::connect(&pending.connect, 80, 24) {
                    Ok(transport) => {
                        let id = self.next_id;
                        self.next_id += 1;
                        self.dock.push_to_focused_leaf(Tab::new_ssh(id, transport));
                        self.ssh_form.error = None;
                    }
                    Err(e) => self.ssh_form.error = Some(e),
                }
            }
            host_key::HostKeyDecision::Cancel | host_key::HostKeyDecision::Pending => {
                self.pending_host_key = Some(pending);
            }
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
    profiles: &'a mut ProfilesView,
    actions: Vec<ProfilesAction>,
}

impl egui_dock::TabViewer for AppTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        match tab {
            Tab::Session { id, .. } => egui::Id::new(("session", *id)),
            Tab::Profiles => egui::Id::new("profiles"),
            Tab::Sftp(view) => egui::Id::new(("sftp", view.profile().to_string())),
            Tab::Tunnels(view) => egui::Id::new(("tunnels", view.profile().to_string())),
            Tab::Telemetry(view) => egui::Id::new(("telemetry", view.profile().to_string())),
            Tab::About => egui::Id::new("about"),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Session { session, .. } => session.show(ui),
            Tab::Profiles => {
                let mut actions = self.profiles.show(ui);
                self.actions.append(&mut actions);
            }
            Tab::Sftp(view) => view.show(ui),
            Tab::Tunnels(view) => view.show(ui),
            Tab::Telemetry(view) => view.show(ui),
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
            Tab::Profiles | Tab::Sftp(_) | Tab::Tunnels(_) | Tab::Telemetry(_) | Tab::About => true,
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
                if ui.button("Profiles").clicked() {
                    self.dock.push_to_focused_leaf(Tab::Profiles);
                }
                if ui.button("About").clicked() {
                    self.dock.push_to_focused_leaf(Tab::About);
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
                profiles: &mut self.profiles,
                actions: Vec::new(),
            };
            DockArea::new(&mut self.dock)
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut viewer);

            for action in viewer.actions.drain(..) {
                match action {
                    ProfilesAction::Connect(name) => self.connect_profile(name),
                    ProfilesAction::OpenSftp(name) => self.add_sftp_tab(name),
                    ProfilesAction::OpenTunnels(name) => self.add_tunnels_tab(name),
                    ProfilesAction::OpenTelemetry(name) => self.add_telemetry_tab(name),
                }
            }
        });

        self.profiles.show_modal(ctx);
        self.host_key_dialog(ctx);
        self.close_confirm(ctx);
    }
}
