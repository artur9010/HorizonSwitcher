#!/usr/bin/env python3
"""Packs the mod as a source-only `.hnmod` (Horizon's native mod file).

The archive holds `horizon-native.json` and `mixins/horizon_switcher/` (`.rs` and `.toml` only),
with fixed dates and modes, so the same sources always give the same bytes and SHA-256.
The launcher compiles it on the pinned Horizon revision.
"""
import hashlib
import json
import re
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GAME = '26.3'


def main():
    compat = json.loads((ROOT / 'compatibility.json').read_text())
    bundle = ROOT / 'mixins' / 'horizon_switcher'
    mixin = tomllib.loads((bundle / 'horizon-mixin.toml').read_text())
    mod = mixin['mod']
    if bundle.name != mod['id']:
        raise SystemExit(f"mixins/{bundle.name}/ must be named after the mod id {mod['id']}")
    if mixin.get('compatibility', {}).get('horizon_revision') != compat['horizon_revision']:
        raise SystemExit('horizon-mixin.toml and compatibility.json pin different Horizon revisions')
    cargo = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    shown = re.search(r'pub const VERSION: &str = "([^"]+)";', (bundle / 'switcher.rs').read_text())
    if not shown or not (shown.group(1) == mod['version'] == cargo):
        raise SystemExit(f"versions differ: horizon-mixin.toml {mod['version']}, Cargo.toml {cargo}, "
                         f"switcher.rs {shown and shown.group(1)}")
    manifest = dict(schema=1, id=mod['id'], version=mod['version'], game=GAME,
                    horizon=compat['horizon_revision'], mixins=compat['horizon_mixins_revision'])
    files = {'horizon-native.json': (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()}
    for path in sorted(bundle.iterdir()):
        if path.is_file() and path.suffix in ('.rs', '.toml'):
            files[path.relative_to(ROOT).as_posix()] = path.read_bytes()
    out = ROOT / 'target' / f"{mod['id']}-{mod['version']}.hnmod"
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, 'w') as archive:
        for name, content in sorted(files.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content)
    print(json.dumps(dict(file=str(out), size=out.stat().st_size,
                          sha256=hashlib.sha256(out.read_bytes()).hexdigest()), indent=2))


if __name__ == '__main__':
    main()
