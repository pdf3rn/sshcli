# Migration State

## Status

- Phase: INCREMENTAL_MIGRATION
- Current unit: native SSH terminal transport integration
- Last verified unit: local PTY to terminal surface wiring
- Overall status: BLOCKED (SSH fixture is corrected and independently validated, but native lifecycle test cannot compile in this environment)

## Target

Tauri + React/TypeScript UI -> native Rust + Slint UI, preserving observable behavior.

## Current priorities

1. Preserve the existing Tauri implementation while establishing a native boundary.
2. Define and test the terminal rendering strategy before broad UI migration.
3. Migrate one bounded vertical slice at a time.

## Inventory status

- Architecture: INVENTORIED
- Screens/components: INVENTORIED
- Tauri boundary: INVENTORIED
- TypeScript responsibilities: INVENTORIED
- Existing Rust: INVENTORIED
- Feature matrix: INVENTORIED

## Previous unit result

- Scope: typed native Rust local PTY start/readiness/output/input/resize/close/error boundary.
- Implemented: `NativeLocalPtyBoundary` in `crates/gui/src/local_shell.rs`; existing Tauri manager and commands remain unchanged for coexistence.
- Not implemented: Slint target/surface and terminal emulator/renderer. No plain text replacement was introduced.
- Status: BLOCKED at the time of that earlier unit. The standalone target was not yet build-verified; the existing GUI build remains independently blocked by GTK/GObject system packages (`gobject-2.0 >= 2.70`).

## Next unit

Resolve the blocked native SSH terminal transport unit with a localhost SSH fixture or configured test credentials, then verify bidirectional streaming, resize, reconnect, and lifecycle errors. Do not begin tabs/splits, SFTP, tunnels, profiles, or settings migration.

## Current unit result

- Scope: one standalone Slint terminal surface backed by `sshcli_core::terminal::TerminalState`; no PTY/SSH IO or other feature migration.
- Implemented: `crates/terminal-ui` with typed row-major cell model, cursor properties, input callback, resize callback, and preferred 800x520 runtime size.
- Verified: `cargo fmt --package sshcli-terminal-ui -- --check`, `cargo check --locked -p sshcli-terminal-ui`, and `cargo test --locked -p sshcli-terminal-ui` passed; runtime screenshot verified full content visibility at 800x520 (`/tmp/opencode/terminal-ui-rerun-window.png`).
- Remaining: connect PTY bytes/input/resize/close and add interaction/error/large-output evidence.
- Status: VERIFIED for standalone terminal surface; full local PTY vertical slice remains incomplete.

## Earlier unit result

- Scope: select and verify a maintained native VT emulator core; no Slint surface or painting.
- Selected: `alacritty_terminal` 0.26.0 (Apache-2.0), wrapped by `sshcli_core::terminal::TerminalState`.
- Rejected for this unit: direct `vte` alone (parser without terminal grid/renderable state) and `termwiz` (not selected because the existing API/renderer integration was less direct for this target).
- Verified: ANSI/cursor movement, alternate screen, Unicode/wide-character state, resize/scrollback, and escape sequences split across input chunks.
- Status: VERIFIED for emulator-core strategy; full local PTY target remains incomplete pending PTY-to-surface wiring.

## Verification summary

- Rust build: `cargo check --locked -p sshcli-terminal-ui` and `cargo check --locked -p sshcli-gui --bin native_terminal` passed
- Rust tests: prior terminal-ui (7) and GUI (8) tests passed; the new SSH fixture lifecycle test fails to receive an event within 5 seconds
- Slint compile: PASSED for the standalone terminal surface
- Visual verification: PASSED at 800x520; screenshot `/tmp/opencode/terminal-ui-rerun-window.png`
- Behavior parity: PARTIAL overall; the local PTY surface unit is verified by headless Slint callbacks, PTY harnesses, and native runtime evidence, while SSH transport and broader terminal features remain unverified

## Current unit result

- Scope: typed controller and worker adapter connecting `NativeLocalPtyBoundary` to `crates/terminal-ui`; no SSH, workspace, SFTP, tunnel, profile, or settings work.
- Implemented: `TerminalController`, typed command/event channels, worker-thread PTY pump, input/resize/close forwarding, and error/closed status handling.
- Verification: focused terminal-ui tests (7), GUI tests (8), GUI check, native PTY boundary/harness tests, and package formatting passed. Native runtime displayed a live PTY prompt at 800x520. Workspace-wide formatting still has pre-existing unrelated failures.
- Status: VERIFIED for the bounded local PTY surface slice. The native launch path mounts the adapter; headless Slint tests cover key, resize, close, status, and large-model callbacks; PTY harness tests cover special keys, resize negotiation, startup gating/order, close propagation, and 64 KiB output; runtime evidence shows a live prompt at 800x520. Host-window geometry automation and full terminal parity remain outside this unit.

## Current unit result

- Scope: native SSH transport adapter for the verified terminal surface; no tabs/splits, SFTP, tunnels, profiles, or settings work.
- Implemented: `SshTerminalSurface`, worker-thread `open_shell` transport, typed bidirectional output/input/resize/close channels, connecting/connected/closed/error lifecycle states, reconnect request path, and `native_terminal` harness entry point. This attempt changed only the test fixture: `PermitRootLogin prohibit-password` replaces `no`, because the environment runs the fixture as `root`.
- Diagnostics: `/usr/sbin/sshd` and `/usr/bin/ssh` are OpenSSH 10.2p1; `sshd -t` passed. With the prior fixture, sshd stderr reported `ROOT LOGIN REFUSED` after `Accepted key`; the corrected fixture's OpenSSH client run returned status 0 and produced `READY` and `ECHO:hello`.
- Verification: the narrow Rust lifecycle test is blocked before compilation by missing `gobject-2.0 >= 2.70`, `glib-2.0 >= 2.70`, and `gio-2.0 >= 2.70` pkg-config packages. `cargo fmt --package sshcli-gui -- --check` fails on existing formatting differences, including this file; `cargo check --locked -p sshcli-core` passed. No native Rust SSH lifecycle, host-key, reconnect, or live resize evidence is claimed.
- Follow-up verification: `cargo check --locked -p sshcli-core` still passes; the required GTK/GLib/GObject/GIO pkg-config metadata remains unavailable, so the narrow GUI lifecycle test cannot be compiled.
- Continuation gate: prerequisite checks still report `gobject-status=1`, `glib-status=1`, and `gio-status=1`. One environment repair was attempted (`apt-get`/`dpkg`), but package configuration timed out while configuring an interrupted PostgreSQL package; the required development metadata remains unavailable.
- Dependency gate: no later migration unit is dependency-safe while native SSH transport remains unverified; tabs/splits and subsequent features are not started.
- Status: BLOCKED; the fixture correction and environment prerequisite attempt did not produce native Rust SSH lifecycle evidence. Stop this unit rather than retrying fixture/transport changes.
