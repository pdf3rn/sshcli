//! The profile-management view (list + create/edit modal), mirroring the
//! React frontend's `ConnectionsView`, `HomeView` and `ProfileModal`.
//!
//! All persistence goes through `sshcli_app::commands` (the Tauri-free service
//! layer extracted in step 1); this module only owns presentation state and
//! delegates every CRUD operation. Business rules (validation, credential
//! handling) live in `sshcli_app` / `sshcli_core`.

use eframe::egui;
use sshcli_app::commands::{self, ProfileInput};
use sshcli_core::profiles::{Authentication, Profile};

use crate::profiles::{format_last_used_table, split_tags, validate_form, ProfileForm};

/// Sort orders, matching `ConnectionsView.tsx`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortBy {
    #[default]
    Favorite,
    Name,
    LastUsed,
}

/// An action requested by the profile view, handled by the owning app.
#[derive(Debug, Clone)]
pub enum ProfilesAction {
    /// Connect to a saved profile (creates an SSH terminal tab).
    Connect(String),
    /// Open the SFTP browser for a profile.
    OpenSftp(String),
    /// Open the tunnel panel for a profile.
    OpenTunnels(String),
    /// Open the telemetry panel for a profile.
    OpenTelemetry(String),
}

/// State for the profile-list view.
#[derive(Default)]
pub struct ProfilesView {
    query: String,
    group: String,
    sort: SortBy,
    profiles: Vec<Profile>,
    loaded: bool,
    error: Option<String>,
    status: Option<String>,

    /// Create/edit modal state.
    modal: Option<ProfileModal>,
}

