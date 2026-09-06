//! The settings view, mirroring `SettingsView.tsx`.
//!
//! Renders the preference controls for typography, terminal behavior,
//! shortcuts, cursor, theme, and feature toggles. Changes are applied to the
//! shared [`crate::prefs::Prefs`] immediately (theme/font) and are persisted by
//! the owning app via [`crate::prefs::Prefs::save`].

use eframe::egui;

use crate::prefs::{CursorStyle, Prefs, Theme};

/// The interactive settings panel. Owns no prefs; it mutates the shared one.
#[derive(Default)]
pub struct SettingsView;

impl SettingsView {
    /// Render the settings controls, mutating `prefs` in place.
    /// Returns `true` if anything changed (the caller should persist).
    pub fn show(&mut self, ui: &mut egui::Ui, prefs: &mut Prefs) -> bool {
        let mut changed = false;
        ui.heading("Ajustes");
        ui.label(
            "Tipografía, comportamiento del terminal y telemetría. Los cambios se aplican al instante en todas las sesiones.",
        );
        ui.separator();

        // Typography.
        ui.heading("Tipografía");
        ui.horizontal(|ui| {
            ui.label("Fuente");
            let mut font_family = prefs.font_family.clone();
            egui::ComboBox::from_id_salt("font-family")
                .selected_text(&font_family)
                .show_ui(ui, |ui| {
                    for f in [
                        "monospace",
                        "JetBrains Mono",
                        "Fira Code",
                        "Cascadia Mono",
                        "Menlo",
                    ] {
                        ui.selectable_value(&mut font_family, f.to_string(), f);
                    }
                });
            if font_family != prefs.font_family {
                prefs.font_family = font_family;
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(format!("Tamaño ({}px)", prefs.font_size));
            let mut size = prefs.font_size as u32;
            if ui
                .add(egui::Slider::new(&mut size, 10..=24).text("px"))
                .changed()
            {
                prefs.font_size = size;
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Interlineado");
            let mut lh = prefs.line_height;
            for (label, value) in [
                ("Compacta (1.0)", 1.0),
                ("Normal (1.2)", 1.2),
                ("Relajada (1.5)", 1.5),
            ] {
                if ui.selectable_value(&mut lh, value, label).changed() {
                    prefs.line_height = lh;
                    changed = true;
                }
            }
        });

        // Behavior.
        ui.heading("Comportamiento");
        ui.horizontal(|ui| {
            ui.label("Líneas de historial (scrollback)");
            let mut sb = prefs.scrollback;
            for (label, value) in [
                ("1.000", 1_000usize),
                ("5.000", 5_000),
                ("10.000", 10_000),
                ("Sin límite", 1_000_000),
            ] {
                if ui.selectable_value(&mut sb, value, label).changed() {
                    prefs.scrollback = sb;
                    changed = true;
                }
            }
        });
        if ui
            .checkbox(&mut prefs.copy_on_select, "Copiar al seleccionar")
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(&mut prefs.right_click_paste, "Pegar con clic derecho")
            .changed()
        {
            changed = true;
        }

        // Shortcuts (informational, matching SettingsView.tsx).
        ui.heading("Atajos de teclado");
        ui.label("Los atajos usan Ctrl en Windows/Linux y Cmd en macOS.");
        for (desc, key) in [
            ("Cambiar a la pestaña 1–9", "Ctrl/Cmd + 1–9"),
            ("Cambiar de pestaña", "Ctrl/Cmd + Tab"),
            ("Cerrar pestaña activa", "Ctrl/Cmd + W"),
            ("Buscar en terminal", "Ctrl/Cmd + F"),
            ("Copiar selección de terminal", "Ctrl/Cmd + Shift + C"),
            ("Limpiar terminal", "Ctrl/Cmd + Shift + K"),
        ] {
            ui.label(format!("{desc:<32} {key}"));
        }

        // Cursor.
        ui.heading("Cursor");
        ui.horizontal(|ui| {
            ui.label("Estilo");
            let mut style = prefs.cursor_style;
            for (label, value) in [
                ("▮ Bloque", CursorStyle::Block),
                ("_ Subrayado", CursorStyle::Underline),
                ("| Barra", CursorStyle::Bar),
            ] {
                if ui.selectable_value(&mut style, value, label).changed() {
                    prefs.cursor_style = style;
                    changed = true;
                }
            }
        });
        if ui
            .checkbox(&mut prefs.cursor_blink, "Cursor parpadeante")
            .changed()
        {
            changed = true;
        }

        // Theme.
        ui.heading("Tema");
        ui.horizontal(|ui| {
            let mut theme = prefs.theme;
            for (label, value) in [("Oscuro", Theme::Dark), ("Claro", Theme::Light)] {
                if ui.selectable_value(&mut theme, value, label).changed() {
                    prefs.theme = theme;
                    changed = true;
                }
            }
        });

        // Feature toggles.
        ui.heading("Telemetría del host");
        if ui
            .checkbox(
                &mut prefs.telemetry_enabled,
                "Mostrar panel de telemetría en las sesiones",
            )
            .changed()
        {
            changed = true;
        }
        ui.heading("Explorador remoto");
        if ui
            .checkbox(
                &mut prefs.remote_explorer_enabled,
                "Mostrar explorador de carpetas en las sesiones",
            )
            .changed()
        {
            changed = true;
        }

        changed
    }
}
