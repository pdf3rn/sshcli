---
name: tauri-to-slint
description: Replace Tauri WebView bridge patterns with direct native Rust services and typed Slint callbacks/models while preserving behavior.
compatibility: opencode
metadata:
  source: tauri
  target: rust-slint
---

# Tauri -> native Rust + Slint

Read `references/bridge-mapping.md`.

Core rule: removing Tauri usually means removing an IPC boundary, not rewriting the underlying Rust feature.

Inventory:
- commands
- events
- plugins
- managed state
- window APIs
- app lifecycle
- tray/menu
- storage
- filesystem
- clipboard
- notifications
- updater if present

For each bridge record source caller, Rust implementation, result/error type, event semantics, target direct boundary, and tests.
