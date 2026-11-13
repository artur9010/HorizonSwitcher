// HEAD of `native_screen_key_pressed_pre`: the name box's typing.
fn switcher_key(screen: &NativeScreenView<'_>, key: &str, modifiers: u32, canceled: &mut bool) {
    if horizon_client_render::auth::account_switcher::key_pressed(screen.id, key, modifiers) {
        *canceled = true;
    }
}
