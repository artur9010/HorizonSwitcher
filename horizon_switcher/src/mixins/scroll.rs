// HEAD of `native_screen_mouse_scrolled_pre`.
fn switcher_scroll(screen: &ScreenView<'_>, scroll: [f64; 2], canceled: &mut bool) {
    let size = [screen.rect[0] + screen.rect[2], screen.rect[1] + screen.rect[3]];
    if crate::game::account_switcher::scrolled(screen.id, size, scroll[1]) {
        *canceled = true;
    }
}
