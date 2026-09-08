# Source to Target Classification

| Source file/symbol | Responsibility | Classification | Existing Rust equivalent | Target/action |
|---|---|---|---|---|
| `ui/src/App.tsx` state/orchestration | views, tabs, dialogs, sessions | UI_STATE | `session.rs`, `local_shell.rs` | Rust view-model + Slint properties/models |
| `App.tsx` `invoke` handlers | profiles/connect/close/retry | TAURI_BRIDGE, ASYNC_IO | `commands.rs`, `session.rs` | replace bridge with typed Rust callbacks |
| `types.ts` `Profile` | profile domain DTO | DOMAIN | `core/src/profiles.rs` | reuse core type/adapt UI model |
| `types.ts` `Tab`, `View` | transient workspace state | UI_STATE | none | Rust workspace model + Slint |
| `prefs.ts` defaults/usePrefs | settings state | UI_STATE, PRESENTATION | none | Slint bindings + Rust persistence |
| `prefs.ts` localStorage | browser persistence | BROWSER_API | none | native preferences service |
| `HomeView.tsx` | home presentation | PRESENTATION | profile/session commands | MIGRATE_TO_SLINT |
| `ConnectionsView.tsx` | search/group/sort/list | PRESENTATION, UI_STATE | profile store | Slint model; Rust mutations |
| `ProfileModal.tsx` | form presentation/validation | UI_STATE, DOMAIN | `commands.rs` validation/persistence | Slint form; centralize validation in Rust |
| `TerminalTab.tsx` xterm | VT rendering, input, scrollback, selection | TERMINAL_RENDERING | `session.rs`, `local_shell.rs` transport only | CUSTOM_RENDERER; do not use TextEdit |
| `TerminalTab.tsx` OSC 7 | terminal protocol/CWD | TERMINAL_RENDERING, DOMAIN | none | Rust/native terminal parser |
| `TerminalDockview.tsx` | tabs/splits/drag/focus | PRESENTATION, UI_STATE | none | Slint panes + recursive Rust topology |
| `SftpPanel.tsx` operations | transfers/navigation | ASYNC_IO, TAURI_BRIDGE | `sftp_session.rs`, `core/src/sftp.rs` | typed Rust SFTP service + Slint model |
| `RemoteExplorerPanel.tsx` | remote shell listing/CWD | ASYNC_IO, TAURI_BRIDGE | `session.rs::ssh_exec` | Rust service + Slint sidebar |
| `TunnelPanel.tsx` lifecycle | forwarding controls | ASYNC_IO, TAURI_BRIDGE | `tunnel.rs`, `core/src/ssh.rs` | Rust manager + Slint list |
| `TelemetryPanel.tsx` sampling | remote metrics | ASYNC_IO, UI_STATE | `telemetry.rs` | Rust task + Slint meters |
| `use-dialog.ts` | focus trap/Escape | PRESENTATION, BROWSER_API | none | Slint focus/key handling |
| `TerminalTab.tsx` clipboard/resize | browser APIs | BROWSER_API | none | native clipboard + Slint resize |
| `main.tsx`, `vite.config.ts`, CSS/icons | web shell/build/presentation | PRESENTATION | none | native entry/components; retire only after parity |

Classification rule: networking, filesystem, credentials, PTY, SSH, SFTP, tunnels, telemetry, and business validation stay in Rust; presentation and transient interaction state move to Slint.
