// HEAD of `native_screen_mouse_scrolled_pre`.
fn switcher_scroll(screen: &NativeScreenView<'_>, scroll: [f64; 2], canceled: &mut bool) {
    let size = [screen.rect[0] + screen.rect[2], screen.rect[1] + screen.rect[3]];
    if horizon_client_render::auth::account_switcher::scrolled(screen.id, size, scroll[1]) {
        *canceled = true;
    }
}
