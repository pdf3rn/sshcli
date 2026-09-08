# Slint layout rules

## Preferred mental model

Before code, describe:

```text
Screen
└── VerticalLayout
    ├── Toolbar [preferred/fixed height]
    ├── HorizontalLayout [stretch]
    │   ├── Sidebar [bounded preferred width]
    │   └── Workspace [stretch]
    └── StatusBar [preferred/fixed height]
```

## Preferred primitives

- `HorizontalLayout`
- `VerticalLayout`
- `GridLayout`

Use nested layouts for complex screens.

Use:
- `min-width`
- `min-height`
- `max-width`
- `max-height`
- `preferred-width`
- `preferred-height`
- `horizontal-stretch`
- `vertical-stretch`
- `spacing`
- `padding`
- alignment properties

A fixed `width`/`height` inside a layout should be intentional.

## Absolute positioning

Explicit `x`/`y` is acceptable for:
- overlays
- custom graphical surfaces
- canvas/editor content
- terminal cell/cursor rendering when justified by the renderer design
- floating controls

Do not use it to emulate flexbox or compensate for misunderstanding a layout.

## Desktop density

sshcli is a technical desktop tool:
- preserve useful information density
- do not inflate controls by default
- preserve toolbar/sidebar/status proportions
- avoid generic AI-dashboard/card styling
