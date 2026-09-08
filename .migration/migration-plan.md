# Dependency-Ordered Migration Plan

1. Preserve and test `sshcli-core` profiles, credentials, host keys, SSH, SFTP, forwarding, and shell detection.
2. Define direct Rust/Slint service boundaries and typed session/progress/error channels while Tauri remains available.
3. Establish a minimal Slint app shell with equivalent window sizing, navigation, status/error model, and theme tokens.
4. Migrate profile persistence and Home/Connections/profile dialogs.
5. Migrate shared password and host-key dialogs with typed security errors.
6. Implement the bounded local PTY terminal vertical slice and select/validate the native terminal renderer.
7. Integrate SSH terminal transport and terminal resize/reconnect behavior.
8. Migrate tabs, recursive split topology, focus, drag/reorder, and global shortcuts.
9. Migrate SFTP navigation/transfers/progress/overwrite/drop.
10. Migrate remote explorer and OSC 7 CWD integration.
11. Migrate tunnels, then telemetry.
12. Replace browser preferences/clipboard/file dialogs with explicit native services.
13. Run cross-platform behavior, keyboard, persistence, visual, and packaging verification; retire Tauri/web code only after each unit is VERIFIED.

## First recommended bounded unit

**Local PTY terminal vertical slice.** Keep `crates/gui/src/local_shell.rs` initially. Add a typed native adapter for start, readiness, byte output, input, resize, and closed/error status; render one local PTY in a dedicated terminal surface. Explicitly exclude SSH authentication, tabs/splits, SFTP, tunnels, and workspace persistence. The slice must test startup buffering/order, ANSI/VT basics, resize, input encoding, focus, close, and large output before broad implementation.
