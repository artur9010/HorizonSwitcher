//! What Horizon Switcher does happens in the game's own functions, through the `[[native.mixin]]`
//! rows of horizon-mod.toml and its support module (`switcher.rs`), which the launcher compiles
//! beside the game. This crate is the library the game loads for it.
use mod_api::{ClientHost, Hooks, ScreenRenderPostEvent};

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Does nothing itself, but must be here: the game builds the screen view and calls
    /// `native_screen_render_post` (where the `render` mixin draws the account button and
    /// overlay) only while some mod listens to a screen render hook.
    fn screen_render_post(&self, _host: &dyn ClientHost, _event: &mut ScreenRenderPostEvent<'_>) {}
}
