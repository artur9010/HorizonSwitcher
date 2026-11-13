// Stands in for the parts of Horizon the injected module uses, so
// `cargo test` runs its tests without the game: `super::CURRENT` (auth.rs),
// `crate::logging` and `horizon_protocol::servers`.
extern crate self as horizon_protocol;

pub mod auth;

pub mod logging {
    pub fn player_dir_override() -> Option<&'static std::path::Path> {
        None
    }
}

/// A one-entry-per-line stand-in for servers.dat ("name\taddress").
pub mod servers {
    use std::path::Path;
    pub struct ServerEntry {
        pub name: String,
        pub ip: String,
    }
    impl ServerEntry {
        pub fn new(name: &str, ip: &str) -> Self {
            Self { name: name.into(), ip: ip.into() }
        }
    }
    pub fn load(path: &Path) -> Vec<ServerEntry> {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        text.lines()
            .filter_map(|l| l.split_once('\t'))
            .map(|(n, a)| ServerEntry::new(n, a))
            .collect()
    }
    pub fn prepend(path: &Path, e: &ServerEntry) -> std::io::Result<()> {
        let old = std::fs::read_to_string(path).unwrap_or_default();
        std::fs::write(path, format!("{}\t{}\n{old}", e.name, e.ip))
    }
}

#[cfg(test)]
mod server_tests {
    use crate::auth::account_switcher::add_default_server_once;

    #[test]
    fn default_server_is_added_once() {
        let dir = std::env::temp_dir().join(format!("hs-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("servers.dat");
        std::fs::write(&file, "Mine\texample.org\n").unwrap();
        assert!(add_default_server_once(&file).unwrap());
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("KiwiMC\tkiwimc.pl\n"), "{text}");
        assert!(text.contains("example.org"), "keeps the player's servers");
        // Deleted by the player: not added again.
        std::fs::write(&file, "Mine\texample.org\n").unwrap();
        assert!(!add_default_server_once(&file).unwrap());
        assert!(!std::fs::read_to_string(&file).unwrap().contains("kiwimc"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_server_already_listed_is_not_duplicated() {
        let dir = std::env::temp_dir().join(format!("hs-test2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("servers.dat");
        std::fs::write(&file, "Kiwi\tKIWIMC.PL\n").unwrap();
        assert!(!add_default_server_once(&file).unwrap());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "Kiwi\tKIWIMC.PL\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
