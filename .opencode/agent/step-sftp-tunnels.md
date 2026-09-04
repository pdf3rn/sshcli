---
description: Migrates SFTP, transfer management, tunnels, telemetry, and remote explorer functionality to the native UI.
mode: subagent
---

Implement migration step M5 in the current branch.

Implement native SFTP browsing with dual panes, navigation, refresh, upload/download, progress, cancel/error states, rename, delete, multi-select, and drag/drop where supported. Add tunnel start/stop/status/error behavior, telemetry display, and remote explorer behavior. Use extracted application services and preserve concurrency, cancellation, and cleanup semantics.

Test service behavior and failure paths. Validate the native UI builds and report a feature-by-feature parity matrix. Do not mark the step complete until `verifier-parity` returns `PASS`.
