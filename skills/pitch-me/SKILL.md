---
name: pitch-me
description: Package a prototype, spec, and deviation log into a single explainer document to get stakeholder buy-in or review approval, leading with a demo if one exists. Use when the user says pitch me, pitch this, buy-in doc, explainer for the team, write this up for review, or package this for Slack.
---

# Pitch Me

Getting approval is a distinct deliverable from the code itself. A reviewer starts with the same unknowns you did — the doc's job is to accelerate them past the ones you already resolved.

## When to apply

- The user wants sign-off, review, or approval from someone who hasn't been following the work.
- Not for routine PR descriptions — this is a persuasive artifact for people catching up cold, not a technical summary for someone already reviewing the diff.

## Procedure

1. Gather what exists: any prototype or artifact from brainstorming, the written plan if there is one, and `deviation-log.md` if one exists.
2. Lead with the demo — a GIF, screenshot, or short recording of the thing working — before any prose. If nothing visual exists and the change has a UI, consider producing one now (see `prototype`) rather than skipping straight to text.
3. Structure the rest to answer the questions a reviewer already has, in the order they'd ask them:
   - What problem this solves, for whom
   - What was actually built (the what, not the how)
   - What tradeoffs were made and why (draw on Deviations)
   - What was considered and rejected, if anything (draw on brainstorming output)
   - What's out of scope or could still go wrong (draw on `blind-spot-pass` findings if that ran)
4. Build it as an HTML artifact (see `artifact-design`) unless the destination is a plain-text Slack message, in which case draft the message directly.
5. This produces a draft only. Posting it anywhere — Slack, email, a PR comment — needs the user's explicit go-ahead. Never auto-send.

## When NOT to apply

- Internal-only technical changes with no external reviewer or stakeholder.
- The user just wants a commit message or PR description — that's plain project convention, not this skill.

## Related

- `deviation-log` — source material for tradeoffs.
- `blind-spot-pass` — source material for what could still go wrong.
- `prototype` — source material for the demo.
