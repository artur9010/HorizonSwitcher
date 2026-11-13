// The named hook `mods`: Horizon Switcher's row in Options > Mods.
fn switcher_listed(mods: &mut Vec<NativeMod>) {
    use horizon_client_render::auth::account_switcher as s;
    mods.push(NativeMod {
        id: s::ID.into(),
        title: format!("{} {}", s::NAME, s::VERSION),
        description: s::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
