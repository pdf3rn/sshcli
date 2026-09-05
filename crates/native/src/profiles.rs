//! Pure, UI-independent helpers for the profile management view.
//!
//! These replicate the formatting / validation / tag-splitting rules of the
//! React frontend (`HomeView.tsx`, `ConnectionsView.tsx`, `ProfileModal.tsx`)
//! *exactly* so the native UI stays behaviorally identical. Everything here is
//! deterministic and free of keyring / filesystem / network dependencies, which
//! makes the `#[cfg(test)]` coverage runnable headless.

use std::time::{SystemTime, UNIX_EPOCH};

/// Replicate `HomeView.tsx`'s `formatLastUsed`: `null` → `"Sin usar"`, otherwise
/// `"Ahora mismo"` / `"Hace X min"` / `"Hace X h"` / `"Hace 1 día"` /
/// `"Hace X días"`.
pub fn format_last_used_card(secs: Option<u64>) -> String {
    match secs {
        None => "Sin usar".to_string(),
        Some(secs) => format_last_used_duration(secs),
    }
}

/// Replicate `ConnectionsView.tsx`'s `formatLastUsed`: `null` → `"—"` (em dash)
/// and the same relative-duration buckets otherwise.
pub fn format_last_used_table(secs: Option<u64>) -> String {
    match secs {
        None => "—".to_string(),
        Some(secs) => format_last_used_duration(secs),
    }
}

/// Shared relative-time buckets (identical thresholds to the React views).
///
/// Both React implementations are identical except for the `null`/`undefined`
/// fallback, so a single helper models the shared body.
fn format_last_used_duration(secs: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let diff_seconds = now.saturating_sub(secs) as i64;
    if diff_seconds < 90 {
        "Ahora mismo".to_string()
    } else if diff_seconds < 3600 {
        let minutes = diff_seconds / 60;
        format!("Hace {minutes} min")
    } else if diff_seconds < 86400 {
        let hours = diff_seconds / 3600;
        format!("Hace {hours} h")
    } else {
        let days = diff_seconds / 86400;
        if days == 1 {
            "Hace 1 día".to_string()
        } else {
            format!("Hace {days} días")
        }
    }
}

/// Split a comma-separated tags string the way `ProfileModal.tsx` does:
/// split on `,`, trim each entry, drop empty ones. Order is preserved.
pub fn split_tags(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

/// A single validation failure with its message. Order matters: the React modal
/// returns the *first* error, so we keep the checks in the same order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError(pub String);

/// Profile form fields (the values the modal collects before building a
/// [`sshcli_app::commands::ProfileInput`]).
#[derive(Debug, Clone, Default)]
pub struct ProfileForm {
    pub original_name: Option<String>,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub identity_file: String,
    pub authentication: String,
    pub accept_unknown_host_key: bool,
    pub group: String,
    pub tags: String,
    pub favorite: bool,
    pub secret: String,
}

/// Validate the form fields in the exact order of `ProfileModal.tsx`'s
/// `validationErrors`, returning the first failure (or `Ok`).
///
/// * name required (trim), no duplicate (case-insensitive, ignoring the edited
///   profile),
/// * host required, no whitespace within host,
/// * port integer in `1..=65535`,
/// * username required.
///
/// `existing_names` is the list of names already present (excluding the profile
/// being edited, which the caller already drops).
pub fn validate_form(form: &ProfileForm, existing_names: &[String]) -> Result<(), ValidationError> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(ValidationError("El nombre es obligatorio.".to_string()));
    }
    if existing_names
        .iter()
        .any(|existing| existing.to_lowercase() == name.to_lowercase())
    {
        return Err(ValidationError(
            "Ya existe una conexión con ese nombre.".to_string(),
        ));
    }
    let host = form.host.trim();
    if host.is_empty() {
        return Err(ValidationError("El host es obligatorio.".to_string()));
    }
    if host.chars().any(char::is_whitespace) {
        return Err(ValidationError(
            "El host no puede contener espacios.".to_string(),
        ));
    }
    let port = form.port.trim();
    let parsed_port: Result<u32, _> = port.parse();
    let valid_port = match parsed_port {
        Ok(value) if (1..=65535).contains(&value) => true,
        _ => false,
    };
    if !valid_port {
        return Err(ValidationError(
            "El puerto debe estar entre 1 y 65535.".to_string(),
        ));
    }
    if form.username.trim().is_empty() {
        return Err(ValidationError("El usuario es obligatorio.".to_string()));
    }
    Ok(())
}

