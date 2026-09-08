---
name: slint-ui
description: Design and migrate native desktop interfaces with idiomatic Slint layouts, components, properties, callbacks, models, and visual verification.
compatibility: opencode
metadata:
  domain: ui
  target: slint
---

# Slint UI skill

Use this skill for `.slint` architecture, layout, visual components, interaction surfaces, and Rust/Slint UI boundaries.

Read as needed:
- `references/layout-rules.md`
- `references/component-boundaries.md`
- `references/visual-validation.md`
- `references/sshcli-layout-patterns.md`

Core rules:
1. Slint is declarative; model the UI as a component/layout tree.
2. Prefer automatic layouts over explicit coordinates.
3. Prefer constraints and stretch to window-specific magic numbers.
4. Keep application/native/async logic in Rust.
5. Use properties/models for data and callbacks for UI-to-Rust actions.
6. Verify visually when possible.
7. Check current official Slint documentation before relying on uncertain API details.
