// HEAD of `native_screen_render_post`: draws the account button and overlay on the title and
// multiplayer screens.
fn switcher_render(game: &Game<'_>, batch: &mut HudBatch, screen: &ScreenView<'_>) {
    let size = [batch.width, batch.height];
    let draws = {
        let measure: &HudBatch = batch;
        crate::game::account_switcher::draw(screen.id, size, screen.mouse, &|t: &str| {
            game.native_text_width(measure, t)
        })
    };
    for d in draws {
        match d {
            crate::game::account_switcher::Draw::Fill(rect, argb) => game.native_fill(batch, rect, argb),
            crate::game::account_switcher::Draw::Text(text, at, argb, shadow) => {
                game.native_text(batch, &text, at, argb, shadow)
            }
        }
    }
}
