# Tauri bridge mapping

Typical conceptual migration:

```text
TypeScript UI
  invoke("connect", args)
        |
        v
Tauri command
        |
        v
Rust service
```

becomes:

```text
Slint callback
        |
        v
Rust UI adapter
        |
        v
Rust service
        |
        v
typed result/state
        |
        v
Slint property/model update
```

Tauri events become typed Rust event/state propagation rather than stringly-typed global event buses when possible.

Do not:
- retain an IPC-shaped API internally without reason
- serialize through JSON just because Tauri did
- duplicate Rust services in frontend logic
- block the Slint event loop with SSH/filesystem/process work
