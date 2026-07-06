---
name: spot-check
description: Ad hoc, AI-driven verification of one specific behavior in a real environment — not a checked-in test. Use when the user says "spot check", "run a spot check on X", "spot check this fix", "spot check on staging", "verify on staging", "verify on prod", or wants to confirm a behavior actually works in the real world rather than write a new automated test.
---

# Spot Check

A **spot check** is an ad hoc, AI-driven, ephemeral verification of **one specific behavior** in a **real environment**. It exists for the gap between "the tests pass" and "it actually works" — the map is not the territory, so go look at the territory.

**It is NOT:** a checked-in `.spec` file, a regression suite, a repeatable CI test, or a unit test against a local fixture. If a green check would prove nothing, it is not a spot check.

## Principles

- **Lightweight** — the smallest set of checks that covers the change. Reuse what already exists; navigate straight to what matters.
- **AI-driven** — improvise the steps. Plan the assertions first, then drive a browser or CLI to observe them. No script is committed.
- **Ephemeral** — the steps live in the chat/report, not a new test file. If a check is worth keeping, *promote* it to the real test suite afterward — that is a separate, deliberate act.
- **Targeted** — verify the behavior under review, not a full regression.
- **Real world** — verify on a deployed/running environment, not a local fixture or unit test, whenever the behavior is environment-sensitive.

## Write the plan first

**STOP.** Do not open the browser or run any query until the plan is written.

Each planned check must name:

1. **Behavior** — what changed, or what you are verifying.
2. **Observable signal** — a specific UI element, an HTTP response, an analytics event + property, a log line, a stored row.
3. **Method** — the concrete step: a browser action, `curl`, a log tail, a read-only query.

Each item must be **independently pass/fail** *before* you run it. If you can't say in advance what "pass" looks like, the check isn't ready.

```text
Spot check plan: <short title>

Target: <env / URL / API path>
  - [ ] <specific observable assertion>   [method]
  - [ ] <specific observable assertion>   [method]

Skipped: <out of scope> — <reason>
```

## Choosing the environment

| Situation                                  | Target                                                     |
|--------------------------------------------|------------------------------------------------------------|
| New feature not yet deployed               | Local dev server / branch preview / tunnel                 |
| Deployed to staging, or a failing pipeline | Staging environment                                        |
| Reproducing or verifying a production bug  | Production directly — **read-only, non-destructive**       |
| Post-release validation                    | The exact environment the release deployed to              |
| Backend-only (API, webhook, job, DB)       | `curl` / CLI against staging or prod                       |

Anchor a **post-release** check on the *release delta* — the commits since the last published tag. Verify the most significant user-visible change with one concrete pass/fail assertion. If the delta is docs/infra-only, say so: "deployed and up; no user-facing change to verify."

## Verification methods

Mix freely — drive the browser, then query the DB, then check analytics.

| Signal                | How                                                                                      |
|-----------------------|------------------------------------------------------------------------------------------|
| UI behavior           | Browser automation (Playwright, claude-in-chrome, …). Scope selectors to the right frame. |
| HTTP / route status   | `curl -sSI <url>` — mind CDN challenges (see gotchas); redirects via `-D - --max-redirs 0` |
| Analytics events      | Your analytics query tool with the correct project **and** a domain/URL property filter    |
| App / server logs     | The platform's log tail (`wrangler tail`, `forge logs`, `vercel logs`, `az webapp log tail`) |
| Database / stored state | A **read-only** query against the right environment — never write to a production DB      |

## Workflow

1. **Plan** — behavior, target, and the expected signal per assertion (above).
2. **Confirm the build** — check that the environment is actually running the code you mean to test (version label, deployed SHA, release tag). A green result on the wrong build is worthless.
3. **Reuse fixtures** — prefer an existing page/record that already exercises the behavior. Create one only if none exists.
4. **Execute** — run each check; capture a screenshot or the raw signal after key steps.
5. **Report** — pass / fail / skipped per assertion, with evidence.

```text
Spot check report: <title>
- Target: <url / env>
- Results: <n> pass, <n> fail, <n> skipped
- Evidence: <screenshots / log lines / report link>
- Failures: <assertion> — <what you actually saw>
```

## Common gotchas

- **Cross-origin iframes** (embedded widgets, Forge/Connect macros): the content lives one frame deeper. Scope selectors to that frame — and not every browser tool crosses that boundary. Prefer one that traverses frames (e.g. Playwright).
- **CDN / bot challenges**: a challenged `403`/`503` from a non-browser client can mean "up but challenged," *not* down. A real browser render is the authoritative liveness signal — don't gate on a raw `curl` `200`.
- **Auth-dependent assertions**: a logged-in session may redirect anonymous-only routes; a logged-out one can't see gated features. Match the session to the assertion.

## Rules

- Never mark **PASS** without observing the planned signal — a real element, a captured response, a green assertion. "It probably works" is not a pass.
- Never run only a unit test or local fixture and call it a real-world spot check.
- Never create a new committed test file during a spot check. Promote it to the suite later if it's worth keeping.
- Never run destructive flows (account deletion, billing, mass writes) against production.

## Related

- **deviation-log** — captures where the plan met reality mid-build; a spot check confirms how those deviations actually behave.
- **merge-quiz** — gate on understanding your change before merge; a spot check gates on the change *working*.
- **Layer your project on top** — real env URLs, credentials, the exact analytics project, and iframe/tooling specifics belong in a per-project `spot-check` skill that extends this one.
