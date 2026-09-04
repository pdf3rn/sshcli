---
description: Connects SSH sessions to the native terminal and replaces Dockview behavior with egui_dock tabs and splits.
mode: subagent
---

Implement migration step M3 in the current branch.

Wire the extracted SSH service into the native GUI. Replace the React Dockview model with egui_dock while preserving tabs, splits, active focus, connection progress, close confirmation, reconnect, errors, and session cleanup. Ensure terminal input/output and resize are connected to the SSH channel, not mocked. Preserve multiple concurrent sessions and prevent events from one tab reaching another.

Test lifecycle, failure, cleanup, and docking behavior with deterministic tests where possible. Run all relevant checks and report concrete evidence and remaining parity gaps. The verifier must approve this step.
