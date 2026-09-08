# Rust / Slint component boundary

## Slint owns

- visual hierarchy
- layout
- presentation state
- widget state
- UI properties
- UI models
- callbacks
- styling
- animation
- focus/interaction declarations where appropriate

## Rust owns

- SSH
- SFTP
- PTY
- terminal session lifecycle
- tunnels
- filesystem
- persistence
- key/credential handling
- networking
- long-running/async work
- domain validation not specific to one widget
- orchestration

## Communication

Prefer typed APIs:
- Slint callback -> Rust handler/service
- Rust state/result -> typed property/model update
- worker/task -> event-loop-safe UI update

Avoid:
- passing generic JSON everywhere
- exposing internal service objects directly to UI
- putting async/network work inside `.slint`
