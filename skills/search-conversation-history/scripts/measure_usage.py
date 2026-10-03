#!/usr/bin/env python3
"""Measure elapsed time and recorded Codex or explicit Claude token deltas, without billing estimates.

Only JSON counters and session metadata are retained. Histories are never modified.
Explicit Claude logs use latest usage per assistant message id; sidechains are excluded.
"""
import argparse
import glob
import json
import os
from pathlib import Path
import time


SCOPE = "Session counter deltas through latest available usage event; excludes final answer generation; counters may lag the latest tool call."


def discover():
    thread = os.environ.get("CODEX_THREAD_ID")
    if not thread:
        return None, "CODEX_THREAD_ID is missing"
    # Filename selection only, followed by metadata verification. Never search transcript text.
    candidates = glob.glob(str(Path.home() / ".codex/sessions" / "**" / ("*" + glob.escape(thread) + ".jsonl")), recursive=True)
    verified = []
    for path in candidates:
        try:
            with open(path) as stream:
                first = json.loads(stream.readline())
            if first.get("type") == "session_meta" and first.get("payload", {}).get("id") == thread:
                verified.append(path)
        except (OSError, ValueError):
            pass
    if len(verified) != 1:
        return None, "No unique session log verified against CODEX_THREAD_ID"
    return verified[0], None


def read_log(path, guard_after=None, baseline=None):
    latest = None
    through = None
    session = None
    identity = None
    reason = None
    previous = baseline
    reset = False
    offset = 0
    try:
        with open(path, "rb") as stream:
            stat = os.fstat(stream.fileno())
            identity = [stat.st_dev, stat.st_ino]
            for index, line in enumerate(stream):
                line_offset = offset
                offset += len(line)
                try:
                    row = json.loads(line)
                except ValueError:
                    # A concurrently written final line is not a completed counter snapshot.
                    continue
                if index == 0:
                    if row.get("type") != "session_meta":
                        return None, None, identity, "Unsupported Codex log format", None, offset
                    session = row.get("payload", {}).get("id")
                payload = row.get("payload", {})
                if row.get("type") == "event_msg" and payload.get("type") == "token_count":
                    info = payload.get("info")
                    if isinstance(info, dict) and "total_token_usage" in info:
                        latest = info["total_token_usage"]
                        through = row.get("timestamp")
                        if guard_after is not None and line_offset >= guard_after:
                            try:
                                current = counters(latest)
                                if previous is not None and (current.keys() != previous.keys() or any(current[k] < previous[k] for k in current)):
                                    reset = True
                                previous = current
                            except ValueError:
                                reset = True
        if reset:
            reason = "Cumulative token counters reset, decreased, or became incomplete"
        elif not session:
            reason = "Session metadata id is missing"
        elif latest is None:
            reason = "No cumulative token snapshot recorded"
    except OSError as error:
        reason = "Cannot read session log: " + error.__class__.__name__
    return latest, session, identity, reason, through, offset


def counters(raw):
    required = ("input_tokens", "cached_input_tokens", "output_tokens")
    if not isinstance(raw, dict) or any(type(raw.get(k)) is not int or raw[k] < 0 for k in required):
        raise ValueError("Incomplete or invalid token counters")
    if raw["cached_input_tokens"] > raw["input_tokens"]:
        raise ValueError("Cached input exceeds input")
    result = {"input": raw["input_tokens"], "cached_input": raw["cached_input_tokens"],
              "uncached_input": raw["input_tokens"] - raw["cached_input_tokens"], "output": raw["output_tokens"]}
    if "cache_write_input_tokens" in raw:
        value = raw["cache_write_input_tokens"]
        if type(value) is not int or value < 0:
            raise ValueError("Invalid cache write counter")
        result["cache_write"] = value
    # Reasoning output is already part of output_tokens; do not add it again.
    return result


def claude_counters(raw):
    required = ("input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens", "output_tokens")
    if not isinstance(raw, dict) or any(type(raw.get(k)) is not int or raw[k] < 0 for k in required):
        raise ValueError("Incomplete or invalid Claude token counters")
    new = raw["input_tokens"] + raw["cache_creation_input_tokens"]
    return {"input": new + raw["cache_read_input_tokens"], "uncached_input": new,
            "cached_input": raw["cache_read_input_tokens"], "cache_write": raw["cache_creation_input_tokens"],
            "output": raw["output_tokens"]}


