#!/usr/bin/env python3
"""Time and account-wide allowance guard for unattended Claude/Codex runs."""

import argparse
import fcntl
import json
import os
import statistics
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from datetime import datetime
from pathlib import Path

WEEK = 604800
FRESH = 300
DEFAULT_COST = {"small": 1.0, "medium": 2.0, "large": 4.0}


def iso_epoch(value):
    if isinstance(value, (int, float)):
        return int(value)
    if not isinstance(value, str):
        raise ValueError("invalid-reset")  # noqa: TRY004
    if value.isdigit():
        return int(value)
    return int(datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp())


def read_json(path):
    with Path(path).open() as handle:
        return json.load(handle)


def atomic_write(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=path.name + ".", dir=path.parent)
    try:
        with os.fdopen(fd, "w") as handle:
            json.dump(value, handle, indent=2, sort_keys=True)
            handle.write("\n")
        os.chmod(tmp, 0o600)
        os.replace(tmp, path)
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)


def normalized(used, reset, source, observed=None, session=None, session_reset=None):
    used = float(used)
    if not 0 <= used <= 100:
        raise ValueError("used-out-of-range")
    return {
        "used_pct": used,
        "reset_at": iso_epoch(reset),
        "source": source,
        "observed_at": int(observed or time.time()),
        "session_pct": session,
        "session_reset_at": iso_epoch(session_reset) if session_reset else None,
    }


def codex_runtime(home, now):
    root = Path(home) / "sessions"
    best = None
    if not root.exists():
        return None
    for path in root.rglob("*.jsonl"):
        try:
            if now - path.stat().st_mtime > FRESH:
                continue
            with path.open("rb") as handle:
                handle.seek(max(0, path.stat().st_size - 1_000_000))
                lines = handle.read().decode(errors="ignore").splitlines()
        except OSError:
            continue
        for line in lines:
            if '"rate_limits"' not in line:
                continue
            try:
                row = json.loads(line)
                limits = row.get("payload", {}).get("rate_limits", {})
                window = limits.get("primary") or {}
                if window.get("window_minutes") != 10080:
                    continue
                seen = iso_epoch(row["timestamp"])
                item = normalized(
                    window["used_percent"], window["resets_at"], "codex-runtime", seen
                )
                if now - seen <= FRESH and (best is None or seen > best[0]):
                    best = (seen, item)
            except (KeyError, TypeError, ValueError, json.JSONDecodeError):
                continue
    return best[1] if best else None


def claude_runtime(cache, now):
    try:
        if now - Path(cache).stat().st_mtime > FRESH:
            return None
        data = read_json(cache)
        weekly = data["rate_limits"]["seven_day"]
        five = data.get("rate_limits", {}).get("five_hour", {})
        return normalized(
            weekly["used_percentage"],
            weekly["resets_at"],
            "claude-runtime",
            data["observed_at"],
            five.get("used_percentage"),
            five.get("resets_at"),
        )
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        return None


def http_json(url, headers, fixture=None):
    if fixture:
        return read_json(fixture)
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=20) as response:
        return json.load(response)


def codex_http(home):
    try:
        auth = read_json(Path(home) / "auth.json")["tokens"]
        data = http_json(
            os.getenv(
                "UNATTENDED_CODEX_URL", "https://chatgpt.com/backend-api/wham/usage"
            ),
            {
                "Authorization": "Bearer " + auth["access_token"],
                "ChatGPT-Account-Id": auth["account_id"],
                "Accept": "application/json",
            },
            os.getenv("UNATTENDED_CODEX_FIXTURE"),
        )
        window = data["rate_limit"]["primary_window"]
        if window["limit_window_seconds"] != WEEK:
            return None
        return normalized(window["used_percent"], window["reset_at"], "codex-http")
    except (
        OSError,
        KeyError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
        urllib.error.URLError,
    ):
        return None


def claude_token():
    if os.getenv("CLAUDE_CODE_OAUTH_TOKEN"):
        return os.environ["CLAUDE_CODE_OAUTH_TOKEN"]
    try:
        raw = subprocess.check_output(
            [
                "security",
                "find-generic-password",
                "-s",
                "Claude Code-credentials",
                "-a",
                os.getenv("USER", ""),
                "-w",
            ],
            stderr=subprocess.DEVNULL,
            text=True,
        )
        return json.loads(raw)["claudeAiOauth"]["accessToken"]
    except (OSError, KeyError, ValueError, subprocess.SubprocessError):
        try:
            return read_json(Path.home() / ".claude/.credentials.json")[
                "claudeAiOauth"
            ]["accessToken"]
        except (OSError, KeyError, ValueError, json.JSONDecodeError):
            return None


