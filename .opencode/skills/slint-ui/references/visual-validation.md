# Visual validation

A compiled UI may still be wrong.

When possible:

```bash
slint-viewer --check path/to/component.slint
slint-viewer --screenshot screenshot.png path/to/component.slint
```

For real screens that require Rust data/callbacks, run the actual application or project UI harness.

Compare:
- original source screenshot/runtime
- new target screenshot/runtime

Check:
- hierarchy
- proportions
- clipping
- scroll
- resize
- alignment
- spacing
- selected/active/focus states
- error/loading/empty states
- dialogs
- content density

Test at multiple window sizes for resizable layouts.

Record screenshot paths/evidence in the feature matrix when useful.
