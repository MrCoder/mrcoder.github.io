---
name: 1by1
description: Walk through a batch of items sequentially — one decision in view at a time; an item waiting on CI, a merge or a background agent goes on a return list while the next item proceeds. Use when the user says "one by one", "1 by 1", "go through these one at a time", "walk me through each", "handle these sequentially", or invokes /1by1 after a listing step (worktrees, open PRs, TODOs, files, tickets, findings).
---

# 1by1

## Intent

`1by1` reduces cognitive load by keeping one substantive user judgment or focus task in view at a time while the AI owns the working state: the remaining list, current position, decisions, authorisation state, and unresolved work. Present one judgment at a time and await the user's response before resolving it or moving to the next user decision. This applies to decisions and discussion, not to every step: once execution is authorised, carry out the current task's tools and internal steps without repeated confirmation, then move to the next authorised task after completion. Work one item at a time through the *Per-item loop*, with no batching of decisions or actions.

Examples:

- Reviewing a table recommendation: present one recommendation and wait for that judgment before resolving it or moving on.
- An approved code change: edit and test without asking before each command, then continue to the next authorised task.

## Preconditions

Needs a list with enough identity per item (name, id, path) to act on. If none exists, produce it first — `gh pr list`, `git worktree list`, a TODO grep — before starting the sequence.

## The gate

Respect the user's actual authorisation. Read-only investigation and already authorised reversible edits, tests and record updates proceed without another approval. Explicit authorisation for a recipient or frozen batch persists across items and skill invocations; a general request to review does not authorise sending messages or unrelated destructive actions. Discussion or agreement about an item does not authorise new writes. If the next action is outside that scope, prepare a concrete reviewable result and ask only for the necessary additional permission. Approval of one customer does not extend to others merely because their roles match.

**One action per item by default.** State the single action you judge correct, with its evidence-based reason, and never leave the recommendation to be inferred from a neutral finding or an unnumbered question. `1by1` already carries the user's decision to work the list in order, so do not re-offer the order or a broad menu. The *Unfinished-item checkpoint* is the required exception: it always offers continue-current and skip-current-next choices.

**One question, one action.** A numbered option carries exactly one action. An internal record that needs no permission (a triage status, a scratchpad note) is not a question: do it and report it. A side finding that needs its own action becomes a new item at the end of the list, presented later on its own.

List alternatives only when the item has a second path that leads to a materially different end state — merge versus close, fix forward versus revert — and you cannot pick between them from the evidence. A different sequencing, a smaller version of the same action, or "skip it" outside the *Unfinished-item checkpoint* is not such a path; pick one and name it.

**Say what the item is, every time.** A list position, branch, worktree, issue or PR number, ticket, or file path gets a parenthesised label naming what it is or does, on every mention: `PR #542 (classify Forge macros by custom content ID)` or `按 ID 归类宏（PR #542）`. A dash, colon or comma does not bind a label, and neither does a paren that starts with the number and continues after a comma. Never leave a number unlabelled.

If the user declines or gives a different instruction, record the item as skipped or follow that instruction, report the result, then move on.

## Unfinished-item checkpoint

Whenever returning control to the user with the current item unfinished, show these concrete numbered choices:

1. **Continue `<current item>`** — `<remaining work to finish it>`.
2. **Skip `<current item>` and move to `<next item>`** — preserve `<remaining work>` as pending.

Name the current and next items with their identity (name, id, path, branch, issue or PR number), state the remaining work, and recommend one choice from the evidence. Preserve all work. A skipped item stays incomplete and pending; say so explicitly and never report it as done. Do not treat silence as a skip; wait for an explicit choice.

Use this checkpoint only when control must return or a decision/permission is needed, never to interrupt ongoing authorised work.

## Prefetch the next items

While the user considers the current item, run background subagents that prepare each following item up to the point of the user's decision:

- Re-verify its state and check it is still worth doing.
- Draft a self-contained brief with one recommended action.
- Build that action locally (a branch in its own worktree with tests run, or a drafted comment or issue), then have an independent verifier check it.
- When the item is a proposal, a design, or a feature, also prebuild its pitch-me page and check that it renders, so presenting it is the link plus the short brief.

Nothing is pushed, posted, merged, deployed, or sent; every outward action stays one item at a time under *The gate*. When presenting an item, state its readiness: brief only, action built, or action verified. Prefetched state can go stale, so re-run the cheap state check (`gh pr view`, `git status`) right before presenting it. Skip the prefetch when only one or two items remain or when the next item depends on the current decision.

## Per-item loop

1. **Re-verify state.** Re-check with a command (`git status`, `gh pr view`, re-read the file). A summary written earlier in the conversation is not current state.
2. **Check it is still worth doing.** Has other work superseded it? Does upstream already contain its useful part under another name? Does its own history admit abandonment (a later commit saying "supersedes X")? A clean merge is not evidence of continued relevance.
3. **Inspect before recommending deletion.** Never infer that a file is disposable from its name, size, or ignored status. Read it, or use a safe viewer or parser for binary or sensitive material, and classify it: source, evidence, transcript, cache, or duplicate of a durable artifact. State that content-based finding in the recommendation. Redact secrets and customer data from the report; do not skip the inspection because they may be present.
4. **Leave another session's uncommitted work alone.** Do not delete, commit, or push uncommitted changes you did not create — that tree may be in active use right now. Report the modified paths and their last-modified time, and recommend leaving the item alone.
5. **Recommend and apply existing authorisation.** Apply *The gate* to this item. An item is done at its terminal state, not at a green intermediate one: a PR is done when merged or closed, not when CI passes on a pushed branch. After an approved push, check CI, then recommend the next action on that same item. While an approved action waits on something slow (CI, merge, deploy, a background agent), present the next item instead of idling: keep the waiting item on a return list, and when it finishes, report it and finish its remaining steps before the next decision. The run ends only when every waiting item has reached its terminal state.
6. **Stop for a decision** when a conflict lands in code the item's own description calls high-risk, or in a subsystem several sessions reworked independently; when the dependency chain turns out broken or superseded; or when the action is destructive, irreversible, or externally visible beyond what is authorized. Present the evidence; if the current item is unfinished, use the *Unfinished-item checkpoint*. Do not fabricate what happens next.
7. **Report in a few lines, then move on.** What changed, why, verification evidence (test count, CI link), and the next recommendation if an action remains. **Keep the current decision alone in view.** A message carries the current item and its one question. Items on the return list get no status lines until one changes state; then report that item in 1–2 lines. List the whole batch only at *End of run* or when the user asks.

Resolve any merge conflict by reading both sides' intent — never a blind `ours` or `theirs`.

## Order and interruption

Rank before starting when investigation is cheap (a `--dry-run` flag, a lint pass): clear the low-risk items first, and state the ranking instead of reordering silently. Call out high-effort items — large conflicts, cross-cutting refactors — early, so the user can redirect before time is spent.

Answer a mid-item question or new instruction directly, then resume where the sequence paused, never from item 1: restore the current position and context, and never ask the user to remember the order or restate prior decisions. If control returns with the item unfinished, use the *Unfinished-item checkpoint*. No needless "should I continue?" question. Plain "continue" resumes the current item.

After a restart or a context loss — a crashed session, a compacted conversation, a turn that begins mid-batch — re-establish the current item's state with fresh evidence before continuing.

If the user removes an item, adds one, or narrows the scope, stop work on any removed item at once, report what was already done to it, and restate the remaining list.

## End of run

Close with a compact list: done (with evidence), flagged or skipped (with the specific reason), still pending. Do not expand it into a full report unless the batch closes out work another person will pick up.
