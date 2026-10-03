#!/usr/bin/env python3
"""Build the public no-bs skill ZIP from an explicit, deterministic allowlist."""
import hashlib
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FILES = (
    ('no-bs/SKILL.md', ROOT / 'skills' / 'no-bs' / 'SKILL.md'),
    ('renhua/SKILL.md', ROOT / 'skills' / 'renhua' / 'SKILL.md'),
)

# Read every required file before creating output: missing assets fail the build.
assets = [(name, path.read_bytes()) for name, path in FILES]
out = ROOT / 'downloads' / 'no-bs.zip'
out.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(out, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
    for name, data in assets:
        info = zipfile.ZipInfo(name, date_time=(2020, 1, 1, 0, 0, 0))
        info.create_system = 3
        info.external_attr = 0o100644 << 16
        info.compress_type = zipfile.ZIP_DEFLATED
        archive.writestr(info, data, compresslevel=9)

digest = hashlib.sha256(out.read_bytes()).hexdigest()
(out.parent / (out.name + '.sha256')).write_text(digest + '  ' + out.name + '\n')
print(digest, out.name, out.stat().st_size)
