// HEAD of `native_screen_render_post`: draws the account button and overlay
// on the title and multiplayer screens.
fn switcher_render(
    game: &crate::game::Game<'_>,
    batch: &mut HudBatch,
    screen: &NativeScreenView<'_>,
) {
    use horizon_client_render::auth::account_switcher::{self as switcher, Draw};
    let size = [batch.width, batch.height];
    let draws = switcher::draw(screen.id, size, screen.mouse, &|t: &str| {
        game.native_text_width(t)
    });
    for d in draws {
        match d {
            Draw::Fill(rect, argb) => game.native_fill(batch, rect, argb),
            Draw::Text(text, at, rgb, shadow) => game.native_text(batch, &text, at, rgb, shadow),
        }
    }
}
