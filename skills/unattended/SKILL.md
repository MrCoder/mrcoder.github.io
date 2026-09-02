---
name: unattended
description: Run valuable work autonomously for a bounded period while the user is away. Use when the user says unattended, AFK, away from keyboard, while I am away, or asks the agent to keep working without waiting for replies.
---

# Unattended

Use the user's away time well. Priority order:

1. Complete the requested objective.
2. Start high-latency work early and overlap useful waits that need no human reply: CI/CD, builds,
   real E2E, deployment verification, and monitoring.
3. Continue with other valuable work that directly advances the objective and stays in scope.
4. Only then use allowance that would otherwise expire.

This is not token-burning mode. Do not manufacture work, expand scope, or favor small predictable
tasks merely to consume allowance.

## Intake

Accept either a duration or absolute end time. Default `unattended` and AFK runs to two hours. An
undated clock time means its next occurrence in the user's timezone. Normalize it to a full
`end_at`; default allowance is 10 percentage points. Accept only whole numbers `1..100` and reject
unlimited.

Before readiness checks, echo the objective, profile, complete end time and timezone, allowance,
provider, and permission boundary. Continue immediately; the echo is not another approval gate.

Run the non-blocking [readiness scan](READINESS.md), then follow the [loop](LOOP.md). Ordinary user
messages add requirements or context without stopping the run. Only an explicit stop or pause ends
it early.

## Contract

- A missing capability, credential, telemetry source, or user reply never cancels the whole away
  window. Surface the gap, use safe fallbacks, and advance other authorized work.
- Telemetry falls back through runtime, HTTP, browser, and bounded estimation. Telemetry failure
  never ends a run.
- Usage is account-wide: other sessions count. A run gets its own allowance, not ownership of the
  observed account change. A reset restarts allowance tracking but never extends `end_at`.
- `run_guard.py` controls new token-consuming dispatches. In-flight work may overshoot. At an
  allowance boundary, preserve useful waits and resume after an in-window reset. At `end_at`, stop
  taking new work and finish the current safe atomic step.
- Run at most two independent workstreams; a waiting stream counts.
- Default external ceiling allows feature-branch push, PR create/update, and CI. Merge, production,
  tracker writes, purchases, messages, and new external resources require explicit authorization.
- Existing credentials may be used only for pre-authorized resources and operations. Their presence
  does not expand scope.

## Model cost boundary

The current model is coordinator regardless of tier. It owns state, work choice, guard checks,
integration, acceptance, and critical judgment.

If the coordinator is Fable, Sol, or unknown, delegate multi-step exploration, coding, testing,
browser work, and waiting aggressively to available lower-cost models. A known low-cost coordinator
may work directly. Escalate only after concrete acceptance failure: one correction, then one tier.
Workers may not exceed the coordinator's known tier without explicit authorization. If no worker is
available, the coordinator continues; model availability never stops the run.

## Run files

Store new runs under `~/.unattended-runs/<repo>/<run-id>/`:

- `PLAN.md`: objective, boundaries, acceptance criteria, and likely follow-ons.
- `state.json`: coordinator-owned profile and task/report state.
- `guard.json`: `run_guard.py`-owned time and usage state; never edit manually.
- `journal.md` and `evidence/`: detailed trace and artifacts.
- `DECISIONS-FOR-USER.md`: follow-ups that did not block progress.
- `REPORT.html`: live and final user-facing index. A preset may rename it.

Re-read disk state after every wake or compaction.
