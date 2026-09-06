//! User preferences: persistence, defaults, and theming.
//!
//! Mirrors the React frontend's `prefs.ts` (`Prefs`, `DEFAULT_PREFS`, the
//! `localStorage`-backed `loadPrefs`/`usePrefs`) but persists to a JSON file in
//! the platform config directory instead of the browser's localStorage.

use std::fs;

use eframe::egui;
use serde::{Deserialize, Serialize};

use sshcli_core::config;

pub const PREFS_FILE: &str = "prefs.json";

/// Interface theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
}

/// Terminal cursor style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

/// The full set of user preferences (field names match `prefs.ts`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefs {
    pub font_family: String,
    pub font_size: u32,
    pub line_height: f32,
    pub scrollback: usize,
    pub copy_on_select: bool,
    pub right_click_paste: bool,
    pub cursor_style: CursorStyle,
    pub cursor_blink: bool,
    pub telemetry_enabled: bool,
    pub remote_explorer_enabled: bool,
    pub local_shell: String,
    pub theme: Theme,
}

impl Default for Prefs {
    fn default() -> Self {
        // Values mirror `DEFAULT_PREFS` in `prefs.ts`.
        Self {
            font_family: "monospace".to_string(),
            font_size: 13,
            line_height: 1.0,
            scrollback: 5_000,
            copy_on_select: false,
            right_click_paste: false,
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
            telemetry_enabled: false,
            remote_explorer_enabled: false,
            local_shell: String::new(),
            theme: Theme::Dark,
        }
    }
}

impl Prefs {
    fn path() -> std::path::PathBuf {
        config::config_dir().join(PREFS_FILE)
    }

    /// Load preferences from disk, falling back to defaults on any error or
    /// missing file (merging so new fields default sanely).
    pub fn load() -> Self {
        let default = Self::default();
        let Ok(content) = fs::read_to_string(Self::path()) else {
            return default;
        };
        match serde_json::from_str::<Self>(&content) {
            Ok(prefs) => prefs,
            Err(_) => default,
        }
    }

    /// Persist the preferences to disk (best-effort; a failure is silently
    /// ignored, matching the React `localStorage.setItem` try/catch).
    pub fn save(&self) {
        let Ok(content) = serde_json::to_string_pretty(self) else {
            return;
        };
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, content);
    }

    /// Apply the interface theme to an egui context immediately.
    pub fn apply_theme(&self, ctx: &egui::Context) {
        match self.theme {
            Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_react() {
        let p = Prefs::default();
        assert_eq!(p.font_size, 13);
        assert_eq!(p.scrollback, 5_000);
        assert!(!p.copy_on_select);
        assert!(p.cursor_blink);
        assert_eq!(p.cursor_style, CursorStyle::Block);
        assert_eq!(p.theme, Theme::Dark);
        assert_eq!(p.line_height, 1.0);
        assert!(!p.telemetry_enabled);
        assert!(!p.remote_explorer_enabled);
        assert!(p.local_shell.is_empty());
    }

    #[test]
    fn roundtrip_serialization_preserves_theme() {
        let p = Prefs {
            theme: Theme::Light,
            font_size: 16,
            copy_on_select: true,
            ..Prefs::default()
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: Prefs = serde_json::from_str(&json).unwrap();
        assert_eq!(back.theme, Theme::Light);
        assert_eq!(back.font_size, 16);
        assert!(back.copy_on_select);
    }

    #[test]
    fn theme_serde_is_lowercase() {
        assert_eq!(serde_json::to_string(&Theme::Dark).unwrap(), "\"dark\"");
        assert_eq!(
            serde_json::to_string(&CursorStyle::Underline).unwrap(),
            "\"underline\""
        );
    }
}
