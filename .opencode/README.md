# OpenCode migration kit: Tauri/TypeScript -> Rust + Slint

This directory is intended to be copied into the root of the existing `sshcli` repository.

The companion `AGENTS.md` in the package must also be placed at the repository root.

## OpenCode compatibility

This package targets **OpenCode V1**. It uses:

- `permission` (singular)
- `permission.task` for subagent access
- `permission.bash` for shell command rules
- `steps` for bounded agent iterations

Do not convert these fields to the OpenCode V2 `permissions` / `subagent` / `shell` schema unless you also switch to `opencode2`.


## What this kit provides

Agents:
- `migration-orchestrator`: coordinates the migration and owns the state machine.
- `source-auditor`: inventories the Tauri/TypeScript application without editing it.
- `tauri-decoupler`: maps Tauri commands/events/plugins into native Rust/Slint boundaries.
- `slint-ui-architect`: migrates HTML/CSS/component structure into idiomatic Slint.
- `ts-rust-migrator`: moves non-presentation TypeScript logic into Rust.
- `sshcli-specialist`: focuses on terminal, PTY, SSH, SFTP, tunnels, tabs and split panes.
- `parity-reviewer`: read-only behavioral parity review.
- `visual-verifier`: validates rendered Slint UI and screenshot evidence.
- `build-verifier`: runs deterministic verification and reports failures.

Skills:
- `slint-ui`
- `html-css-to-slint`
- `tauri-to-slint`
- `migration-control`
- `sshcli-domain`
- `behavior-parity`

Commands:
- `/migration-start`
- `/migration-audit`
- `/migration-plan`
- `/migration-next`
- `/migration-ui <screen/component>`
- `/migration-module <module>`
- `/migration-verify [scope]`
- `/migration-status`
- `/migration-finish`

## Recommended first run

From the existing repository root:

```bash
opencode
```

Then:

```text
/migration-start
```

The orchestrator should create `.migration/` from the templates, inventory the current application, and stop before destructive changes.

Then inspect the plan and use:

```text
/migration-next
```

repeatedly for bounded migration units.

## Tooling recommended on the machine

Rust:

```bash
rustup component add rustfmt clippy
cargo install slint-viewer
```

Do not blindly upgrade project dependencies during migration. Let the agent inspect the existing Rust toolchain, `Cargo.toml`, `Cargo.lock`, Node package manager, and build system first.

## Visual verification

If there is a directly previewable `.slint` entry component:

```bash
slint-viewer --check path/to/main.slint
slint-viewer --screenshot /tmp/sshcli-slint.png path/to/main.slint
```

For a real application screen requiring Rust-provided data/callbacks, prefer running the application itself and capturing the rendered UI through the platform or project-specific test harness.

Set `SLINT_ENTRY` before using the included verification scripts if a standalone entry can be checked:

Linux/macOS:
```bash
export SLINT_ENTRY=ui/app.slint
```

PowerShell:
```powershell
$env:SLINT_ENTRY = "ui/app.slint"
```

## Important design decision

This kit intentionally does not implement a blind HTML -> Slint text translator.

The migration pipeline is:

```text
HTML/TS/CSS source
       |
       v
semantic inventory
       |
       v
layout/component/behavior model
       |
       +---- presentation ----------> Slint
       |
       +---- domain/native/async ---> Rust
       |
       +---- Tauri bridge ----------> direct Rust <-> Slint boundary
       |
       v
behavior + visual verification
```

The old application remains the behavioral reference until feature parity is verified.
