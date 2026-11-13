// HEAD of `native_screen_mouse_button_released_pre`: a press the switcher
// took keeps its release too.
fn switcher_release(button: i32, canceled: &mut bool) {
    if horizon_client_render::auth::account_switcher::mouse_released(button) {
        *canceled = true;
    }
}
