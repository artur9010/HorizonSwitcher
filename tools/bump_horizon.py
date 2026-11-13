#!/usr/bin/env python3
"""Moves the mod's Horizon pin to a newer Nightly.

    python3 tools/bump_horizon.py --horizon PATH [--revision SHA | --latest]

PATH is a Horizon git checkout (any branch; the commit is fetched into it if missing). With --latest
the newest `nightly-source/<date>-<sha>` tag of Horizon's repository is used.

For that commit it:
  * checks every named hook (`hook` + `hook_version`) is still in native-hooks.json;
  * checks every raw `target` still names a function, and when the function only moved (another file
    or module, as `Options` did from game.rs to game/launch_options.rs) rewrites the target to where
    it is now;
  * checks the `[[module]]` destination files still exist;
  * reads the launcher's horizon-mixins revision (MIXINS_REV);
  * writes the new pins into horizon-mixin.toml and compatibility.json.

It prints a JSON report. Exit 0: pinned (or already up to date); 1: the mod needs a human (a hook or
function is gone). Nothing is compiled here: tools/verify.sh does that.
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / 'mixins' / 'horizon_switcher'
MANIFEST = BUNDLE / 'horizon-mixin.toml'
COMPAT = ROOT / 'compatibility.json'
REPO = 'https://github.com/horizon-rust/horizon.git'


def git(horizon, *args):
    return subprocess.run(['git', '-C', str(horizon), *args], check=True,
                          capture_output=True, text=True).stdout


def latest_nightly():
    out = subprocess.run(['git', 'ls-remote', '--tags', REPO, 'nightly-source/*'],
                         check=True, capture_output=True, text=True).stdout
    tags, peeled = {}, {}
    for line in out.splitlines():
        sha, ref = line.split('\t')
        if ref.endswith('^{}'):
            peeled[ref.removesuffix('^{}')] = sha  # an annotated tag's commit
            continue
        m = re.fullmatch(r'refs/tags/nightly-source/(\d{4}-\d{2}-\d{2})-([0-9a-f]+)', ref)
        if m:
            tags[ref] = (m.group(1), sha)
    tags = [(date, ref, peeled.get(ref, sha)) for ref, (date, sha) in tags.items()]
    if not tags:
        raise SystemExit('no nightly-source tags found')
    _, ref, sha = max(tags)
    return sha, ref.removeprefix('refs/tags/')


class Source:
    """Files of Horizon at one commit, read with git (nothing is checked out)."""

    def __init__(self, horizon, rev):
        self.horizon, self.rev = horizon, rev
        try:
            git(horizon, 'cat-file', '-e', f'{rev}^{{commit}}')
        except subprocess.CalledProcessError:
            git(horizon, 'fetch', '--quiet', '--depth', '1', 'origin', rev)
        self.files = set(git(horizon, 'ls-tree', '-r', '--name-only', rev).splitlines())
        self._cache = {}

    def read(self, path):
        if path not in self._cache:
            self._cache[path] = git(self.horizon, 'show', f'{self.rev}:{path}') if path in self.files else None
        return self._cache[path]


def crate_dir(crate):
    return f"crates/{crate.replace('_', '-')}/src"


def module_of(path, src):
    """`crates/x/src/game/launch_options.rs` -> ['game', 'launch_options']."""
    rel = path[len(src) + 1:].removesuffix('.rs').split('/')
    if rel[-1] in ('lib', 'main', 'mod'):
        rel = rel[:-1]
    return rel


def defines(text, ty, fn):
    if text is None:
        return False
    has_fn = re.search(rf'\bfn\s+{re.escape(fn)}\s*[<(]', text)
    if not ty:
        return bool(has_fn) and bool(re.search(rf'^(pub(\([^)]*\))?\s+)?fn\s+{re.escape(fn)}\s*[<(]', text, re.M))
    has_impl = re.search(rf'\bimpl(<[^>]*>)?\s+{re.escape(ty)}\b(<[^>]*>)?\s*\{{', text)
    return bool(has_fn and has_impl)


def locate(source, target):
    """Where `crate::mod::Type::fn` is at this commit: the same target, a moved one, or None."""
    parts = target.split('::')
    crate, fn = parts[0], parts[-1]
    middle = parts[1:-1]
    ty = ty_text = None
    if middle and middle[-1][:1].isupper():
        ty = re.sub(r'<.*>$', '', middle[-1])
        ty_text = middle[-1]
        middle = middle[:-1]
    src = crate_dir(crate)
    if src.replace('/src', '/Cargo.toml') not in source.files:
        return None, f'crate {crate} is gone'
    candidates = []
    for path in sorted(source.files):
        if path.startswith(src + '/') and path.endswith('.rs') and '/tests' not in path \
                and not path.endswith('_tests.rs') and defines(source.read(path), ty, fn):
            candidates.append(module_of(path, src))
    if middle in candidates:
        return target, None
    if len(candidates) == 1:
        moved = '::'.join([crate, *candidates[0], *([ty_text] if ty_text else []), fn])
        return moved, None
    if not candidates:
        return None, f'{target}: no `fn {fn}`' + (f' in an `impl {ty}`' if ty else '') + ' anymore'
    return None, f'{target}: several places define it now: ' + ', '.join('::'.join(c) for c in candidates)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--horizon', required=True, type=Path)
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument('--revision')
    g.add_argument('--latest', action='store_true')
    args = ap.parse_args()

    tag = None
    rev = args.revision
    if args.latest:
        rev, tag = latest_nightly()
    compat = json.loads(COMPAT.read_text())
    manifest = MANIFEST.read_text()
    old = compat['horizon_revision']

    source = Source(args.horizon, rev)
    problems, moved = [], {}

    catalog = json.loads(source.read('native-hooks.json') or '{"hooks": []}')
    have = {(h['name'], h['version']) for h in catalog.get('hooks', [])}
    for name, version in re.findall(r'^hook\s*=\s*"([^"]+)"\s*\nhook_version\s*=\s*(\d+)', manifest, re.M):
        if (name, int(version)) not in have:
            problems.append(f'named hook {name} v{version} is not in native-hooks.json')

    for target in re.findall(r'^target\s*=\s*"([^"]+)"', manifest, re.M):
        now, why = locate(source, target)
        if why:
            problems.append(why)
        elif now != target:
            moved[target] = now

    for file in re.findall(r'^file\s*=\s*"([^"]+)"', manifest, re.M):
        if file not in source.files:
            problems.append(f'module destination {file} is gone')

    launcher = source.read('launcher/src/native_mods.rs') or ''
    m = re.search(r'pub const MIXINS_REV: &str = "([0-9a-f]{40})";', launcher)
    mixins = m.group(1) if m else compat['horizon_mixins_revision']

    report = dict(old=old, new=rev, tag=tag, mixins=mixins, moved=moved, problems=problems)
    if problems:
        print(json.dumps(report, indent=2))
        return 1
    for a, b in moved.items():
        manifest = manifest.replace(f'target = "{a}"', f'target = "{b}"')
    manifest = re.sub(r'horizon_revision = "[0-9a-f]{40}"', f'horizon_revision = "{rev}"', manifest)
    if tag:
        manifest = re.sub(r'^# Horizon Nightly .*$', f'# Horizon Nightly {tag.split("/")[-1][:10]} (tag {tag}). Installing',
                          manifest, count=1, flags=re.M)
    MANIFEST.write_text(manifest)
    compat.update(horizon_revision=rev, horizon_mixins_revision=mixins)
    COMPAT.write_text(json.dumps(compat, indent=2) + '\n')
    report['changed'] = rev != old or bool(moved)
    print(json.dumps(report, indent=2))
    return 0


if __name__ == '__main__':
    sys.exit(main())