/// Parse a host string: reject any internal or surrounding whitespace.
/// (Kept separate from `validate_form` for focused unit testing.)
pub fn validate_host(host: &str) -> Result<String, ValidationError> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err(ValidationError("El host es obligatorio.".to_string()));
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(ValidationError(
            "El host no puede contener espacios.".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validate a port string as an integer in `1..=65535`.
pub fn parse_port(port: &str) -> Result<u16, ValidationError> {
    match port.trim().parse::<u32>() {
        Ok(value) if (1..=65535).contains(&value) => Ok(value as u16),
        _ => Err(ValidationError(
            "El puerto debe estar entre 1 y 65535.".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // In practice `now` is provided by the system clock; these tests only
    // exercise the bucket boundaries via absolute seconds in the past relative
    // to "now".

    #[test]
    fn table_format_nulls_show_em_dash() {
        assert_eq!(format_last_used_table(None), "—");
    }

    #[test]
    fn card_format_nulls_show_sin_usar() {
        assert_eq!(format_last_used_card(None), "Sin usar");
    }

    #[test]
    fn format_now_is_ahora_mismo() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Just now (within 90s).
        assert_eq!(format_last_used_table(Some(now)), "Ahora mismo");
    }

    #[test]
    fn format_minutes() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let secs = now - 60 * 5; // 5 minutes ago
        assert_eq!(format_last_used_table(Some(secs)), "Hace 5 min");
    }

    #[test]
    fn format_hours() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let secs = now - 3600 * 3; // 3 hours ago
        assert_eq!(format_last_used_table(Some(secs)), "Hace 3 h");
    }

    #[test]
    fn format_one_day() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let secs = now - 86400; // 1 day ago
        assert_eq!(format_last_used_table(Some(secs)), "Hace 1 día");
    }

    #[test]
    fn format_many_days() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let secs = now - 86400 * 7; // 7 days ago
        assert_eq!(format_last_used_table(Some(secs)), "Hace 7 días");
    }

    #[test]
    fn split_tags_trims_and_drops_empty() {
        assert_eq!(split_tags("web, api, , db "), vec!["web", "api", "db"]);
        assert_eq!(split_tags(""), Vec::<String>::new());
        assert_eq!(split_tags("   "), Vec::<String>::new());
        assert_eq!(split_tags("single"), vec!["single"]);
    }

    fn form() -> ProfileForm {
        ProfileForm {
            name: "prod".into(),
            host: "example.com".into(),
            port: "22".into(),
            username: "deploy".into(),
            authentication: "password".into(),
            ..Default::default()
        }
    }

    #[test]
    fn validate_rejects_missing_name() {
        let mut f = form();
        f.name = "  ".into();
        let err = validate_form(&f, &[]).unwrap_err();
        assert_eq!(err.0, "El nombre es obligatorio.");
    }

    #[test]
    fn validate_rejects_duplicate_name_case_insensitive() {
        let mut f = form();
        f.name = "Prod".into();
        let err = validate_form(&f, &["prod".into()]).unwrap_err();
        assert_eq!(err.0, "Ya existe una conexión con ese nombre.");
    }

    #[test]
    fn validate_rejects_whitespace_in_host() {
        let mut f = form();
        f.host = "example .com".into();
        let err = validate_form(&f, &[]).unwrap_err();
        assert_eq!(err.0, "El host no puede contener espacios.");
    }

    #[test]
    fn validate_rejects_out_of_range_port() {
        for bad in ["0", "65536", "-1", "abc"] {
            let mut f = form();
            f.port = bad.into();
            let err = validate_form(&f, &[]).unwrap_err();
            assert_eq!(err.0, "El puerto debe estar entre 1 y 65535.");
        }
    }

    #[test]
    fn validate_rejects_missing_username() {
        let mut f = form();
        f.username = "  ".into();
        let err = validate_form(&f, &[]).unwrap_err();
        assert_eq!(err.0, "El usuario es obligatorio.");
    }

    #[test]
    fn validate_accepts_valid_form() {
        assert!(validate_form(&form(), &[]).is_ok());
    }

    #[test]
    fn validate_host_functions() {
        assert_eq!(validate_host("example.com").unwrap(), "example.com");
        assert_eq!(validate_host("  host  ").unwrap(), "host");
        assert!(validate_host("").is_err());
        assert!(validate_host("a b").is_err());
    }

    #[test]
    fn parse_port_function() {
        assert_eq!(parse_port("22").unwrap(), 22);
        assert_eq!(parse_port("65535").unwrap(), 65535);
        assert_eq!(parse_port("1").unwrap(), 1);
        assert!(parse_port("0").is_err());
        assert!(parse_port("65536").is_err());
        assert!(parse_port("x").is_err());
    }
}
