---
name: merge-quiz
description: MUST use before the first merge-adjacent action, meaning creating or readying a PR for merge, running land-pr, ship-branch, or submit-branch, or pushing to main, after any session that includes a non-trivial logic change such as a new feature, a bugfix touching business logic, or a multi-file refactor. Not required for docs-only, config-only, or single-line changes. Generates an HTML report and quiz on what changed, and the user must pass it before the merge step proceeds.
---

# Merge Quiz

## The Iron Law

NO MERGE-ADJACENT ACTION ON A NON-TRIVIAL CHANGE WITHOUT A PASSED QUIZ.

Reading the diff gives a light understanding of what happened — much of the actual behavior depends on existing code paths the diff doesn't show. This closes that gap by testing understanding directly, not by trusting that skimming the diff was enough.

## When this applies

Any session whose diff includes a new feature, a bugfix touching business logic, or a multi-file change with real logic — not just formatting, renames, or config.

## When this does NOT apply

Docs-only, config-only, dependency-bump-only, or single-line changes. If genuinely unsure whether a change counts as non-trivial, default to requiring the quiz — a false positive costs five minutes, a false negative risks an unreviewed merge.

## Procedure

1. Before the first merge-adjacent action, stop.
2. Build an HTML artifact (see `artifact-design` skill) with two parts:
   - **Report**: plain-English account of what changed and why, the shape of the solution, and any deviations from plan (pull from `deviation-log.md` if one exists).
   - **Quiz**: 3-6 questions on the actual behavior of the change — not "which file did you touch" trivia, but "what happens if X", "why Y over Z", "what edge case does this NOT handle". Multiple choice or short answer.
3. Have the user answer. Grade honestly — don't soften a wrong answer into a pass.
4. Passed: proceed to the merge-adjacent action.
5. Not passed: explain the missed point using the actual code, not a re-statement of the report, then re-quiz only on the missed items.

## Relationship to verification-before-completion

That skill gates on evidence the code works — tests run, commands executed. This one gates on evidence the human understands what shipped. Both apply; neither substitutes for the other.

## Related

- `deviation-log` — feeds the report.
- `superpowers:verification-before-completion` — sibling gate on code evidence rather than human comprehension.
- This project's `land-pr` / `ship-branch` / `submit-branch` skills — the quiz runs before invoking them, it doesn't replace their merge mechanics.
