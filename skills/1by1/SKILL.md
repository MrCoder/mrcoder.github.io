---
name: 1by1
description: Walk through a batch of items sequentially — one item fully handled before the next starts. Use when the user says "one by one", "1 by 1", "go through these one at a time", "walk me through each", "handle these sequentially", or invokes /1by1 after a listing step (worktrees, open PRs, TODOs, files, tickets, findings).
---

# 1by1

Sequential processing of a batch: investigate → verify it's still worth doing → propose → obtain approval → act or flag → report → next. No batching, no parallel fan-out — the user asked for items one at a time because they want to follow along and redirect mid-stream.

## Preconditions

Needs an existing list with enough identity per item (name/id/path) to act on. If no list exists yet, produce one first (or run whatever listing step the domain implies — `gh pr list`, `git worktree list`, a TODO grep) before starting the sequence.

## Mandatory per-item approval

Treat each listed item as separately approval-gated. Read-only investigation and re-verification may proceed, but before any state-changing action — including a normally mechanical, low-risk action such as testing, pushing, renaming, editing, moving, deleting, committing, merging, or closing — state the exact proposed action and wait for the user's explicit approval for that named item. Do not infer approval for later items from approval of an earlier item, or from a general “continue.”

For every item, state a clear, numbered recommendation before asking for approval. Make option **1. Recommended** the preferred path; give the exact action and the evidence-based reason. Number any viable alternative or skip path. Do not make the user infer the recommendation from a neutral finding or an unnumbered question.

**Never make the user remember what an item is.** Every time you name an item — a
position ("item 3", "第 3 条"), a branch, a worktree, a GitHub issue or PR number, a
ticket, a file path — restate in a short clause what it is or does. Do this on every
mention, not only the first. Write `PR #542 — classify Forge macros by custom content
ID` or `branch fix/kvs-timeout — retry the KVS read after a 5s timeout`, never a bare
`#542` or `item 3`. The user is judging one item at a time and is not holding the
earlier listing in memory.

If the user declines or gives a different instruction, record the item as skipped or follow that instruction, then move to the next item only after reporting the result.

## Per-item loop

For each item, in order:

1. **Re-verify current state.** Don't trust a summary written earlier in the conversation — re-check (`git status`, `gh pr view`, re-read the file). State drifts between the listing step and the item's turn.
2. **Check it's still valuable, not just mechanically doable.** Before investing effort: has something else already superseded this item? Does main/upstream already contain its useful part under a different name? Does its own history admit abandonment (a later commit saying "supersedes X")? A clean merge is not evidence of continued relevance — check separately.
   **Before recommending deletion, inspect the actual content first.** Do not infer that a file is disposable from its name, size, ignored status, or a structural/key-only scan. Read the text, or use the appropriate safe viewer/parser for binary or sensitive material; identify whether it is source, evidence, a transcript, a cache, or a duplicate of a durable artifact. State that content-based finding in the numbered recommendation. Redact secrets and customer data from the report, but do not skip the inspection because they may be present.
   **Uncommitted work you did not create is not yours to resolve.** If an item carries
   uncommitted changes from another session or agent, do not delete them, do not commit
   them, and do not push them on their author's behalf — that tree may be in active use
   right now. Report the modified paths and the last-modified time, and recommend leaving
   the item alone. Recency is the signal: recent uncommitted work is live work.
3. **Make a numbered recommendation, then wait for approval before acting.** Name the item and present **1. Recommended** with the exact state-changing action and its evidence-based rationale; number any viable alternative or skip path. Then wait for explicit approval for that item. Do this even when the action is mechanical and low-risk. **For a PR item, "done" means merged or closed on GitHub — pushing a rebased branch with green tests is not the finish line.** After an approved push, check CI (`gh pr checks`); once green, make a new numbered recommendation for the next required state-changing action for that same PR rather than silently treating it as complete. Green CI proves the code is safe to land; it says nothing about whether the PR is still worth landing. Never batch-approve a queue: every merge or close needs its own approval.
4. **Stop and ask for a decision when judgment is required** — in addition to the mandatory approval gate:
   - a conflict lands in code the item's own description calls high-risk, or in a subsystem multiple people/sessions touched independently
   - the item's dependency chain turns out to be broken or superseded
   - the action is destructive/irreversible/externally visible beyond what's already authorized
   Present the finding with evidence, a numbered set of options, and a recommendation — then wait. Do not fabricate what happens next.
5. **Report tersely and move on.** One item's report should be a few lines: what changed, why, verification evidence (test count, CI link), and its numbered recommendation if an action remains. Do not summarize the whole remaining batch after every item — that's the four-questions report shape, reserved for the end of the run or a genuine stop.
6. **Move to the next item after reporting, but do not act on it yet.** Re-verify it, then present its separately approval-gated proposal. Only pause the whole sequence at a genuine blocker (step 4) or when the user interrupts.

## Ranking before diving in

When investigation is cheap (a dry-run, a `--dry-run` flag, a lint pass), rank the batch by risk/effort before processing and clear the low-risk items first — surface the ranking, don't silently reorder without saying so. High-effort items that need real judgment (large conflicts, cross-cutting refactors) are worth calling out early rather than discovering them mid-sequence, so the user can redirect ("skip the hard ones for now") before time is sunk.

## Mid-stream interruption

If the user asks a question or gives a new instruction mid-item, answer/act on it directly, then resume the sequence from where it paused — don't restart from item 1, don't ask "should I continue?". Plain "continue" means resume the loop.

After a restart or a context loss — a crashed session, a compacted conversation, a new
turn that begins mid-batch — do not assume the last reported state still holds. Name the
current item, re-establish what is already done for it with fresh evidence (`git status`,
`gh pr view`), and continue only then. A summary written before the interruption is a
claim about the past, not the current state.

The batch itself can change mid-run. If the user removes an item, adds one, or narrows
the scope, stop work on any removed item at once, report what was already done to it, and
restate the remaining list before the next action.

## End of run

Close with a compact table or list: done (with evidence), flagged/skipped (with the specific reason), still pending. Don't pad this into a full four-questions report unless the batch closes out a task the user will hand to someone else.

## Example (from a live session)

Listing step: `/worktree-cleanup` found 12 open-PR worktrees. User: "one by one." For each: `git merge origin/main` (re-verify staleness), resolve any conflict by reading both sides' intent (not blind ours/theirs), reinstall deps, run the full unit suite, typecheck-diff against main, push. Two items turned out superseded by already-merged work — closed on GitHub with a comment naming the superseding PR, worktrees removed, instead of merged. One item hit a 7-file conflict in a subsystem two other sessions had independently reworked — flagged with the evidence and paused for a decision rather than resolved blind.
