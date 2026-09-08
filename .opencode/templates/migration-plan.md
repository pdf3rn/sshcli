# Migration Plan

Order units by dependency and verification value.

| # | Unit | Source scope | Target scope | Dependencies | Risk | Verification | Status |
|---:|---|---|---|---|---|---|---|
| 1 | TBD | TBD | TBD | - | TBD | TBD | PLANNED |

## Planning rules

- Prefer vertical slices that can be verified independently.
- Preserve the source application during coexistence.
- Establish shared design tokens/components before duplicating them across screens.
- Migrate high-risk terminal rendering only after its responsibilities are explicitly inventoried.
- Do not postpone all verification until the end.
