---
description: Migrate one specified Tauri web screen/component to idiomatic Slint with visual and behavioral verification.
agent: migration-orchestrator
---

Migrate this UI scope: $ARGUMENTS

Required sequence:
1. inventory source component and all behavior
2. classify non-presentation TypeScript
3. derive a Slint layout tree
4. reuse/create design-system components
5. implement Slint UI and Rust-facing callbacks/properties/models
6. do not pull domain/native logic into Slint
7. run build/Slint checks
8. perform visual verification when renderable
9. run parity review
10. update migration state

Do not delete the source component unless full migration cleanup has been explicitly approved.
