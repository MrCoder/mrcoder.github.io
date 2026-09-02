# Unattended loop

## Dispatch gate

`scripts/run_guard.py` is the only writer of `guard.json`. Call it before each token-consuming
dispatch and after each task, using stable task IDs and `small`, `medium`, or `large` size:

```bash
python3 <skill>/scripts/run_guard.py begin --provider codex --profile unattended \
  --run-dir "$RUN_DIR" --task T1 --size medium --model-tier luna
python3 <skill>/scripts/run_guard.py finish --provider codex --profile unattended \
  --run-dir "$RUN_DIR" --task T1
```

Exit `0` allows dispatch, `10` pauses or closes dispatch with a JSON reason, and `2` means invalid
configuration. Telemetry failure must use fallback, never exit `2`.

At an allowance or session pause, keep useful waits alive and use a cheap worker to hold:

```bash
python3 <skill>/scripts/run_guard.py wait --provider codex --profile unattended \
  --run-dir "$RUN_DIR"
```

`wait` retries every five minutes and returns when dispatch may resume, browser input is due, or the
time window ends. On browser success call `observe --used PCT --reset-at EPOCH`; on failure call
`browser-failed`. Try browser on entering estimate mode and every 30 minutes thereafter.

Fallback order is runtime telemetry, internal read-only HTTP, authenticated Usage page, then bounded
estimate. Runtime data expires after five minutes. Within a usage window, lower readings do not
refund allowance. Reset markers within five minutes are equivalent; at a known reset, tracking
restarts even without telemetry and later reconciles. Estimate completed dispatches with the latest
three-sample median for `{model tier, task size}`, defaulting small/medium/large to 1/2/4 percentage
points. Zero deltas do not train.

## Work cycle

1. Re-read `state.json`, `guard.json`, plan, and recent journal entries.
2. Reconcile completed and waiting work; inspect evidence against acceptance criteria.
3. Choose up to two independent streams: objective first, then high-latency work, then valuable
   follow-ons, and only then expiring allowance.
4. Pass the guard, route work under the model-cost boundary, and start long waits early.
5. Integrate and independently accept results. Give one evidence-based correction, then escalate one
   tier if needed.
6. Record state, evidence, model, escalation reason, and consequential decisions. Update the live
   report after material milestones.
7. When the initial plan ends, search only for useful work that directly advances the objective and
   remains authorized.

Incoming user context changes future work but does not end the loop unless it explicitly says stop
or pause. Safely finish an atomic in-flight step before applying a conflicting instruction.

## Verification and report

For visual work, capture and inspect the final UI yourself, explain what is visible, compare it with
acceptance, fix discrepancies, and re-check. Prefer a real end-to-end path when feasible; builds,
unit tests, test collection, or an unexamined screenshot are not UI evidence. If capability is
unavailable, mark verification `BLOCKED` or `SKIPPED` and continue other work.

Every demoable functional outcome needs a final-revision screenshot in the report with meaningful
alt text, a one-to-three-sentence observation/acceptance/limitation caption, time, revision,
environment, and capture method. Show final failures; if an earlier state passed, show both `Last
known good` and `Final observed state`. Generated media is `Illustration`, never evidence. Add short
video only when a relevant skill or existing validated pipeline is available and temporal media is
materially clearer than screenshots.

Create `REPORT.html` at start and update it at material milestones. It is a concise index, not a
journal copy. Follow the template, remove empty optional sections, but never hide verification gaps,
estimate intervals, or unfinished work. Keep useful local context, exclude reusable secrets and
anything project rules forbid storing locally, and use relative artifact paths without JavaScript.

## Wind down

Stop taking new work at `end_at`, on explicit stop, or when no useful authorized work remains. An
allowance boundary pauses rather than ends the window if a reset can restore capacity before
`end_at`. Finish the current safe atomic step, preserve meaningful wait results, finalize state and
the report, and record the exact end reason. Ordinary readiness or telemetry failures never finish
the run by themselves.