impl ProfilesView {
    fn refresh(&mut self) {
        match commands::list_profiles() {
            Ok(profiles) => {
                self.profiles = profiles;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
        self.loaded = true;
    }

    fn groups(&self) -> Vec<(String, usize)> {
        let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
        for p in &self.profiles {
            let key = p.group.clone().unwrap_or_default();
            *counts.entry(key).or_default() += 1;
        }
        counts.into_iter().map(|(k, v)| (k, v)).collect()
    }

    fn ordered(&self) -> Vec<Profile> {
        let needle = self.query.trim().to_lowercase();
        let mut out: Vec<Profile> = self
            .profiles
            .iter()
            .filter(|p| {
                if self.group == "__ungrouped__" {
                    if p.group.is_some() {
                        return false;
                    }
                } else if self.group != "__all__" && p.group.as_deref() != Some(self.group.as_str())
                {
                    return false;
                }
                if needle.is_empty() {
                    return true;
                }
                let haystack = format!(
                    "{} {} {} {} {}",
                    p.name,
                    p.host,
                    p.username,
                    p.group.clone().unwrap_or_default(),
                    p.tags.join(" ")
                )
                .to_lowercase();
                haystack.contains(&needle)
            })
            .cloned()
            .collect();
        out.sort_by(|a, b| match self.sort {
            SortBy::Name => a.name.cmp(&b.name),
            SortBy::LastUsed => (b.last_used.unwrap_or(0))
                .cmp(&a.last_used.unwrap_or(0))
                .then_with(|| a.name.cmp(&b.name)),
            SortBy::Favorite => (b.favorite as u8)
                .cmp(&(a.favorite as u8))
                .then_with(|| a.name.cmp(&b.name)),
        });
        out
    }

    /// Render the list. Returns actions (e.g. Connect) for the app to handle.
    pub fn show(&mut self, ui: &mut egui::Ui) -> Vec<ProfilesAction> {
        let mut actions = Vec::new();

        if !self.loaded {
            self.refresh();
        }

        ui.horizontal(|ui| {
            ui.heading("Conexiones");
            if ui.button("Actualizar").clicked() {
                self.refresh();
            }
            ui.separator();
            if ui.button("Nueva conexión").clicked() {
                self.modal = Some(ProfileModal::new(None, &self.profiles));
            }
            if ui.button("Importar").clicked() {
                self.modal = Some(ProfileModal::new_import());
            }
            if ui.button("Exportar").clicked() {
                match commands::export_profiles() {
                    Ok(content) => {
                        ui.ctx().copy_text(content);
                        self.status = Some(
                            "Configuración exportada como texto; copiada al portapapeles.".into(),
                        );
                    }
                    Err(e) => self.error = Some(format!("Error al exportar: {e}")),
                }
            }
        });

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Buscar hosts…")
                    .desired_width(220.0),
            );
            egui::ComboBox::from_label("Ordenar")
                .selected_text(match self.sort {
                    SortBy::Favorite => "Favoritos primero",
                    SortBy::Name => "Nombre",
                    SortBy::LastUsed => "Última conexión",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.sort, SortBy::Favorite, "Favoritos primero");
                    ui.selectable_value(&mut self.sort, SortBy::Name, "Nombre");
                    ui.selectable_value(&mut self.sort, SortBy::LastUsed, "Última conexión");
                });
        });

        // Group filter.
        ui.horizontal_wrapped(|ui| {
            let groups = self.groups();
            ui.selectable_value(
                &mut self.group,
                "__all__".to_string(),
                "Todas las conexiones",
            );
            for (g, _) in &groups {
                if g.is_empty() {
                    continue;
                }
                ui.selectable_value(&mut self.group, g.clone(), g);
            }
            if self.profiles.iter().any(|p| p.group.is_none()) {
                ui.selectable_value(&mut self.group, "__ungrouped__".to_string(), "Sin grupo");
            }
        });

        if let Some(err) = &self.error {
            ui.colored_label(egui::Color32::RED, err);
        }
        if let Some(status) = &self.status {
            ui.label(status);
        }

        if self.profiles.is_empty() {
            ui.separator();
            ui.label("No hay perfiles todavía.");
            if ui.button("Crear la primera conexión").clicked() {
                self.modal = Some(ProfileModal::new(None, &self.profiles));
            }
            return actions;
        }

        let ordered = self.ordered();
        if ordered.is_empty() {
            ui.separator();
            ui.label(format!("Sin resultados para «{}».", self.query));
            return actions;
        }

        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for profile in ordered {
                ui.horizontal(|ui| {
                    // Favorite star.
                    let star = if profile.favorite { "★" } else { "☆" };
                    if ui.button(star).on_hover_text("Favorito").clicked() {
                        if let Err(e) = commands::toggle_favorite(profile.name.clone()) {
                            self.error = Some(e);
                        } else {
                            self.refresh();
                        }
                    }

                    // Name + endpoint.
                    ui.label(&profile.name);
                    ui.label(format!(
                        "{}@{}:{}",
                        profile.username, profile.host, profile.port
                    ));

                    // Tags / group chips (rendered as simple labels).
                    if let Some(group) = &profile.group {
                        ui.colored_label(egui::Color32::from_gray(120), format!("[{group}]"));
                    }
                    for tag in &profile.tags {
                        ui.colored_label(egui::Color32::from_gray(150), tag);
                    }

                    ui.label(format_last_used_table(profile.last_used));

                    if ui.button("Conectar").clicked() {
                        actions.push(ProfilesAction::Connect(profile.name.clone()));
                    }
                    if ui.button("SFTP").clicked() {
                        actions.push(ProfilesAction::OpenSftp(profile.name.clone()));
                    }
                    if ui.button("Túneles").clicked() {
                        actions.push(ProfilesAction::OpenTunnels(profile.name.clone()));
                    }
                    if ui.button("Telemetría").clicked() {
                        actions.push(ProfilesAction::OpenTelemetry(profile.name.clone()));
                    }
                    if ui.button("Editar").clicked() {
                        self.modal = Some(ProfileModal::new(Some(profile.clone()), &self.profiles));
                    }
                    if ui.button("Duplicar").clicked() {
                        self.open_duplicate(&profile.name);
                    }
                    if ui.button("Borrar").clicked() {
                        self.open_delete(&profile.name);
                    }
                });
            }
        });

        actions
    }

    fn open_duplicate(&mut self, name: &str) {
        let new_name = format!("{name} (copia)");
        self.modal = Some(ProfileModal::new_duplicate(name.to_string(), new_name));
    }

    fn open_delete(&mut self, name: &str) {
        // Simple confirm via a small modal-like window state.
        self.modal = Some(ProfileModal::new_delete(name.to_string()));
    }

    /// Show the create/edit/delete/duplicate/import modal if one is open.
    /// Returns `Some(action)` when the user commits a change that should refresh.
    pub fn show_modal(&mut self, ctx: &egui::Context) -> Option<()> {
        let mut modal = self.modal.take()?;
        let mut changed = false;
        let mut done = false;

        let mut open = true;
        let title = modal.title();
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                match &mut modal.kind {
                    ModalKind::Edit {
                        form,
                        keys,
                        editing,
                        existing_names,
                    } => {
                        ui.label("Nombre:");
                        ui.text_edit_singleline(&mut form.name);
                        ui.label("Host:");
                        ui.text_edit_singleline(&mut form.host);
                        ui.horizontal(|ui| {
                            ui.label("Puerto:");
                            ui.text_edit_singleline(&mut form.port);
                            ui.label("Usuario:");
                            ui.text_edit_singleline(&mut form.username);
                        });

                        // Auth segmented control.
                        ui.label("Autenticación");
                        let auths = ["password", "private-key", "none"];
                        ui.horizontal(|ui| {
                            for a in auths {
                                ui.selectable_value(&mut form.authentication, a.to_string(), a);
                            }
                        });

                        if form.authentication == "private-key" {
                            ui.label("Clave privada");
                            let mut id_file = form.identity_file.clone();
                            ui.text_edit_singleline(&mut id_file);
                            form.identity_file = id_file;
                            egui::ComboBox::from_id_salt("ssh-keys")
                                .selected_text(if form.identity_file.is_empty() {
                                    "~/.ssh/id_ed25519".to_string()
                                } else {
                                    form.identity_file.clone()
                                })
                                .show_ui(ui, |ui| {
                                    for key in keys.iter() {
                                        ui.selectable_value(
                                            &mut form.identity_file,
                                            key.clone(),
                                            key,
                                        );
                                    }
                                });
                            if keys.is_empty() {
                                ui.weak("No se encontraron claves en ~/.ssh");
                            }
                        }

                        if form.authentication == "password" || form.authentication == "private-key"
                        {
                            let label = if form.authentication == "password" {
                                "Contraseña"
                            } else {
                                "Passphrase (vacío = sin)"
                            };
                            ui.label(label);
                            let mut secret = form.secret.clone();
                            ui.add(
                                egui::TextEdit::singleline(&mut secret)
                                    .password(true)
                                    .hint_text(label),
                            );
                            form.secret = secret;
                        }

                        ui.checkbox(
                            &mut form.accept_unknown_host_key,
                            "Aceptar clave de host desconocida",
                        );

                        ui.horizontal(|ui| {
                            ui.label("Grupo (opcional)");
                            ui.text_edit_singleline(&mut form.group);
                        });
                        ui.label("Etiquetas (separadas por coma)");
                        ui.text_edit_singleline(&mut form.tags);

                        if let Some(err) = &modal.error {
                            ui.colored_label(egui::Color32::RED, err);
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Cancelar").clicked() {
                                done = true;
                            }
                            if ui.button("Probar conexión").clicked() {
                                modal.testing = true;
                                modal.test_result = None;
                                match validate_form(form, existing_names) {
                                    Err(e) => modal.error = Some(e.0),
                                    Ok(()) => match build_input(form, editing.as_ref()) {
                                        Ok(input) => {
                                            modal.testing = false;
                                            modal.test_result = Some(run_test_profile(input));
                                        }
                                        Err(e) => modal.error = Some(e),
                                    },
                                }
                            }
                            if let Some(result) = modal.test_result.take() {
                                modal.error = result;
                            }
                            if modal.testing {
                                ui.label("Probando…");
                            }
                            if ui.button("Guardar").clicked() {
                                match validate_form(form, existing_names) {
                                    Err(e) => modal.error = Some(e.0),
                                    Ok(()) => match build_input(form, editing.as_ref()) {
                                        Ok(input) => {
                                            let result = if editing.is_some() {
                                                commands::update_profile(input)
                                            } else {
                                                commands::create_profile(input)
                                            };
                                            match result {
                                                Ok(()) => {
                                                    changed = true;
                                                    done = true;
                                                }
                                                Err(e) => modal.error = Some(e),
                                            }
                                        }
                                        Err(e) => modal.error = Some(e),
                                    },
                                }
                            }
                        });
                    }
                    ModalKind::Duplicate { source, new_name } => {
                        ui.label(format!("Duplicar «{source}» como:"));
                        ui.text_edit_singleline(new_name);
                        if ui.button("Duplicar").clicked() {
                            match commands::duplicate_profile(
                                source.clone(),
                                new_name.trim().to_string(),
                            ) {
                                Ok(()) => {
                                    changed = true;
                                    done = true;
                                }
                                Err(e) => modal.error = Some(e),
                            }
                        }
                        if ui.button("Cancelar").clicked() {
                            done = true;
                        }
                    }
                    ModalKind::Delete { name } => {
                        ui.label(format!("¿Borrar la conexión «{name}»?"));
                        ui.horizontal(|ui| {
                            if ui.button("Borrar").clicked() {
                                match commands::delete_profile(name.clone()) {
                                    Ok(()) => {
                                        changed = true;
                                        done = true;
                                    }
                                    Err(e) => modal.error = Some(e),
                                }
                            }
                            if ui.button("Cancelar").clicked() {
                                done = true;
                            }
                        });
                    }
                    ModalKind::Import { content, status } => {
                        ui.label("Pega el contenido de un archivo profiles TOML:");
                        ui.add(
                            egui::TextEdit::multiline(content)
                                .desired_rows(8)
                                .desired_width(420.0),
                        );
                        if let Some(status) = status.as_deref() {
                            ui.label(status);
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Importar").clicked() {
                                match commands::import_profiles(content.clone()) {
                                    Ok(0) => {
                                        *status = Some(
                                            "Nada que importar: todos los perfiles ya existen."
                                                .into(),
                                        );
                                    }
                                    Ok(n) => {
                                        *status = Some(format!("{n} perfil(es) importados."));
                                        self.refresh();
                                    }
                                    Err(e) => *status = Some(format!("Error al importar: {e}")),
                                }
                            }
                            if ui.button("Cerrar").clicked() {
                                done = true;
                            }
                        });
                    }
                }
            });

        if done || !open {
            if changed {
                self.refresh();
            }
            None
        } else {
            self.modal = Some(modal);
            None
        }
    }
}

