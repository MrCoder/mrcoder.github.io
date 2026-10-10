---
name: 1by1
description: Walk through a batch of items sequentially — one item fully handled before the next starts. Use when the user says "one by one", "1 by 1", "go through these one at a time", "walk me through each", "handle these sequentially", or invokes /1by1 after a listing step (worktrees, open PRs, TODOs, files, tickets, findings).
---

# 1by1

## Intent

`1by1` reduces cognitive load by keeping one substantive user judgment or focus task in view at a time while the AI owns the working state: the remaining list, current position, decisions, authorisation state, and unresolved work. Present one substantive user judgment at a time and await the user's response before resolving that judgment or moving to the next user decision. This applies to decisions and discussion; it does not impose confirmation before every step. Once execution is authorised, carry out the current task's tools and internal steps without repeated confirmation, then move to the next authorised task after completion. After an interruption, restore the current position and context; never ask the user to remember the order or restate prior decisions.

Examples:

- Reviewing a table recommendation: present one recommendation and wait for that judgment before resolving it or moving on.
- An approved code change: edit and test without asking before each command, then continue to the next authorised task.

One item at a time: re-verify → check it is still worth doing → recommend one action → apply existing authorisation → act or flag → report → next. No batching or parallel fan-out. Use one recommendation by default; the unfinished-item checkpoint below is the exception.

## Preconditions

Needs a list with enough identity per item (name, id, path) to act on. If none exists, produce it first — `gh pr list`, `git worktree list`, a TODO grep — before starting the sequence.

## The gate

Respect the user's actual authorisation while keeping the review one item at a time. Read-only investigation and already authorised reversible edits, tests and record updates proceed without another approval. Explicit authorisation for a recipient or frozen batch persists across items and skill invocations; a general request to review does not authorise sending messages or unrelated destructive actions.

If the next action is outside that scope, prepare a concrete reviewable result and ask only for the necessary additional permission. Approval of one customer does not extend to others merely because their roles match. Do not turn a continuous authorised workflow into repeated confirmation questions.

**One action per item by default.** State the single action you judge correct, with its evidence-based reason. Ask for approval only when the action needs additional permission. `1by1` already carries the user's decision to work the list in order, so do not re-offer the order or a broad menu. When the current item is unfinished and control must return to the user, use the unfinished-item checkpoint below.

List alternatives only when the item has a second path that leads to a materially different end state — merge versus close, fix forward versus revert — and you cannot pick between them from the evidence. The unfinished-item checkpoint is the required exception: it always offers concrete continue-current and skip-current-next choices. A different sequencing, a smaller version of the same action, or "skip it" outside that checkpoint is not such a path; pick one and name it.

Never leave the recommendation to be inferred from a neutral finding or an unnumbered question.

**Say what the item is, every time.** A position ("item 3", "第 3 条"), branch, worktree, issue or PR number, ticket, or file path gets a short clause naming what it is or does, on every mention — `PR #542 — classify Forge macros by custom content ID`, `branch fix/kvs-timeout — retry the KVS read after a 5s timeout` — never a bare `#542` or `item 3`.

If the user declines or gives a different instruction, record the item as skipped or follow that instruction, report the result, then move on.

## Unfinished-item checkpoint

Whenever returning control to the user with the current item unfinished, show these concrete numbered choices:

1. **Continue `<current item>`** — `<remaining work to finish it>`.
2. **Skip `<current item>` and move to `<next item>`** — preserve `<remaining work>` as pending.

Name the current and next items with their identity (name, id, path, branch, issue or PR number), state the remaining work, and recommend one choice from the evidence. Preserve all work. Record a skipped item as incomplete and never report it as done. Do not treat silence as a skip; wait for an explicit choice.

Use this checkpoint only when control must return or a decision/permission is needed; do not interrupt ongoing authorised work with a needless question. When an item is complete, proceed automatically to the next item within existing authorisation. Discussion or agreement about an item does not authorise new writes.

## Per-item loop

1. **Re-verify state.** Re-check with a command (`git status`, `gh pr view`, re-read the file). A summary written earlier in the conversation is not current state.
2. **Check it is still worth doing.** Has other work superseded it? Does upstream already contain its useful part under another name? Does its own history admit abandonment (a later commit saying "supersedes X")? A clean merge is not evidence of continued relevance.
3. **Inspect before recommending deletion.** Never infer that a file is disposable from its name, size, or ignored status. Read it, or use a safe viewer or parser for binary or sensitive material, and classify it: source, evidence, transcript, cache, or duplicate of a durable artifact. State that content-based finding in the recommendation. Redact secrets and customer data from the report; do not skip the inspection because they may be present.
4. **Leave another session's uncommitted work alone.** Do not delete, commit, or push uncommitted changes you did not create — that tree may be in active use right now. Report the modified paths and their last-modified time, and recommend leaving the item alone.
5. **Recommend and apply existing authorisation.** Apply *The gate* to this item; ask only when additional permission is necessary. An item is done at its terminal state, not at a green intermediate one: a PR is done when merged or closed, not when CI passes on a pushed branch. After an approved push, check CI, then recommend the next action on that same item. If the item completes, proceed automatically to the next item within existing authorisation; if it remains unfinished and control returns, use the unfinished-item checkpoint.
6. **Stop for a decision** when a conflict lands in code the item's own description calls high-risk, or in a subsystem several sessions reworked independently; when the dependency chain turns out broken or superseded; or when the action is destructive, irreversible, or externally visible beyond what is authorized. Present the evidence and, if the current item is unfinished, use the unfinished-item checkpoint with the remaining work and one recommended choice; then wait for an explicit choice. Ask for approval only when needed. Number alternatives here only under the materially-different-end-state test in *The gate*, apart from the required unfinished-item checkpoint. Do not fabricate what happens next.
7. **Report in a few lines, then move on.** What changed, why, verification evidence (test count, CI link), and the next recommendation if an action remains. A skipped item remains incomplete and pending; say so explicitly. Do not re-summarize the whole batch after every item.

Resolve any merge conflict by reading both sides' intent — never a blind `ours` or `theirs`.

## Order and interruption

Rank before starting when investigation is cheap (a `--dry-run` flag, a lint pass): clear the low-risk items first, and state the ranking instead of reordering silently. Call out high-effort items — large conflicts, cross-cutting refactors — early, so the user can redirect before time is spent.

Answer a mid-item question or new instruction directly, then resume the sequence where it paused. If the current item remains unfinished and control returns, use the unfinished-item checkpoint and wait for the user's explicit choice; silence never means skip. No restart from item 1 and no needless "should I continue?" question. Plain "continue" resumes the current item. Do not interrupt ongoing authorised work merely to offer the checkpoint. Discussion or agreement still does not authorise new writes.

After a restart or a context loss — a crashed session, a compacted conversation, a turn that begins mid-batch — re-establish the current item's state with fresh evidence before continuing.

If the user removes an item, adds one, or narrows the scope, stop work on any removed item at once, report what was already done to it, and restate the remaining list.

## End of run

Close with a compact list: done (with evidence), flagged or skipped (with the specific reason), still pending. Do not expand it into a full report unless the batch closes out work another person will pick up.
