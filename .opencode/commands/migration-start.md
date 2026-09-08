---
description: Initialize migration state, inventory sshcli, and create the first bounded Tauri/TypeScript -> Rust/Slint migration plan.
agent: migration-orchestrator
---

Start the sshcli migration.

If `.migration/` does not exist, create it and copy/adapt the templates from `.opencode/templates/`.

Then perform a source inventory using the appropriate subagents.

Do not begin broad implementation yet.

Produce:
1. current architecture summary
2. screen inventory
3. feature matrix
4. Tauri command/event/plugin inventory
5. TypeScript responsibility classification
6. existing Rust code worth preserving
7. high-risk areas
8. dependency-ordered migration plan
9. first recommended bounded migration unit

Record the results under `.migration/`.
