# Multi-Agent Development Protocol

Use this protocol for non-trivial implementation, refactoring, bug fixing, performance work, concurrency changes, or unsafe code.

The workflow separates reasoning, implementation, review, and repair so that the agent that writes a change is not the only agent judging it.

All agents must obey `AGENTS.md` and the relevant `.agent/*.md` files. Role files add responsibilities; they do not override shared engineering rules.

## Roles

The standard pipeline uses five roles:

1. **Orchestrator** — owns the workflow, task packet, handoffs, and completion gate.
2. **Planner** — investigates the repository and designs the smallest correct implementation before code is written.
3. **Implementer** — writes the code from the approved plan and verifies it locally.
4. **Reviewer** — independently reviews the implementation for correctness, architecture, performance, and maintainability. It does not approve its own code.
5. **Fixer** — fixes review findings with the smallest complete patch, then hands the result back for re-review.

Role instructions live in `.agent/roles/`.

## Shared Source of Truth

Every task must have one compact **Task Packet**. The orchestrator maintains it across handoffs.

Do not rely on chat history as the only source of constraints. Do not silently drop a constraint because another agent did not mention it.

Use this structure:

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
Costs that must not regress:
Measurement required before completion:

VERIFICATION
Focused checks:
Workspace checks:
Benchmarks/profiling:

OPEN QUESTIONS / RISKS
...
```

The Task Packet should be concise, but the `PERFORMANCE CONTRACT` must survive unchanged unless new repository evidence justifies an update.

## Optimization Preservation Rule

Performance information is first-class task state.

When any agent discovers an optimization idea, bottleneck, scale assumption, allocation issue, data-layout issue, I/O issue, synchronization cost, benchmark baseline, or relevant profiling result, it must add it to the Task Packet's `PERFORMANCE CONTRACT` or explicitly mark it rejected with a reason.

No downstream agent may silently discard a recorded optimization idea.

For each recorded idea, use one of these states:

- `candidate` — plausible but not measured
- `required` — part of the task or necessary to avoid a known regression
- `validated` — supported by measurement or clear complexity analysis
- `rejected` — tested or reasoned against; include why
- `deferred` — useful but outside the current task; include why

Prefer this optimization order from `AGENTS.md`:

1. algorithmic complexity
2. amount of work performed
3. data structures
4. data layout and cache locality
5. I/O
6. allocations and copying
7. synchronization
8. parallelism
9. branch behavior
10. SIMD or unsafe micro-optimization

Do not replace measured optimization work with stylistic refactoring.

## Workflow

### 1. Orchestrator creates the Task Packet

The orchestrator extracts the user's goal, constraints, acceptance criteria, and explicit performance requirements.

For a non-trivial task it delegates repository investigation to the Planner.

### 2. Planner investigates and designs

The Planner reads the owning code, callers, tests, and relevant shared rule files.

It returns a plan containing:

- current behavior and ownership
- proposed change by file/module
- invariants and error behavior
- algorithm and data structures
- ownership/lifetime strategy
- performance implications
- tests and benchmarks
- migration/removal of obsolete paths
- risks and assumptions

The Planner must search for an existing implementation before proposing a new one.

The Planner does not write production code unless the environment cannot separate roles.

### 3. Implementer writes the change

The Implementer receives the Task Packet and plan.

It writes the smallest complete change, updates tests, and runs focused verification while iterating.

It may deviate from the plan only when repository evidence makes the plan incorrect or materially worse. Any deviation must be recorded in the handoff with the reason and performance impact.

Before handoff, it inspects its own diff and removes temporary diagnostics, dead code, duplicated paths, and accidental unrelated changes.

### 4. Reviewer performs independent review

The Reviewer receives the Task Packet, plan, implementation handoff, and final diff.

It independently checks:

- correctness and edge cases
- invariant preservation
- architectural ownership and dependency direction
- API compatibility
- error handling
- unnecessary complexity or duplication
- allocations, copies, repeated work, I/O, locks, atomics, and data layout on hot paths
- concurrency safety and shutdown/cancellation where relevant
- unsafe invariants where relevant
- tests and benchmark adequacy
- whether every recorded optimization idea has a disposition

The Reviewer must inspect the actual diff and relevant surrounding code. It must not approve from the Implementer's summary alone.

Review findings use these severities:

- `BLOCKER` — correctness, soundness, data loss, security, deadlock, serious regression, broken acceptance criterion, or unverified unsafe invariant
- `MAJOR` — architectural error, likely bug, meaningful performance regression, missing regression test, or incomplete implementation
- `MINOR` — worthwhile maintainability or low-risk issue that does not block correctness
- `NOTE` — optional observation; never used to force churn

Each actionable finding should include location, problem, impact, and the smallest reasonable fix.

The Reviewer normally does not edit code.

### 5. Fixer repairs findings

The Fixer receives the exact review findings plus the Task Packet and diff.

It fixes all `BLOCKER` and `MAJOR` findings. It fixes `MINOR` findings when the change is local and justified; otherwise it records why the finding is deferred.

The Fixer must not use the review as permission for unrelated refactoring.

It reruns the checks affected by its changes and updates benchmark evidence when performance-sensitive code changed.

### 6. Reviewer re-reviews

After fixes, the Reviewer inspects the new diff and verifies that findings are resolved without regressions.

Repeat Reviewer → Fixer until there are no unresolved `BLOCKER` or `MAJOR` findings.

### 7. Completion gate

The Orchestrator may declare the task complete only when:

- acceptance criteria are satisfied
- no unresolved `BLOCKER` or `MAJOR` review findings remain
- applicable tests/checks actually ran successfully, or any inability to run them is stated explicitly
- performance-sensitive changes have the required measurements
- unsafe changes have an explicit safety review
- the diff contains no known unrelated changes
- every optimization idea in the Performance Contract is `validated`, `rejected`, `deferred`, or still explicitly marked `candidate`

Never claim a command, test, benchmark, or review passed unless it actually ran.

## Handoff Format

Agents should return compact structured handoffs rather than prose transcripts.

```text
ROLE: <planner|implementer|reviewer|fixer>
STATUS: <ready|blocked|changes-requested|approved>

SUMMARY
...

CHANGES / FINDINGS
...

PERFORMANCE CONTRACT UPDATES
- <idea>: <state> — <evidence/reason>

VERIFICATION
- <command>: <pass/fail/not-run> — <relevant result>

RISKS / OPEN ITEMS
...

NEXT ROLE
<role and exact work expected>
```

Do not include hidden chain-of-thought. Record conclusions, evidence, decisions, measurements, and unresolved questions only.

## Context Discipline

Give each role enough context to work correctly, not the entire conversation by default.

Always pass:

- Task Packet
- relevant plan or findings
- current diff or changed files for implementation/review roles
- applicable repository instructions
- exact verification results already obtained

Also pass relevant code and tests when the agent cannot inspect the repository directly.

Do not pass stale summaries instead of source code when direct inspection is possible.

## Parallelism

Parallel agents are useful for independent investigation, not conflicting edits.

Safe examples:

- one agent profiles CPU while another inspects allocation behavior
- one agent investigates architecture while another locates tests
- separate reviewers inspect correctness and performance, with the main Reviewer consolidating findings

Avoid parallel writes to the same files or overlapping implementation responsibilities unless the repository and merge process make conflicts explicit and manageable.

A single canonical implementation must remain the result.

## Small Tasks

For trivial changes, the Orchestrator may collapse Planner and Implementer, or Fixer into Implementer, but the independent Reviewer should remain for changes where correctness or performance matters.

Do not create ceremony that costs more than the change itself.
