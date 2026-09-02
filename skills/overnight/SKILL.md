---
name: overnight
description: Run valuable work through the night without waiting for replies. Use for overnight or all-night requests; use unattended for shorter AFK periods.
---

# Overnight

Read and follow the shared core at `../unattended/SKILL.md`, including its linked `READINESS.md` and
`LOOP.md`. Use the core scripts and templates directly; do not copy or restate them.

Apply only these preset overrides:

1. Set `profile` to `overnight` and default the time window to eight hours.
2. Run a complete all-night readiness scan, including credentials and browser paths likely to be
   needed later, long-running CI/E2E/deploy waits, launching-process survival, and persistence for
   the actual local, remote, or cloud runtime.
3. Every readiness result remains non-blocking. Echo gaps and possible fixes, then start without
   waiting; later user context joins the live run.
4. Pass `--profile overnight` to `run_guard.py` and keep the same default 10-point allowance.
5. Copy the shared `REPORT.html` template as `MORNING-REPORT.html`; all content and evidence rules
   remain identical.

Store new runs under the core `~/.unattended-runs/` root. Leave historical `~/.overnight-runs/`
untouched and do not provide a compatibility shim.