/// Run the async `test_profile` synchronously on a short-lived runtime.
/// Returns `Some("Conexión correcta.")` on success, or the error message.
fn run_test_profile(input: ProfileInput) -> Option<String> {
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())
        .and_then(|rt| rt.block_on(commands::test_profile(input)));
    match result {
        Ok(()) => Some("Conexión correcta.".to_string()),
        Err(e) => Some(format!("No se pudo conectar: {e}")),
    }
}

/// Build a `ProfileInput` from a validated form, mirroring `ProfileModal.tsx`.
fn build_input(form: &ProfileForm, editing: Option<&Profile>) -> Result<ProfileInput, String> {
    let needs_secret = form.authentication == "password" || form.authentication == "private-key";
    Ok(ProfileInput {
        original_name: editing
            .map(|p| p.name.clone())
            .or_else(|| form.original_name.clone()),
        name: form.name.trim().to_string(),
        host: form.host.trim().to_string(),
        port: form
            .port
            .trim()
            .parse::<u16>()
            .map_err(|_| "puerto inválido".to_string())?,
        username: form.username.trim().to_string(),
        identity_file: if form.authentication == "private-key" {
            let s = form.identity_file.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        } else {
            None
        },
        authentication: form.authentication.clone(),
        accept_unknown_host_key: form.accept_unknown_host_key,
        group: {
            let g = form.group.trim();
            if g.is_empty() {
                None
            } else {
                Some(g.to_string())
            }
        },
        tags: Some(split_tags(&form.tags)),
        favorite: editing.map(|p| p.favorite).unwrap_or(false),
        secret: if needs_secret {
            Some(form.secret.clone())
        } else {
            None
        },
    })
}

