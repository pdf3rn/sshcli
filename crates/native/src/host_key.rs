//! Host-key confirmation dialog.
//!
//! Mirrors `HostKeyDialog.tsx`: an unknown host key prompts "Host key no
//! reconocida" with the first-connection explanation, while a *changed* key
//! prompts "¡ADVERTENCIA: host key cambió!" with the MITM warning. Both show
//! the fingerprint and a Cancel button next to "Confiar y conectar" (unknown)
//! or "Confiar a pesar del cambio" (changed).

use eframe::egui;

/// A pending host-key decision surfaced from a recent SSH connect failure.
#[derive(Debug, Clone)]
pub struct HostKeyRequest {
    pub host: String,
    pub port: u16,
    /// The `key` field from the host-key payload is the fingerprint we display.
    pub key: String,
    pub changed: bool,
}

impl HostKeyRequest {
    /// Parse a `sshcli:host-key:` + JSON error string (as produced by
    /// [`crate::ssh::map_ssh_error`]) into a [`HostKeyRequest`], if it is one.
    pub fn from_error(error: &str) -> Option<Self> {
        let payload = error.strip_prefix(crate::ssh::HOST_KEY_PREFIX)?;
        let value: serde_json::Value = serde_json::from_str(payload).ok()?;
        Some(HostKeyRequest {
            host: value.get("host")?.as_str()?.to_string(),
            port: value.get("port")?.as_u64()? as u16,
            key: value.get("key")?.as_str()?.to_string(),
            changed: value.get("changed")?.as_bool().unwrap_or(false),
        })
    }
}

/// The user's decision for a shown host-key dialog.
pub enum HostKeyDecision {
    /// Trust and retry (unknown) / trust despite change (changed).
    Trust,
    /// Dismiss without trusting.
    Cancel,
    /// The dialog is still open.
    Pending,
}

/// Draw the dialog window. Returns the decision taken this frame (if any).
///
/// `request` is shown until a decision is made; the caller owns the state and
/// clears it after acting on `Trust` / `Cancel`.
pub fn show(ctx: &egui::Context, request: &HostKeyRequest) -> HostKeyDecision {
    let mut decision = HostKeyDecision::Pending;
    let mut open = true;
    let title = if request.changed {
        "¡ADVERTENCIA: host key cambió!"
    } else {
        "Host key no reconocida"
    };
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            if request.changed {
                ui.label(format!(
                    "La clave del host {}:{} ha cambiado desde la última conexión.",
                    request.host, request.port
                ));
                ui.separator();
                ui.colored_label(
                    egui::Color32::from_rgb(255, 120, 80),
                    "Esto puede indicar un posible ataque de intermediario (MITM) o que el servidor \
                     reinstaló sus claves. Continúa solo si confías en el servidor actual.",
                );
            } else {
                ui.label(format!(
                    "La autenticidad de host {}:{} no se puede establecer. No está en la lista de \
                     hosts conocidos. Confía solo si esperas conectarte a este servidor por primera vez.",
                    request.host, request.port
                ));
            }
            ui.separator();
            ui.label("Fingerprint");
            ui.add(
                egui::TextEdit::singleline(&mut request.key.as_str().to_owned())
                    .interactive(false),
            );
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cancelar").clicked() {
                    decision = HostKeyDecision::Cancel;
                }
                let confirm_label = if request.changed {
                    "Confiar a pesar del cambio"
                } else {
                    "Confiar y conectar"
                };
                let confirm = if request.changed {
                    ui.button(confirm_label)
                } else {
                    ui.button(confirm_label)
                };
                if confirm.clicked() {
                    decision = HostKeyDecision::Trust;
                }
            });
        });
    if !open {
        decision = HostKeyDecision::Cancel;
    }
    decision
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unknown_host_key_payload() {
        let err = "sshcli:host-key:{\"host\":\"example.com\",\"port\":22,\"key\":\"ssh-ed25519 AAA\",\"changed\":false}";
        let req = HostKeyRequest::from_error(err).expect("parses");
        assert_eq!(req.host, "example.com");
        assert_eq!(req.port, 22);
        assert_eq!(req.key, "ssh-ed25519 AAA");
        assert!(!req.changed);
    }

    #[test]
    fn parses_changed_host_key_payload() {
        let err = "sshcli:host-key:{\"host\":\"a\",\"port\":2222,\"key\":\"k\",\"changed\":true}";
        let req = HostKeyRequest::from_error(err).expect("parses");
        assert!(req.changed);
    }

    #[test]
    fn ignores_non_host_key_errors() {
        assert!(HostKeyRequest::from_error("ssh authentication failed").is_none());
        assert!(HostKeyRequest::from_error("").is_none());
    }
}
