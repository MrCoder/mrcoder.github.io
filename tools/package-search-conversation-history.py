#!/usr/bin/env python3
"""Build the public skill ZIP from an explicit, deterministic allowlist."""
import hashlib
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parents[1]
NAME = 'search-conversation-history'
FILES = (
    'SKILL.md', 'agents/openai.yaml', 'references/benchmark.md',
    'references/install.md', 'scripts/extract_history.py',
    'scripts/test_extract_history.py', 'scripts/measure_usage.py',
    'scripts/test_measure_usage.py',
)
source = ROOT / 'skills' / NAME
# Read every required file before creating output: missing assets fail the build.
assets = [(name, (source / name).read_bytes()) for name in FILES]
out = ROOT / 'downloads' / (NAME + '.zip')
out.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(out, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
    for name, data in assets:
        info = zipfile.ZipInfo(NAME + '/' + name, date_time=(2020, 1, 1, 0, 0, 0))
        info.create_system = 3
        info.external_attr = 0o100644 << 16
        info.compress_type = zipfile.ZIP_DEFLATED
        archive.writestr(info, data, compresslevel=9)
digest = hashlib.sha256(out.read_bytes()).hexdigest()
(out.parent / (out.name + '.sha256')).write_text(digest + '  ' + out.name + '\n')
print(digest, out.name, out.stat().st_size)
