import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import measure_usage as measure


class MeasurementTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.log = Path(self.tmp.name) / "log.jsonl"
        self.state = str(Path(self.tmp.name) / "state.json")
        self.log.write_text(json.dumps({"type": "session_meta", "payload": {"id": "test-session"}}) + "\n")

    def append(self, input=100, cached=20, output=10, **extra):
        usage = {"input_tokens": input, "cached_input_tokens": cached, "output_tokens": output, **extra}
        with self.log.open("a") as stream:
            stream.write(json.dumps({"type": "event_msg", "timestamp": "2026-10-03T00:00:00Z", "payload": {"type": "token_count", "info": {"total_token_usage": usage}}}) + "\n")

    def test_latest_snapshot_difference_and_cache_separation(self):
        self.append()
        self.append(input=200, cached=40, output=20)
        measure.start(self.state, self.log)
        self.append(input=250, cached=60, output=30, reasoning_output_tokens=9)
        self.append(input=300, cached=70, output=40, reasoning_output_tokens=12)
        result = measure.finish(self.state)
        self.assertEqual(result["tokens"], {"input": 100, "cached_input": 30, "uncached_input": 70, "output": 20})
        self.assertEqual(result["tokens_through"], "2026-10-03T00:00:00Z")

    def test_reset_guard(self):
        self.append()
        measure.start(self.state, self.log)
        self.append(input=20, cached=0, output=0)
        result = measure.finish(self.state)
        self.assertIsNone(result["tokens"])
        self.assertIn("decreased", result["unavailable_reason"])

    def test_reset_followed_by_recovery_is_unavailable(self):
        self.append()
        measure.start(self.state, self.log)
        self.append(input=20, cached=0, output=0)
        self.append(input=200, cached=40, output=20)
        self.assertIsNone(measure.finish(self.state)["tokens"])

    def test_missing_log_preserves_timing(self):
        measure.start(self.state, Path(self.tmp.name) / "missing.jsonl")
        result = measure.finish(self.state)
        self.assertIsNone(result["tokens"])
        self.assertGreaterEqual(result["elapsed_seconds"], 0)

    def test_incomplete_counters_preserve_timing(self):
        self.append(cached=None)
        measure.start(self.state, self.log)
        result = measure.finish(self.state)
        self.assertIsNone(result["tokens"])
        self.assertGreaterEqual(result["elapsed_seconds"], 0)

    def test_refuses_existing_state(self):
        measure.start(self.state, self.log)
        with self.assertRaises(FileExistsError):
            measure.start(self.state, self.log)

    def test_replaced_session_is_unavailable(self):
        self.append()
        measure.start(self.state, self.log)
        self.log.write_text(json.dumps({"type": "session_meta", "payload": {"id": "other"}}) + "\n")
        self.append()
        self.assertIn("identity changed", measure.finish(self.state)["unavailable_reason"])

    def test_discovery_requires_metadata_match(self):
        with patch.dict("os.environ", {"CODEX_THREAD_ID": "expected"}), patch.object(measure.glob, "glob", return_value=[str(self.log)]):
            self.assertIsNone(measure.discover()[0])
        with patch.dict("os.environ", {"CODEX_THREAD_ID": "test-session"}), patch.object(measure.glob, "glob", return_value=[str(self.log)]):
            self.assertEqual(measure.discover()[0], str(self.log))

    def test_cache_write_reported_separately(self):
        self.append(cache_write_input_tokens=2)
        measure.start(self.state, self.log)
        self.append(input=120, cached=25, output=15, cache_write_input_tokens=7)
        self.assertEqual(measure.finish(self.state)["tokens"]["cache_write"], 5)

    def claude(self, message_id="one", output=5, session="claude-session", sidechain=False, **extra):
        usage = {"input_tokens": 2, "cache_read_input_tokens": 10, "cache_creation_input_tokens": 20, "output_tokens": output, **extra}
        row = {"type": "assistant", "sessionId": session, "isSidechain": sidechain,
               "message": {"id": message_id, "usage": usage}, "timestamp": "usage-time"}
        with self.log.open("a") as stream:
            stream.write(json.dumps(row) + "\n")

    def test_claude_streaming_dedup_and_boundary_growth(self):
        self.log.write_text('{"type":"queue-operation"}\n')
        self.claude()
        self.claude()
        measure.start(self.state, self.log)
        self.claude(output=8, output_tokens_details={"thinking_tokens": 3}, iterations=[{"output_tokens": 999}])
        self.claude(message_id="two", output=7)
        self.claude(message_id="two", output=7)
        self.claude(message_id="child", session="child-session", sidechain=True)
        result = measure.finish(self.state)
        self.assertEqual(result["runtime"], "Claude")
        self.assertEqual(result["tokens"], {"input": 32, "cached_input": 10, "uncached_input": 22, "cache_write": 20, "output": 10})

    def test_claude_mixed_session_rejected(self):
        self.log.write_text("")
        self.claude()
        self.claude(message_id="two", session="other")
        measure.start(self.state, self.log)
        self.assertIn("Mixed", measure.finish(self.state)["unavailable_reason"])

    def test_claude_incomplete_snapshot_rejected(self):
        self.log.write_text("")
        self.claude()
        measure.start(self.state, self.log)
        self.claude(cache_creation_input_tokens=None)
        result = measure.finish(self.state)
        self.assertIsNone(result["tokens"])
        self.assertGreaterEqual(result["elapsed_seconds"], 0)

    def test_claude_counter_decrease_rejected(self):
        self.log.write_text("")
        self.claude()
        measure.start(self.state, self.log)
        self.claude(output=1)
        self.assertIsNone(measure.finish(self.state)["tokens"])


if __name__ == "__main__":
    unittest.main()
