# sshcli

Cross-platform SSH client with a terminal user interface.

## Current status

Phase 1 provides the TUI foundation:

- Connection list placeholder.
- Keyboard navigation with `j`/`k` or arrow keys.
- Phase status messages with `n` and `Enter`.
- Safe terminal restoration on exit with `q` or `Esc`.
- Platform-specific configuration directory discovery.

SSH sessions, profiles, secure credentials, SFTP, and port forwarding are planned for later phases.

## Development

```bash
cargo run
cargo run -- --print-config-dir
cargo fmt -- --check
cargo check
```
