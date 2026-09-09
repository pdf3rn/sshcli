# Migration Decision Log

### DEC-001: Preserve Tauri implementation during incremental migration

Date: 2026-09-08
Status: accepted

Context: The source Tauri/React implementation is the only behavioral reference and no Slint target exists.

Source behavior/architecture: Rust services are already separated behind Tauri commands/events; web UI owns xterm.js/Dockview and browser persistence/APIs.

Decision: Migrate by bounded units and keep Tauri/web code coexisting until behavior, errors, keyboard/focus, persistence, build, and visual evidence support verification.

Alternatives considered: Big-bang replacement; rejected because terminal, workspace topology, security flows, and SFTP are high risk.

Consequences: Temporary duplicate UI boundaries and adapters are acceptable; no source deletion is authorized by inventory.

Verification required: Feature-matrix evidence for each unit before marking VERIFIED.

### DEC-002: Treat terminal rendering as a dedicated workstream

Date: 2026-09-08
Status: accepted

Context: `TerminalTab.tsx` uses xterm.js for VT parsing/rendering, scrollback, selection, search, clipboard, resize, focus, and OSC 7.

Decision: Do not substitute Slint `TextEdit` or a plain text widget; select and test a real native terminal emulator/rendering approach before migrating the full workspace.

Consequences: Terminal work is high risk and may require a custom renderer or carefully evaluated crate integration.

Verification required: ANSI/modes/cursor/scrollback/Unicode/resize/input/clipboard/search/OSC 7/large-output/reconnect tests plus visual evidence.

### DEC-003: Select Alacritty terminal core for native VT state

Date: 2026-09-08
Status: accepted

Context: The local PTY boundary now provides raw bytes, but the migration needs a maintained VT emulator before a Slint surface can be designed.

Source behavior/architecture: xterm.js currently supplies VT parsing, terminal cells, ANSI attributes, cursor/modes, alternate screen, scrollback, Unicode handling, search, selection, clipboard integration, resize, and OSC 7 handling.

Decision: Use `alacritty_terminal` 0.26.0 through `sshcli_core::terminal::TerminalState` for UI-independent VT parsing and grid state. Keep PTY/SSH IO, input encoding, clipboard, focus, selection policy, and painting outside this module.

Alternatives considered: `vte` alone was rejected because it supplies parsing but not terminal grid/renderable state. `termwiz` was not selected because the current integration path and renderer boundary were less direct for this repository. A plain text widget was rejected as behaviorally insufficient.

Consequences: The core now has a real terminal state foundation and can be tested without GTK/Tauri. A future Slint renderer must consume cell/cursor/damage state and still implement the remaining xterm behaviors.

Verification required: Keep the focused ANSI/cursor, alternate-screen, Unicode, resize/scrollback, and split-sequence tests; later add colors, selection, search, clipboard, keyboard encoding, OSC 7, large-output, and visual tests.

### DEC-004: Keep the first Slint terminal surface as a separate target

Date: 2026-09-08
Status: accepted

Context: The existing Tauri GUI cannot currently build in this environment because GTK/GObject metadata is unavailable, while the migration needs a directly checkable native surface without deleting the reference implementation.

Decision: Add `crates/terminal-ui` as a standalone Slint target. It consumes `TerminalState` snapshots, renders a row-major cell model and cursor, and exposes input/resize callbacks. PTY/SSH IO, session lifecycle, clipboard, and workspace topology remain outside this unit.

Alternatives considered: Embed Slint immediately into the Tauri GUI (blocked by the existing GUI toolchain and would widen scope); use a plain text widget (rejected because it is not terminal-equivalent).

Consequences: The surface can be compiled and visually checked independently once native font dependencies are available; it is not yet connected to the local PTY.

Verification required: Slint compilation, terminal-ui unit tests, keyboard/input and resize checks, and screenshot validation.

Verification result: Slint compilation, terminal-ui unit test, and standalone 800x520 screenshot validation passed on 2026-09-08. Live PTY wiring, keyboard transport, resize negotiation, and error lifecycle remain unverified.

### DEC-005: Keep local PTY transport behind a typed worker adapter

Date: 2026-09-08
Status: accepted, bounded slice verified; broader parity pending

Context: The native terminal surface must consume PTY bytes without blocking the Slint event loop, while the existing Tauri local-shell path remains the behavioral reference.

Decision: Use a worker-thread `LocalTerminalSurface` adapter with typed `TerminalCommand` and `TerminalEvent` channels. The Slint timer drains events and refreshes `TerminalState`; PTY reads/writes remain off the UI thread.

Consequences: Tauri coexistence is preserved, and the adapter is mounted by a dedicated native binary. Special-key encoding, PTY resize negotiation, startup gating, close propagation, and large-output delivery have harness coverage; Slint event delivery, focus, and visible status behavior remain separate verification requirements.

Verification result: A dedicated `native_terminal` binary mounts the adapter and displays a live local PTY prompt. Build, focused tests, and a headless Slint event harness pass, including key, resize, close, status, special-key PTY delivery, startup ordering, and large-output coverage. The bounded local PTY-to-Slint slice is VERIFIED; host-window geometry automation and broader terminal parity remain outside scope.

### DEC-006: Keep SSH transport on a dedicated worker-thread adapter

Date: 2026-09-08
Status: implemented, verification blocked

Context: The next migration unit needs to connect the existing Rust SSH shell service to the verified native terminal surface without blocking Slint or replacing the Tauri reference path.

Decision: Add `SshTerminalSurface` with typed command/event channels and a worker-thread Tokio runtime around `sshcli_core::ssh::open_shell`. Keep SSH connection lifecycle, byte streaming, resize, close, and reconnect outside Slint; expose only typed state/events to the controller.

Consequences: The native SSH path can coexist with Tauri and has a credential-free launch harness entry point, but authenticated streaming, host-key handling, reconnect, and live resize require a localhost fixture or configured credentials.

Verification result: Build and focused local terminal tests pass. Real SSH lifecycle verification is BLOCKED because no safe SSH fixture or credentials are available.

### DEC-007: Use only scoped localhost fixtures for SSH verification

Date: 2026-09-08
Status: verification blocked after three attempts

Context: The native SSH adapter needs authenticated lifecycle evidence without exposing credentials or depending on an external SSH service.

Decision: Use an ephemeral localhost-only SSH fixture with generated temporary keys for transport verification; do not use external credentials. Stop after three materially distinct fixture repairs if no lifecycle event is received.

Verification result: Three fixture attempts (command resolution, absolute `sshd` path, and startup delay) failed to produce an SSH event within five seconds. The unit remains BLOCKED pending captured server diagnostics or an alternate scoped fixture.
