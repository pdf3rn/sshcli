---
name: behavior-parity
description: Compare source and target implementations with evidence-based parity criteria across happy paths, errors, state, keyboard interaction, persistence, and visual behavior.
compatibility: opencode
metadata:
  quality: parity
---

# Behavior parity

For each migrated feature define:
- source observable behavior
- target observable behavior
- test/reproduction steps
- evidence
- differences

Check more than the happy path:
- invalid inputs
- unavailable resources
- cancellation
- reconnect/retry
- empty state
- loading state
- disabled state
- focus/keyboard
- resize
- persistence
- restart restoration
- concurrent operations
- errors

Use source behavior as reference unless a recorded decision explicitly changes it.

Status vocabulary:
- VERIFIED
- UNVERIFIED
- REGRESSION
- BLOCKED
- INTENTIONAL_DIFFERENCE
