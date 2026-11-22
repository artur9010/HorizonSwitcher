#!/usr/bin/env python3
"""Moves the mod's Horizon pin to a newer Nightly.

    python3 tools/bump_horizon.py [--revision SHA | --latest]

With --latest (the default) the newest `nightly-source/<date>-<sha>` tag of Horizon's repository
is used. The launcher builds native mods only on a commit with an SDK release (`sdk-<sha>`), so a
Nightly whose SDK isn't published yet is left for the next run.

For that commit it checks the mod as tools/verify.py does (tools/horizon_check.py), and also that
no target's signature changed since the pinned commit: the helpers' bindings were written for it.
A target that only moved (another file or module) is rewritten to where it is now. When the mod
fits, `[native] horizon` and `mixins` (the horizon-mixins revision Horizon's sdk.json names) are
written into horizon-mod.toml.

It prints a JSON report. Exit 0: pinned (or nothing to do); 1: the mod needs a human.
"""
import argparse
import json
import re
import subprocess
import sys

import horizon_check as hc


def latest_nightly():
    out = subprocess.run(['git', 'ls-remote', '--tags', f'https://github.com/{hc.REPO}.git', 'nightly-source/*'],
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


def main():
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group()
    g.add_argument('--revision')
    g.add_argument('--latest', action='store_true')
    args = ap.parse_args()

    tag = None
    rev = args.revision
    if not rev:
        rev, tag = latest_nightly()
    mod = hc.manifest()
    old = mod['native']['horizon']
    report = dict(old=old, new=rev, tag=tag, mixins=None, moved={}, problems=hc.static_problems(mod),
                  changed=False)

    def done(status):
        print(json.dumps(report, indent=2))
        return status

    if rev == old:
        return done(1 if report['problems'] else 0)
    try:
        sdk = hc.Sdk(rev)
    except hc.MissingSdk:
        if args.revision:
            report['problems'].append(f'Horizon {rev} has no SDK release: the launcher cannot build on it')
            return done(1)
        report['waiting'] = f'sdk-{rev} is not published yet'
        return done(0)
    # The signatures the helpers were written for: the pinned commit's (when its SDK is still up).
    try:
        _, _, signatures = hc.check(hc.Sdk(old), mod)
    except hc.MissingSdk:
        signatures = None
        report['warning'] = f'sdk-{old} is gone: signatures not compared'
    # The pin the mod would get, so the mixins check is against the new one.
    mod['native']['mixins'] = sdk.mixins
    problems, moved, _ = hc.check(sdk, mod, signatures)
    report.update(mixins=sdk.mixins, moved=moved)
    report['problems'] += problems
    if report['problems']:
        return done(1)

    text = hc.MANIFEST.read_text()
    for a, b in moved.items():
        text = text.replace(f'target = "{a}"', f'target = "{b}"')
    text = re.sub(r'(?m)^horizon = "[0-9a-f]{40}"', f'horizon = "{rev}"', text, count=1)
    text = re.sub(r'(?m)^mixins = "[0-9a-f]{40}"', f'mixins = "{sdk.mixins}"', text, count=1)
    if tag:
        text = re.sub(r'(?m)^# Horizon Nightly \S+ \(tag [^)]*\)\.', f'# Horizon Nightly {tag.split("/")[-1][:10]} (tag {tag}).',
                      text, count=1)
    hc.MANIFEST.write_text(text)
    report['changed'] = True
    return done(0)


if __name__ == '__main__':
    sys.exit(main())
