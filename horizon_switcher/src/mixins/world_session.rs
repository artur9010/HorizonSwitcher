// `horizon_client::auth::current()` in `Game::open_integrated`: no session while an offline
// account plays, so the world's player is the offline name (`Options::player_name`).
fn switcher_world_session(
    session: Option<::std::sync::Arc<horizon_client::auth::AuthSession>>,
) -> Option<::std::sync::Arc<horizon_client::auth::AuthSession>> {
    if crate::game::account_switcher::offline_active() {
        None
    } else {
        session
    }
}
