# sshcli Migration Instructions

This repository is being migrated from a Tauri + TypeScript UI to a native Rust + Slint UI.

## Primary objective

Preserve observable behavior and capabilities while replacing the web UI layer with Slint. Do not perform a cosmetic rewrite that silently drops functionality.

## Non-negotiable rules

1. Treat the existing application as the behavioral reference until a feature is explicitly verified in the Slint target.
2. Do not delete or broadly rewrite the existing Tauri implementation before parity is demonstrated.
3. Prefer incremental migration by feature/screen over a big-bang rewrite.
4. Do not translate HTML/CSS literally into Slint.
5. Convert visual intent into Slint layouts, constraints, components, properties, callbacks, and Rust models.
6. Do not reproduce browser-only implementation details when a simpler native design exists.
7. Keep networking, SSH, SFTP, PTY, tunnels, persistence, filesystem access, async work, and business logic in Rust.
8. Keep Slint focused on presentation, interaction, UI state, callbacks, models, styling, and animation.
9. Compilation is necessary but not sufficient for UI work. Visual verification is required when the affected screen can be rendered.
10. Never claim parity without evidence recorded in `.migration/feature-matrix.md`.
11. Never invent an API, Slint property, widget, crate capability, or Tauri behavior. Inspect the repository or current official documentation first.
12. Do not change public behavior merely to simplify migration unless the change is recorded as an explicit decision.
13. Preserve keyboard-heavy workflows. sshcli is a technical desktop application, not a mobile-first form application.
14. Avoid hidden infinite retry loops. Follow the migration stop conditions below.

## Required migration state

Migration work must use these files when they exist:

- `.migration/state.md`
- `.migration/feature-matrix.md`
- `.migration/screen-inventory.md`
- `.migration/decision-log.md`
- `.migration/blockers.md`
- `.migration/source-target-classification.md`

If they do not exist and the user requested migration work, initialize them from `.opencode/templates/`.

Update migration state after every coherent migration unit.

## Migration stages

Use this order unless repository evidence requires a different dependency order:

1. Inventory the source application.
2. Map screens, components, routes, commands, events, plugins, state, storage, and native integrations.
3. Identify Rust code in `src-tauri` that should be preserved.
4. Classify TypeScript responsibilities.
5. Define target Rust/Slint architecture.
6. Build a minimal Slint application shell.
7. Migrate reusable design tokens and basic components.
8. Migrate screens/features in dependency order.
9. Migrate interactions and Tauri bridges to direct Rust/Slint boundaries.
10. Verify behavior, visual structure, keyboard operation, error handling, and persistence.
11. Only after full verification, propose removal of obsolete Tauri/web code.

## TypeScript classification

For each TypeScript unit, classify it before translating:

- `PRESENTATION`: layout/style/display logic -> Slint.
- `UI_STATE`: transient view state -> Slint property/model or Rust view-model, depending on ownership.
- `DOMAIN`: application/business logic -> Rust.
- `TAURI_BRIDGE`: `invoke`, events, plugin wrappers -> direct Rust service/callback/channel boundary.
- `BROWSER_API`: DOM, clipboard, localStorage, Web APIs -> native Rust/Slint equivalent.
- `ASYNC_IO`: network/filesystem/process/PTY -> Rust async/service layer.
- `TERMINAL_RENDERING`: specialized terminal behavior -> dedicated migration plan; never replace with a plain text widget.
- `UNKNOWN`: investigate before changing.

## Slint layout mental model

Before editing UI, derive a layout tree.

Think in terms of:
- parent and children
- row/column/grid structure
- preferred/min/max size
- stretch factors
- spacing and padding
- alignment
- scrolling
- repeated models
- component boundaries

Prefer:
- `HorizontalLayout`
- `VerticalLayout`
- `GridLayout`
- reusable components
- automatic layout constraints

Avoid explicit `x`/`y` positioning unless the UI is genuinely overlay/canvas/custom-rendering oriented.

## HTML/CSS migration rules

Good semantic mappings include:

- flex row -> `HorizontalLayout`
- flex column -> `VerticalLayout`
- simple grid -> `GridLayout`
- gap -> layout `spacing`
- padding -> layout padding
- min/max/preferred dimensions -> Slint constraints
- flex grow -> stretch factor
- buttons/inputs/text/images -> appropriate Slint widgets/elements

Do NOT mechanically map:
- `position: absolute`
- transforms used for centering
- pseudo-elements
- complex CSS Grid
- responsive media-query systems
- browser-specific overflow hacks
- DOM measurement code

Determine the intent first.

## sshcli-specific high-risk features

Treat these as explicit parity workstreams:

- SSH connection lifecycle and reconnect behavior
- local PTY
- remote terminal rendering/emulation
- bidirectional terminal streaming
- terminal resize propagation
- tabs
- nested/resizable splits
- drag-and-drop between tabs/panes if present
- SFTP navigation and transfers
- transfer progress/cancellation/error handling
- tunnels/port forwarding
- saved connection profiles, groups, tags, favorites
- quick connect syntax and validation
- keyboard shortcuts and focus behavior
- secure credential/key handling
- theme and font behavior
- notifications and connection status
- persistence and restoration of workspace state

If the existing UI uses xterm.js or another browser terminal renderer, do not assume a Slint `TextEdit` is an equivalent replacement. Inventory the terminal emulator/rendering responsibilities and design a native terminal surface separately.

## Verification gates

A migration unit is complete only when applicable checks pass:

- target compiles
- formatting passes
- tests pass
- lint/clippy is reviewed
- Slint file checks pass
- behavior parity checks pass
- visual inspection passes
- no known regression is hidden
- state files are updated

## Anti-loop policy

For one failing issue:

1. Attempt at most 3 materially different fixes.
2. Do not repeat the same command/change without new evidence.
3. After the second failed attempt, re-read the relevant source and diagnostics.
4. After the third failed attempt, stop modifying that issue and record it in `.migration/blockers.md` with:
   - symptom
   - attempted fixes
   - evidence
   - likely root cause
   - next recommended investigation
5. Continue independent migration work if safe.
6. Never endlessly alternate between two implementations.
7. Never weaken or delete a test merely to make the migration appear successful.

## Git safety

Do not:
- force-push
- reset hard
- delete branches
- rewrite unrelated history
- remove the original Tauri implementation without explicit user approval

Prefer small coherent commits when the user asks for commits.