/// The create/edit/delete/duplicate modal.
struct ProfileModal {
    kind: ModalKind,
    error: Option<String>,
    testing: bool,
    test_result: Option<Option<String>>,
}

enum ModalKind {
    Edit {
        form: ProfileForm,
        keys: Vec<String>,
        editing: Option<Profile>,
        existing_names: Vec<String>,
    },
    Duplicate {
        source: String,
        new_name: String,
    },
    Delete {
        name: String,
    },
    Import {
        content: String,
        status: Option<String>,
    },
}

impl ProfileModal {
    fn new(editing: Option<Profile>, all: &[Profile]) -> Self {
        let keys = commands::list_identity_keys().unwrap_or_default();
        let existing_names = all
            .iter()
            .filter(|p| !editing.as_ref().map(|e| e.name == p.name).unwrap_or(false))
            .map(|p| p.name.clone())
            .collect();
        let form = editing
            .as_ref()
            .map(|p| ProfileForm {
                original_name: Some(p.name.clone()),
                name: p.name.clone(),
                host: p.host.clone(),
                port: p.port.to_string(),
                username: p.username.clone(),
                identity_file: p.identity_file.clone().unwrap_or_default(),
                authentication: match p.authentication {
                    Authentication::None => "none".into(),
                    Authentication::Password => "password".into(),
                    Authentication::PrivateKey => "private-key".into(),
                },
                accept_unknown_host_key: p.accept_unknown_host_key,
                group: p.group.clone().unwrap_or_default(),
                tags: p.tags.join(", "),
                favorite: p.favorite,
                secret: String::new(),
            })
            .unwrap_or_default();
        Self {
            kind: ModalKind::Edit {
                form,
                keys,
                editing,
                existing_names,
            },
            error: None,
            testing: false,
            test_result: None,
        }
    }

    fn new_duplicate(source: String, new_name: String) -> Self {
        Self {
            kind: ModalKind::Duplicate { source, new_name },
            error: None,
            testing: false,
            test_result: None,
        }
    }

    fn new_delete(name: String) -> Self {
        Self {
            kind: ModalKind::Delete { name },
            error: None,
            testing: false,
            test_result: None,
        }
    }

    fn new_import() -> Self {
        Self {
            kind: ModalKind::Import {
                content: String::new(),
                status: None,
            },
            error: None,
            testing: false,
            test_result: None,
        }
    }

    fn title(&self) -> String {
        match &self.kind {
            ModalKind::Edit {
                editing: Some(p), ..
            } => format!("Editar {}", p.name),
            ModalKind::Edit { editing: None, .. } => "Nueva conexión".to_string(),
            ModalKind::Duplicate { source, .. } => format!("Duplicar {source}"),
            ModalKind::Delete { name } => format!("Borrar {name}"),
            ModalKind::Import { .. } => "Importar config".to_string(),
        }
    }
}
