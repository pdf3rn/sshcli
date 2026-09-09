# Current Architecture Summary

Evidence: `README.md:20-30`, `Cargo.toml`, `crates/gui/src/lib.rs`, `crates/gui/ui/src/App.tsx`.

```text
React + TypeScript + xterm.js + Dockview
        | Tauri invoke/listen IPC
crates/gui Rust command/state managers
        | direct Rust APIs
sshcli-core: SSH, SFTP, profiles, credentials, host keys, shells, forwarding
```

- Rust workspace contains `crates/core` and `crates/gui` (edition 2021, version 1.6.0).
- `sshcli-core` is UI-independent and owns networking, profiles, credentials, host keys, SFTP, and forwarding.
- `sshcli-gui` currently owns Tauri registration, SSH/local PTY session managers, SFTP session state, tunnels, telemetry, and command adapters.
- The UI has four in-memory views: `home`, `connections`, `session`, and `settings`; there is no router.
- Window baseline is 1100x720, minimum 800x520, one `main` window (`crates/gui/tauri.conf.json`).
- Frontend persistence is browser `localStorage` for preferences; profiles and secrets use native Rust persistence/keyring. Tabs, layout, SFTP paths, and terminal buffers are not persisted by source evidence.
- A standalone `crates/terminal-ui` Slint target now exists for the native terminal-surface slice, but it is not build-verified in the current environment. No Tauri plugins are used directly; the only configured capability is `core:default`.
- Existing target behavior remains the reference. No source Tauri code is removed by this inventory.
