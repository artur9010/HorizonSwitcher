//! Horizon Switcher subscribes to no hook: what it does happens in the game's own functions, through
//! the `[[native.mixin]]` rows of horizon-mod.toml and its support module (`switcher.rs`), which
//! the launcher compiles beside the game. This crate is the library the game loads for it.
use mod_api::Hooks;

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {}
