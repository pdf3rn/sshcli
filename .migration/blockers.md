# Migration Blockers

## Local PTY terminal vertical slice — BLOCKED

- Symptom: the typed native PTY boundary and standalone Slint terminal surface exist, but the surface cannot be compiled or visually verified.
- Evidence: `cargo check -p sshcli-terminal-ui` is blocked because `fontconfig` and `freetype2` are unavailable through pkg-config. The existing Tauri GUI independently remains blocked by `gobject-2.0 >= 2.70`/`glib-2.0 >= 2.70`.
- Attempted fixes: (1) preserved Tauri coexistence and created a separate Slint target; (2) verified the core terminal model independently; (3) checked pkg-config for the required fontconfig/freetype2 metadata. No fake renderer or dependency suppression was used.
- Likely root cause: required native GUI/font development packages are absent from the environment.
- Next investigation: install/provide supported `fontconfig` and `freetype2` development packages, then run Slint compile, terminal-ui tests, and screenshot validation.

The following remain risks requiring investigation:

- Native terminal emulator core is selected and verified; native renderer/Slint integration is present but not build-verified.
- Slint target exists, but migration parity tests and visual evidence do not yet exist.
- Source claims nested split behavior, but runtime topology/persistence needs direct verification.
- Native replacements for localStorage, clipboard, file dialogs, and drag/drop need design.
