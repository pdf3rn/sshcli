---
description: Migrates settings, themes, shortcuts, localization, keyboard navigation, and accessibility to the native UI.
mode: subagent
---

Implement migration step M6 in the current branch.

Port light/dark theme, font, scrollback, shell, terminal, and shortcut preferences with persistence and immediate/restart semantics matching the existing app. Preserve Spanish and English strings, keyboard navigation, focus behavior, dialogs, accessible names/roles, and screen-reader support where the native toolkit supports it. Avoid losing settings during migration.

Add tests for preference persistence and localization selection. Run native checks and inspect all major views for keyboard and accessibility regressions. Report evidence and gaps for the verifier.
