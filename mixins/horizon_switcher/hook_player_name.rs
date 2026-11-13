// HEAD of `Options::player_name`: the offline name that plays. Splitscreen
// guests keep their own names.
fn switcher_player_name(options: &Options) -> std::ops::ControlFlow<String> {
    if options.split_guest {
        return std::ops::ControlFlow::Continue(());
    }
    match horizon_client_render::auth::account_switcher::offline_name() {
        Some(name) => std::ops::ControlFlow::Break(name),
        None => std::ops::ControlFlow::Continue(()),
    }
}
