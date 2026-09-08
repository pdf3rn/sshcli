---
name: migration-control
description: Control a long-running incremental migration with persistent state, dependency-aware units, verification gates, blocker recording, and anti-loop limits.
compatibility: opencode
metadata:
  workflow: migration
---

# Migration control

Use `.migration/` as persistent external memory.

Read:
- `references/state-machine.md`
- `references/anti-loop.md`

Every feature has a state:

- `NOT_INVENTORIED`
- `INVENTORIED`
- `PLANNED`
- `IN_PROGRESS`
- `IMPLEMENTED`
- `VERIFYING`
- `VERIFIED`
- `BLOCKED`
- `INTENTIONAL_DIFFERENCE`

Never jump directly from unknown to verified.

Migrate one bounded unit at a time.

Prefer coexistence until parity is demonstrated.
