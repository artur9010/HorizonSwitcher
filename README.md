# Horizon Switcher (native mod for Horizon)

An in-game account switcher for [Horizon](https://github.com/horizon-rust/horizon), in the spirit of Java's
In-Game Account Switcher: keep a list of **offline accounts** next to your Microsoft account and switch between
them on the title or multiplayer screen, with no restart.

![The Accounts screen: the Microsoft account and an offline account, which is playing](docs/screenshot.png)

## Why this mod exists

Horizon's launcher used to have offline names. On 2026-11-09
[horizon-rust/horizon#14127](https://github.com/horizon-rust/horizon/pull/14127) ("launcher: remove offline names;
play only as an account that owns the game") removed them, with their skins and capes, the hidden developer
option, and the CLI's offline account commands. Since then the game plays only as a Microsoft account that owns
Minecraft: Java Edition. This mod puts offline accounts back, inside the game, behind that same Microsoft account.

## You still need a Microsoft account that owns the game

Horizon's launcher only starts the game for a Microsoft account that owns Minecraft: Java Edition, and hands that
session to the game. Offline accounts work **only while that session is there**. Start the game any other way and
the list is shown greyed out with "Start Horizon from the launcher with a Microsoft account". The mod adds offline
names; it does not get around buying the game.

Offline names only get into **offline-mode** servers (and single player). Online-mode servers refuse them, as
they do on Java.

## Install

From the launcher: **Mods → Add from GitHub** and paste this repository's URL, or:

```sh
horizon-launcher mods add-github https://github.com/artur9010/HorizonSwitcher --trust-native-code
```

The launcher takes the latest release's `.hmod` (or, without a release, packs the `horizon_switcher/` project from
the commit) and builds the mod against the prebuilt game of its pinned Horizon. Native mods run with full access
to your computer: read the source first.

The mod is pinned to one Horizon Nightly (`[native] horizon` in `horizon_switcher/horizon-mod.toml`). A daily
workflow moves the pin to each new Nightly the mod fits, and releases it.

### From a file

`python3 tools/package.py` writes `target/horizon_switcher-<version>.hmod`. Put it in the instance's
`horizon-native-mods/` folder, next to `options.txt`, and let the launcher build.

## Use

* **Account: <name>** button, top left of the title and multiplayer screens. It opens the list.
* Click a row to play as it: the Microsoft account, or an offline name (marked green with `>`).
* Type a name (3–16 of `A-Z a-z 0-9 _`; Shift+`-` types `_`) and press **Add** or Enter. It is added and selected.
* **x** removes an offline name. Removing the active one switches back to Microsoft.
* **Done**, Escape or a click outside closes it.

The choice is saved and comes back the next time you start the game. Files:

* `$HORIZON_HOME/horizon-switcher.json`: the offline names and which one plays. It is shared by every instance.
  No tokens are stored; the Microsoft session stays in the game's memory as before.

## How it hooks in

`horizon_switcher/` is a flat native mod project: `horizon-mod.toml`, `Cargo.toml` and `src/`. Its crate
(`src/lib.rs`) subscribes to no hook; the work is done inside the game's own functions by `[[native.mixin]]` rows,
which the launcher copies out of the pinned source, patches and installs over the prebuilt game:

| Row | Where | What |
|---|---|---|
| `[[native.module]] account_switcher` | `crates/horizon-client/src/game.rs` (`src/switcher.rs`) | State, account file, overlay layout/input, server-list add |
| `join-session` (HEAD, cancellable) | `game::remote::polling::Game::auth_body` | No Microsoft session for server joins and chat signing while an offline account plays (splitscreen guests keep theirs) |
| `world-session` (MODIFY_EXPRESSION_VALUE) | `auth::current()` in `game::integrated::Game::open_integrated` | Single-player worlds play as the offline name |
| `player-name` (HEAD, cancellable) | `game::launch_options::Options::player_name` | The offline name wherever the game names its player (splitscreen guests untouched) |
| `render`, `press`, `release`, `scroll`, `key` (HEAD) | `game::native_hooks::base::Game::native_screen_*` | Draw the button and overlay on the title and multiplayer screens, take its clicks, wheel and typing |

The row in Options → Mods comes from `horizon-mod.toml` itself.

`horizon_client_render::auth::current()` lives below the game crate and can't take a mixin, so the mod patches the
callers that matter instead. Callers in `horizon_client_lib` can't be patched either: hosting a LAN world, Realms
and the title's account status still see the Microsoft session while an offline name plays, and so does the
cosmetics service.

`switcher.rs` is pure Rust except for `horizon_client_render::{auth, logging}` and `horizon_protocol::servers`,
which `shim/` stands in for, so its logic has unit tests without the game:

```sh
cargo test
```

`cargo test` covers the logic only (the name rules, saving the list, layout and clicks, key mapping, adding the
server once). `python3 tools/verify.py` checks the mod against the SDK release of its pinned commit: every target
is in that build's function index and patchable, every selector matches, and the helpers take what their rows
bind. Neither compiles the mod: the launcher's build (`horizon-launcher native-build horizon_switcher`) is the
real check.

## Maintenance

| Workflow | When | What |
|---|---|---|
| `ci.yml` | push, PR | `cargo test`, clippy, `tools/verify.py` and `tools/package.py` |
| `bump-horizon.yml` | daily, or by hand (optionally with a commit) | `tools/bump_horizon.py` picks the newest `nightly-source/*` tag that has an SDK release, and checks the mod against it as `verify.py` does. It also requires every target's signature to be unchanged since the current pin. Targets that only moved are rewritten to their new path, and `[native] mixins` follows the horizon-mixins revision of Horizon's `sdk.json`. **If that passes**, it commits the new pin to `master`, raises the patch version (`tools/bump_version.py`), tags `v<version>` and publishes the release. If it fails, `master` is unchanged and an issue is opened. |
| `release.yml` | tag `v<version>`, or called by the bump | Checks that the tag matches the version, tests and checks (skipped when the bump already did), then `tools/package.py` and a GitHub release with the `.hmod` |

To release by hand: `python3 tools/bump_version.py` (patch +1, or pass a version), commit, then
`git tag v<version> && git push origin master --tags`.

The bump pushes with the workflow's own token, so `master` must accept pushes from GitHub Actions; branch
protection that requires PRs would block it. A tag pushed with that token starts no workflow, so the bump calls
`release.yml` itself.

## Credit

Inspired by In-Game Account Switcher
for Java. Its features are reimplemented here; none of its code or assets are used. Layout modelled on
[ResourcifyHorizon](https://github.com/tariksabanuz/ResourcifyHorizon).
