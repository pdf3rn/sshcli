# sshcli

Cross-platform SSH client with a terminal user interface.

## Current status

Phase 1 provides the TUI foundation and Phase 2 adds direct SSH sessions:

- Connection list placeholder.
- Keyboard navigation with `j`/`k` or arrow keys.
- Phase status messages with `n` and `Enter`.
- Safe terminal restoration on exit with `q` or `Esc`.
- Platform-specific configuration directory discovery.
- Interactive SSH sessions through a PTY.
- Private-key authentication through `russh`.
- Explicit host-key acceptance for the current direct-connect path.

Profiles, secure credentials, SFTP, and port forwarding are planned for later phases.

## Direct SSH session

Use a private key and explicitly accept the server key while `known_hosts` integration is being built:

```bash
cargo run -- connect server.example.com --user deploy --identity-file ~/.ssh/id_ed25519 --accept-unknown-host-key
```

The `--accept-unknown-host-key` flag is intentionally required. Do not use it for unattended or production workflows until the profile security phase adds persistent `known_hosts` validation.

## Development

```bash
cargo run
cargo run -- --print-config-dir
cargo fmt -- --check
cargo check
```
