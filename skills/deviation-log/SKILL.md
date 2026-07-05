---
name: deviation-log
description: Keep a running deviation-log.md during a non-trivial build, logging any deviation from the plan when an edge case forces a different approach mid-implementation. Use when executing a multi-step implementation plan, when the user says keep a deviation log, keep implementation notes, or track deviations, or whenever a discovered edge case would force a change from the stated design.
---

# Deviation Log

No matter how good the plan, unknown unknowns surface during implementation. The value isn't avoiding deviation — it's making deviation legible instead of silent, so the next session (or the next reviewer) can learn from it.

## When to apply

- Any non-trivial implementation: multi-file, a new feature, or a bugfix with real logic — with or without a formal written plan.
- Skip it for trivial single-file changes or tasks with nothing to deviate from.

## Procedure

1. At the start of the implementation, create `deviation-log.md` at the repo or worktree root. Mention it once; don't ask permission each time you write to it — it's a scratch file, fully reversible.
2. Update it live, as decisions happen — not reconstructed from memory at the end.
3. When an edge case forces a deviation from the plan or original approach:
   - Pick the conservative option: least surprising, smallest blast radius, easiest to revert.
   - Log it under `## Deviations`: what the plan/approach said, what forced the change, what you did instead, why it's safe to keep going.
   - Keep going — don't stop to ask unless the deviation changes something the user would care about strategically (a scope change, a user-facing tradeoff), not just an implementation detail.
4. The file is scratch, not a deliverable. Before merge:
   - Fold anything reviewers need into the PR description, then delete the file, or
   - Feed it into `pitch-me` if a stakeholder pitch is being built, or into `merge-quiz`'s report.
   Don't leave `deviation-log.md` sitting in the repo past the PR.

## Anti-patterns

- Writing it retroactively right before merge — the point is capturing the deviation reasoning while it's fresh, not reconstructing a clean narrative after the fact.
- Logging routine, expected work as a "deviation" — this file is for surprises, not a running commit log.

## Related

- `superpowers:writing-plans` / `superpowers:executing-plans` — the plan this deviates from, when one exists.
- `pitch-me` — consumes this for stakeholder pitches.
- `merge-quiz` — draws on this to explain what actually happened, versus what was planned.
