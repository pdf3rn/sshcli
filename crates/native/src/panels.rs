//! Native SFTP, tunnel, and telemetry panels.
//!
//! All persistence and I/O delegate to the `sshcli-app` service layer (the
//! Tauri-free managers extracted in step 1). This module owns only presentation
//! state and runs the async operations on a shared tokio runtime.

use std::sync::Arc;

use eframe::egui;
use sshcli_app::{
    events::{NullEmitter, SharedEmitter},
    sftp_session::{self, SftpState},
    telemetry::{self, TelemetrySample, TelemetryState},
    tunnel::{self, TunnelInfo, TunnelState},
};

/// A shared no-op emitter for operations that don't need progress events
/// (the native UI polls/refreshes directly instead of subscribing to the Tauri
/// event stream).
fn null_emitter() -> SharedEmitter {
    Arc::new(NullEmitter)
}

/// Format a byte count the way `SftpPanel.tsx`'s `fmtSize` does:
/// `<1024` → `"N B"`, else `KB`/`MB`/`GB` with one decimal.
pub fn fmt_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = "KB";
    for next in ["MB", "GB"] {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = next;
    }
    format!("{value:.1} {unit}")
}

/// Format a KB/s rate the way `TelemetryPanel.tsx`'s `fmtRate` does.
pub fn format_rate(kbps: f64) -> String {
    if kbps >= 1024.0 {
        format!("{:.1} MB/s", kbps / 1024.0)
    } else {
        format!("{kbps:.1} KB/s")
    }
}

/// A local or remote directory entry (rendered identically per pane).

/// A dual-pane SFTP browser for a single profile, mirroring `SftpPanel.tsx`.
pub struct SftpView {
    profile: String,
    runtime: Arc<tokio::runtime::Runtime>,
    state: SftpState,
    session_id: Option<String>,

    remote_path: String,
    remote_entries: Vec<sftp_session::SftpEntry>,
    local_path: String,
    local_entries: Vec<sftp_session::LocalEntry>,

    message: Option<String>,
    busy: bool,
}

