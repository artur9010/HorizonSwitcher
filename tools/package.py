#!/usr/bin/env python3
"""Packs the mod as its `.hmod`, the source package the launcher builds.

The archive is horizon_switcher/ flat, as the launcher packs a project (its "Add from GitHub" does
the same from the source): `horizon-mod.toml` first, then `Cargo.toml`, `Cargo.lock` when there is
one, and the `.rs` and `.toml` files under `src/`. Fixed dates and modes, so the same sources
always give the same bytes and SHA-256.
"""
import hashlib
import json
import tomllib
import zipfile

import horizon_check as hc


def main():
    mod = hc.manifest()
    problems = hc.static_problems(mod)
    cargo = tomllib.loads((hc.MOD / 'Cargo.toml').read_text())['package']['version']
    if cargo != mod['version']:
        problems.append(f"versions differ: horizon-mod.toml {mod['version']}, Cargo.toml {cargo}")
    if problems:
        raise SystemExit('\n'.join(problems))
    names = ['Cargo.toml'] + (['Cargo.lock'] if (hc.MOD / 'Cargo.lock').is_file() else [])
    names += sorted(p.relative_to(hc.MOD).as_posix() for p in (hc.MOD / 'src').rglob('*')
                    if p.is_file() and p.suffix in ('.rs', '.toml'))
    out = hc.ROOT / 'target' / f"{mod['id']}-{mod['version']}.hmod"
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, 'w') as archive:
        for name in ['horizon-mod.toml', *names]:
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, (hc.MOD / name).read_bytes())
    print(json.dumps(dict(file=str(out), size=out.stat().st_size,
                          sha256=hashlib.sha256(out.read_bytes()).hexdigest()), indent=2))


if __name__ == '__main__':
    main()