def read_claude(path):
    messages, session, through, offset, identity = {}, None, None, 0, None
    reason = None
    try:
        with open(path, "rb") as stream:
            stat = os.fstat(stream.fileno())
            identity = [stat.st_dev, stat.st_ino]
            for line in stream:
                offset += len(line)
                try:
                    row = json.loads(line)
                except ValueError:
                    continue
                if row.get("isSidechain") is True:
                    continue
                row_session = row.get("sessionId")
                if row_session:
                    if session and row_session != session:
                        reason = "Mixed Claude session ids"
                    session = row_session
                if row.get("type") != "assistant":
                    continue
                message = row.get("message", {})
                message_id = message.get("id")
                try:
                    if not row_session or not message_id:
                        raise ValueError("Claude assistant session or message id is missing")
                    usage = claude_counters(message.get("usage"))
                    if message_id in messages and any(usage[k] < messages[message_id][k] for k in usage):
                        raise ValueError("Claude message counters reset or decreased")
                    messages[message_id] = usage
                    through = row.get("timestamp")
                except ValueError as error:
                    reason = str(error)
        if not session or not messages:
            reason = reason or "No supported Claude assistant usage snapshots"
    except OSError as error:
        reason = "Cannot read session log: " + error.__class__.__name__
    return messages, session, identity, reason, through, offset


def runtime_for(path):
    try:
        with open(path) as stream:
            first = json.loads(stream.readline())
        return "Codex" if first.get("type") == "session_meta" else "Claude"
    except (OSError, ValueError):
        return "Codex"


def start(state, session_log=None):
    monotonic, wall = time.monotonic(), time.time()
    path, reason = (str(Path(session_log).expanduser().resolve()), None) if session_log else discover()
    raw, session, identity = None, None, None
    log_size = None
    runtime = runtime_for(path) if path else None
    if path:
        raw, session, identity, reason, _, log_size = (read_claude(path) if runtime == "Claude" else read_log(path))
    agent_path = None
    if path and session:
        try:
            with open(path) as stream:
                agent_path = json.loads(stream.readline()).get("payload", {}).get("agent_path")
        except (OSError, ValueError):
            pass
    baseline = None
    if not reason:
        try:
            baseline = raw if runtime == "Claude" else counters(raw)
        except ValueError as error:
            reason = str(error)
    data = {"version": 1, "monotonic": monotonic, "wall_time": wall,
            "path": path, "log_size": log_size, "session_id": session, "agent_path": agent_path, "identity": identity, "baseline": baseline,
            "unavailable_reason": reason, "runtime": runtime, "boot_id": boot_id()}
    descriptor = os.open(state, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w") as stream:
        json.dump(data, stream)
    return {"agent_path": agent_path, "started": True, "tokens_available": baseline is not None, "unavailable_reason": reason}


def boot_id():
    # Monotonic clocks are shared across processes but reset on restart.
    try:
        return Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    except OSError:
        return None


def finish(state):
    data = json.loads(Path(state).read_text())
    elapsed = time.monotonic() - data["monotonic"]
    timing_reason = None
    if elapsed < 0 or data.get("boot_id") != boot_id():
        elapsed = None
        timing_reason = "Monotonic clock restarted"
    reason = data["unavailable_reason"]
    delta = None
    through = None
    if not reason:
        runtime = data.get("runtime", "Codex")
        raw, session, identity, reason, through, _ = (read_claude(data["path"]) if runtime == "Claude" else read_log(data["path"], data.get("log_size"), data["baseline"]))
        if not reason and (session != data["session_id"] or identity != data["identity"]):
            reason = "Session log identity changed"
        if not reason:
            try:
                baseline = data["baseline"]
                if runtime == "Claude":
                    if not baseline.keys() <= raw.keys():
                        raise ValueError("Claude baseline messages disappeared")
                    delta = {key: 0 for key in next(iter(raw.values()))}
                    for message_id, usage in raw.items():
                        before = baseline.get(message_id, {key: 0 for key in usage})
                        for key in usage:
                            difference = usage[key] - before[key]
                            if difference < 0:
                                raise ValueError("Claude message counters reset or decreased")
                            delta[key] += difference
                    current = None
                else:
                    current = counters(raw)
                if current is not None and current.keys() != baseline.keys():
                    raise ValueError("Counter fields changed between boundaries")
                if current is not None:
                    delta = {key: current[key] - baseline[key] for key in current}
                if any(value < 0 for value in delta.values()):
                    raise ValueError("Cumulative token counters reset or decreased")
            except ValueError as error:
                delta, reason = None, str(error)
    return {"elapsed_seconds": round(elapsed, 3) if elapsed is not None else None,
            "timing_unavailable_reason": timing_reason, "tokens": delta,
            "runtime": data.get("runtime") if data.get("session_id") else None,
            "source": ("local assistant usage snapshots deduplicated by message id" if data.get("runtime") == "Claude" else "local session cumulative token_count snapshots") if data.get("session_id") else None,
            "scope": SCOPE, "tokens_through": through, "unavailable_reason": reason}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    begin = commands.add_parser("start")
    begin.add_argument("--state", required=True)
    begin.add_argument("--session-log")
    end = commands.add_parser("finish")
    end.add_argument("--state", required=True)
    args = parser.parse_args()
    try:
        result = start(args.state, args.session_log) if args.command == "start" else finish(args.state)
    except (OSError, ValueError, KeyError) as error:
        parser.exit(1, "Measurement failed: " + str(error) + "\n")
    print(json.dumps(result, separators=(",", ":")))


if __name__ == "__main__":
    main()
