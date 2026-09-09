# Tauri Command, Event, and Plugin Inventory

Evidence: `crates/gui/src/lib.rs`, `commands.rs`, `session.rs`, `local_shell.rs`, `sftp_session.rs`, `tunnel.rs`, `telemetry.rs`, and UI `invoke`/`listen` calls.

## Commands (45 registered)

| Family | Commands |
|---|---|
| Profiles/credentials | `list_profiles`, `list_identity_keys`, `create_profile`, `update_profile`, `duplicate_profile`, `test_profile`, `delete_profile`, `save_profile_secret`, `ssh_trust_host_key`, `toggle_favorite`, `touch_last_used`, `import_profiles`, `export_profiles` |
| SSH sessions | `ssh_connect`, `ssh_connect_adhoc`, `ssh_write`, `ssh_resize`, `ssh_exec`, `ssh_list`, `ssh_close` |
| Local PTY | `local_shell_detect`, `local_shell_start`, `local_shell_ready`, `local_write`, `local_resize`, `local_close` |
| SFTP/filesystem | `sftp_connect`, `sftp_close`, `sftp_list_dir`, `sftp_pwd`, `sftp_download`, `sftp_upload`, `sftp_mkdir`, `sftp_rename`, `sftp_rm_file`, `sftp_rm_dir`, `sftp_file_exists`, `list_local_dir`, `local_home`, `local_file_exists` |
| Tunnels | `tunnel_start`, `tunnel_list`, `tunnel_stop` |
| Telemetry | `telemetry_sample`, `telemetry_disconnect` |

All commands currently return `Result<T, String>` over Tauri. Special error strings are `sshcli:password-required` and `sshcli:host-key:<JSON>`; native target should use typed errors while retaining distinct UI states.

## Events

| Event | Producer/payload | Consumer/behavior |
|---|---|---|
| `ssh-data` | SSH reader/local PTY; `{id, data: base64}` | `TerminalTab`; filter by id and write to xterm |
| `ssh-status` | SSH/local lifecycle; `{id, profile, status, message}` | `App`/`TerminalTab`; only `closed` currently drives UI behavior; `connected`, `profile`, and `message` have no observed UI consumer |
| `sftp-progress` | SFTP transfer; `{id,name,direction,transferred,total}` | `SftpPanel`; 256 KiB throttling plus initial/final |

## Plugins, capabilities, windows

- No application Tauri plugins found (no dialog, fs, shell, clipboard, notification, updater, tray, or window plugin use).
- `crates/gui/capabilities/default.json` grants only `core:default` to window `main`.
- One configured window, title `sshcli`, 1100x720, minimum 800x520; no runtime window API/lifecycle use.
- Tauri builder manages session, SFTP, local shell, tunnel, and telemetry state in `crates/gui/src/lib.rs`.

## Replacement boundary

`Slint callback -> Rust service/view-model -> typed result/channel -> Slint property/model`. Keep IO off the UI thread. Replace base64/global event bus internally with typed bytes/events where safe, while preserving ordering and startup readiness semantics.

## Browser/WebView boundaries requiring native replacements

The absence of Tauri plugins does not mean the source has no native-facing APIs. The React UI currently uses:

| Source behavior | Evidence | Target boundary |
|---|---|---|
| Preferences JSON persistence | `crates/gui/ui/src/prefs.ts:61-85`; key `sshcli.prefs.v1` | Rust preferences store; preserve defaults and silent fallback/write-failure behavior |
| Profile import | `HomeView.tsx:47,88-102` | Native file picker/read service -> `import_profiles` content |
| Profile export | `HomeView.tsx:104-115` | Native save-file service; source uses `Blob`, object URL, and synthetic download |
| Clipboard | `TerminalTab.tsx:213-255`, `RemoteExplorerPanel.tsx:71-79` | Native clipboard service; copy-on-select, copy/paste, and OSC 7 snippet copy |
| File drag/drop | `SftpPanel.tsx:447-465` | Native drop path service; source relies on nonstandard `File.path` |
| DOM/window events | `App.tsx:50-56,321-345`, `use-dialog.ts`, `TerminalTab.tsx:187-200` | Slint focus/key/window/layout callbacks; preserve shortcut precedence and resize policy |

## Lifecycle and cancellation notes

- No central Tauri shutdown handler is present; cleanup is distributed across React effects and best-effort command calls.
- SSH reader tasks are aborted on explicit close; local PTY children are killed; tunnel stop awaits forwarding shutdown; SFTP close removes the session handle; telemetry disconnect removes cached exec state.
- SFTP transfers have no cancellation token or UI cancellation behavior. Do not claim cancellation parity during migration.
- `ssh-status.connected` is emitted by Rust but has no observed frontend state transition; only `closed` is acted upon.

## Native coexistence inventory

- `crates/terminal-ui` provides the verified standalone Slint surface.
- `crates/gui/src/native_terminal.rs` provides local PTY and native SSH adapters plus the `native_terminal` harness entry point.
- These adapters coexist with, but do not replace, the Tauri command/event paths.
