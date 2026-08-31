#!/usr/bin/env python3
"""Fixture tests for extract_history.py."""

from __future__ import annotations

import json
import hashlib
import sqlite3
import subprocess
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path


SCRIPT = Path(__file__).with_name("extract_history.py")


def write_jsonl(path: Path, values: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(json.dumps(value) + "\n" for value in values), encoding="utf-8")


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


class ExtractHistoryTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.codex = self.root / "codex"
        self.claude = self.root / "claude"
        self.cursor = self.root / "cursor"
        self.output = self.root / "output"
        self.recent = datetime.now(timezone.utc) - timedelta(days=2)
        self.old = datetime.now(timezone.utc) - timedelta(days=60)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def run_extract(self, *window: str) -> subprocess.CompletedProcess[str]:
        args = [
            "python3",
            str(SCRIPT),
            *window,
            "--output",
            str(self.output),
            "--codex-root",
            str(self.codex),
            "--claude-root",
            str(self.claude),
            "--cursor-root",
            str(self.cursor),
            "--cursor-agent-root",
            str(self.cursor / "agent-chats"),
        ]
        return subprocess.run(args, check=True, capture_output=True, text=True)

    def seed_codex(self) -> None:
        path = self.codex / "sessions/2026/08/fixture.jsonl"
        values = [
            {"type": "session_meta", "payload": {"id": "codex-session", "cwd": "/project"}},
            {
                "type": "event_msg",
                "timestamp": self.recent.isoformat(),
                "payload": {"type": "user_message", "message": "recent codex question"},
            },
            {
                "type": "response_item",
                "timestamp": self.recent.isoformat(),
                "payload": {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "recent codex answer"}]},
            },
            {
                "type": "response_item",
                "timestamp": self.recent.isoformat(),
                "payload": {"type": "function_call_output", "output": "tool echo"},
            },
            {
                "type": "event_msg",
                "timestamp": self.old.isoformat(),
                "payload": {"type": "user_message", "message": "old codex question"},
            },
        ]
        write_jsonl(path, values)
        with path.open("a", encoding="utf-8") as handle:
            handle.write("not-json\n")

    def seed_claude(self) -> None:
        write_jsonl(
            self.claude / "projects/project/session.jsonl",
            [
                {
                    "type": "user",
                    "timestamp": self.recent.isoformat(),
                    "sessionId": "claude-session",
                    "cwd": "/project",
                    "origin": {"kind": "human"},
                    "promptSource": "typed",
                    "message": {"role": "user", "content": "recent claude question"},
                },
                {
                    "type": "assistant",
                    "timestamp": self.recent.isoformat(),
                    "sessionId": "claude-session",
                    "cwd": "/project",
                    "message": {"role": "assistant", "content": [{"type": "text", "text": "recent claude answer"}]},
                },
                {
                    "type": "user",
                    "timestamp": self.recent.isoformat(),
                    "sessionId": "claude-session",
                    "promptSource": "sdk",
                    "entrypoint": "sdk-cli",
                    "message": {"role": "user", "content": "hook prompt"},
                },
                {
                    "type": "user",
                    "timestamp": self.recent.isoformat(),
                    "sessionId": "claude-session",
                    "message": {"role": "user", "content": [{"type": "tool_result", "content": "tool echo"}]},
                },
            ],
        )

    def seed_cursor(self, recognized: bool = True) -> None:
        db = self.cursor / "User/globalStorage/state.vscdb"
        db.parent.mkdir(parents=True, exist_ok=True)
        connection = sqlite3.connect(db)
        connection.execute("CREATE TABLE ItemTable (key TEXT, value BLOB)")
        if recognized:
            connection.execute("CREATE TABLE composerHeaders (composerId TEXT PRIMARY KEY, workspaceId TEXT, createdAt INTEGER, lastUpdatedAt INTEGER, isArchived INTEGER, isSubagent INTEGER, recency INTEGER, checkpointAt INTEGER, value TEXT)")
            connection.execute("CREATE TABLE cursorDiskKV (key TEXT UNIQUE, value BLOB)")
            recent_ms = int(self.recent.timestamp() * 1000)
            header = {"workspaceIdentifier": {"uri": {"fsPath": "/project"}}}
            connection.execute("INSERT INTO composerHeaders VALUES (?,?,?,?,?,?,?,?,?)", ("cursor-session", "workspace", recent_ms, recent_ms, 0, 0, 0, 0, json.dumps(header)))
            bubbles = [
                ("user", {"type": 1, "createdAt": self.recent.isoformat(), "text": "recent cursor question"}),
                ("assistant", {"type": 2, "createdAt": self.recent.isoformat(), "text": "recent cursor answer"}),
                ("empty", {"type": 2, "createdAt": self.recent.isoformat(), "text": ""}),
            ]
            composer = {
                "composerId": "cursor-session",
                "fullConversationHeadersOnly": [
                    {"bubbleId": suffix, "type": bubble["type"], "createdAt": bubble["createdAt"]}
                    for suffix, bubble in bubbles
                ],
            }
            connection.execute("INSERT INTO cursorDiskKV VALUES (?,?)", ("composerData:cursor-session", json.dumps(composer)))
            for suffix, bubble in bubbles:
                connection.execute("INSERT INTO cursorDiskKV VALUES (?,?)", (f"bubbleId:cursor-session:{suffix}", json.dumps(bubble)))
        connection.commit()
        connection.close()

    @staticmethod
    def proto_field(number: int, value: bytes) -> bytes:
        key = (number << 3) | 2
        return bytes([key]) + bytes([len(value)]) + value

    @staticmethod
    def proto_varint_field(number: int, value: int) -> bytes:
        encoded = bytearray()
        while True:
            byte = value & 0x7F
            value >>= 7
            encoded.append(byte | (0x80 if value else 0))
            if not value:
                break
        return bytes([(number << 3) | 0]) + bytes(encoded)

    def seed_cursor_agent(self) -> None:
        store = self.cursor / "agent-chats/project-scope/agent-session/store.db"
        store.parent.mkdir(parents=True, exist_ok=True)
        connection = sqlite3.connect(store)
        connection.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)")
        connection.execute("CREATE TABLE blobs (id TEXT PRIMARY KEY, data BLOB)")

        def put_blob(data: bytes) -> bytes:
            identifier = hashlib.sha256(data).digest()
            connection.execute("INSERT INTO blobs VALUES (?,?)", (identifier.hex(), data))
            return identifier

        user = self.proto_field(1, b"agent user question") + self.proto_field(2, b"message-id")
        assistant = self.proto_field(1, b"agent assistant answer")
        step = self.proto_field(1, assistant)
        user_id = put_blob(user)
        step_id = put_blob(step)
        agent_turn = self.proto_field(1, user_id) + self.proto_field(2, step_id)
        turn = self.proto_field(1, agent_turn)
        turn_id = put_blob(turn)
        timing = self.proto_varint_field(2, int(self.recent.timestamp() * 1000))
        root = self.proto_field(8, turn_id) + self.proto_field(14, timing)
        root_id = put_blob(root).hex()
        metadata = {
            "agentId": "agent-session",
            "latestRootBlobId": root_id,
            "createdAt": int(self.recent.timestamp() * 1000),
        }
        connection.execute("INSERT INTO meta VALUES ('0',?)", (json.dumps(metadata).encode().hex(),))
        connection.commit()
        connection.close()

    def test_recent_extraction_filters_noise_and_old_records(self) -> None:
        self.seed_codex()
        self.seed_claude()
        self.seed_cursor()
        self.seed_cursor_agent()
        self.run_extract("--days", "30")
        self.assertEqual([row["text"] for row in read_jsonl(self.output / "codex.jsonl")], ["recent codex answer", "recent codex question"])
        self.assertEqual({row["text"] for row in read_jsonl(self.output / "claude.jsonl")}, {"recent claude question", "recent claude answer"})
        self.assertEqual(
            {row["text"] for row in read_jsonl(self.output / "cursor.jsonl")},
            {"recent cursor question", "recent cursor answer", "agent user question", "agent assistant answer"},
        )
        summary = json.loads((self.output / "summary.json").read_text())
        self.assertEqual(summary["sources"]["cursor"]["status"], "ok")
        self.assertTrue(summary["sources"]["codex"]["errors"])

    def test_all_includes_old_records_and_deduplicates(self) -> None:
        self.seed_codex()
        self.run_extract("--all")
        texts = [row["text"] for row in read_jsonl(self.output / "codex.jsonl")]
        self.assertIn("old codex question", texts)
        self.assertEqual(texts.count("recent codex question"), 1)

    def test_unrecognized_cursor_schema_fails_closed(self) -> None:
        self.seed_cursor(recognized=False)
        self.run_extract("--days", "30")
        summary = json.loads((self.output / "summary.json").read_text())
        self.assertEqual(summary["sources"]["cursor"]["status"], "unrecognized_schema")
        self.assertEqual(read_jsonl(self.output / "cursor.jsonl"), [])

    def test_missing_sources_are_reported(self) -> None:
        self.run_extract("--days", "30")
        summary = json.loads((self.output / "summary.json").read_text())
        self.assertEqual(summary["sources"]["codex"]["status"], "missing")
        self.assertEqual(summary["sources"]["claude"]["status"], "missing")
        self.assertEqual(summary["sources"]["cursor"]["status"], "missing")


if __name__ == "__main__":
    unittest.main()
