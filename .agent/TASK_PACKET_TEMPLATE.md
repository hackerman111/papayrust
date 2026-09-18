# Task Packet Template

Copy this block into the orchestrator's task state for each non-trivial change.

```text
TASK
User goal:
Acceptance criteria:
Non-goals:
Repository constraints:

SCOPE
Owning module/crate:
Relevant callers/tests:
Files likely affected:

INVARIANTS
Behavioral invariants:
Data/ownership invariants:
Compatibility constraints:

PERFORMANCE CONTRACT
Performance-sensitive: yes/no/unknown
Hot paths:
Expected input scale:
Baseline measurements:
Optimization ideas/hypotheses:
- [candidate|required|validated|rejected|deferred] idea — evidence/reason
Costs that must not regress:
Measurement required before completion:

VERIFICATION
Focused checks:
Workspace checks:
Benchmarks/profiling:

OPEN QUESTIONS / RISKS
...
```

Rules:

- Keep one Task Packet as the canonical source of task constraints.
- Preserve the Performance Contract across every handoff.
- Add new optimization discoveries instead of replacing earlier ones.
- Change an optimization state only with evidence or an explicit reason.
- Do not claim tests, checks, profiles, or benchmarks ran unless they actually ran.
