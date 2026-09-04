# sshcli-native

Native terminal surface for sshcli: a real, interactive local terminal rendered
with a native GUI stack — `eframe`/`egui`, an `alacritty_terminal` emulator core,
and a `portable-pty`-owned shell session. This crate is independent of the
Tauri/WebView frontend (`sshcli-gui`) and does not depend on it.

## Engine choice: Option B (direct `alacritty_terminal` + `portable-pty`)

| Option | Description |
| --- | --- |
| A. `egui_term` | Wraps `alacritty_terminal` and ships a `TerminalView` widget. |
| B. direct `alacritty_terminal` + `portable-pty` | Own the PTY and render the grid ourselves. |

**We chose Option B.** Rationale:

1. **PTY ownership.** `egui_term::TerminalBackend` calls
   `alacritty_terminal::tty::new()` internally and owns its own read loop and
   `Notifier`. It exposes no hook to supply an external PTY. The requirement that
   *"the session is owned by `sshcli-native` (spawn via `portable-pty`, feed input,
   handle resize, emit closed state)"* and feature _closed/reconnect_ are
   impossible without forking the crate.
2. **Future SSH channels.** A later step must connect the terminal widget to an
   SSH channel instead of a local PTY. That is a transport swap in our
   `session`/`widget` layer — trivial with B, a rewrite with A.
3. **Missing features in A.** Search UI, clear scrollback, and a proper
   closed/restart state are not implemented by `egui_term` (it sends a
   `PtyEvent::Exit` and the demo just closes the window).

The one-time cost (a ~200-line grid renderer and a keyboard mapping) is small and
kept under unit test for the deterministic parts.

## Crate layout

```
src/
  main.rs     — binary: eframe::run_native, one dockable terminal tab
  lib.rs      — re-exports + crate docs
  app.rs      — NativeApp (eframe::App) + egui_dock TabViewer + shortcuts()
  widget.rs   — TerminalSession: PTY session + emulator + input/render/search
  session.rs  — PtySession: spawn/read-loop/resize/write/closed/restart
  term.rs     — TermModel: alacritty_terminal::Term + VTE Processor wrapper
  render.rs   — grid → egui shapes (cells, cursor, selection, scrollback)
  color.rs    — ANSI 16/256/truecolor → egui::Color32
```

## Parity features

1. **Real local shell PTY** — `PtySession::start` spawns `%COMSPEC%` on Windows
   and `$SHELL` (fallback `/bin/sh`) elsewhere via `sshcli_core::shells::detect_shell`,
   with `TERM=xterm-256color`, cwd = home, and an initial size.
2. **Input** — `TerminalSession::handle_key` + `Event::Text` write bytes to the PTY;
   Enter, backspace, tab, arrows, Home/End, Delete, Insert, Ctrl-C are mapped.
3. **Output** — ANSI colors, cursor movement, clear, CR/LF handled by the
   alacritty emulator; rendered by `render.rs`.
4. **Resize** — window resize → `cols/rows` recomputed → `session.resize(PtySize)`.
5. **Scrollback** — alacritty grid retains history (10k lines); wheel/PageUp scroll;
   `display_iter` renders the right slice.
6. **Selection + clipboard** — mouse drag selects; Ctrl+Shift+C copies;
   Ctrl+Shift+V / Paste forwards to PTY.
7. **Search** — Ctrl+Shift+F opens the search bar; Find-next wraps forward.
8. **Clear** — Ctrl+L clears screen; scrollback cleared via emulator API.
9. **Shortcuts** — see `app::shortcuts()` and the About tab (also below).
10. **Closed / reconnect** — when the PTY exits, a "Session closed" overlay is
   shown with a restart action (press `R`); `PtySession::restart` respawns.

### Keyboard shortcuts

| Keys | Action |
| --- | --- |
| `Enter` | submit (CR) |
| `Backspace` | delete (DEL) |
| `Tab` / `Shift+Tab` | tab / back-tab |
| `Arrow keys`, `Home`, `End`, `PgUp`, `PgDn`, `Delete`, `Insert` | VT100 sequences / scroll |
| `Ctrl+C` | SIGINT |
| `Ctrl+L` | clear screen |
| `Ctrl+Shift+C` | copy selection |
| `Ctrl+Shift+V` | paste |
| `Ctrl+Shift+F` | find in scrollback |
| `Mouse wheel` / `PgUp`/`PgDn` | scroll scrollback |
| `Mouse drag` | select text |
| `R` (when closed) | restart / reconnect |

## Building & running

```sh
cargo build -p sshcli-native
cargo run   -p sshcli-native
```

## Known limitations

- **GUI behavior is verified by compile + deterministic tests only** in this
  environment (headless); interactive behavior is validated by unit tests for
  the color mapping, emulator feed/resize/selection/search/scrollback, and PTY
  round-trip, not by manual runtime.
- **Alt-screen (alternate buffer) applications** rely on the emulator core and
  are rendered correctly, but our scrolling shortcuts always scroll the primary
  grid; mouse-mode (e.g. `vim` mouse) is not wired to the PTY.
- **Bracketed paste / OSC-52** are not enabled at the crate level; paste is
  forwarded as plain text.
- **Search** provides find-next only (forward wrap); no reverse search or match
  count UI.
- **Colors** use the default alacritty 16-color palette; user palette config is
  not exposed yet.
- `egui_term` is intentionally *not* used; see the engine-choice rationale above.