import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

SCRIPT = Path(__file__).parents[1] / "scripts/run_guard.py"
SPEC = importlib.util.spec_from_file_location("run_guard", SCRIPT)
guard = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(guard)


def reading(used, reset=2000000000, source="test", seen=1000, session=None):
    return guard.normalized(
        used, reset, source, seen, session, reset - 600 if session else None
    )


class GuardLogicTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.run = Path(self.tmp.name) / "run"
        self.path = self.run / "guard.json"

    def tearDown(self):
        self.tmp.cleanup()

    def state(self, provider="codex", allowance=10):
        return guard.initial(
            provider, "unattended", allowance, 80, end_at=3000000000, now=1000
        )

    def test_default_allowance_and_no_unlimited_value(self):
        self.assertEqual(guard.load_state(self.path, "codex")["allowance_pct"], 10)
        self.assertEqual(guard.iso_epoch("2000000000"), 2000000000)
        with self.assertRaises(ValueError):
            guard.iso_epoch(None)
        for bad in (0, 101, True, "unlimited"):
            with self.assertRaises(ValueError):
                guard.load_state(self.path, "codex", allowance=bad)

    def test_high_water_does_not_refund_usage(self):
        state = self.state()
        guard.reconcile(state, reading(40), 1000)
        guard.reconcile(state, reading(38), 1010)
        self.assertEqual(guard.effective(state), 40)

    def test_reset_marker_jitter_is_not_a_reset(self):
        state = self.state()
        guard.reconcile(state, reading(40, 2000), 1000)
        guard.reconcile(state, reading(41, 2001), 1100)
        self.assertEqual(state["reset_count"], 0)
        self.assertEqual(state["baseline_pct"], 40)

    def test_scheduled_reset_restarts_even_without_telemetry(self):
        state = self.state("claude")
        guard.reconcile(state, reading(40, 2000, session=80), 1000)
        state["last_real"]["session_reset_at"] = 2500
        state["inflight"]["cross-reset"] = {"size": "small"}
        guard.reconcile(state, None, 2000)
        self.assertEqual(state["reset_count"], 1)
        self.assertEqual(state["baseline_pct"], 0)
        self.assertEqual(state["reset_at"], 2000 + guard.WEEK)
        self.assertIn("cross-reset", state["inflight"])
        self.assertTrue(guard.decision(state, 2000)["browser_due"])
        self.assertEqual(state["last_real"]["session_pct"], 80)

    def test_allowance_pause_resumes_after_reset_without_extending_time(self):
        state = self.state()
        state["end_at"] = 5000
        guard.reconcile(state, reading(40, reset=2000), 1000)
        guard.reconcile(state, reading(50, reset=2000), 1100)
        self.assertEqual(guard.decision(state, 1100)["reason"], "allowance-consumed")
        guard.reconcile(state, None, 2000)
        state["last_browser_attempt_at"] = 2000
        self.assertEqual(guard.decision(state, 2000)["decision"], "allow")
        self.assertEqual(state["end_at"], 5000)

    def test_account_consumption_pauses_dispatch(self):
        state = self.state(allowance=10)
        guard.reconcile(state, reading(40), 1000)
        guard.reconcile(state, reading(50), 1010)
        self.assertEqual(guard.decision(state, 1010)["reason"], "allowance-consumed")

    def test_time_window_end_has_priority_over_browser_retry(self):
        state = self.state()
        guard.reconcile(state, None, 1000)
        state["end_at"] = 1010
        result = guard.decision(state, 1010)
        self.assertEqual(result["decision"], "pause")
        self.assertEqual(result["reason"], "time-window-ended")

    def test_zero_delta_is_not_a_training_sample(self):
        state = self.state()
        state["inflight"]["t"] = {
            "size": "small",
            "model_tier": "cheap",
            "before_used": 40,
            "before_reset": 2000000000,
        }
        args = type("Args", (), {"provider": "codex", "command": "finish", "task": "t"})
        with patch.object(guard, "probe", return_value=reading(40)):
            guard.run_once(args, state, self.path, 1010)
        self.assertEqual(state["samples"], {})

    def test_recent_three_samples_use_median(self):
        state = self.state()
        state["samples"]["cheap:medium"] = [1, 9, 2, 3][-3:]
        self.assertEqual(guard.estimate_cost(state, "cheap", "medium"), 3)
        self.assertEqual(guard.estimate_cost(state, "new", "large"), 4)

    def test_finish_in_estimate_mode_accumulates(self):
        state = self.state()
        guard.reconcile(state, reading(40), 1000)
        state["inflight"]["t"] = {
            "size": "medium",
            "model_tier": "cheap",
            "before_used": 40,
            "before_reset": 2000000000,
        }
        args = type("Args", (), {"provider": "codex", "command": "finish", "task": "t"})
        with patch.object(guard, "probe", return_value=None):
            result = guard.run_once(args, state, self.path, 1010)
        self.assertEqual(result["used_pct"], 42)
        self.assertEqual(state["estimated_extra_pct"], 2)

    def test_stale_runtime_does_not_erase_newer_estimate(self):
        state = self.state()
        guard.reconcile(state, reading(40), 1000)
        state.update({"estimated_extra_pct": 2, "estimated_at": 1200})
        guard.reconcile(state, reading(40, seen=1100), 1300)
        self.assertEqual(state["estimated_extra_pct"], 2)
        guard.reconcile(state, reading(41, seen=1300), 1300)
        self.assertEqual(state["estimated_extra_pct"], 0)

    def test_browser_observation_preserves_live_claude_session_pause(self):
        state = self.state("claude")
        guard.reconcile(state, reading(20, reset=3000, session=80), 1000)
        args = SimpleNamespace(
            provider="claude", command="observe", used=21, reset_at="3000"
        )
        with patch.object(guard, "probe", return_value=None):
            result = guard.run_once(args, state, self.path, 1100)
        self.assertEqual(result["reason"], "session-limit")

    def test_browser_due_prevents_begin_until_attempted(self):
        state = self.state()
        args = SimpleNamespace(
            provider="codex",
            command="begin",
            task="t",
            size="small",
            model_tier="cheap",
        )
        with patch.object(guard, "probe", return_value=None):
            result = guard.run_once(args, state, self.path, 1000)
        self.assertEqual(result["reason"], "browser-check-due")
        self.assertNotIn("t", state["inflight"])

    def test_wait_returns_at_time_boundary(self):
        state = self.state()
        state["end_at"] = 1001
        guard.reconcile(state, reading(20), 1000)
        guard.atomic_write(self.path, state)
        args = SimpleNamespace(
            provider="codex",
            profile="unattended",
            command="wait",
            run_dir=self.run,
            allowance=None,
            session_stop=80,
            end_at=None,
            task=None,
            size=None,
            model_tier=None,
            used=None,
            reset_at=None,
        )
        fake_parser = SimpleNamespace(parse_args=lambda: args)
        with (
            patch.object(guard, "parser", return_value=fake_parser),
            patch.object(guard, "probe", return_value=reading(20)),
            patch.object(guard.time, "time", return_value=1001),
            patch.object(
                guard.time,
                "sleep",
                side_effect=AssertionError("wait slept after time boundary"),
            ),
            patch("builtins.print"),
            self.assertRaises(SystemExit) as ended,
        ):
            guard.main()
        self.assertEqual(ended.exception.code, 10)

    def test_codex_runtime_uses_latest_snapshot_across_sessions(self):
        home = Path(self.tmp.name) / "codex"
        sessions = home / "sessions"
        sessions.mkdir(parents=True)
        for name, seen, used in (("a.jsonl", 995, 20), ("b.jsonl", 999, 24)):
            row = {
                "timestamp": f"1970-01-01T00:16:{seen - 960:02d}Z",
                "payload": {
                    "rate_limits": {
                        "primary": {
                            "used_percent": used,
                            "window_minutes": 10080,
                            "resets_at": 2000000000,
                        }
                    }
                },
            }
            path = sessions / name
            path.write_text(json.dumps(row) + "\n")
            os.utime(path, (1000, 1000))
        self.assertEqual(guard.codex_runtime(home, 1000)["used_pct"], 24)

    def test_codex_http_fixture_requires_seven_day_window(self):
        home = Path(self.tmp.name) / "codex-http"
        home.mkdir()
        (home / "auth.json").write_text(
            json.dumps({"tokens": {"access_token": "test", "account_id": "test"}})
        )
        fixture = Path(self.tmp.name) / "usage.json"
        payload = {
            "rate_limit": {
                "primary_window": {
                    "used_percent": 31,
                    "limit_window_seconds": guard.WEEK,
                    "reset_at": 2000000000,
                }
            }
        }
        fixture.write_text(json.dumps(payload))
        with patch.dict(os.environ, {"UNATTENDED_CODEX_FIXTURE": str(fixture)}):
            self.assertEqual(guard.codex_http(home)["used_pct"], 31)
            payload["rate_limit"]["primary_window"]["limit_window_seconds"] = 18000
            fixture.write_text(json.dumps(payload))
            self.assertIsNone(guard.codex_http(home))


if __name__ == "__main__":
    unittest.main()
