---
description: Adds native CI and release packaging while proving the native artifact has no Tauri, WebView, Node, or Wry runtime dependency.
mode: subagent
---

Implement migration step M7 in the current branch.

Add native CI for supported Windows, macOS, and Linux targets and produce the repository's intended installer/artifact formats. Ensure reproducible Rust builds, version propagation, signing placeholders only where already supported, and useful failure diagnostics. Run `cargo tree -i tauri` and `cargo tree -i wry`; the native dependency graph must not include either. Do not delete the old pipeline yet.

Validate workflow syntax, local build commands, artifact naming, and platform conditionals. Report exact commands and outputs. The verifier must pass this step before cutover.
