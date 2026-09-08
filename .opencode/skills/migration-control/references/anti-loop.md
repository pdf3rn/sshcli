# Anti-loop protocol

For a single failure:

Attempt 1:
- diagnose from current evidence
- apply one focused fix
- rerun the narrowest meaningful check

Attempt 2:
- must be materially different
- inspect deeper source/diagnostics
- update hypothesis

Attempt 3:
- must be evidence-driven and materially different
- run focused verification

If still failing:
- stop changing that issue
- record blocker
- preserve diagnostic output
- state likely cause and next experiment
- continue only independent tasks

Never:
- alternate A/B endlessly
- rerun unchanged failing commands repeatedly
- broaden refactors just because a focused fix failed
- disable failing tests to gain green status
- pretend a warning/error is irrelevant without evidence
