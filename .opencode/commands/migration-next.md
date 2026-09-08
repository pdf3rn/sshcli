---
description: Execute exactly the next safe bounded migration unit, verify it, update state, and stop.
agent: migration-orchestrator
---

Execute exactly one next migration unit from `.migration/state.md` / the migration plan.

Do not start a second unrelated unit.

Use specialized subagents as needed.

Run applicable verification and update migration state before finishing.

If blocked after the allowed distinct attempts, record the blocker and stop that unit rather than looping.
