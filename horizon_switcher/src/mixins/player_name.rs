// HEAD of `Options::player_name`: the offline name that plays. Splitscreen guests keep their
// own names.
fn switcher_player_name(options: &Options) -> ::core::ops::ControlFlow<String> {
    if options.split_guest {
        return ::core::ops::ControlFlow::Continue(());
    }
    match crate::game::account_switcher::offline_name() {
        Some(name) => ::core::ops::ControlFlow::Break(name),
        None => ::core::ops::ControlFlow::Continue(()),
    }
}
