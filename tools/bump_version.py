#!/usr/bin/env python3
"""Raises the mod's patch version everywhere it is written, and prints the new one.

    python3 tools/bump_version.py            0.1.0 -> 0.1.1
    python3 tools/bump_version.py 0.2.0      sets it

horizon-mixin.toml ([mod] version), Cargo.toml and Cargo.lock (the package), and switcher.rs (VERSION,
shown in Options > Mods) must agree: tools/package.py and the release refuse otherwise.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILES = {
    ROOT / 'mixins/horizon_switcher/horizon-mixin.toml': r'(?m)^(version = ")([^"]+)(")',
    ROOT / 'Cargo.toml': r'(?m)^(version = ")([^"]+)(")',
    ROOT / 'Cargo.lock': r'(name = "horizon-switcher"\nversion = ")([^"]+)(")',
    ROOT / 'mixins/horizon_switcher/switcher.rs': r'(pub const VERSION: &str = ")([^"]+)(")',
}


def main():
    current = set()
    for path, rx in FILES.items():
        m = re.search(rx, path.read_text())
        if not m:
            raise SystemExit(f'no version in {path}')
        current.add(m.group(2))
    if len(current) != 1:
        raise SystemExit(f'the versions differ already: {sorted(current)}')
    (old,) = current
    if len(sys.argv) > 1:
        new = sys.argv[1]
    else:
        major, minor, patch = map(int, old.split('.'))
        new = f'{major}.{minor}.{patch + 1}'
    if not re.fullmatch(r'\d+\.\d+\.\d+', new):
        raise SystemExit(f'not a version: {new}')
    for path, rx in FILES.items():
        text, n = re.subn(rx, lambda m: m.group(1) + new + m.group(3), path.read_text(), count=1)
        if n != 1:
            raise SystemExit(f'no version in {path}')
        path.write_text(text)
    print(new)


if __name__ == '__main__':
    main()