def claude_http():
    try:
        token = claude_token()
        if not token:
            return None
        version = subprocess.check_output(
            ["claude", "--version"], stderr=subprocess.DEVNULL, text=True
        ).split()[0]
        data = http_json(
            "https://api.anthropic.com/api/oauth/usage",
            {
                "Authorization": "Bearer " + token,
                "anthropic-beta": "oauth-2025-04-20",
                "User-Agent": f"claude-cli/{version} (external, cli)",
            },
            os.getenv("UNATTENDED_CLAUDE_FIXTURE"),
        )
        five, week = data["five_hour"], data["seven_day"]
        return normalized(
            week["utilization"],
            week["resets_at"],
            "claude-http",
            session=five.get("utilization"),
            session_reset=five.get("resets_at"),
        )
    except (
        OSError,
        KeyError,
        TypeError,
        ValueError,
        json.JSONDecodeError,
        subprocess.SubprocessError,
        urllib.error.URLError,
    ):
        return None


def probe(provider, now):
    if provider == "codex":
        home = os.getenv("CODEX_HOME", str(Path.home() / ".codex"))
        return codex_runtime(home, now) or codex_http(home)
    cache = os.getenv(
        "UNATTENDED_CLAUDE_CACHE",
        str(Path.home() / ".claude/unattended-rate-limits.json"),
    )
    return claude_runtime(cache, now) or claude_http()


def initial(provider, profile, allowance, session_stop, end_at, now):
    if end_at is None:
        end_at = now + (28800 if profile == "overnight" else 7200)
    else:
        end_at = iso_epoch(end_at)
    if end_at <= now:
        raise ValueError("end-at-must-be-in-the-future")
    return {
        "provider": provider,
        "profile": profile,
        "end_at": end_at,
        "allowance_pct": allowance,
        "baseline_pct": None,
        "session_stop_pct": session_stop if provider == "claude" else None,
        "high_water_pct": None,
        "reset_at": None,
        "reset_count": 0,
        "estimated_extra_pct": 0,
        "estimated_at": None,
        "last_real": None,
        "last_probe": None,
        "last_browser_attempt_at": None,
        "samples": {},
        "inflight": {},
    }


def new_window(state, reading, now):
    baseline = reading["used_pct"] if reading else 0
    previous = state.get("last_real") or {}
    last_real = reading or (
        previous if (previous.get("session_reset_at") or 0) > now else None
    )
    state.update(
        {
            "baseline_pct": baseline,
            "high_water_pct": baseline,
            "estimated_extra_pct": 0,
            "estimated_at": None,
            "last_real": last_real,
            "reset_at": reading["reset_at"] if reading else state["reset_at"] + WEEK,
            "reset_count": state["reset_count"] + 1,
        }
    )
    state["last_probe"] = {
        "source": "reset" if reading else "estimate",
        "observed_at": now,
    }


def reconcile(state, reading, now):
    reset = state.get("reset_at")
    if reset and now >= reset:
        next_reading = (
            reading if reading and reading["reset_at"] > reset + 300 else None
        )
        new_window(state, next_reading, now)
        return "reset"
    if not reading:
        state["last_probe"] = {"source": "estimate", "observed_at": now}
        return "estimate"
    if reset and reading["reset_at"] > reset + 300 and now >= reset - 300:
        new_window(state, reading, now)
        return "reset"
    if state["baseline_pct"] is None:
        state.update(
            {
                "baseline_pct": reading["used_pct"],
                "high_water_pct": reading["used_pct"],
                "reset_at": reading["reset_at"],
            }
        )
    else:
        state["high_water_pct"] = max(state["high_water_pct"], reading["used_pct"])
        if not reset or abs(reading["reset_at"] - reset) > 300:
            state["reset_at"] = reading["reset_at"]
    state.update({"last_real": reading, "last_probe": reading})
    if not state.get("estimated_at") or reading["observed_at"] >= state["estimated_at"]:
        state.update({"estimated_extra_pct": 0, "estimated_at": None})
    return "real"


def effective(state):
    base = state["high_water_pct"] if state["high_water_pct"] is not None else 0
    return base + state["estimated_extra_pct"]


def decision(state, now):
    consumed = effective(state) - (state["baseline_pct"] or 0)
    browser_due = state["last_probe"]["source"] == "estimate" and (
        not state["last_browser_attempt_at"]
        or now - state["last_browser_attempt_at"] >= 1800
    )
    session = (state.get("last_real") or {}).get("session_pct")
    session_pause = session is not None and session >= (
        state.get("session_stop_pct") or 101
    )
    allowance_pause = consumed >= state["allowance_pct"]
    time_ended = now >= state["end_at"]
    reason = "time-window-ended" if time_ended else (
        "session-limit"
        if session_pause
        else ("allowance-consumed" if allowance_pause else "headroom")
    )
    if browser_due and not time_ended:
        reason = "browser-check-due"
    return {
        "decision": "pause"
        if time_ended or session_pause or allowance_pause or browser_due
        else "allow",
        "reason": reason,
        "provider": state["provider"],
        "source": state["last_probe"]["source"],
        "used_pct": effective(state),
        "consumed_pct": consumed,
        "allowance_pct": state["allowance_pct"],
        "end_at": state["end_at"],
        "reset_at": state["reset_at"],
        "browser_due": browser_due,
    }


