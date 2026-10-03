#!/usr/bin/env python3
"""Run the bundled local history CLI. No MCP or client configuration required."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]

def native_binary():
    server = ROOT / 'server'
    if platform.system() == 'Darwin' and platform.machine() == 'arm64':
        asset = json.loads((server / 'provenance.json').read_text())['binaries']['darwin-arm64']
        binary = server / asset['path']
        if hashlib.sha256(binary.read_bytes()).hexdigest() != asset['sha256']:
            raise RuntimeError('Bundled native binary checksum mismatch; download the package again.')
        binary.chmod(binary.stat().st_mode | 0o111)
        return binary.resolve()
    cargo = shutil.which('cargo')
    if not cargo:
        raise RuntimeError('No native binary for this platform. Install Rust/Cargo, then run this installer again.')
    cache = Path.home() / '.cache/conversation-history/native'
    cache.mkdir(parents=True, exist_ok=True)
    cache.chmod(0o700)
    subprocess.run([cargo, 'build', '--release', '--locked', '--manifest-path', str(server / 'Cargo.toml'),
                    '--target-dir', str(cache)], check=True)
    return (cache / 'release' / ('vcs.exe' if os.name == 'nt' else 'vcs')).resolve()



def prepare_index(database):
    database.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    try:
        fd = os.open(database, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        os.close(fd)
    except FileExistsError:
        pass
    if os.name == 'posix':
        database.chmod(0o600)


def run(arguments):
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('--database', type=Path, default=Path.home()/'.local/share/visible-conversation-search/index.sqlite')
    options, command = parser.parse_known_args(arguments)
    database = options.database.expanduser().absolute()
    if not command:
        command = ['--help']
    if command[0] == 'index':
        prepare_index(database)
    elif command[0] not in {'--help', '-h', '--version', '-V', 'detect'} and not database.exists():
        raise RuntimeError('Local index is missing. Run history.py index first.')
    binary = native_binary()
    invocation = [str(binary), '--database', str(database), *command]
    if command[0] != 'index':
        return subprocess.run(invocation).returncode
    result = subprocess.run(invocation, text=True, capture_output=True)
    if result.stderr:
        print(result.stderr, end='', file=sys.stderr)
    if result.stdout:
        print(result.stdout, end='')
    if result.returncode:
        return result.returncode
    report = json.loads(result.stdout)
    sources = report.get('sources', report.get('reports', []))
    if report.get('state') != 'done' or any(r.get('state') in {'failed', 'error'} for r in sources):
        raise RuntimeError('Index refresh failed; do not treat stale results as complete.')
    if any(r.get('errors') for r in sources):
        print('Some history files could not be indexed; inspect source errors before drawing conclusions.', file=sys.stderr)
    return 0


def main():
    try:
        return run(sys.argv[1:])
    except (RuntimeError, OSError, ValueError, subprocess.SubprocessError) as error:
        print('History CLI failed: ' + str(error), file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
