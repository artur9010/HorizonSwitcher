#!/usr/bin/env python3
"""Raises the mod's patch version everywhere it is written, and prints the new one.

    python3 tools/bump_version.py            0.2.0 -> 0.2.1
    python3 tools/bump_version.py 0.3.0      sets it

horizon-mod.toml (the version the launcher and Options > Mods show) and the mod crate's
Cargo.toml must agree: tools/package.py and the release refuse otherwise.
"""
import re
import sys
from pathlib import Path

MOD = Path(__file__).resolve().parents[1] / 'horizon_switcher'
FILES = {
    MOD / 'horizon-mod.toml': r'(?m)^(version = ")([^"]+)(")',
    MOD / 'Cargo.toml': r'(?m)^(version = ")([^"]+)(")',
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
