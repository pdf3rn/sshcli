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

Port forwarding is planned for the next phase.

## Profiles and credentials

Profiles contain only connection metadata in the platform configuration directory. Passwords and key passphrases are stored through the native keyring using the `keyring` crate.

```bash
cargo run -- profile add production \
  --host server.example.com \
  --user deploy \
  --identity-file ~/.ssh/id_ed25519 \
  --accept-unknown-host-key

cargo run -- profile list
cargo run -- profile remove production
```

`profile add` prompts for secrets without echoing them. For an unencrypted private key, leave the passphrase prompt empty. The current `--accept-unknown-host-key` option is an explicit temporary measure until persistent `known_hosts` validation is implemented.

## SFTP

SFTP operations reuse a saved profile and never expose its credentials in command arguments:

```bash
cargo run -- sftp production pwd
cargo run -- sftp production ls /var/log
cargo run -- sftp production get /var/log/app.log ./app.log
cargo run -- sftp production put ./release.tar.gz /tmp/release.tar.gz
cargo run -- sftp production mkdir /tmp/releases
cargo run -- sftp production rm /tmp/release.tar.gz
cargo run -- sftp production rmdir /tmp/releases
```

From the TUI, select a profile and press `s` to open the remote browser. Use the arrow keys or `j/k` to navigate, `Enter` to open a directory, `Backspace` to go up, `d` to download the selected file, and `q` to return.

## Keyboard shortcuts

Connection list:

- `j/k` or arrow keys: navigate.
- `n`: open the new connection modal.
- `Enter`: open an interactive SSH session.
- `s`: open the SFTP browser.
- `f`: start local forwarding.
- `d`: delete the selected profile (press twice to confirm).
- `x`: close the background session of the selected profile.
- `q` or `Esc`: quit.

SSH session:

- The session runs as a real terminal passthrough without the alternate screen: colors, full-screen programs (vim, htop), native scrollback, bracketed paste, and mouse forwarding work correctly.
- `Ctrl+Q` detaches the session and returns to the connection list; the session keeps running in the background (marked with a green dot). Press `Enter` on it again to reattach, or `x` to close it.
- Set `SSHCLI_DETACH_KEY` to change the detach key (for example `ctrl-t`) or `none` to disable it.
- On connection failures, sshcli offers an interactive reconnect prompt.
- Keepalives are sent every 30 seconds so idle sessions survive NAT timeouts.

## Creating connections from the TUI

Press `n` in the connection list to open the new connection modal. Navigate with `Tab` or the arrow keys, use `Space` to cycle authentication options and existing `~/.ssh` keys, `Ctrl+U` to clear the focused field, and press `Enter` to save. Validation errors appear inside the modal in red. The password or key passphrase is requested after the form without echoing it.

SSH key files are read-only in this application. Creating or updating a profile never writes to, replaces, or deletes a selected key. Duplicate profile names are rejected instead of overwritten.

## Local port forwarding

Forward a local listener through a saved SSH profile:

```bash
cargo run -- forward production \
  --bind-host 127.0.0.1 \
  --bind-port 8080 \
  --target-host 127.0.0.1 \
  --target-port 80
```

The process remains in the foreground and stops cleanly with `Ctrl-C`. Remote and dynamic forwarding are planned extensions.

From the TUI, select a profile and press `f` to enter the same local forwarding flow interactively.

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
