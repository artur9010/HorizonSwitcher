// Stands in for `crates/horizon-client-render/src/auth.rs`.
use std::sync::{Arc, OnceLock};
pub struct AuthSession {
    pub name: String,
}
pub static CURRENT: OnceLock<Arc<AuthSession>> = OnceLock::new();

#[path = "../../mixins/horizon_switcher/switcher.rs"]
pub mod account_switcher;
