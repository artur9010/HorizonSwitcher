// HEAD of `horizon_client_render::auth::current`: while an offline account
// plays, the game has no Microsoft session (joins, LAN, single player, chat
// signing and Realms all read it through here).
fn switcher_current() -> std::ops::ControlFlow<Option<Arc<AuthSession>>> {
    if account_switcher::offline_active() {
        std::ops::ControlFlow::Break(None)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
