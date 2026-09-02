import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "scripts/run_guard.py"


class RunGuardCliTest(unittest.TestCase):
    def run_guard(self, run_dir, profile, end_at=None, allowance=None):
        command = [
                sys.executable,
                str(SCRIPT),
                "check",
                "--provider",
                "codex",
                "--profile",
                profile,
                "--run-dir",
                str(run_dir),
            ]
        if end_at is not None:
            command.extend(("--end-at", end_at))
        if allowance is not None:
            command.extend(("--allowance", str(allowance)))
        return subprocess.run(
            command,
            env={**os.environ, "CODEX_HOME": str(run_dir.parent / "codex")},
            capture_output=True,
            text=True,
            check=False,
        )

    def test_unattended_defaults_to_two_hours(self):
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp) / "run"
            started = int(time.time())
            result = self.run_guard(run_dir, "unattended")
            self.assertIn(result.returncode, (0, 10), result.stderr or result.stdout)
            state = json.loads((run_dir / "guard.json").read_text())
            self.assertEqual(state["profile"], "unattended")
            self.assertGreaterEqual(state["end_at"], started + 7195)
            self.assertLessEqual(state["end_at"], started + 7205)

    def test_overnight_defaults_to_eight_hours(self):
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp) / "run"
            started = int(time.time())
            result = self.run_guard(run_dir, "overnight")
            self.assertIn(result.returncode, (0, 10), result.stderr or result.stdout)
            state = json.loads((run_dir / "guard.json").read_text())
            self.assertEqual(state["profile"], "overnight")
            self.assertGreaterEqual(state["end_at"], started + 28795)
            self.assertLessEqual(state["end_at"], started + 28805)

    def test_existing_run_rejects_profile_change(self):
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp) / "run"
            self.run_guard(run_dir, "unattended")
            result = self.run_guard(run_dir, "overnight")
            self.assertEqual(result.returncode, 2)
            self.assertEqual(json.loads(result.stdout)["reason"], "profile-mismatch")

    def test_existing_run_rejects_end_time_change(self):
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp) / "run"
            first = "2090-01-01T01:00:00+00:00"
            second = "2090-01-01T02:00:00+00:00"
            self.run_guard(run_dir, "unattended", first)
            result = self.run_guard(run_dir, "unattended", second)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(json.loads(result.stdout)["reason"], "end-at-mismatch")

    def test_existing_run_rejects_allowance_change(self):
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp) / "run"
            self.run_guard(run_dir, "unattended", allowance=10)
            result = self.run_guard(run_dir, "unattended", allowance=11)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(json.loads(result.stdout)["reason"], "allowance-mismatch")

    def test_expired_initial_end_time_is_invalid_configuration(self):
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp) / "run"
            result = self.run_guard(
                run_dir, "unattended", "2000-01-01T00:00:00+00:00"
            )
            self.assertEqual(result.returncode, 2)
            self.assertEqual(
                json.loads(result.stdout)["reason"], "end-at-must-be-in-the-future"
            )
            self.assertFalse((run_dir / "guard.json").exists())


if __name__ == "__main__":
    unittest.main()
