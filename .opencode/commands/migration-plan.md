---
description: Build or refresh a dependency-ordered incremental migration plan from current repository evidence and migration state.
agent: migration-orchestrator
---

Create or refresh the migration plan for: $ARGUMENTS

Use current `.migration/` state and source evidence.

Each plan item must include:
- scope
- source files
- target files/modules
- dependencies
- behavior to preserve
- tests/verification
- visual verification if applicable
- risk
- rollback/coexistence notes

Prefer small independently verifiable units.
