"""What tools/verify.py and tools/bump_horizon.py share: the mod's manifest, Horizon's SDK release
for a commit, and the check of the mod against it.

The launcher builds a native mod against the prebuilt game of its pinned commit: each
`[[native.mixin]]` target is copied out of that commit's source (its function index,
`functions.json` in the SDK's build data) and patched there. So the mod fits a commit when:

  * the commit has an SDK release (`sdk-<commit>`), and `[native] mixins` is the horizon-mixins
    revision it names (sdk.json);
  * every target is in the index and not refused (a function that only moved is found again by
    its type and name);
  * every `selector` matches the target's body `require` times;
  * the support module's destination file is a file of the game crate;
  * the helper files and module source exist, and each helper takes as many parameters as its
    row binds.

Nothing is compiled here: the launcher's build is the only full check.
"""
import io
import json
import re
import tarfile
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MOD = ROOT / 'horizon_switcher'
MANIFEST = MOD / 'horizon-mod.toml'
REPO = 'horizon-rust/horizon'
GAME_CRATE = 'horizon_client'
# The kinds that rewrite one selected place in the body (horizon-mixins' `selects_site`).
SITE_KINDS = {'BEFORE_CALL', 'AFTER_CALL', 'AFTER_ASSIGN', 'MODIFY_ARG', 'MODIFY_ARGS', 'MODIFY_CONSTANT',
              'MODIFY_VARIABLE', 'MODIFY_EXPRESSION_VALUE', 'WRAP_OPERATION'}


def fetch(url):
    request = urllib.request.Request(url, headers={'User-Agent': 'horizon-mod-check'})
    with urllib.request.urlopen(request, timeout=120) as response:
        return response.read()


def manifest():
    return tomllib.loads(MANIFEST.read_text())


class Sdk:
    """The SDK release of one Horizon commit: sdk.json and its function index."""

    def __init__(self, commit):
        self.commit = commit
        base = f'https://github.com/{REPO}/releases/download/sdk-{commit}'
        try:
            self.info = json.loads(fetch(f'{base}/sdk.json'))
        except urllib.error.HTTPError as e:
            if e.code == 404:
                raise MissingSdk(commit) from None
            raise
        data = tarfile.open(fileobj=io.BytesIO(fetch(f'{base}/build-data.tar.xz')), mode='r:xz')
        member = next(m for m in data.getmembers() if m.name.lstrip('./') == 'functions.json')
        listed = data.extractfile(member)
        assert listed is not None, 'functions.json is a file'
        index = json.load(listed)
        if index['build']['commit'] != commit:
            raise SystemExit(f'functions.json of sdk-{commit} describes {index["build"]["commit"]}')
        self.functions = index['functions']
        self.refused = index['refused']
        self.files = {
            entry['path']
            for listed in index['crates'][GAME_CRATE]['modules'].values()
            for entry in listed
        }
        self._sources = {}

    @property
    def mixins(self):
        return self.info['mixins']['rev']

    def source(self, path):
        if path not in self._sources:
            self._sources[path] = fetch(f'https://raw.githubusercontent.com/{REPO}/{self.commit}/{path}')
        return self._sources[path]

    def body(self, key):
        start, end = self.functions[key]['span']
        return self.source(self.functions[key]['file'])[start:end].decode()


class MissingSdk(Exception):
    pass


def key_of(target):
    """A row's target as the index writes it: `Game<'_>::f` is `Game::f`."""
    return re.sub(r"<[^<>]*>(?=::)", '', target)


def tail(key):
    """`horizon_client::game::remote::polling::Game::auth_body` -> `Game::auth_body`."""
    parts = key.split('::')
    return '::'.join(parts[-2:]) if len(parts) > 2 and parts[-2][:1].isupper() else parts[-1]


def locate(sdk, target):
    """(the target at this commit, the index key, a problem): moved targets keep their generics."""
    key = key_of(target)
    if key in sdk.refused:
        return None, None, f"{target}: {sdk.refused[key]['code']}: {sdk.refused[key]['why']}"
    if key in sdk.functions:
        return target, key, None
    found = [k for k in sdk.functions if k.startswith(GAME_CRATE + '::') and tail(k) == tail(key)]
    if len(found) == 1:
        old_module = key[:len(key) - len(tail(key))]
        new_module = found[0][:len(found[0]) - len(tail(key))]
        return new_module + target[len(old_module):], found[0], None
    if not found:
        return None, None, f'{target}: not in the function index of {sdk.commit} anymore'
    return None, None, f'{target}: several functions are {tail(key)} now: ' + ', '.join(found)