impl SftpView {
    pub fn new(profile: String, runtime: Arc<tokio::runtime::Runtime>, state: SftpState) -> Self {
        Self {
            profile,
            runtime,
            state,
            session_id: None,
            remote_path: String::new(),
            remote_entries: Vec::new(),
            local_path: String::new(),
            local_entries: Vec::new(),
            message: None,
            busy: false,
        }
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    fn connect(&mut self) {
        let profile = self.profile.clone();
        let state = self.state.clone();
        let runtime = self.runtime.clone();
        match runtime.block_on(sftp_session::sftp_connect(&state, profile.clone(), None)) {
            Ok(id) => {
                let home = runtime
                    .block_on(sftp_session::sftp_pwd(&state, id.clone()))
                    .ok();
                let local_home = sftp_session::local_home().ok();
                self.session_id = Some(id.clone());
                if let Some(home) = home {
                    self.remote_path = home.clone();
                    if let Ok(entries) =
                        runtime.block_on(sftp_session::sftp_list_dir(&state, id.clone(), home))
                    {
                        self.remote_entries = entries;
                    }
                }
                if let Some(local) = local_home {
                    self.local_path = local.clone();
                    if let Ok(entries) = sftp_session::list_local_dir(local) {
                        self.local_entries = entries;
                    }
                }
            }
            Err(e) => self.message = Some(e),
        }
    }

    fn refresh_remote(&mut self, path: String) {
        let Some(id) = self.session_id.clone() else {
            return;
        };
        match self
            .runtime
            .block_on(sftp_session::sftp_list_dir(&self.state, id, path.clone()))
        {
            Ok(entries) => {
                self.remote_entries = entries;
                self.remote_path = path;
            }
            Err(e) => self.message = Some(e),
        }
    }

    fn refresh_local(&mut self, path: String) {
        match sftp_session::list_local_dir(path.clone()) {
            Ok(entries) => {
                self.local_entries = entries;
                self.local_path = path;
            }
            Err(e) => self.message = Some(e),
        }
    }

    fn open_remote_dir(&mut self, name: &str) {
        let base = self.remote_path.trim_end_matches('/');
        let path = if base.is_empty() || base == "/" {
            format!("/{name}")
        } else {
            format!("{base}/{name}")
        };
        self.refresh_remote(path);
    }

    fn open_local_dir(&mut self, name: &str) {
        let base = self.local_path.trim_end_matches(['/', '\\']);
        let path = if base.is_empty() {
            name.to_string()
        } else {
            format!("{base}/{name}")
        };
        self.refresh_local(path);
    }

    fn download(&mut self, entry: &sftp_session::SftpEntry) {
        let Some(id) = self.session_id.clone() else {
            return;
        };
        let remote = format!("{}/{}", self.remote_path.trim_end_matches('/'), entry.name);
        let local = format!(
            "{}/{}",
            self.local_path.trim_end_matches(['/', '\\']),
            entry.name
        );
        self.busy = true;
        let result = self.runtime.block_on(sftp_session::sftp_download(
            &null_emitter(),
            &self.state,
            id,
            remote.clone(),
            local,
        ));
        self.busy = false;
        match result {
            Ok(()) => {
                self.message = Some(format!("Descargado {}", entry.name));
                self.refresh_local(self.local_path.clone());
            }
            Err(e) => self.message = Some(e),
        }
    }

    fn upload(&mut self, entry: &sftp_session::LocalEntry) {
        let Some(id) = self.session_id.clone() else {
            return;
        };
        let local = format!(
            "{}/{}",
            self.local_path.trim_end_matches(['/', '\\']),
            entry.name
        );
        let remote = format!("{}/{}", self.remote_path.trim_end_matches('/'), entry.name);
        self.busy = true;
        let result = self.runtime.block_on(sftp_session::sftp_upload(
            &null_emitter(),
            &self.state,
            id,
            local,
            remote,
        ));
        self.busy = false;
        match result {
            Ok(()) => {
                self.message = Some(format!("Subido {}", entry.name));
                self.refresh_remote(self.remote_path.clone());
            }
            Err(e) => self.message = Some(e),
        }
    }

    fn delete(&mut self, entry: &sftp_session::SftpEntry) {
        let Some(id) = self.session_id.clone() else {
            return;
        };
        let path = format!("{}/{}", self.remote_path.trim_end_matches('/'), entry.name);
        let result = if entry.is_dir {
            self.runtime
                .block_on(sftp_session::sftp_rm_dir(&self.state, id, path))
        } else {
            self.runtime
                .block_on(sftp_session::sftp_rm_file(&self.state, id, path))
        };
        match result {
            Ok(()) => {
                self.message = Some(format!("Borrado {}", entry.name));
                self.refresh_remote(self.remote_path.clone());
            }
            Err(e) => self.message = Some(e),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        if self.session_id.is_none() && self.message.is_none() {
            self.connect();
        }

        ui.horizontal(|ui| {
            ui.heading(format!("SFTP · {}", self.profile));
            if let Some(msg) = self.message.clone() {
                ui.colored_label(egui::Color32::LIGHT_YELLOW, msg);
                if ui.button("cerrar").clicked() {
                    self.message = None;
                }
            }
        });
        ui.add_space(4.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.busy {
                ui.label("Transferencia en curso…");
                ui.separator();
            }

            ui.columns(2, |cols| {
                // Local pane.
                cols[0].heading("Local");
                cols[0].horizontal(|ui| {
                    if ui.button("↑").on_hover_text("Subir").clicked() {
                        let parent = parent_local_path(&self.local_path);
                        self.refresh_local(parent);
                    }
                    ui.label(&self.local_path);
                });
                if self.local_entries.is_empty() {
                    cols[0].weak("Carpeta vacía");
                } else {
                    let entries: Vec<sftp_session::LocalEntry> = self.local_entries.clone();
                    for entry in entries {
                        let icon = if entry.is_dir { "📁" } else { "📄" };
                        cols[0].horizontal(|ui| {
                            if entry.is_dir {
                                if ui.link(format!("{icon} {}", entry.name)).clicked() {
                                    self.open_local_dir(&entry.name);
                                }
                            } else {
                                ui.label(format!("{icon} {}", entry.name));
                                ui.label(fmt_size(entry.size));
                                if ui.small_button("↑ Subir").clicked() {
                                    self.upload(&entry);
                                }
                            }
                        });
                    }
                }

                // Remote pane.
                cols[1].heading("Remoto");
                cols[1].horizontal(|ui| {
                    if ui.button("↑").on_hover_text("Subir nivel").clicked() {
                        let parent = parent_remote_path(&self.remote_path);
                        self.refresh_remote(parent);
                    }
                    ui.label(&self.remote_path);
                });
                if self.remote_entries.is_empty() {
                    cols[1].weak("Carpeta vacía");
                } else {
                    let entries: Vec<sftp_session::SftpEntry> = self.remote_entries.clone();
                    for entry in entries {
                        let icon = if entry.is_dir { "📁" } else { "📄" };
                        cols[1].horizontal(|ui| {
                            if entry.is_dir {
                                if ui.link(format!("{icon} {}", entry.name)).clicked() {
                                    self.open_remote_dir(&entry.name);
                                }
                            } else {
                                ui.label(format!("{icon} {}", entry.name));
                                ui.label(fmt_size(entry.size));
                                if ui.small_button("↓ Descargar").clicked() {
                                    self.download(&entry);
                                }
                            }
                            if ui.small_button("✕").on_hover_text("Borrar").clicked() {
                                self.delete(&entry);
                            }
                        });
                    }
                }
            });
        });
    }
}

/// Compute the parent of a local path (Windows/Unix aware).
fn parent_local_path(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        return ".".to_string();
    }
    match trimmed.rsplit_once(['/', '\\']) {
        Some((parent, _)) => {
            if parent.is_empty() {
                trimmed[..1].to_string()
            } else {
                parent.to_string()
            }
        }
        None => ".".to_string(),
    }
}

