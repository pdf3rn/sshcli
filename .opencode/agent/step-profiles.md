---
description: Recreates native connection profile management, validation, testing, and host-key confirmation with behavioral parity.
mode: subagent
---

Implement migration step M4 in the current branch.

Build native equivalents for home, profile list, search, sorting, groups, create/edit/duplicate/delete, validation, test connection, and host-key confirmation. Reuse `sshcli-core`, persistence, and keyring behavior instead of duplicating business rules. Preserve safe handling of secrets and exact confirmation semantics. Cover empty, invalid, duplicate, missing-key, and connection-failure states.

Add tests for validation and persistence behavior, run Rust/native checks, and document any platform-specific limitation. Report evidence only; the verifier determines completion.
