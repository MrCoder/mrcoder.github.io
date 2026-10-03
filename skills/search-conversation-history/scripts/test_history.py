#!/usr/bin/env python3
import contextlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import history

class HistoryLauncherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.database = self.root/'private/index.sqlite'

    def execute(self, *args):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            return history.run(['--database', str(self.database), *args])

    def test_missing_index_does_not_build_or_return_no_matches(self):
        with patch.object(history, 'native_binary') as build:
            with self.assertRaisesRegex(RuntimeError, 'index is missing'):
                self.execute('search', 'example')
            build.assert_not_called()
        self.assertFalse(self.database.exists())

    def test_index_is_private_and_never_writes_client_configs(self):
        config = self.root/'config.toml'; config.write_text('unchanged')
        result = history.subprocess.CompletedProcess([], 0, json.dumps({'state':'done','sources':[]}), '')
        with patch.object(history, 'native_binary', return_value=Path('/fake/vcs')), patch.object(history.subprocess, 'run', return_value=result) as execute:
            self.assertEqual(self.execute('index'), 0)
        self.assertEqual(config.read_text(), 'unchanged')
        self.assertEqual(execute.call_args.args[0], ['/fake/vcs','--database',str(self.database),'index'])
        if os.name == 'posix':
            self.assertEqual(self.database.stat().st_mode & 0o777, 0o600)
            self.assertEqual(self.database.parent.stat().st_mode & 0o777, 0o700)

    def test_failed_source_with_zero_exit_is_failure(self):
        result = history.subprocess.CompletedProcess([], 0, json.dumps({'state':'done','sources':[{'state':'failed'}]}), '')
        with patch.object(history, 'native_binary', return_value=Path('/fake/vcs')), patch.object(history.subprocess, 'run', return_value=result):
            with self.assertRaisesRegex(RuntimeError, 'refresh failed'):
                self.execute('index')

    def test_partial_parse_errors_are_disclosed(self):
        result = history.subprocess.CompletedProcess([], 0, json.dumps({'state':'done','sources':[{'state':'done','errors':['bad record']}]}), '')
        output = io.StringIO()
        with patch.object(history, 'native_binary', return_value=Path('/fake/vcs')), patch.object(history.subprocess, 'run', return_value=result), contextlib.redirect_stderr(output), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(history.run(['--database',str(self.database),'index']), 0)
        self.assertIn('could not be indexed', output.getvalue())

    def test_native_errors_propagate(self):
        result = history.subprocess.CompletedProcess([], 7, '', 'failure')
        with patch.object(history, 'native_binary', return_value=Path('/fake/vcs')), patch.object(history.subprocess, 'run', return_value=result):
            self.assertEqual(self.execute('index'), 7)

    def test_search_filters_pass_through_without_refresh(self):
        history.prepare_index(self.database)
        result = history.subprocess.CompletedProcess([], 0)
        flags = ['search','example','--source','claude','--offset','20','--since','2026-01-01','--limit','20']
        with patch.object(history, 'native_binary', return_value=Path('/fake/vcs')), patch.object(history.subprocess, 'run', return_value=result) as execute:
            self.assertEqual(self.execute(*flags), 0)
        self.assertEqual(execute.call_args.args[0][3:], flags)
        execute.assert_called_once()

    def test_corrupted_bundled_binary_is_rejected(self):
        server = self.root/'server'; (server/'bin/darwin-arm64').mkdir(parents=True)
        (server/'bin/darwin-arm64/vcs').write_bytes(b'bad')
        (server/'provenance.json').write_text(json.dumps({'binaries':{'darwin-arm64':{'path':'bin/darwin-arm64/vcs','sha256':'wrong'}}}))
        with patch.object(history, 'ROOT', self.root), patch.object(history.platform, 'system', return_value='Darwin'), patch.object(history.platform, 'machine', return_value='arm64'):
            with self.assertRaisesRegex(RuntimeError, 'checksum mismatch'):
                history.native_binary()

    def test_missing_cargo_is_actionable(self):
        with patch.object(history.platform, 'system', return_value='Linux'), patch.object(history.shutil, 'which', return_value=None):
            with self.assertRaisesRegex(RuntimeError, 'Install Rust/Cargo'):
                history.native_binary()

if __name__ == '__main__':
    unittest.main()
