---
name: sshcli-domain
description: Migration rules for an SSH desktop client with terminals, PTY, SFTP, tunnels, tabs, split panes, profiles, transfers, and keyboard-centric workflows.
compatibility: opencode
metadata:
  app: sshcli
---

# sshcli domain migration

Read:
- `references/terminal.md`
- `references/splits.md`
- `references/feature-checklist.md`

The target must remain a serious desktop SSH tool. Preserve technical workflows before visual polish.

Do not assume a web terminal component can be replaced by a text control.
Do not flatten pane topology.
Do not move SSH/SFTP/PTY IO onto the UI thread.