/// Compute the parent of a remote (Unix) path.
fn parent_remote_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    match trimmed.rsplit_once('/') {
        Some((parent, _)) => {
            if parent.is_empty() {
                "/".to_string()
            } else {
                parent.to_string()
            }
        }
        None => ".".to_string(),
    }
}

/// A tunnel manager panel, mirroring `TunnelPanel.tsx`.
pub struct TunnelView {
    profile: String,
    runtime: Arc<tokio::runtime::Runtime>,
    state: TunnelState,
    tunnels: Vec<TunnelInfo>,
    bind_port: String,
    target_host: String,
    target_port: String,
    message: Option<String>,
}

impl TunnelView {
    pub fn new(profile: String, runtime: Arc<tokio::runtime::Runtime>, state: TunnelState) -> Self {
        Self {
            profile,
            runtime,
            state,
            tunnels: Vec::new(),
            bind_port: "8080".to_string(),
            target_host: "127.0.0.1".to_string(),
            target_port: "80".to_string(),
            message: None,
        }
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    fn refresh(&mut self) {
        match tunnel::tunnel_list(&self.state) {
            Ok(tunnels) => self.tunnels = tunnels,
            Err(e) => self.message = Some(e),
        }
    }

    fn start(&mut self) {
        let Ok(bind_port) = self.bind_port.trim().parse::<u16>() else {
            self.message = Some("Puerto local inválido".into());
            return;
        };
        let Ok(target_port) = self.target_port.trim().parse::<u16>() else {
            self.message = Some("Puerto destino inválido".into());
            return;
        };
        let result = self.runtime.block_on(tunnel::tunnel_start(
            &self.state,
            self.profile.clone(),
            None,
            "127.0.0.1".to_string(),
            bind_port,
            self.target_host.trim().to_string(),
            target_port,
        ));
        match result {
            Ok(_) => {
                self.message = Some("Túnel iniciado".into());
                self.refresh();
            }
            Err(e) => self.message = Some(e),
        }
    }

    fn stop(&mut self, id: &str) {
        match self
            .runtime
            .block_on(tunnel::tunnel_stop(&self.state, id.to_string()))
        {
            Ok(()) => self.refresh(),
            Err(e) => self.message = Some(e),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        if self.tunnels.is_empty() {
            // Lazy initial refresh is cheap and safe; do it once per open.
        }
        ui.heading(format!("Túneles · {}", self.profile));
        ui.horizontal(|ui| {
            ui.label("Puerto local:");
            ui.add(egui::TextEdit::singleline(&mut self.bind_port).desired_width(60.0));
            ui.label("Host destino:");
            ui.text_edit_singleline(&mut self.target_host);
            ui.label("Puerto destino:");
            ui.add(egui::TextEdit::singleline(&mut self.target_port).desired_width(60.0));
            if ui.button("Iniciar").clicked() {
                self.start();
            }
            if ui.button("Actualizar").clicked() {
                self.refresh();
            }
        });

        if let Some(msg) = self.message.clone() {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::LIGHT_YELLOW, msg);
                if ui.button("cerrar").clicked() {
                    self.message = None;
                }
            });
        }

