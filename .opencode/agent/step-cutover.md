---
description: Performs the final native UI parity gate, staged release, and removal of the obsolete Tauri/React/WebView implementation.
mode: subagent
---

Implement migration step M8 only after M1-M7 have verifier PASS results.

Run the complete parity matrix against the existing application and native build. Fix every discrepancy before removal. Stage a native prerelease such as `v2.0.0-native.1` only after all checks pass. Then make the native binary the primary product and remove Tauri, React, Vite, Node, and obsolete WebView workflows only when their behavior is covered by native code and the verifier explicitly approves the removal. Preserve release documentation and migration notes.

Do not claim final completion based on a local compile. Record platform test evidence, dependency-tree evidence, artifact evidence, and all known limitations. The independent verifier remains the final authority.
