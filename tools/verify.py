#!/usr/bin/env python3
"""Checks the mod against the Horizon commit it pins (`[native] horizon` in horizon-mod.toml).

    python3 tools/verify.py

Reads that commit's SDK release (sdk.json and the function index in build-data.tar.xz) and the
target files from GitHub: see tools/horizon_check.py for what is checked. Prints a JSON report;
exit 1 when the mod doesn't fit. Nothing is compiled: the launcher's build is the full check.
"""
import json
import sys

import horizon_check as hc


def main():
    mod = hc.manifest()
    problems = hc.static_problems(mod)
    commit = mod['native']['horizon']
    try:
        sdk = hc.Sdk(commit)
    except hc.MissingSdk:
        problems.append(f'Horizon {commit} has no SDK release (sdk-{commit}): the launcher cannot build on it')
        moved = {}
    else:
        found, moved, _ = hc.check(sdk, mod)
        problems += found
        problems += [f'{a} is {b} at this commit' for a, b in moved.items()]
    print(json.dumps(dict(horizon=commit, problems=problems), indent=2))
    return 1 if problems else 0


if __name__ == '__main__':
    sys.exit(main())
