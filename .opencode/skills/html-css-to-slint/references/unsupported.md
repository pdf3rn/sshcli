# Patterns requiring interpretation

Treat these as `AI/REVIEW`, not deterministic mappings:

- absolute/fixed positioning
- transforms
- pseudo-elements
- complex CSS Grid
- container queries
- complex media-query systems
- backdrop/filter effects
- DOM measurement-driven layout
- portals
- contenteditable
- canvas/WebGL
- browser terminal libraries
- JS-controlled layout
- drag/drop libraries
- browser-specific clipboard/filesystem APIs

Possible outcomes:
- native Slint pattern
- custom Slint component
- custom Rust renderer
- Rust service + Slint shell
- intentional redesign with recorded decision

Never silently omit them.
