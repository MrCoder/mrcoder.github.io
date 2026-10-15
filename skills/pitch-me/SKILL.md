---
name: pitch-me
description: Create a concise, self-contained visual explainer for a proposal, design, plan, implementation, or existing system. Use when the user says pitch me, pitch this, or asks to package work as a visual explainer or review page.
---

# Pitch Me

Make the subject understandable to the user, team, or stakeholder without reading the preceding conversation. Approval or buy-in may be requested, but it is optional.

## Procedure

1. Gather whatever exists: proposal, design, plan, implementation, screenshots, prototype, notes, or decision history. Missing artifacts are fine; prototype and deviation logs are optional. Reuse established visual design and components without reopening settled design choices.
2. Lead with the clearest visual:
   - built behavior: demo or screenshots;
   - proposal or design: mockup or diagram;
   - timing or rollout: timeline;
   - explanation: the visual that clarifies the relationship.

   Embed relevant screenshots and assets in the HTML so it remains self-contained. Label visuals and claims `Proposed`, `Verified`, or another accurate status.
3. Add only useful context: problem or question, audience, current state, key idea or behavior, evidence, tradeoffs, risks, and next step when relevant. Keep prose concise. Include a decision or request section only when a real decision is needed.
4. Create one self-contained HTML page by default. If Claude's `artifact-design` guidance is available, use it; otherwise use portable HTML with inline CSS and embedded assets. Use another format or a full site only when explicitly requested. If plain text is requested, write directly in that format.
5. Preview or open the HTML in an available browser or artifact viewer. Verify that the lead visual, embedded screenshots, status labels, and layout render correctly.

Send or post only with explicit user authorization. Authorization already given in the conversation counts; do not ask again. Without authorization, return the artifact for review.

## When not to apply

- Routine commit messages or PR descriptions unless the user explicitly asks for a pitch or explainer.