def sample_key(tier, size):
    return tier + ":" + size


def estimate_cost(state, tier, size):
    samples = state["samples"].get(sample_key(tier, size), [])
    return statistics.median(samples) if samples else DEFAULT_COST[size]


def load_state(
    path, provider, profile="unattended", allowance=None, session_stop=80,
    end_at=None, now=None
):
    if path.exists():
        state = read_json(path)
        if state.get("provider") != provider:
            raise ValueError("provider-mismatch")
        if state.get("profile") != profile:
            raise ValueError("profile-mismatch")
        if end_at is not None and state.get("end_at") != iso_epoch(end_at):
            raise ValueError("end-at-mismatch")
        if allowance is not None and state.get("allowance_pct") != allowance:
            raise ValueError("allowance-mismatch")
        return state
    value = 10 if allowance is None else allowance
    if isinstance(value, bool) or not isinstance(value, int) or not 1 <= value <= 100:
        raise ValueError("allowance-must-be-integer-1..100")
    if not 1 <= session_stop <= 100:
        raise ValueError("session-stop-must-be-1..100")
    return initial(provider, profile, value, session_stop, end_at, int(now or time.time()))


def run_once(args, state, path, now):
    reading = probe(args.provider, now)
    mode = reconcile(state, reading, now)
    if args.command == "observe":
        reading = normalized(args.used, args.reset_at, "browser", now)
        previous = state.get("last_real") or {}
        if (
            args.provider == "claude"
            and previous.get("session_reset_at")
            and previous["session_reset_at"] > now
        ):
            reading["session_pct"] = previous.get("session_pct")
            reading["session_reset_at"] = previous["session_reset_at"]
        reconcile(state, reading, now)
        state["last_browser_attempt_at"] = now
    elif args.command == "browser-failed":
        state["last_browser_attempt_at"] = now
    elif args.command == "begin":
        result = decision(state, now)
        if result["decision"] == "allow":
            state["inflight"][args.task] = {
                "size": args.size,
                "model_tier": args.model_tier,
                "before_used": reading["used_pct"] if reading else None,
                "before_reset": reading["reset_at"] if reading else state["reset_at"],
            }
    elif args.command == "finish":
        task = state["inflight"].pop(args.task)
        key = sample_key(task["model_tier"], task["size"])
        if (
            reading
            and task["before_used"] is not None
            and abs(task["before_reset"] - reading["reset_at"]) <= 300
        ):
            delta = reading["used_pct"] - task["before_used"]
            if delta > 0:
                state["samples"][key] = (state["samples"].get(key, []) + [delta])[-3:]
        elif mode in ("estimate", "reset"):
            state["estimated_extra_pct"] += estimate_cost(
                state, task["model_tier"], task["size"]
            )
            state["estimated_at"] = now
    atomic_write(path, state)
    return decision(state, now)


def parser():
    result = argparse.ArgumentParser()
    result.add_argument(
        "command",
        choices=["check", "begin", "finish", "observe", "browser-failed", "wait"],
    )
    result.add_argument("--provider", required=True, choices=["claude", "codex"])
    result.add_argument(
        "--profile", default="unattended", choices=["unattended", "overnight"]
    )
    result.add_argument("--run-dir", required=True, type=Path)
    result.add_argument("--end-at")
    result.add_argument("--allowance", type=int)
    result.add_argument("--session-stop", type=int, default=80)
    result.add_argument("--task")
    result.add_argument("--size", choices=DEFAULT_COST)
    result.add_argument("--model-tier")
    result.add_argument("--used", type=float)
    result.add_argument("--reset-at")
    return result


def main():
    args = parser().parse_args()
    path = args.run_dir / "guard.json"
    try:
        if args.command == "begin" and not all((args.task, args.size, args.model_tier)):
            raise ValueError("task-size-model-tier-required")
        if args.command == "finish" and not args.task:
            raise ValueError("task-required")
        if args.command == "observe" and (args.used is None or args.reset_at is None):
            raise ValueError("used-and-reset-required")
        while True:
            now = int(time.time())
            args.run_dir.mkdir(parents=True, exist_ok=True)
            with (args.run_dir / "guard.lock").open("w") as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                state = load_state(
                    path,
                    args.provider,
                    args.profile,
                    args.allowance,
                    args.session_stop,
                    args.end_at,
                    now,
                )
                out = run_once(args, state, path, now)
            if (
                args.command != "wait"
                or out["decision"] == "allow"
                or out["browser_due"]
                or out["reason"] == "time-window-ended"
            ):
                print(json.dumps(out, sort_keys=True))
                raise SystemExit(0 if out["decision"] == "allow" else 10)
            time.sleep(int(os.getenv("UNATTENDED_WAIT_SECONDS", "300")))
    except (OSError, KeyError, ValueError, json.JSONDecodeError) as error:
        print(json.dumps({"decision": "error", "reason": str(error)}))
        raise SystemExit(2)


if __name__ == "__main__":
    main()
