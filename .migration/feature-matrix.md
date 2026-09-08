# Feature Matrix

All features remain unverified in Slint. The native VT emulator core strategy is verified, but the local PTY terminal surface remains blocked before visual/UI parity can be built and verified.

| Area | Feature | Source evidence | Target | State | Behavior evidence | Visual evidence | Risk | Notes |
|---|---|---|---|---|---|---|---|---|
| Connections | Profiles, validation, groups, tags, favorites | `core/src/profiles.rs`, `commands.rs`, `ProfileModal.tsx` | Rust core + Slint forms/list | INVENTORIED | source only | source only | MEDIUM | Secrets excluded from TOML |
| Connections | Quick connect and adhoc auth | `adhoc.ts`, `session.rs` | Rust parser/service + Slint dialog | INVENTORIED | source only | source only | HIGH | Preserve password/host-key retry |
| Connections | Host-key trust/changed-key flow | `host_keys.rs`, `HostKeyDialog.tsx` | Rust typed security flow + Slint dialog | INVENTORIED | source only | source only | HIGH | Changed keys must not silently connect |
| Terminal | Remote SSH lifecycle/streaming | `session.rs`, `TerminalTab.tsx` | Rust session channels + native terminal | INVENTORIED | source only | source only | VERY HIGH | Async bidirectional bytes |
| Terminal | Local PTY lifecycle | `local_shell.rs`, `TerminalTab.tsx` | `NativeLocalPtyBoundary` + `crates/terminal-ui` surface | BLOCKED | boundary tests added; Slint target test blocked by missing fontconfig/freetype2 metadata | no visual evidence; Slint compile blocked | HIGH | Surface structure exists; PTY wiring remains intentionally out of scope for this unit |
| Terminal | VT parser, cells, ANSI, cursor, modes | xterm config/use in `TerminalTab.tsx` | `sshcli_core::terminal::TerminalState` + future native renderer | VERIFIED | 5 focused core tests: ANSI/cursor, alternate screen, Unicode, resize/scrollback, split escape sequences | n/a for core strategy; no Slint target | VERY HIGH | `alacritty_terminal` 0.26.0 selected under Apache-2.0; rendering, selection, search, clipboard, keyboard, resize fit, and OSC 7 surface integration remain unverified |
| Terminal | Scrollback, selection, search, clipboard | `TerminalTab.tsx`, xterm addons | Native terminal surface/services | INVENTORIED | none | source only | VERY HIGH | Keyboard-heavy |
| Terminal | Resize and OSC 7 CWD | `TerminalTab.tsx`, `App.tsx` | typed resize + parser/CWD model | INVENTORIED | none | source only | HIGH | Preserve split-sequence behavior |
| Workspace | Tabs, splits, drag/reorder, focus | `TerminalDockview.tsx`, `App.tsx` | Recursive Rust topology + Slint | INVENTORIED | none | source only | VERY HIGH | Source topology not persisted |
| SFTP | Navigation/listing/local browser | `SftpPanel.tsx`, `sftp_session.rs` | Rust service + dual Slint models | INVENTORIED | core path tests | source only | HIGH | Preserve sorting/empty/error states |
| SFTP | Upload/download/progress/overwrite | `core/src/sftp.rs`, `sftp_session.rs` | Rust transfer service + progress model | INVENTORIED | core unit coverage partial | source only | VERY HIGH | No current cancellation |
| SFTP | Drag/drop, mkdir, rename, delete | `SftpPanel.tsx` | native drop + Rust ops | INVENTORIED | none | source only | HIGH | Browser `.path` behavior |
| Tunnels | Local forwarding start/list/stop | `core/src/ssh.rs`, `tunnel.rs` | Rust manager + Slint list | INVENTORIED | none | source only | HIGH | No remote/SOCKS evidence |
| Telemetry | Optional remote metrics | `telemetry.rs`, `TelemetryPanel.tsx` | Rust sampler + Slint meters | INVENTORIED | none | source only | HIGH | Linux `/proc` assumptions |
| Settings | Terminal/theme/preferences | `prefs.ts`, `SettingsView.tsx` | Rust persistence + Slint controls | INVENTORIED | none | source only | MEDIUM | Current storage is localStorage |
| Desktop | Window/startup/build packaging | `tauri.conf.json`, `main.rs` | Slint window/native packaging | INVENTORIED | source config only | source only | MEDIUM | Keep 1100x720 / 800x520 baseline |
| Security | Keyring/DPAPI/credential migration | `core/src/credentials.rs` | Preserve and expose Rust service | INVENTORIED | core tests partial | source only | VERY HIGH | Do not expose secrets to UI model |
| Persistence | Profiles/known hosts | `profiles.rs`, `host_keys.rs` | Preserve Rust stores | INVENTORIED | core tests partial | n/a | HIGH | Atomic-ish writes |
