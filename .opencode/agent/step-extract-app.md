---
description: Extracts UI-independent SSH, SFTP, shell, tunnel, telemetry, and profile logic from Tauri into reusable Rust application services.
mode: subagent
---

Implement migration step M1 in the current branch.

Inspect `crates/core` and `crates/gui` before editing. Create the smallest reusable application layer needed by both the existing Tauri UI and the future native UI. Move or refactor session, SFTP, local shell, tunnel, telemetry, and profile operations away from `tauri::State`, `AppHandle`, and `Emitter`. Prefer typed Rust APIs and channels/events owned by the application layer. Keep the current GUI compiling and behavior unchanged.

Do not build the native UI in this step. Add focused tests for extracted behavior and run the relevant Rust and existing frontend checks. Report files changed, API decisions, tests, and known parity gaps. Do not claim completion; the independent verifier decides that.
