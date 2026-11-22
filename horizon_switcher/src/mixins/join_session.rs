// HEAD of `Game::auth_body` (polling.rs), the account a server join and chat signing use: none
// while an offline account plays, so the join is offline. A splitscreen guest keeps its own.
// horizon-mixins compares the return type with the target's as text: spell it as polling.rs does.
fn switcher_join_session(
    game: &Game<'_>,
) -> ::core::ops::ControlFlow<Option<std::sync::Arc<horizon_client::auth::AuthSession>>> {
    if !game.options.split_guest && crate::game::account_switcher::offline_active() {
        ::core::ops::ControlFlow::Break(None)
    } else {
        ::core::ops::ControlFlow::Continue(())
    }
}
