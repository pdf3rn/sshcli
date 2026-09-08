---
description: Summarize current migration progress, blockers, risks, and the next recommended unit from persisted migration state.
agent: migration-orchestrator
---

Read `.migration/` and report:
- completed and verified features
- implemented but unverified features
- remaining features
- current blockers
- high-risk unresolved items
- intentional differences
- next dependency-safe migration unit

Do not modify implementation code unless state files are internally inconsistent.
