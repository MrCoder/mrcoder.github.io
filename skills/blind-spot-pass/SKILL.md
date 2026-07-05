---
name: blind-spot-pass
description: Surface unknown-unknowns before starting work in unfamiliar territory, such as a new part of the codebase, an unfamiliar domain, or a task type not done before in this project. Use when the user says blind spot pass, unknown unknowns, what am I missing here, I don't know this area, or otherwise flags unfamiliarity before diving in.
---

# Blind Spot Pass

You know more about the average topic than the user does, and you can search the codebase and its history far faster. Use that asymmetry to surface what they don't know to ask, before they start work rather than after it breaks.

Distinct from `zoom-out`: zoom-out is a one-shot request for a module/caller map. This is broader — it also surfaces historical gotchas, conventions, and the questions the user doesn't know they should be asking, calibrated to their stated experience.

## When to apply

- User is about to work in a codebase area, domain, or task type they've flagged as unfamiliar.
- Skip it when they're already demonstrably fluent in the area (recent related work, detailed prompting) or the task is trivial enough that unknown-unknowns can't matter much (single-line fix, well-trodden path).

## Procedure

1. **Calibrate**: get the user's starting point — what they already know, their experience with this area or domain. If they haven't said, ask one direct question before doing the audit. The value of this pass depends on calibration: don't explain what they already know, don't skip what they don't.
2. **Search for landmines**: `git log`/`blame` on the relevant files for past incidents (commit messages like fix, revert, regression, hotfix), existing docs/ADRs/CONTEXT.md touching the area, related closed issues or PRs, code comments flagging non-obvious constraints.
3. **Search for conventions**: how does existing code already solve similar problems here? What patterns are expected, what's been tried and abandoned?
4. **Report as a categorized list**, not prose:
   - Historical gotchas — what already broke here before
   - Conventions you're expected to follow
   - Likely edge cases or failure modes specific to this area
   - Questions worth asking that the user might not know to ask
5. This is informational, not a plan. End by offering to feed the findings into a brainstorming session or plan — don't design the solution yourself here.

## Anti-patterns

- Generic best-practice advice unconnected to this specific codebase — the value is in codebase-specific landmines, not textbook caution.
- Skipping step 1 and dumping everything you know — an uncalibrated dump buries the two things that actually matter under ten things the user already knows.

## Related

- `zoom-out` — narrower, structural-only (module/caller map), no calibration or history search.
- `superpowers:brainstorming` — natural next step once blind spots are surfaced.
