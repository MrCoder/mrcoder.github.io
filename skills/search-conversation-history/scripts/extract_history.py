#!/usr/bin/env python3
"""Normalize local Codex, Claude Code, and Cursor histories into JSONL."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sqlite3
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    window = parser.add_mutually_exclusive_group()
    window.add_argument("--days", type=int, default=30, help="Recent-day window (default: 30)")
    window.add_argument("--all", action="store_true", help="Include all available history")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--codex-root", type=Path, default=Path.home() / ".codex")
    parser.add_argument("--claude-root", type=Path, default=Path.home() / ".claude")
    parser.add_argument(
        "--cursor-root",
        type=Path,
        default=Path.home() / "Library/Application Support/Cursor",
    )
    parser.add_argument("--cursor-agent-root", type=Path, default=Path.home() / ".cursor/chats")
    parser.add_argument("--max-chars", type=int, default=12_000)
    args = parser.parse_args()
    if args.days is not None and args.days <= 0:
        parser.error("--days must be positive")
    if args.max_chars <= 0:
        parser.error("--max-chars must be positive")
    return args


def timestamp_epoch(value: Any) -> float | None:
    if value is None:
        return None
    if isinstance(value, (int, float)):
        number = float(value)
        return number / 1000 if number > 100_000_000_000 else number
    if isinstance(value, str):
        stripped = value.strip()
        if not stripped:
            return None
        try:
            return timestamp_epoch(float(stripped))
        except ValueError:
            pass
        try:
            return datetime.fromisoformat(stripped.replace("Z", "+00:00")).timestamp()
        except ValueError:
            return None
    return None


def iso_timestamp(value: Any) -> str | None:
    epoch = timestamp_epoch(value)
    if epoch is None:
        return None
    return datetime.fromtimestamp(epoch, timezone.utc).isoformat().replace("+00:00", "Z")


def in_window(value: Any, cutoff: float | None) -> bool:
    if cutoff is None:
        return True
    epoch = timestamp_epoch(value)
    return epoch is not None and epoch >= cutoff


def text_from_blocks(content: Any, allowed_types: set[str] | None = None) -> str:
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return ""
    parts: list[str] = []
    for block in content:
        if not isinstance(block, dict):
            continue
        block_type = str(block.get("type", ""))
        if allowed_types is not None and block_type not in allowed_types:
            continue
        text = block.get("text")
        if isinstance(text, str) and text.strip():
            parts.append(text)
    return "\n".join(parts)


def compact_text(text: str) -> str:
    return "\n".join(line.rstrip() for line in text.replace("\r\n", "\n").split("\n")).strip()


def record(
    source: str,
    session_id: str,
    timestamp: Any,
    role: str,
    cwd: str | None,
    text: str,
    max_chars: int,
) -> dict[str, Any] | None:
    cleaned = compact_text(text)
    if not cleaned:
        return None
    truncated = len(cleaned) > max_chars
    return {
        "source": source,
        "session_id": session_id,
        "timestamp": iso_timestamp(timestamp),
        "role": role,
        "cwd": cwd,
        "text": cleaned[:max_chars],
        "truncated": truncated,
    }


def read_json_lines(path: Path, errors: list[str]) -> Iterable[dict[str, Any]]:
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            for line_number, line in enumerate(handle, 1):
                try:
                    value = json.loads(line)
                except json.JSONDecodeError:
                    errors.append(f"{path}:{line_number}: malformed JSON")
                    continue
                if isinstance(value, dict):
                    yield value
    except OSError as exc:
        errors.append(f"{path}: {exc}")


def codex_records(root: Path, cutoff: float | None, max_chars: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    errors: list[str] = []
    session_files = sorted((root / "sessions").glob("**/*.jsonl")) if (root / "sessions").exists() else []
    session_files += sorted((root / "archived_sessions").glob("*.jsonl")) if (root / "archived_sessions").exists() else []
    output: list[dict[str, Any]] = []
    for path in session_files:
        values = list(read_json_lines(path, errors))
        metadata = next((v.get("payload", {}) for v in values if v.get("type") == "session_meta"), {})
        if metadata.get("parent_thread_id") is not None or isinstance(metadata.get("source"), dict):
            continue
        session_id = str(metadata.get("id") or path.stem.rsplit("-", 1)[-1])
        cwd = metadata.get("cwd") if isinstance(metadata.get("cwd"), str) else None
        for value in values:
            if value.get("type") == "event_msg":
                payload = value.get("payload")
                if not isinstance(payload, dict) or payload.get("type") != "user_message":
                    continue
                timestamp = value.get("timestamp")
                if not in_window(timestamp, cutoff):
                    continue
                item = record(
                    "codex",
                    session_id,
                    timestamp,
                    "user",
                    cwd,
                    payload.get("message", "") if isinstance(payload.get("message"), str) else "",
                    max_chars,
                )
                if item:
                    output.append(item)
                continue
            if value.get("type") != "response_item":
                continue
            payload = value.get("payload")
            if not isinstance(payload, dict) or payload.get("type") != "message":
                continue
            role = payload.get("role")
            if role != "assistant":
                continue
            timestamp = value.get("timestamp")
            if not in_window(timestamp, cutoff):
                continue
            text = text_from_blocks(payload.get("content"), {"output_text", "text"})
            item = record("codex", session_id, timestamp, role, cwd, text, max_chars)
            if item:
                output.append(item)

    history = root / "history.jsonl"
    if history.exists():
        for value in read_json_lines(history, errors):
            if not in_window(value.get("ts"), cutoff):
                continue
            item = record(
                "codex",
                str(value.get("session_id", "unknown")),
                value.get("ts"),
                "user",
                None,
                str(value.get("text", "")),
                max_chars,
            )
            if item:
                output.append(item)
        session_files.append(history)

    status = "ok" if session_files else "missing"
    return output, {"status": status, "files_read": len(session_files), "errors": errors}


def is_human_claude_prompt(value: dict[str, Any]) -> bool:
    if value.get("isMeta") is True or value.get("toolUseResult") is not None:
        return False
    if value.get("sourceToolAssistantUUID") is not None:
        return False
    origin = value.get("origin")
    if isinstance(origin, dict) and origin.get("kind") == "task-notification":
        return False
    if isinstance(origin, dict) and origin.get("kind") == "human":
        return True
    prompt_source = value.get("promptSource")
    if prompt_source in {"system", "sdk"} or value.get("entrypoint") == "sdk-cli":
        return False
    content = value.get("message", {}).get("content") if isinstance(value.get("message"), dict) else None
    if isinstance(content, list) and any(isinstance(block, dict) and block.get("type") == "tool_result" for block in content):
        return False
    return prompt_source in {None, "typed", "queued", "suggestion_accepted"}


def claude_records(root: Path, cutoff: float | None, max_chars: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    errors: list[str] = []
    project_root = root / "projects"
    files = sorted(project_root.glob("**/*.jsonl")) if project_root.exists() else []
    output: list[dict[str, Any]] = []
    for path in files:
        for value in read_json_lines(path, errors):
            if value.get("type") not in {"user", "assistant"} or value.get("isSidechain") is True:
                continue
            message = value.get("message")
            if not isinstance(message, dict):
                continue
            role = message.get("role")
            if role not in {"user", "assistant"}:
                continue
            if role == "user" and not is_human_claude_prompt(value):
                continue
            if role == "assistant" and value.get("entrypoint") == "sdk-cli":
                continue
            timestamp = value.get("timestamp")
            if not in_window(timestamp, cutoff):
                continue
            text = text_from_blocks(message.get("content"), {"text"})
            item = record(
                "claude",
                str(value.get("sessionId") or value.get("session_id") or path.stem),
                timestamp,
                role,
                value.get("cwd") if isinstance(value.get("cwd"), str) else None,
                text,
                max_chars,
            )
            if item:
                output.append(item)
    return output, {"status": "ok" if files else "missing", "files_read": len(files), "errors": errors}


def sqlite_tables(connection: sqlite3.Connection) -> set[str]:
    return {row[0] for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table'")}


def cursor_records(root: Path, cutoff: float | None, max_chars: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    errors: list[str] = []
    db = root / "User/globalStorage/state.vscdb"
    if not db.exists():
        return [], {"status": "missing", "files_read": 0, "errors": []}
    output: list[dict[str, Any]] = []
    try:
        connection = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
        tables = sqlite_tables(connection)
        required = {"composerHeaders", "cursorDiskKV"}
        if not required.issubset(tables):
            connection.close()
            return [], {
                "status": "unrecognized_schema",
                "files_read": 1,
                "errors": [f"missing tables: {', '.join(sorted(required - tables))}"],
            }
        headers = connection.execute(
            "SELECT composerId, createdAt, lastUpdatedAt, value FROM composerHeaders"
        ).fetchall()
        for composer_id, created_at, updated_at, header_value in headers:
            if cutoff is not None and not (in_window(updated_at, cutoff) or in_window(created_at, cutoff)):
                continue
            cwd = None
            try:
                header = json.loads(header_value) if header_value else {}
                workspace = header.get("workspaceIdentifier", {}) if isinstance(header, dict) else {}
                uri = workspace.get("uri", {}) if isinstance(workspace, dict) else {}
                if isinstance(uri, dict) and isinstance(uri.get("fsPath"), str):
                    cwd = uri["fsPath"]
            except (json.JSONDecodeError, TypeError):
                errors.append(f"composer {composer_id}: malformed header JSON")
            bubble_headers: list[dict[str, Any]] = []
            composer_row = connection.execute(
                "SELECT value FROM cursorDiskKV WHERE key = ?",
                (f"composerData:{composer_id}",),
            ).fetchone()
            if composer_row:
                try:
                    composer = json.loads(composer_row[0])
                    candidate_headers = composer.get("fullConversationHeadersOnly", []) if isinstance(composer, dict) else []
                    bubble_headers = [item for item in candidate_headers if isinstance(item, dict) and item.get("bubbleId")]
                except (json.JSONDecodeError, TypeError):
                    errors.append(f"composer {composer_id}: malformed composer JSON")
            if not bubble_headers:
                bubble_headers = [
                    {"bubbleId": key.rsplit(":", 1)[-1]}
                    for (key,) in connection.execute(
                        "SELECT key FROM cursorDiskKV WHERE key LIKE ?",
                        (f"bubbleId:{composer_id}:%",),
                    )
                ]
            for bubble_header in bubble_headers:
                bubble_id = bubble_header["bubbleId"]
                bubble_row = connection.execute(
                    "SELECT value FROM cursorDiskKV WHERE key = ?",
                    (f"bubbleId:{composer_id}:{bubble_id}",),
                ).fetchone()
                if not bubble_row:
                    errors.append(f"composer {composer_id}: missing bubble {bubble_id}")
                    continue
                bubble_value = bubble_row[0]
                try:
                    bubble = json.loads(bubble_value)
                except (json.JSONDecodeError, TypeError):
                    errors.append(f"composer {composer_id}: malformed bubble JSON")
                    continue
                if not isinstance(bubble, dict):
                    continue
                role = {1: "user", 2: "assistant"}.get(bubble.get("type"))
                if role is None or bubble.get("skipRendering") is True or bubble.get("isDisplayOnly") is True:
                    continue
                bubble_timestamp = bubble.get("createdAt") or bubble_header.get("createdAt")
                if cutoff is not None and bubble_timestamp is not None and not in_window(bubble_timestamp, cutoff):
                    continue
                item = record(
                    "cursor",
                    str(composer_id),
                    bubble_timestamp,
                    role,
                    cwd,
                    bubble.get("text", "") if isinstance(bubble.get("text"), str) else "",
                    max_chars,
                )
                if item:
                    output.append(item)
        connection.close()
    except sqlite3.Error as exc:
        return [], {"status": "error", "files_read": 1, "errors": [str(exc)]}
    return output, {"status": "ok", "files_read": 1, "errors": errors}


def read_varint(data: bytes, offset: int) -> tuple[int, int]:
    value = 0
    shift = 0
    while offset < len(data) and shift < 70:
        byte = data[offset]
        offset += 1
        value |= (byte & 0x7F) << shift
        if not byte & 0x80:
            return value, offset
        shift += 7
    raise ValueError("invalid protobuf varint")


def protobuf_fields(data: bytes) -> dict[int, list[tuple[int, int | bytes]]]:
    fields: dict[int, list[tuple[int, int | bytes]]] = {}
    offset = 0
    while offset < len(data):
        key, offset = read_varint(data, offset)
        field_number, wire_type = key >> 3, key & 7
        if field_number == 0:
            raise ValueError("invalid protobuf field zero")
        if wire_type == 0:
            value, offset = read_varint(data, offset)
        elif wire_type == 1:
            if offset + 8 > len(data):
                raise ValueError("truncated fixed64 field")
            value, offset = data[offset : offset + 8], offset + 8
        elif wire_type == 2:
            length, offset = read_varint(data, offset)
            if offset + length > len(data):
                raise ValueError("truncated length-delimited field")
            value, offset = data[offset : offset + length], offset + length
        elif wire_type == 5:
            if offset + 4 > len(data):
                raise ValueError("truncated fixed32 field")
            value, offset = data[offset : offset + 4], offset + 4
        else:
            raise ValueError(f"unsupported protobuf wire type {wire_type}")
        fields.setdefault(field_number, []).append((wire_type, value))
    return fields


def bytes_fields(fields: dict[int, list[tuple[int, int | bytes]]], number: int) -> list[bytes]:
    return [value for wire, value in fields.get(number, []) if wire == 2 and isinstance(value, bytes)]


def int_field(fields: dict[int, list[tuple[int, int | bytes]]], number: int) -> int | None:
    for wire, value in fields.get(number, []):
        if wire == 0 and isinstance(value, int):
            return value
    return None


def decoded_text(values: list[bytes]) -> str:
    for value in values:
        try:
            text = value.decode("utf-8")
        except UnicodeDecodeError:
            continue
        if text.strip():
            return text
    return ""


def cursor_agent_records(root: Path, cutoff: float | None, max_chars: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    errors: list[str] = []
    stores = sorted(root.glob("*/*/store.db")) if root.exists() else []
    output: list[dict[str, Any]] = []
    for store in stores:
        connection: sqlite3.Connection | None = None
        try:
            connection = sqlite3.connect(f"file:{store}?mode=ro", uri=True)
            if not {"meta", "blobs"}.issubset(sqlite_tables(connection)):
                errors.append(f"{store}: unrecognized Agent Host schema")
                continue
            meta_row = connection.execute("SELECT value FROM meta WHERE key='0'").fetchone()
            if not meta_row or not isinstance(meta_row[0], str):
                errors.append(f"{store}: missing meta root")
                continue
            try:
                metadata = json.loads(bytes.fromhex(meta_row[0]).decode("utf-8"))
            except (ValueError, UnicodeDecodeError, json.JSONDecodeError):
                errors.append(f"{store}: malformed meta root")
                continue
            session_id = str(metadata.get("agentId") or store.parent.name)
            session_created = metadata.get("createdAt")
            if cutoff is not None and not in_window(session_created, cutoff):
                continue

            def fetch_blob(blob_id: bytes | str) -> bytes | None:
                identifier = blob_id.hex() if isinstance(blob_id, bytes) else blob_id
                if len(identifier) != 64:
                    errors.append(f"agent {session_id}: invalid blob id")
                    return None
                row = connection.execute("SELECT data FROM blobs WHERE id=?", (identifier,)).fetchone()
                if not row or not isinstance(row[0], bytes):
                    errors.append(f"agent {session_id}: missing blob {identifier}")
                    return None
                if hashlib.sha256(row[0]).hexdigest() != identifier:
                    errors.append(f"agent {session_id}: blob hash mismatch {identifier}")
                    return None
                return row[0]

            root_id = metadata.get("latestRootBlobId")
            root_blob = fetch_blob(root_id) if isinstance(root_id, str) else None
            if root_blob is None:
                continue
            try:
                root_fields = protobuf_fields(root_blob)
            except ValueError as exc:
                errors.append(f"agent {session_id}: malformed root protobuf: {exc}")
                continue
            turn_refs = bytes_fields(root_fields, 8)
            timings = bytes_fields(root_fields, 14)
            timing_values: list[int | None] = []
            for timing in timings:
                try:
                    timing_values.append(int_field(protobuf_fields(timing), 2))
                except ValueError:
                    timing_values.append(None)
            for index, turn_ref in enumerate(turn_refs):
                turn_blob = fetch_blob(turn_ref)
                if turn_blob is None:
                    continue
                try:
                    turn_fields = protobuf_fields(turn_blob)
                    agent_payloads = bytes_fields(turn_fields, 1)
                    if not agent_payloads:
                        continue
                    agent_fields = protobuf_fields(agent_payloads[0])
                except ValueError as exc:
                    errors.append(f"agent {session_id}: malformed turn protobuf: {exc}")
                    continue
                timestamp_ms = timing_values[index] if index < len(timing_values) else None
                timestamp = timestamp_ms or session_created
                user_refs = bytes_fields(agent_fields, 1)
                if user_refs:
                    user_blob = fetch_blob(user_refs[0])
                    if user_blob:
                        try:
                            user_fields = protobuf_fields(user_blob)
                            user_text = decoded_text(bytes_fields(user_fields, 1))
                            item = record("cursor", session_id, timestamp, "user", None, user_text, max_chars)
                            if item:
                                output.append(item)
                        except ValueError as exc:
                            errors.append(f"agent {session_id}: malformed user protobuf: {exc}")
                for step_ref in bytes_fields(agent_fields, 2):
                    step_blob = fetch_blob(step_ref)
                    if not step_blob:
                        continue
                    try:
                        step_fields = protobuf_fields(step_blob)
                        assistant_payloads = bytes_fields(step_fields, 1)
                        if not assistant_payloads:
                            continue
                        assistant_fields = protobuf_fields(assistant_payloads[0])
                        assistant_text = decoded_text(bytes_fields(assistant_fields, 1))
                        item = record("cursor", session_id, timestamp, "assistant", None, assistant_text, max_chars)
                        if item:
                            output.append(item)
                    except ValueError as exc:
                        errors.append(f"agent {session_id}: malformed assistant protobuf: {exc}")
        except sqlite3.Error as exc:
            errors.append(f"{store}: {exc}")
        finally:
            if connection is not None:
                connection.close()
    return output, {
        "status": "ok" if stores else "missing",
        "records": len(output),
        "files_read": len(stores),
        "errors": errors,
    }


def deduplicate(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    positions: dict[tuple[Any, ...], int] = {}
    result: list[dict[str, Any]] = []
    for item in sorted(records, key=lambda row: (row.get("timestamp") or "", row["session_id"], row["role"])):
        key = (item["source"], item["session_id"], item["role"], item["text"])
        if key in positions:
            previous = result[positions[key]]
            if previous.get("cwd") is None and item.get("cwd") is not None:
                result[positions[key]] = item
            continue
        positions[key] = len(result)
        result.append(item)
    return result


def atomic_write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            handle.write(content)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    args = parse_args()
    cutoff = None if args.all else time.time() - args.days * 86_400
    extractors = {
        "codex": (codex_records, args.codex_root),
        "claude": (claude_records, args.claude_root),
    }
    summary: dict[str, Any] = {
        "window": "all" if args.all else {"days": args.days},
        "generated_at": iso_timestamp(time.time()),
        "sources": {},
    }
    args.output.mkdir(parents=True, exist_ok=True)
    for source, (extractor, root) in extractors.items():
        records, details = extractor(root, cutoff, args.max_chars)
        records = deduplicate(records)
        details["records"] = len(records)
        details["root"] = str(root)
        summary["sources"][source] = details
        body = "".join(json.dumps(item, ensure_ascii=False) + "\n" for item in records)
        atomic_write(args.output / f"{source}.jsonl", body)
    composer_records, composer_details = cursor_records(args.cursor_root, cutoff, args.max_chars)
    agent_records, agent_details = cursor_agent_records(args.cursor_agent_root, cutoff, args.max_chars)
    composer_details["records"] = len(composer_records)
    cursor_output = deduplicate(composer_records + agent_records)
    cursor_statuses = {composer_details["status"], agent_details["status"]}
    cursor_status = "ok" if "ok" in cursor_statuses else (
        "unrecognized_schema" if "unrecognized_schema" in cursor_statuses else (
            "error" if "error" in cursor_statuses else "missing"
        )
    )
    summary["sources"]["cursor"] = {
        "status": cursor_status,
        "records": len(cursor_output),
        "files_read": composer_details["files_read"] + agent_details["files_read"],
        "errors": composer_details["errors"] + agent_details["errors"],
        "root": str(args.cursor_root),
        "agent_root": str(args.cursor_agent_root),
        "composer": composer_details,
        "agent_host": agent_details,
    }
    atomic_write(
        args.output / "cursor.jsonl",
        "".join(json.dumps(item, ensure_ascii=False) + "\n" for item in cursor_output),
    )
    atomic_write(args.output / "summary.json", json.dumps(summary, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({source: data["records"] for source, data in summary["sources"].items()}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