        ui.separator();
        if self.tunnels.is_empty() {
            ui.weak("No hay túneles activos.");
        }
        let tunnels = self.tunnels.clone();
        for tunnel in tunnels {
            ui.horizontal(|ui| {
                ui.label(format!("● {} → {}", tunnel.local, tunnel.target));
                if ui.button("Detener").clicked() {
                    self.stop(&tunnel.id);
                }
            });
        }
    }
}

/// A host metrics panel, mirroring `TelemetryPanel.tsx`.
pub struct TelemetryView {
    profile: String,
    runtime: Arc<tokio::runtime::Runtime>,
    state: TelemetryState,
    sample: Option<TelemetrySample>,
    error: Option<String>,
}

impl TelemetryView {
    pub fn new(
        profile: String,
        runtime: Arc<tokio::runtime::Runtime>,
        state: TelemetryState,
    ) -> Self {
        Self {
            profile,
            runtime,
            state,
            sample: None,
            error: None,
        }
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    fn sample(&mut self) {
        match self.runtime.block_on(telemetry::telemetry_sample(
            &self.state,
            self.profile.clone(),
            None,
        )) {
            Ok(sample) => {
                self.sample = Some(sample);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Host Telemetry");
        ui.label(&self.profile);
        ui.separator();

        if ui.button("Actualizar").clicked() {
            self.sample();
        }

        match &self.error {
            Some(_) => {
                ui.weak("No disponible");
            }
            None => match &self.sample {
                Some(s) => {
                    meter(
                        ui,
                        "CPU",
                        s.cpu_percent,
                        100.0,
                        &format!("{:.0}%", s.cpu_percent),
                    );
                    meter(
                        ui,
                        "MEM",
                        s.mem_used_mb,
                        s.mem_total_mb,
                        &format!(
                            "{:.1} / {:.1} GB",
                            s.mem_used_mb / 1024.0,
                            s.mem_total_mb / 1024.0
                        ),
                    );
                    meter(
                        ui,
                        "Storage",
                        s.disk_used_gb,
                        s.disk_total_gb,
                        &format!("{:.1} / {:.1} GB", s.disk_used_gb, s.disk_total_gb),
                    );
                    ui.horizontal(|ui| {
                        ui.label("Network RX");
                        ui.label(format_rate(s.rx_kbps));
                    });
                    ui.horizontal(|ui| {
                        ui.label("TX");
                        ui.label(format_rate(s.tx_kbps));
                    });
                }
                None => {
                    ui.weak("Muestreando…");
                }
            },
        }
    }
}

/// A simple labeled progress meter (label, value, max, text).
fn meter(ui: &mut egui::Ui, label: &str, value: f64, max: f64, text: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label(text);
    });
    let fraction = if max > 0.0 {
        (value / max).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };
    let desired = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(desired, 10.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect(
        rect,
        2.0,
        egui::Color32::from_gray(60),
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    let fill =
        egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * fraction, rect.height()));
    painter.rect(
        fill,
        2.0,
        egui::Color32::from_rgb(80, 160, 255),
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_size_bytes() {
        assert_eq!(fmt_size(0), "0 B");
        assert_eq!(fmt_size(512), "512 B");
        assert_eq!(fmt_size(1023), "1023 B");
    }

    #[test]
    fn fmt_size_scaled() {
        assert_eq!(fmt_size(1024), "1.0 KB");
        assert_eq!(fmt_size(1536), "1.5 KB");
        assert_eq!(fmt_size(1024 * 1024), "1.0 MB");
        assert_eq!(fmt_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn fmt_rate_kbps() {
        assert_eq!(format_rate(0.0), "0.0 KB/s");
        assert_eq!(format_rate(512.0), "512.0 KB/s");
        assert_eq!(format_rate(2048.0), "2.0 MB/s");
    }

    #[test]
    fn parent_remote_paths() {
        assert_eq!(parent_remote_path("/var/log"), "/var");
        assert_eq!(parent_remote_path("/"), "/");
        assert_eq!(parent_remote_path("file.txt"), ".");
        assert_eq!(parent_remote_path("/var"), "/");
    }

    #[test]
    fn parent_local_paths() {
        assert_eq!(parent_local_path("/home/user"), "/home");
        assert_eq!(parent_local_path("file.txt"), ".");
        assert_eq!(parent_local_path("C:\\Users\\x"), "C:\\Users");
    }
}
