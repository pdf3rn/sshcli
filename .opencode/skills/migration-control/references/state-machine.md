# Migration state machine

For each feature:

```text
NOT_INVENTORIED
      |
      v
INVENTORIED
      |
      v
PLANNED
      |
      v
IN_PROGRESS
      |
      v
IMPLEMENTED
      |
      v
VERIFYING
   /      \
  v        v
VERIFIED  BLOCKED
```

`INTENTIONAL_DIFFERENCE` requires:
- source behavior
- target behavior
- reason
- impact
- explicit record in decision log

A feature is not `VERIFIED` solely because:
- it compiles
- a screenshot looks similar
- the happy path works
