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

- Symptom: the typed SSH adapter and native launch harness are implemented, but no authenticated SSH session is available for end-to-end verification.
- Evidence: `cargo check --locked -p sshcli-gui --bin native_terminal`, GUI tests, terminal-ui tests, and headless surface harness pass. `SshTerminalSurface` uses `sshcli_core::ssh::open_shell` on a worker thread and exposes streaming/input/resize/close/reconnect paths, but no live SSH runtime artifact exists.
- Remaining gaps: authentication/host-key behavior, bidirectional remote bytes, live resize negotiation, remote disconnect/reconnect, and SSH error presentation.
- Attempted fixes: (1) added the typed worker transport, lifecycle events, reconnect request path, and native SSH harness entry point; (2) added an ephemeral `sshd` fixture with generated Ed25519 host/user keys; (3) switched to absolute `/usr/sbin/sshd` and added startup delay. The fixture lifecycle test still receives no event within 5 seconds. No unrelated migration work was started.
- Likely root cause: the scoped `sshd` fixture is not producing a connection event for the Rust worker; server stderr was not captured before the three-attempt limit.
- Next investigation: capture the fixture's server stderr and validate its config/host-key/auth setup, or use a different localhost SSH fixture, then rerun the SSH lifecycle and visual checks.