def helper_params(text):
    m = re.search(r'\bfn\s+\w+\s*\((.*?)\)\s*(->|\{)', text, re.S)
    if not m:
        return None
    params = m.group(1).strip().rstrip(',')
    depth, count = 0, (1 if params else 0)
    for c in params:
        depth += c in '<([{'
        depth -= c in '>)]}'
        count += c == ',' and depth == 0
    return count


def check(sdk, mod, signatures=None):
    """Problems of `mod` (a parsed manifest) at `sdk`'s commit, the targets that moved, and each
    target's signature hash. With `signatures` (the hashes at the commit the mod fit), a changed
    signature is a problem too: the helpers' bindings were written for the old one."""
    native = mod['native']
    problems, moved, hashes = [], {}, {}
    if native['mixins'] != sdk.mixins:
        problems.append(f"[native] mixins is {native['mixins']}, Horizon {sdk.commit} builds with {sdk.mixins}")
    for module in native.get('module', []):
        if module['file'] not in sdk.files:
            problems.append(f"module {module['id']}: {module['file']} is not a file of {GAME_CRATE}")
        if not (MOD / module['source']).is_file():
            problems.append(f"module {module['id']}: no {module['source']}")
    for row in native.get('mixin', []):
        rid = f"{mod['id']}:{row['id']}"
        now, key, why = locate(sdk, row['target'])
        if why:
            problems.append(f'{rid}: {why}')
            continue
        if now != row['target']:
            moved[row['target']] = now
        hashes[now] = sdk.functions[key]['signature_hash']
        old = (signatures or {}).get(row['target'])
        if old and old != hashes[now]:
            problems.append(f"{rid}: {row['target']}'s signature changed: check the bindings")
        if 'selector' in row:
            found = sdk.body(key).count(row['selector'])
            if found != row.get('require', 1) or row.get('ordinal', 0) >= found:
                problems.append(f"{rid}: `{row['selector']}` is in {now} {found} times, the row requires "
                                f"{row.get('require', 1)}")
        helper = MOD / row['helper']
        if not helper.is_file():
            problems.append(f"{rid}: no {row['helper']}")
            continue
        params = helper_params(helper.read_text())
        if params != len(row.get('bindings', [])):
            problems.append(f"{rid}: {row['helper']} takes {params} parameters, the row binds "
                            f"{len(row.get('bindings', []))}")
    return problems, moved, hashes


def static_problems(mod):
    """What the launcher refuses in the manifest itself, before any build."""
    problems = []
    native = mod.get('native') or {}
    if mod.get('api') != '0.2':
        problems.append('api must be "0.2"')
    if mod.get('capabilities') != ['registries']:
        problems.append('capabilities must be ["registries"]')
    if not re.fullmatch(r'[a-z0-9_]{1,64}', mod.get('id', '')) or MOD.name != mod.get('id'):
        problems.append(f"the mod's folder {MOD.name}/ must be its id")
    for pin in ('horizon', 'mixins'):
        if not re.fullmatch(r'[0-9a-f]{40}', native.get(pin, '')):
            problems.append(f'[native] {pin} must be an exact commit')
    for row in native.get('mixin', []):
        if 'hook' in row or 'hook_version' in row:
            problems.append(f"{row['id']}: a mixin names a target, not a hook")
        if not (row.get('helper', '').startswith('src/') and row['helper'].endswith('.rs')):
            problems.append(f"{row['id']}: the helper must be a .rs file under src/")
        # horizon-mixins' own rules (manifest.rs): a kind that selects a site needs a selector
        # (MODIFY_VARIABLE: a variable), a zero-based ordinal and a positive require; the others
        # take none of them.
        kind = row.get('kind')
        if kind in SITE_KINDS:
            if (kind != 'MODIFY_VARIABLE' and 'selector' not in row) or not isinstance(row.get('ordinal'), int) \
                    or not (isinstance(row.get('require'), int) and row['require'] > 0):
                problems.append(f"{row['id']}: {kind} needs selector, ordinal (from 0) and require")
        elif any(k in row for k in ('selector', 'ordinal', 'require')):
            problems.append(f"{row['id']}: selector/ordinal/require apply only to site kinds, not {kind}")
        if row.get('cancellable') and kind not in ('HEAD', 'BEFORE_CALL', 'AFTER_CALL', 'AFTER_ASSIGN'):
            problems.append(f"{row['id']}: cancellable needs HEAD, BEFORE_CALL, AFTER_CALL or AFTER_ASSIGN")
    return problems
