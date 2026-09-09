# Screen and Component Inventory

## Home

Source: `crates/gui/ui/src/HomeView.tsx`, `App.tsx`.
- Route/state: `View = 'home'`; `App` owns profiles and connection state.
- Layout: recent/favorite profiles, quick-connect, create/import/export, local terminal, browse-all.
- Interactions: connect profile, quick connect `user@host[:port]`, create, import/export TOML, open local shell.
- Native dependencies: `list_profiles`, `import_profiles`, `export_profiles`, SSH adhoc, `local_shell_start`.
- States: empty profiles, recent/favorite lists, connecting, error toast.
- Target: Slint view/model; Rust profile and session services; native file picker/export boundary. Risk MEDIUM.

## Connections

Source: `ConnectionsView.tsx`, `ProfileModal.tsx`, `NewConnectionModal.tsx`.
- Layout: searchable/grouped/sorted profile list and context actions.
- Interactions: Enter/Space/double-click connect; favorite, edit, duplicate, delete, SFTP, tunnels; profile form and test connection.
- Keyboard/focus: row `tabIndex`, Enter/Space activation, Escape context menu; modal focus trap/Escape/restore.
- Native dependencies: profile commands, identity keys, SSH test, host-key/password flows.
- States: loading, empty, selected, connecting, validation, password-required, host-key, errors.
- Target: Slint list/forms/dialogs; Rust profile/credential/host-key services. Risk MEDIUM/HIGH.

## Session workspace

Source: `TerminalDockview.tsx`, `TerminalTab.tsx`, `App.tsx`.
- Layout: tabs plus Dockview groups, resizable panels, splits, drag/reorder; minimum panel 240x140.
- Interactions: SSH/local terminal, tab close/reconnect, split movement, focus, terminal input, search, clear, copy/paste.
- Keyboard: Ctrl/Cmd-T/W/Tab/PageUp/PageDown/1-9; Ctrl/Cmd-F, Shift-K, Shift-C; modal focus rules.
- Native dependencies: SSH/local commands, `ssh-data`, `ssh-status`, resize/write/close, clipboard.
- States: connecting, connected, closed/disconnected, reconnecting, empty workspace, terminal search.
- Target: Rust session/channel layer; dedicated terminal emulator/rendering surface hosted by Slint; recursive pane topology model. Risk VERY HIGH.

## Settings

Source: `SettingsView.tsx`, `prefs.ts`.
- Layout: terminal appearance, scrollback, cursor, clipboard, theme, telemetry/explorer, local shell.
- State/persistence: `usePrefs` with `localStorage` key `sshcli.prefs.v1`; shell detection via `local_shell_detect`.
- Target: Slint controls plus Rust preferences persistence service. Risk MEDIUM.

## SFTP panel

Source: `SftpPanel.tsx`, `sftp_session.rs`.
- Layout: dual local/remote file browsers, path fields, progress/confirmation dialogs.
- Interactions: navigation, selection, upload/download, overwrite, mkdir, rename, delete, drag/drop upload.
- Native dependencies: SFTP/local filesystem commands and `sftp-progress` event; password/host-key retry.
- States: connecting, loading, busy, progress, overwrite prompt, errors, empty directory.
- Target: Rust SFTP/transfer service and typed progress model; Slint dual-pane browser. Risk VERY HIGH.

## Remote explorer

Source: `RemoteExplorerPanel.tsx`, `TerminalTab.tsx` OSC 7 parser.
- Layout: optional remote directory sidebar with breadcrumbs and entries.
- Behavior: follows terminal CWD, executes shell `ls`, stale-request suppression, copies shell OSC 7 snippets.
- Target: Rust command/CWD service plus Slint model and native clipboard. Risk HIGH.

## Tunnels

Source: `TunnelPanel.tsx`, `tunnel.rs`, `core/src/ssh.rs`.
- Layout: local bind/target form and active tunnel list.
- Behavior: local forwarding start/stop, two-second list polling, password/host-key retry.
- Target: Rust tunnel manager with Slint form/list. Current source supports local forwarding only. Risk HIGH.

## Telemetry

Source: `TelemetryPanel.tsx`, `telemetry.rs`.
- Layout: CPU/memory/disk/network meters.
- Behavior: optional three-second remote `/proc` sampling, cached exec sessions, retry once, disconnect cleanup.
- Target: Rust sampling service/task and typed sample model; Slint meters/errors. Risk HIGH and Linux-specific.

## Shared dialogs/components

`PromptDialog.tsx`, `HostKeyDialog.tsx`, `use-dialog.ts`, `TopBar.tsx`, `StatusBar.tsx`, `icons.tsx`, `styles.css`.
- Preserve modal focus trap, Escape, focus restoration, security warning states, navigation, live-session count, and theme tokens.
- Target reusable Slint components; Rust owns secrets, host-key verification, and async operations. Risk MEDIUM.

## Audit refresh: cross-cutting source behavior

### Shell, keyboard, and focus

Evidence: `App.tsx`, `TopBar.tsx`, `StatusBar.tsx`, `use-dialog.ts`, `TerminalDockview.tsx`, `TerminalTab.tsx`.

- The single-window shell keeps the session workspace mounted but hidden when another view is selected.
- Global shortcuts include Ctrl/Cmd-T, W, Tab, PageUp/PageDown, and 1–9; terminal shortcuts include search, clear, and copy.
- Dialogs trap Tab/Shift+Tab, close on Escape/backdrop click, and restore prior focus.
- Native target must preserve shortcut precedence in text fields/dialogs and terminal refocus on panel activation.

### Browser APIs

- Preferences use `localStorage` key `sshcli.prefs.v1` with merged defaults and silent fallback on parse/storage errors (`ui/src/prefs.ts`).
- Profile import uses HTML file input and `File.text()`; export uses `Blob`, object URLs, and synthetic download (`HomeView.tsx`).
- Terminal and explorer use `navigator.clipboard` (`TerminalTab.tsx`, `RemoteExplorerPanel.tsx`).
- SFTP drag/drop depends on browser-specific `File.path` (`SftpPanel.tsx:447-465`).

### Source behavior clarifications

- Dockview owns the effective group topology; `App.tsx` tracks tabs/active tab and a split-present flag, not a persisted recursive layout tree.
- `ssh-status.connected` is emitted but current UI consumers only act on `closed`.
- Password and host-key retry orchestration is duplicated across App, SFTP, tunnels, and telemetry.
- No frontend/component test suite was found; Rust tests are the primary existing automated evidence.
