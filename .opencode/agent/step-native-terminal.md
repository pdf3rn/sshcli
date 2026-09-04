---
description: Builds the native eframe terminal surface with PTY input/output and feature parity for local terminal behavior.
mode: subagent
---

Implement migration step M2 in the current branch using the native spike at `/root/sshcli-native-spike` as research only.

Create or extend the native GUI crate with eframe/egui and egui_dock. Select and document whether `egui_term` or direct `alacritty_terminal` is used. Implement a real local PTY, including `cmd.exe` on Windows, input, output, ANSI colors, cursor, resize, scrollback, selection, clipboard, search, clear, keyboard shortcuts, and closed/reconnect states. Do not leave a read-only demo or text-only ANSI renderer. Keep native terminal code independent from Tauri and WebView.

Add tests or deterministic harnesses for parser/color/input/resize behavior where GUI automation is unavailable. Run formatting, checks, and relevant tests. Report evidence and gaps; do not claim completion before `verifier-parity` passes.
