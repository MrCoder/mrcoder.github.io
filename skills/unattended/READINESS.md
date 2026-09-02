# Readiness scan

The user may need to leave immediately. This scan exposes readiness; it is never a global gate.
Report each relevant check as `ready`, `degraded`, or `unavailable`, do not wait for a reply, and
incorporate any later user message while the run continues.

## Admit the work

- Read repository guidance and relevant design documents.
- Write the objective, ordered initial work, observable acceptance criteria, and evidence method.
- Identify high-latency work to start early and likely follow-ons that directly advance the goal.
- Protect dirty checkouts with a feature branch or separate worktree; never disturb another session.

## Fix the boundary

Record allowed repositories, branches/worktrees, accounts/environments, credentials, network use,
and external mutations. Default to branch push, PR create/update, and CI. Do not infer a tenant,
account, profile, or authority from old files or credential presence.

Missing access narrows the executable work but does not cancel the away window. Explain the gap and
continue useful work inside the remaining boundary.

## Check only relevant capabilities

- Browser-dependent work: find Playwright, agent-browser, or an equivalent skill/capability;
  actually launch or connect; verify the authorized credential or logged-in profile; read the
  target page; and save a test screenshot. Never invent a profile or authentication command.
- CI, E2E, deploy, or monitoring: verify the command, target, credentials, and a non-mutating read
  path when available.
- Long local work: verify the launching process survives and the machine can remain awake. Remote
  or cloud work instead proves its runner remains available; do not require `caffeinate` everywhere.

If a check fails, show what is unavailable and how the user could fix it if still present, then
continue without waiting. Never fabricate evidence for the missing capability.

## Start the run

Create the run directory, copy the plan, state, and report templates, and create empty `journal.md`,
`DECISIONS-FOR-USER.md`, and `evidence/`. Initialize the guard with an explicit provider and the
already-echoed normalized end time:

```bash
python3 <skill>/scripts/run_guard.py check --provider codex --profile unattended \
  --run-dir "$RUN_DIR" --end-at "<ISO timestamp>" --allowance 10
```

Use `claude` in Claude Code. If no real reading is available, try authenticated browser observation;
if that also fails, begin in estimate mode. Print readiness, model routing, concurrency, external
ceiling, and report path, then start work immediately.
