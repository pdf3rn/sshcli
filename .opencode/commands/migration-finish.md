---
description: Run final parity and verification gates and determine whether the old Tauri/web implementation is safe to remove.
agent: migration-orchestrator
---

Perform final migration readiness review.

Required:
1. complete feature matrix
2. source-vs-target parity review
3. Rust build/tests/lint review
4. visual review of all major screens
5. terminal/PTY/SSH/SFTP/tunnel regression review
6. persistence/settings/workspace restoration review
7. keyboard/focus review
8. packaging/runtime review

Do not remove the Tauri/web implementation.

Instead produce a cleanup/removal plan and clearly state whether evidence supports removal. Actual deletion requires explicit user approval.
