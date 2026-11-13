#!/usr/bin/env bash
# Applies the mod to the pinned Horizon with the real horizon-mixins, as the launcher's build does.
#
#   tools/verify.sh            plan + apply: every target, binding, hook and module resolves
#   tools/verify.sh --compile  then `cargo check` the modded horizon-client (slow: the whole game)
#
# Works in target/verify/ (WORK= to change it). Needs git, cargo/rustup and python3.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=${WORK:-$ROOT/target/verify}
COMPILE=0
[[ ${1:-} == --compile ]] && COMPILE=1

pin() { python3 -c "import json,sys; print(json.load(open('$ROOT/compatibility.json'))[sys.argv[1]])" "$1"; }
HORIZON_REV=$(pin horizon_revision)
HORIZON_REPO=$(pin horizon_repository)
MIXINS_REV=$(pin horizon_mixins_revision)
MIXINS_REPO=$(pin horizon_mixins_repository)

# Same pin in both files.
grep -q "horizon_revision = \"$HORIZON_REV\"" "$ROOT/mixins/horizon_switcher/horizon-mixin.toml" ||
  { echo "horizon-mixin.toml and compatibility.json pin different Horizon revisions" >&2; exit 1; }

checkout() { # dir repo rev
  if [[ ! -d $1/.git ]]; then git init -q "$1" && git -C "$1" remote add origin "$2"; fi
  if [[ $(git -C "$1" rev-parse -q --verify HEAD || true) != "$3" ]]; then
    git -C "$1" fetch -q --depth 1 origin "$3"
    git -C "$1" checkout -q --detach FETCH_HEAD
  fi
}

mkdir -p "$WORK"
echo "Horizon $HORIZON_REV, horizon-mixins $MIXINS_REV"
checkout "$WORK/horizon" "$HORIZON_REPO" "$HORIZON_REV"
checkout "$WORK/horizon-mixins" "$MIXINS_REPO" "$MIXINS_REV"

cargo build -q --locked --manifest-path "$WORK/horizon-mixins/Cargo.toml" -p horizon-mixins-cli
MX=$WORK/horizon-mixins/target/debug/horizon-mixins

# horizon-mixins takes no symlinks (Horizon commits CLAUDE.md as one), so it gets an export of the
# commit with each link replaced by a copy, and the `.horizon-revision` stamp that stands for git
# there (as horizon-native-mod-template and the launcher do).
SOURCE=$WORK/source-$HORIZON_REV
if [[ ! -f $SOURCE/.horizon-revision ]]; then
  rm -rf "$SOURCE" "$SOURCE.tmp"
  mkdir -p "$SOURCE.tmp"
  git -C "$WORK/horizon" archive --format=tar "$HORIZON_REV" | tar -x -C "$SOURCE.tmp"
  find "$SOURCE.tmp" -type l | while read -r link; do
    cp --remove-destination "$(readlink -f "$link")" "$link"
  done
  echo "$HORIZON_REV" > "$SOURCE.tmp/.horizon-revision"
  mv "$SOURCE.tmp" "$SOURCE"
fi

"$MX" plan --workspace "$SOURCE" --mods "$ROOT/mixins" --out "$WORK/plan.json"
"$MX" apply --workspace "$SOURCE" --mods "$ROOT/mixins" --out "$WORK/modded"

if (( COMPILE )); then
  cargo fetch --locked --manifest-path "$WORK/modded/Cargo.toml"
  # Audio and controllers off, as the native mod template builds: fewer system libraries.
  (cd "$WORK/modded" && cargo check --locked --offline -p horizon-client --no-default-features)
fi
echo "OK: the mod applies to Horizon $HORIZON_REV$( (( COMPILE )) && echo ' and compiles')"
