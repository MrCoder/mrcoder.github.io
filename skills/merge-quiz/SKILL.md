---
name: merge-quiz
description: Unless the user explicitly asks to skip the quiz, MUST use before the first merge-adjacent action, meaning creating or readying a PR for merge, running land-pr, ship-branch, or submit-branch, or pushing to main, after any session that includes a non-trivial logic change such as a new feature, a bugfix touching business logic, or a multi-file refactor. Not required for docs-only, config-only, or single-line changes. Generates an HTML report and asks a one-at-a-time multiple-choice quiz about what changed; an imperfect result does not block unless every answer is wrong.
---

# Merge Quiz

## The Iron Law

NO MERGE-ADJACENT ACTION ON A NON-TRIVIAL CHANGE WITHOUT COMPLETING THE QUIZ, UNLESS THE USER EXPLICITLY ASKS TO SKIP IT.

Reading the diff gives a light understanding of what happened — much of the actual behavior depends on existing code paths the diff doesn't show. This closes that gap by testing understanding directly, not by trusting that skimming the diff was enough.

## When this applies

Any session whose diff includes a new feature, a bugfix touching business logic, or a multi-file change with real logic — not just formatting, renames, or config.

## When this does NOT apply

Docs-only, config-only, dependency-bump-only, or single-line changes. If genuinely unsure whether a change counts as non-trivial, default to requiring the quiz — a false positive costs five minutes, a false negative risks an unreviewed merge.

## Explicit skip

If the user explicitly asks to skip the quiz (for example, “skip the quiz” or “跳过测验”), skip it and continue the already-authorized workflow without asking for confirmation. This exception also applies to a quiz already in progress or a required re-quiz. Record it as skipped at the user’s request, not completed. Skipping the quiz does not skip technical verification or authorize additional actions.

## Procedure

1. Check for an explicit request to skip the quiz. If present, follow **Explicit skip** above; otherwise, stop before the first merge-adjacent action.
2. Build an HTML artifact (see `artifact-design` skill) with two parts:
   - **Report**: plain-English account of what changed and why, the shape of the solution, and any deviations from plan (pull from `deviation-log.md` if one exists).
   - **Quiz**: 3-6 actual-behavior questions — not file-name trivia. Every question must be single-choice, contain 3-4 options, and have exactly one correct answer. All options must have explicit numeric labels starting at 1 (1, 2, 3, and optionally 4); never use letters, unnumbered options, or open-ended questions. This applies to every quiz question, including conversational prompts, HTML artifacts, and focused re-quizzes. When using a question tool, include the numeric labels in its option text. Accept the option number as a complete answer.
3. Ask exactly one question at a time, always presenting its numbered options together with the question. Do not reveal later questions until the current one has been answered and graded.
4. Grade each answer immediately and honestly. For a wrong answer, say it is wrong and explain the missed point from the actual code or behavior before asking the next question.
5. After all questions:
   - If at least one answer is correct, record the quiz as completed and proceed. Wrong answers must not block merging or release.
   - If every answer is wrong, explain the key missed behavior and require a focused re-quiz before proceeding.

## Relationship to verification-before-completion

That skill gates on evidence the code works — tests run, commands executed. This one gates on evidence the human understands what shipped. Both apply unless the user explicitly skips the quiz; technical verification still applies, and neither substitutes for the other.

## Related

- `deviation-log` — feeds the report.
- `superpowers:verification-before-completion` — sibling gate on code evidence rather than human comprehension.
- This project's `land-pr` / `ship-branch` / `submit-branch` skills — the quiz runs before invoking them, it doesn't replace their merge mechanics.
