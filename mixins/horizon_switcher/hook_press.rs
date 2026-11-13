// HEAD of `native_screen_mouse_button_pressed_pre`. On the title's own
// screens `screen.rect` is the whole window (API 326).
fn switcher_press(screen: &NativeScreenView<'_>, button: i32, canceled: &mut bool) {
    let size = [screen.rect[0] + screen.rect[2], screen.rect[1] + screen.rect[3]];
    if horizon_client_render::auth::account_switcher::mouse_pressed(screen.id, size, screen.mouse, button) {
        *canceled = true;
    }
}
