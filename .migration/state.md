# Migration State

## Status

- Phase: INCREMENTAL_MIGRATION
- Current unit: native terminal surface
- Last verified unit: native VT emulator core strategy
- Overall status: BLOCKED (Slint build prerequisites remain)

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
- Status: BLOCKED. The repository has no Slint target, and the focused GUI build cannot run because GTK/GObject system packages are unavailable (`gobject-2.0 >= 2.70`).

## Next unit

Resume verification of the native terminal surface after `fontconfig`/`freetype2` development metadata is available. Do not begin SSH, tabs/splits, SFTP, tunnels, profiles, or settings migration.

## Current unit result

- Scope: select and verify a maintained native VT emulator core; no Slint surface or painting.
- Selected: `alacritty_terminal` 0.26.0 (Apache-2.0), wrapped by `sshcli_core::terminal::TerminalState`.
- Rejected for this unit: direct `vte` alone (parser without terminal grid/renderable state) and `termwiz` (not selected because the existing API/renderer integration was less direct for this target).
- Verified: ANSI/cursor movement, alternate screen, Unicode/wide-character state, resize/scrollback, and escape sequences split across input chunks.
- Status: VERIFIED for emulator-core strategy; full local PTY target remains blocked pending a Slint surface and GUI prerequisites.

## Current unit result

- Scope: one standalone Slint terminal surface backed by `sshcli_core::terminal::TerminalState`; no PTY/SSH IO or other feature migration.
- Implemented: `crates/terminal-ui` with typed row-major cell model, cursor properties, input callback, resize callback, and a directly checkable `ui/terminal.slint` entry.
- Status: BLOCKED. `cargo check -p sshcli-terminal-ui` cannot run because `fontconfig` and `freetype2` development packages are unavailable through pkg-config. Visual verification was not possible.

## Verification summary

- Rust build: `cargo check -p sshcli-core` passed; `sshcli-terminal-ui` blocked by missing fontconfig/freetype2 metadata; `sshcli-gui` remains blocked by GTK/GObject metadata
- Rust tests: 12 core tests passed; terminal-ui tests could not compile because its Slint build dependency is blocked
- Slint compile: BLOCKED by missing fontconfig/freetype2 development metadata
- Visual verification: NOT RUN
- Behavior parity: BLOCKED; boundary tests are present but could not execute
