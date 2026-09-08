# Mapping guidance

These are semantic starting points, not mandatory one-to-one translations.

| Web intent | Slint intent |
|---|---|
| flex row | `HorizontalLayout` |
| flex column | `VerticalLayout` |
| simple grid | `GridLayout` |
| gap | `spacing` |
| padding | layout padding |
| min/max/preferred sizing | Slint layout constraints |
| flex-grow | stretch factor |
| text | `Text` |
| image | `Image` |
| button | Slint button/control or project component |
| single-line input | Slint line-edit/control or project component |
| repeated list | model + repeated component/list pattern |
| overflow content | appropriate scroll pattern |
| conditional class/state | Slint state/property binding |

Always inspect existing project components before creating new primitives.

## Centering

Do not translate web transform hacks used only for centering. Express centering using Slint layout/alignment.

## Responsive behavior

Do not blindly copy media queries. Identify the actual desktop window behavior:
- minimum width
- sidebar collapse/hide
- pane minimums
- wrapping
- toolbar overflow
- dialog sizing

Implement that behavior explicitly.
