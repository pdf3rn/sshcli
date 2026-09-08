---
name: html-css-to-slint
description: Semantically convert HTML/CSS/Tailwind-style web presentation into Slint layout intent instead of performing brittle tag-by-tag translation.
compatibility: opencode
metadata:
  source: html-css
  target: slint
---

# HTML/CSS -> Slint semantic migration

Never perform blind string substitution.

Pipeline:

```text
source component
  -> structure
  -> style/layout intent
  -> interaction states
  -> semantic UI tree
  -> Slint layouts/components
```

Read:
- `references/mapping.md`
- `references/unsupported.md`

If Tailwind is used, derive the resulting layout/style semantics; do not reproduce utility-class architecture inside Slint.

If CSS positioning is a workaround for a web layout, migrate the intent rather than the workaround.
