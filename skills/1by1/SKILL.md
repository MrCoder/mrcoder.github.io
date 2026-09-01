---
name: 1by1
description: Walk through a batch of items sequentially — one item fully handled before the next starts. Use when the user says "one by one", "1 by 1", "go through these one at a time", "walk me through each", "handle these sequentially", or invokes /1by1 after a listing step (worktrees, open PRs, TODOs, files, tickets, findings).
---

# 1by1

One item at a time: re-verify → check it is still worth doing → recommend one action → wait for approval → act or flag → report → next. No batching, no parallel fan-out, no option menus.

## Preconditions

Needs a list with enough identity per item (name, id, path) to act on. If none exists, produce it first — `gh pr list`, `git worktree list`, a TODO grep — before starting the sequence.

## The gate

Every item is separately approval-gated.

**Wait for approval, per item.** Read-only investigation and re-verification may proceed. Before any state-changing action — including a mechanical, low-risk one such as testing, pushing, renaming, editing, moving, deleting, committing, merging, or closing — state the exact action and wait for explicit approval of that named item. Approval of an earlier item, or a general "continue", is not approval of this one.

**One action per item. No menu.** State the single action you judge correct, with its evidence-based reason, and ask for approval of that one action. `1by1` already carries the user's decision to work the list in order, so re-offering the order, the skip path, or a "do nothing" branch as choices returns a decision the user has already made.

List alternatives only when the item has a second path that leads to a materially different end state — merge versus close, fix forward versus revert — and you cannot pick between them from the evidence. Then number them, put the recommended one first, and say why the evidence does not separate them. A different sequencing, a smaller version of the same action, or "skip it" is not such a path; pick one and name it.

Never leave the recommendation to be inferred from a neutral finding or an unnumbered question.

**Say what the item is, every time.** A position ("item 3", "第 3 条"), branch, worktree, issue or PR number, ticket, or file path gets a short clause naming what it is or does, on every mention — `PR #542 — classify Forge macros by custom content ID`, `branch fix/kvs-timeout — retry the KVS read after a 5s timeout` — never a bare `#542` or `item 3`.

If the user declines or gives a different instruction, record the item as skipped or follow that instruction, report the result, then move on.

## Per-item loop

1. **Re-verify state.** Re-check with a command (`git status`, `gh pr view`, re-read the file). A summary written earlier in the conversation is not current state.
2. **Check it is still worth doing.** Has other work superseded it? Does upstream already contain its useful part under another name? Does its own history admit abandonment (a later commit saying "supersedes X")? A clean merge is not evidence of continued relevance.
3. **Inspect before recommending deletion.** Never infer that a file is disposable from its name, size, or ignored status. Read it, or use a safe viewer or parser for binary or sensitive material, and classify it: source, evidence, transcript, cache, or duplicate of a durable artifact. State that content-based finding in the recommendation. Redact secrets and customer data from the report; do not skip the inspection because they may be present.
4. **Leave another session's uncommitted work alone.** Do not delete, commit, or push uncommitted changes you did not create — that tree may be in active use right now. Report the modified paths and their last-modified time, and recommend leaving the item alone.
5. **Propose, wait, then act.** Apply *The gate* to this item. An item is done at its terminal state, not at a green intermediate one: a PR is done when merged or closed, not when CI passes on a pushed branch. After an approved push, check CI, then recommend the next action on that same item.
6. **Stop for a decision** when a conflict lands in code the item's own description calls high-risk, or in a subsystem several sessions reworked independently; when the dependency chain turns out broken or superseded; or when the action is destructive, irreversible, or externally visible beyond what is authorized. Present the evidence and one recommended action, then wait. Number alternatives here only under the materially-different-end-state test in *The gate*. Do not fabricate what happens next.
7. **Report in a few lines, then move on.** What changed, why, verification evidence (test count, CI link), and the next recommendation if an action remains. Do not re-summarize the whole batch after every item.

Resolve any merge conflict by reading both sides' intent — never a blind `ours` or `theirs`.

## Order and interruption

Rank before starting when investigation is cheap (a `--dry-run` flag, a lint pass): clear the low-risk items first, and state the ranking instead of reordering silently. Call out high-effort items — large conflicts, cross-cutting refactors — early, so the user can redirect before time is spent.

Answer a mid-item question or new instruction directly, then resume the sequence where it paused. No restart from item 1, no "should I continue?". Plain "continue" resumes the loop.

After a restart or a context loss — a crashed session, a compacted conversation, a turn that begins mid-batch — re-establish the current item's state with fresh evidence before continuing.

If the user removes an item, adds one, or narrows the scope, stop work on any removed item at once, report what was already done to it, and restate the remaining list.

## End of run

Close with a compact list: done (with evidence), flagged or skipped (with the specific reason), still pending. Do not expand it into a full report unless the batch closes out work another person will pick up.
