# Migration Blockers

## Local PTY terminal surface — VERIFIED

- Evidence: `cargo fmt`, `cargo check --locked -p sshcli-terminal-ui`, and `cargo test --locked -p sshcli-terminal-ui` pass. Runtime screenshot `/tmp/opencode/terminal-ui-rerun-window.png` shows full content at 800x520.
- Resolution: corrected Slint modulo/property errors, imported the required `slint::Model` trait, and set a usable preferred runtime size. No fake renderer or dependency suppression was used.
- The standalone surface is verified; live local PTY lifecycle evidence is recorded in the bounded wiring section below.

The following remain risks requiring investigation:

- Native terminal emulator core and standalone renderer/Slint integration are verified; full renderer behavior remains incomplete.
- Slint target has build, screenshot, headless event, and live PTY harness evidence; broader terminal parity remains unverified.
- Source claims nested split behavior, but runtime topology/persistence needs direct verification.
- Native replacements for localStorage, clipboard, file dialogs, and drag/drop need design.

## Local PTY-to-Slint wiring — VERIFIED (bounded slice)

- Evidence: `cargo check --locked -p sshcli-gui --bin native_terminal`, GUI tests (8), terminal-ui tests (7), and the headless Slint harness pass. The harness delivers key, resize, close, status, and large-model events. Runtime evidence is in `/tmp/opencode/native-terminal-runtime.log` and `/tmp/opencode/native-terminal-window.png`; the prompt `pedro@linuxpc:~$` was visible at 800x520.
- Verification: PTY harnesses cover special-key delivery, resize negotiation, startup gating/order, explicit close, and 64 KiB output. The bounded local PTY-to-Slint slice is VERIFIED.
- Remaining risks: host-window geometry automation and broader terminal parity (selection, search, clipboard, OSC 7, colors, and reconnect) remain unverified. Workspace-wide formatting also reports pre-existing unrelated diffs.
- Next investigation: integrate SSH transport with the verified surface before migrating tabs/splits or other features.

## Native SSH terminal transport — BLOCKED

- Symptom: the typed SSH adapter and native launch harness are implemented, but the native Rust lifecycle test cannot compile in this environment, so no authenticated Rust SSH session is available for end-to-end verification.
- Evidence: `/usr/sbin/sshd` and `/usr/bin/ssh` are OpenSSH 10.2p1; `sshd -t` passed. Captured prior-fixture stderr reports `Accepted key ...` followed by `ROOT LOGIN REFUSED` because `$USER` is `root` and the fixture had `PermitRootLogin no`. After the focused test-only correction to `PermitRootLogin prohibit-password`, an independent OpenSSH client run returned status 0 with `READY` and `ECHO:hello`; sshd logged accepted public-key authentication and forced-command start/close. `SshTerminalSurface` still has no native Rust runtime artifact.
- Remaining gaps: authentication/host-key behavior, bidirectional remote bytes, live resize negotiation, remote disconnect/reconnect, and SSH error presentation.
- Attempted fixes: (1) added the typed worker transport, lifecycle events, reconnect request path, and native SSH harness entry point; (2) added an ephemeral `sshd` fixture with generated Ed25519 host/user keys; (3) switched to absolute `/usr/sbin/sshd` and added startup delay; (4, this bounded attempt) captured sshd diagnostics, identified the root-login refusal, and changed only the fixture to `PermitRootLogin prohibit-password`. No production transport change or unrelated migration work was started.
- Likely root cause confirmed for the prior no-event fixture: authentication was refused by sshd's `PermitRootLogin no`, despite the key being accepted. The corrected fixture is independently healthy, but Rust lifecycle evidence is still unavailable.
- Current blocker: the narrow Rust lifecycle test stops before compilation because pkg-config cannot find `gobject-2.0 >= 2.70`, `glib-2.0 >= 2.70`, or `gio-2.0 >= 2.70`. One environment repair attempt (`apt-get`/`dpkg`) was interrupted while configuring an existing PostgreSQL package, and the required metadata is still absent. Separately, `cargo fmt --package sshcli-gui -- --check` fails on existing formatting differences, including this file. `cargo check --locked -p sshcli-core` passes.
- Next investigation: repair the host package manager state and provide the required GTK/GLib/GObject/GIO development metadata, then run the one narrow Rust lifecycle test. Do not infer host-key, authenticated Rust streaming, reconnect, remote-close, or live resize parity from the standalone OpenSSH fixture check.
