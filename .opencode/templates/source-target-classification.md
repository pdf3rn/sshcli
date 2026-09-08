# Source to Target Classification

Use this table while auditing TypeScript/Tauri code.

| Source file/symbol | Responsibility | Classification | Existing Rust equivalent | Target | Action | Evidence/notes |
|---|---|---|---|---|---|---|
| TBD | TBD | UNKNOWN | TBD | TBD | investigate | |

Classification values:
- PRESENTATION
- UI_STATE
- DOMAIN
- TAURI_BRIDGE
- BROWSER_API
- ASYNC_IO
- TERMINAL_RENDERING
- UNKNOWN

Action values:
- KEEP_RUST
- REFACTOR_RUST
- MIGRATE_TO_RUST
- MIGRATE_TO_SLINT
- REPLACE_BRIDGE
- CUSTOM_RENDERER
- INTENTIONAL_RETIREMENT
- INVESTIGATE
