// Horizon Switcher: offline accounts beside the launcher's Microsoft account.
//
// The launcher puts this file into the game as `crate::game::account_switcher`, a
// `[[native.module]]` of horizon-mod.toml, where the mixin helpers in `mixins/` call it. It reads
// the game through `horizon_client_render` (the Microsoft session, the game folder) and
// `horizon_protocol` (servers.dat), which `shim/` stands in for so it also builds as a standalone
// library for the unit tests.
//
// The rule: offline accounts can only be used while the game was started with a Microsoft
// session (the launcher only hands one over for an account that owns Minecraft: Java Edition).
// Without that session the mod shows the list but plays nothing offline.
//
// No inner attributes, `//!` comments or external `mod` here: a support module's items are
// copied as they are.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard};

/// The account list, kept in the launcher's game home (`HORIZON_HOME`), so
/// every instance shares it.
pub const ACCOUNTS_FILE: &str = "horizon-switcher.json";
/// Beside `servers.dat`: the server below was added to that list once.
pub const SERVER_MARKER: &str = "horizon-switcher-server.txt";
/// The server put in the server list on the first launch.
pub const DEFAULT_SERVER_NAME: &str = "KiwiMC";
pub const DEFAULT_SERVER_ADDRESS: &str = "kiwimc.pl";
pub const MAX_NAME: usize = 16;

// ---------------------------------------------------------------------------
// Accounts
// ---------------------------------------------------------------------------

/// Vanilla's username rules for offline play: 3 to 16 of A-Z, a-z, 0-9 and _.
/// (Servers in offline mode refuse much else, and the login packet takes 16.)
pub fn valid_name(name: &str) -> bool {
    (3..=MAX_NAME).contains(&name.len())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The saved list: offline names, and the one that plays (None: Microsoft).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Store {
    pub offline: Vec<String>,
    pub active: Option<String>,
}

impl Store {
    pub fn parse(text: &str) -> Self {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
            return Self::default();
        };
        let mut store = Self::default();
        for name in v["offline"].as_array().into_iter().flatten() {
            if let Some(name) = name.as_str() {
                // Ignore broken or duplicated entries rather than refuse the file.
                let _ = store.add(name);
            }
        }
        store.active = v["active"]
            .as_str()
            .and_then(|a| store.find(a).map(|i| store.offline[i].clone()));
        store
    }
    pub fn to_json(&self) -> String {
        let v = serde_json::json!({
            "offline": self.offline,
            "active": self.active,
        });
        serde_json::to_string_pretty(&v).unwrap_or_default() + "\n"
    }
    /// The index of `name`, any case (servers treat names case-insensitively).
    pub fn find(&self, name: &str) -> Option<usize> {
        self.offline.iter().position(|n| n.eq_ignore_ascii_case(name))
    }
    /// Adds an offline name; an error says why not.
    pub fn add(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if !valid_name(name) {
            return Err("Use 3-16 letters, digits or _".into());
        }
        if self.find(name).is_some() {
            return Err(format!("{name} is already on the list"));
        }
        self.offline.push(name.to_owned());
        Ok(())
    }
    pub fn remove(&mut self, index: usize) -> Option<String> {
        if index >= self.offline.len() {
            return None;
        }
        let name = self.offline.remove(index);
        if self
            .active
            .as_deref()
            .is_some_and(|a| a.eq_ignore_ascii_case(&name))
        {
            self.active = None;
        }
        Some(name)
    }
    /// Plays as offline `index`, or as the Microsoft account with None.
    pub fn select(&mut self, index: Option<usize>) {
        self.active = index.and_then(|i| self.offline.get(i).cloned());
    }
    /// The offline name that plays now, given whether a Microsoft session exists.
    pub fn playing_offline(&self, microsoft: bool) -> Option<&str> {
        self.active.as_deref().filter(|_| microsoft)
    }
}

// ---------------------------------------------------------------------------
// The overlay (pure: layout, drawing commands and input)
// ---------------------------------------------------------------------------

/// What to draw, in GUI pixels: the helper turns these into `native_fill` and
/// `native_text` calls (both ARGB).
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    /// `[x, y, w, h]`, ARGB.
    Fill([f32; 4], u32),
    /// The text, its top left, ARGB, shadow.
    Text(String, [f32; 2], u32, bool),
}

/// The screens the switcher shows on (`NativeScreenView.id`).
pub fn shown_on(screen: &str) -> bool {
    matches!(screen, "title" | "multiplayer")
}

const ROW_H: f32 = 18.0;
const PANEL_W: f32 = 240.0;
const PAD: f32 = 6.0;

// Text colours are ARGB too: `native_text` draws nothing at alpha 0.
const WHITE: u32 = 0xFFFFFFFF;
const GREY: u32 = 0xFFA0A0A0;
const GREEN: u32 = 0xFF55FF55;
const RED: u32 = 0xFFFF5555;
const YELLOW: u32 = 0xFFFFFF55;
const DIM: u32 = 0xA0000000;
const PANEL: u32 = 0xF0101018;
const BUTTON: u32 = 0xFF3A3A44;
const HOVER: u32 = 0xFF5A5A6E;
const ACTIVE_ROW: u32 = 0xFF24442A;
const BORDER: u32 = 0xFF8080A0;

fn inside(r: [f32; 4], p: [f32; 2]) -> bool {
    p[0] >= r[0] && p[0] < r[0] + r[2] && p[1] >= r[1] && p[1] < r[1] + r[3]
}

/// Where everything is for a window `size` in GUI pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub toggle: [f32; 4],
    pub panel: [f32; 4],
    /// The list's first row; rows go down by ROW_H.
    pub list: [f32; 4],
    pub visible_rows: usize,
    pub input: [f32; 4],
    pub add: [f32; 4],
    pub message: [f32; 2],
    pub done: [f32; 4],
}

impl Layout {
    pub fn new(size: [f32; 2]) -> Self {
        let [w, h] = size;
        let toggle = [4.0, 4.0, 220.0_f32.min(w - 8.0), 20.0];
        let pw = PANEL_W.min(w - 8.0).max(120.0);
        // Title, list, input, message, Done.
        let fixed = PAD + 14.0 + PAD + ROW_H + PAD + 12.0 + 20.0 + PAD;
        let room = (h - 16.0 - fixed).max(ROW_H);
        let visible_rows = ((room / ROW_H) as usize).clamp(1, 8);
        let ph = fixed + visible_rows as f32 * ROW_H + PAD;
        let px = ((w - pw) / 2.0).floor();
        let py = ((h - ph) / 2.0).floor().max(8.0);
        let mut y = py + PAD + 14.0 + PAD;
        let list = [px + PAD, y, pw - 2.0 * PAD, visible_rows as f32 * ROW_H];
        y += list[3] + PAD;
        let add_w = 46.0;
        let input = [px + PAD, y, pw - 3.0 * PAD - add_w, ROW_H];
        let add = [px + pw - PAD - add_w, y, add_w, ROW_H];
        y += ROW_H + 2.0;
        let message = [px + PAD, y];
        y += 12.0 + PAD;
        let done = [px + PAD, y, pw - 2.0 * PAD, 20.0];
        Self {
            toggle,
            panel: [px, py, pw, ph],
            list,
            visible_rows,
            input,
            add,
            message,
            done,
        }
    }
    /// Row `i` of the visible list.
    pub fn row(&self, i: usize) -> [f32; 4] {
        [
            self.list[0],
            self.list[1] + i as f32 * ROW_H,
            self.list[2],
            ROW_H - 2.0,
        ]
    }
    /// The remove button of row `i`.
    pub fn remove(&self, i: usize) -> [f32; 4] {
        let r = self.row(i);
        [r[0] + r[2] - r[3], r[1], r[3], r[3]]
    }
}

/// One entry of the list: the Microsoft account (index None) or an offline one.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub offline: Option<usize>,
    pub name: String,
}

/// What a click on the open overlay did.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    None,
    Close,
    Select(Option<usize>),
    Remove(usize),
    Add,
}

/// The overlay's own state (not saved).
#[derive(Clone, Debug, Default)]
pub struct Ui {
    pub open: bool,
    pub input: String,
    pub scroll: usize,
    /// A line under the input: text, and whether it is an error.
    pub message: Option<(String, bool)>,
    /// A press this mod cancelled: its release is cancelled too.
    pub held: Vec<i32>,
}

impl Ui {
    pub fn entries(store: &Store, microsoft: Option<&str>) -> Vec<Entry> {
        let mut out = vec![Entry {
            offline: None,
            name: microsoft.unwrap_or("(no Microsoft session)").to_owned(),
        }];
        out.extend(store.offline.iter().enumerate().map(|(i, n)| Entry {
            offline: Some(i),
            name: n.clone(),
        }));
        out
    }

    fn clamp_scroll(&mut self, entries: usize, visible: usize) {
        self.scroll = self.scroll.min(entries.saturating_sub(visible));
    }

    /// The toggle button's label.
    pub fn toggle_label(store: &Store, microsoft: Option<&str>) -> String {
        match (microsoft, store.playing_offline(microsoft.is_some())) {
            (Some(_), Some(name)) => format!("Account: {name} (offline)"),
            (Some(name), None) => format!("Account: {name}"),
            (None, _) => "Account: not signed in".to_owned(),
        }
    }

    /// What a left click at `p` does while open.
    pub fn click(&mut self, layout: &Layout, store: &Store, microsoft: Option<&str>, p: [f32; 2]) -> Action {
        if !inside(layout.panel, p) {
            return Action::Close;
        }
        if inside(layout.done, p) {
            return Action::Close;
        }
        if inside(layout.add, p) {
            return Action::Add;
        }
        let entries = Self::entries(store, microsoft);
        self.clamp_scroll(entries.len(), layout.visible_rows);
        for i in 0..layout.visible_rows {
            let Some(entry) = entries.get(self.scroll + i) else {
                break;
            };
            if let Some(index) = entry.offline
                && inside(layout.remove(i), p)
            {
                return Action::Remove(index);
            }
            if inside(layout.row(i), p) {
                return Action::Select(entry.offline);
            }
        }
        Action::None
    }

    pub fn draw(
        &mut self,
        layout: &Layout,
        store: &Store,
        microsoft: Option<&str>,
        mouse: [f32; 2],
        text_width: &dyn Fn(&str) -> f32,
    ) -> Vec<Draw> {
        let mut out = Vec::new();
        let button = |out: &mut Vec<Draw>, r: [f32; 4], label: &str, enabled: bool| {
            let hover = enabled && inside(r, mouse);
            out.push(Draw::Fill(r, if hover { HOVER } else { BUTTON }));
            let tw = text_width(label);
            out.push(Draw::Text(
                label.to_owned(),
                [(r[0] + (r[2] - tw) / 2.0).floor(), (r[1] + (r[3] - 8.0) / 2.0).floor()],
                if enabled { WHITE } else { GREY },
                true,
            ));
        };

        // The toggle, always.
        let label = Self::toggle_label(store, microsoft);
        button(&mut out, layout.toggle, &label, true);
        if !self.open {
            return out;
        }

        // The overlay.
        out.push(Draw::Fill([0.0, 0.0, 100_000.0, 100_000.0], DIM));
        let p = layout.panel;
        out.push(Draw::Fill([p[0] - 1.0, p[1] - 1.0, p[2] + 2.0, p[3] + 2.0], BORDER));
        out.push(Draw::Fill(p, PANEL));
        let title = "Accounts";
        out.push(Draw::Text(
            title.into(),
            [(p[0] + (p[2] - text_width(title)) / 2.0).floor(), p[1] + PAD + 2.0],
            WHITE,
            true,
        ));

        let entries = Self::entries(store, microsoft);
        self.clamp_scroll(entries.len(), layout.visible_rows);
        let playing = store.playing_offline(microsoft.is_some());
        for i in 0..layout.visible_rows {
            let Some(entry) = entries.get(self.scroll + i) else {
                break;
            };
            let r = layout.row(i);
            let active = match entry.offline {
                None => microsoft.is_some() && playing.is_none(),
                Some(_) => playing.is_some_and(|a| a.eq_ignore_ascii_case(&entry.name)),
            };
            let usable = microsoft.is_some();
            let fill = if active {
                ACTIVE_ROW
            } else if usable && inside(r, mouse) {
                HOVER
            } else {
                BUTTON
            };
            out.push(Draw::Fill(r, fill));
            let kind = match entry.offline {
                None => "Microsoft",
                Some(_) => "Offline",
            };
            let marker = if active { "> " } else { "" };
            out.push(Draw::Text(
                format!("{marker}{}", entry.name),
                [r[0] + 4.0, r[1] + 4.0],
                if !usable {
                    GREY
                } else if active {
                    GREEN
                } else {
                    WHITE
                },
                true,
            ));
            let kw = text_width(kind);
            let right = if entry.offline.is_some() { r[3] + 4.0 } else { 4.0 };
            out.push(Draw::Text(
                kind.into(),
                [r[0] + r[2] - right - kw, r[1] + 4.0],
                GREY,
                false,
            ));
            if entry.offline.is_some() {
                let x = layout.remove(i);
                let hover = inside(x, mouse);
                out.push(Draw::Fill(x, if hover { 0xFFA03030 } else { 0xFF603030 }));
                out.push(Draw::Text(
                    "x".into(),
                    [(x[0] + (x[2] - text_width("x")) / 2.0).floor(), x[1] + 4.0],
                    WHITE,
                    true,
                ));
            }
        }
        if entries.len() > layout.visible_rows {
            let more = format!(
                "{}-{} of {} (scroll)",
                self.scroll + 1,
                (self.scroll + layout.visible_rows).min(entries.len()),
                entries.len()
            );
            out.push(Draw::Text(
                more.clone(),
                [p[0] + p[2] - PAD - text_width(&more), p[1] + PAD + 2.0],
                GREY,
                false,
            ));
        }

        // Input and Add.
        let input = layout.input;
        out.push(Draw::Fill(input, 0xFF000000));
        out.push(Draw::Fill([input[0], input[1] + input[3] - 1.0, input[2], 1.0], BORDER));
        let (text, color) = if self.input.is_empty() {
            ("Offline name...".to_owned(), GREY)
        } else {
            (format!("{}_", self.input), WHITE)
        };
        out.push(Draw::Text(text, [input[0] + 4.0, input[1] + 5.0], color, false));
        button(&mut out, layout.add, "Add", microsoft.is_some());

        // The message, or why offline accounts don't work.
        let message = match (&self.message, microsoft) {
            (_, None) => Some(("Start Horizon from the launcher with a Microsoft account".to_owned(), true)),
            (m, _) => m.clone(),
        };
        if let Some((text, error)) = message {
            out.push(Draw::Text(text, layout.message, if error { RED } else { YELLOW }, false));
        }
        button(&mut out, layout.done, "Done", true);
        out
    }
}

/// The character a key types into the name box (shift held or not).
pub fn key_char(key: &str, shift: bool) -> Option<char> {
    let rest = key.strip_prefix("key.keyboard.")?;
    let mut chars = rest.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return (rest == "minus" && shift).then_some('_');
    };
    if c.is_ascii_lowercase() {
        return Some(if shift { c.to_ascii_uppercase() } else { c });
    }
    (c.is_ascii_digit() && !shift).then_some(c)
}

// ---------------------------------------------------------------------------
// The game's side: global state, files, and the hooks' entry points
// ---------------------------------------------------------------------------

struct State {
    store: Store,
    ui: Ui,
    path: PathBuf,
    server_checked: bool,
}

/// Mirrors "an offline account plays": the session helpers ask it on every join.
static OFFLINE: AtomicBool = AtomicBool::new(false);

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    let path = accounts_path();
    let store = std::fs::read_to_string(&path)
        .map(|t| Store::parse(&t))
        .unwrap_or_default();
    OFFLINE.store(store.active.is_some(), Ordering::Relaxed);
    Mutex::new(State {
        store,
        ui: Ui::default(),
        path,
        server_checked: false,
    })
});

fn state() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn accounts_path() -> PathBuf {
    let home = std::env::var_os("HORIZON_HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();
    home.join(ACCOUNTS_FILE)
}

/// The launcher's Microsoft session. `auth::current()` itself is untouched (the
/// mod patches its callers), so this is the real one. Present only when the
/// launcher started the game for an owner.
fn microsoft_name() -> Option<String> {
    ::horizon_client_render::auth::current().map(|s| s.name.clone())
}

fn microsoft_session() -> bool {
    ::horizon_client_render::auth::current().is_some()
}

fn save(state: &mut State) {
    OFFLINE.store(state.store.active.is_some(), Ordering::Relaxed);
    if let Some(dir) = state.path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = state.path.with_extension("json.tmp");
    let result = std::fs::write(&tmp, state.store.to_json())
        .and_then(|()| std::fs::rename(&tmp, &state.path));
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        log::warn!("Horizon Switcher: could not save {}: {e}", state.path.display());
        state.ui.message = Some((format!("Not saved: {e}"), true));
    }
}

/// For the session helpers: true hides the Microsoft session, so server joins
/// and single-player worlds play offline.
pub fn offline_active() -> bool {
    // The saved choice applies from the first call (a `--server` join at start
    // asks before the title draws).
    if !LOADED.swap(true, Ordering::Relaxed) {
        LazyLock::force(&STATE);
    }
    OFFLINE.load(Ordering::Relaxed) && microsoft_session()
}

static LOADED: AtomicBool = AtomicBool::new(false);

/// For the `Options::player_name()` helper: the offline name that plays.
pub fn offline_name() -> Option<String> {
    if !offline_active() {
        return None;
    }
    let state = state();
    state.store.playing_offline(true).map(str::to_owned)
}

/// `servers.dat` for the game folder the game runs in (the multiplayer
/// screen's own rule: beside `saves/`, or the `--player-dir`).
fn servers_file() -> PathBuf {
    match ::horizon_client_render::logging::player_dir_override() {
        Some(dir) => dir.join("servers.dat"),
        None => std::env::current_dir().unwrap_or_default().join("servers.dat"),
    }
}

/// Adds the default server to `servers.dat` once per game folder: a marker
/// beside it remembers, so a player who deletes it doesn't get it back.
pub fn add_default_server_once(servers: &Path) -> std::io::Result<bool> {
    let marker = servers.with_file_name(SERVER_MARKER);
    if marker.exists() {
        return Ok(false);
    }
    // `load` and `prepend` are the server list's own reader and writer
    // (`ServerList.load`/`save`), present in every Horizon this mod targets.
    let known = ::horizon_protocol::servers::load(servers)
        .iter()
        .any(|s| s.ip.eq_ignore_ascii_case(DEFAULT_SERVER_ADDRESS));
    if !known {
        let entry =
            ::horizon_protocol::servers::ServerEntry::new(DEFAULT_SERVER_NAME, DEFAULT_SERVER_ADDRESS);
        ::horizon_protocol::servers::prepend(servers, &entry)?;
    }
    let added = !known;
    std::fs::write(&marker, format!("added {DEFAULT_SERVER_ADDRESS}\n"))?;
    Ok(added)
}

fn check_server(state: &mut State) {
    if state.server_checked {
        return;
    }
    state.server_checked = true;
    let file = servers_file();
    match add_default_server_once(&file) {
        Ok(true) => log::info!("Horizon Switcher: added {DEFAULT_SERVER_ADDRESS} to {}", file.display()),
        Ok(false) => {}
        Err(e) => log::warn!("Horizon Switcher: could not add {DEFAULT_SERVER_ADDRESS}: {e}"),
    }
}

/// `native_screen_render_post`: the toggle, and the overlay when open.
pub fn draw(screen: &str, size: [f32; 2], mouse: [f32; 2], text_width: &dyn Fn(&str) -> f32) -> Vec<Draw> {
    let mut state = state();
    // The title screen is up: the game folder is final, servers.dat not read yet.
    check_server(&mut state);
    if !shown_on(screen) {
        state.ui.open = false;
        return Vec::new();
    }
    let layout = Layout::new(size);
    let ms = microsoft_name();
    let State { store, ui, .. } = &mut *state;
    ui.draw(&layout, store, ms.as_deref(), mouse, text_width)
}

fn add_typed(state: &mut State) {
    let name = state.ui.input.trim().to_owned();
    match state.store.add(&name) {
        Ok(()) => {
            let index = state.store.offline.len() - 1;
            state.store.select(Some(index));
            state.ui.input.clear();
            state.ui.message = Some((format!("Playing as {name} (offline)"), false));
            save(state);
        }
        Err(e) => state.ui.message = Some((e, true)),
    }
}

/// `native_screen_mouse_button_pressed_pre`: true cancels the press.
pub fn mouse_pressed(screen: &str, size: [f32; 2], mouse: [f32; 2], button: i32) -> bool {
    if !shown_on(screen) {
        return false;
    }
    let mut state = state();
    let layout = Layout::new(size);
    if !state.ui.open {
        if button == 0 && inside(layout.toggle, mouse) {
            state.ui.open = true;
            state.ui.message = None;
            state.ui.held.push(button);
            return true;
        }
        return false;
    }
    state.ui.held.push(button);
    if button != 0 {
        return true;
    }
    let ms = microsoft_name();
    let action = {
        let State { store, ui, .. } = &mut *state;
        ui.click(&layout, store, ms.as_deref(), mouse)
    };
    match action {
        Action::None => {}
        Action::Close => state.ui.open = false,
        Action::Add if ms.is_some() => add_typed(&mut state),
        Action::Add => {}
        Action::Select(index) if ms.is_some() => {
            state.store.select(index);
            let name = match index {
                Some(i) => format!("{} (offline)", state.store.offline[i]),
                None => ms.clone().unwrap_or_default(),
            };
            state.ui.message = Some((format!("Playing as {name}"), false));
            save(&mut state);
        }
        Action::Select(_) => {}
        Action::Remove(index) => {
            if let Some(name) = state.store.remove(index) {
                state.ui.message = Some((format!("Removed {name}"), false));
                save(&mut state);
            }
        }
    }
    true
}

/// `native_screen_mouse_button_released_pre`: the release of a cancelled press.
pub fn mouse_released(button: i32) -> bool {
    let mut state = state();
    match state.ui.held.iter().position(|b| *b == button) {
        Some(i) => {
            state.ui.held.remove(i);
            true
        }
        None => false,
    }
}

/// `native_screen_mouse_scrolled_pre`: scrolls the list while open.
pub fn scrolled(screen: &str, size: [f32; 2], dy: f64) -> bool {
    let mut state = state();
    if !state.ui.open || !shown_on(screen) {
        return false;
    }
    let layout = Layout::new(size);
    let ms = microsoft_name();
    let entries = Ui::entries(&state.store, ms.as_deref()).len();
    if dy > 0.0 {
        state.ui.scroll = state.ui.scroll.saturating_sub(1);
    } else if dy < 0.0 {
        state.ui.scroll += 1;
    }
    state.ui.clamp_scroll(entries, layout.visible_rows);
    true
}

/// `native_screen_key_pressed_pre`: typing, Enter, Backspace, Escape while open.
pub fn key_pressed(screen: &str, key: &str, modifiers: u32) -> bool {
    let mut state = state();
    if !state.ui.open || !shown_on(screen) {
        return false;
    }
    let shift = modifiers & 1 != 0;
    match key {
        "key.keyboard.escape" => state.ui.open = false,
        "key.keyboard.backspace" => {
            state.ui.input.pop();
        }
        "key.keyboard.enter" | "key.keyboard.keypad.enter" => {
            if microsoft_name().is_some() {
                add_typed(&mut state);
            }
        }
        _ => {
            if let Some(c) = key_char(key, shift)
                && state.ui.input.len() < MAX_NAME
            {
                state.ui.input.push(c);
                state.ui.message = None;
            }
        }
    }
    // Nothing reaches the screen under the overlay while it is open.
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_vanilla_rules() {
        assert!(valid_name("Steve"));
        assert!(valid_name("a_b_1"));
        assert!(!valid_name("ab"));
        assert!(!valid_name("seventeen_chars_x"));
        assert!(!valid_name("bad name"));
        assert!(!valid_name("zażółć"));
    }

    #[test]
    fn store_adds_removes_and_selects() {
        let mut s = Store::default();
        s.add("Steve").unwrap();
        s.add("Alex").unwrap();
        assert!(s.add("steve").is_err(), "duplicates are case-insensitive");
        s.select(Some(1));
        assert_eq!(s.playing_offline(true), Some("Alex"));
        assert_eq!(s.playing_offline(false), None, "no Microsoft session, no offline play");
        assert_eq!(s.remove(1).as_deref(), Some("Alex"));
        assert_eq!(s.active, None, "removing the active one goes back to Microsoft");
    }

    #[test]
    fn store_round_trips_and_ignores_junk() {
        let mut s = Store::default();
        s.add("Steve").unwrap();
        s.add("Alex").unwrap();
        s.select(Some(0));
        assert_eq!(Store::parse(&s.to_json()), s);
        let junk = r#"{"offline":["ok_name","no spaces","ok_name",3],"active":"missing"}"#;
        let parsed = Store::parse(junk);
        assert_eq!(parsed.offline, vec!["ok_name".to_owned()]);
        assert_eq!(parsed.active, None);
        assert_eq!(Store::parse("not json"), Store::default());
    }

    #[test]
    fn keys_type_name_characters() {
        assert_eq!(key_char("key.keyboard.a", false), Some('a'));
        assert_eq!(key_char("key.keyboard.a", true), Some('A'));
        assert_eq!(key_char("key.keyboard.7", false), Some('7'));
        assert_eq!(key_char("key.keyboard.7", true), None);
        assert_eq!(key_char("key.keyboard.minus", true), Some('_'));
        assert_eq!(key_char("key.keyboard.minus", false), None);
        assert_eq!(key_char("key.keyboard.space", false), None);
        assert_eq!(key_char("key.keyboard.f1", false), None);
        assert_eq!(key_char("key.mouse.left", false), None);
    }

    #[test]
    fn layout_fits_small_and_large_windows() {
        for size in [[320.0, 240.0], [427.0, 240.0], [960.0, 540.0]] {
            let l = Layout::new(size);
            assert!(l.panel[1] >= 0.0 && l.panel[1] + l.panel[3] <= size[1], "{size:?}: {l:?}");
            assert!(l.panel[0] >= 0.0 && l.panel[0] + l.panel[2] <= size[0]);
            assert!(l.done[1] + l.done[3] <= l.panel[1] + l.panel[3]);
            assert!(l.visible_rows >= 1);
        }
    }

    #[test]
    fn clicks_pick_rows_remove_and_close() {
        let l = Layout::new([427.0, 240.0]);
        let mut s = Store::default();
        s.add("Steve").unwrap();
        let mut ui = Ui { open: true, ..Ui::default() };
        let mid = |r: [f32; 4]| [r[0] + 2.0, r[1] + r[3] / 2.0];
        assert_eq!(ui.click(&l, &s, Some("Owner"), mid(l.row(0))), Action::Select(None));
        assert_eq!(ui.click(&l, &s, Some("Owner"), mid(l.row(1))), Action::Select(Some(0)));
        let x = l.remove(1);
        assert_eq!(ui.click(&l, &s, Some("Owner"), [x[0] + 2.0, x[1] + 2.0]), Action::Remove(0));
        assert_eq!(ui.click(&l, &s, Some("Owner"), mid(l.add)), Action::Add);
        assert_eq!(ui.click(&l, &s, Some("Owner"), mid(l.done)), Action::Close);
        assert_eq!(ui.click(&l, &s, Some("Owner"), [1.0, 230.0]), Action::Close);
    }

    #[test]
    fn toggle_label_says_who_plays() {
        let mut s = Store::default();
        s.add("Steve").unwrap();
        assert_eq!(Ui::toggle_label(&s, Some("Owner")), "Account: Owner");
        s.select(Some(0));
        assert_eq!(Ui::toggle_label(&s, Some("Owner")), "Account: Steve (offline)");
        assert_eq!(Ui::toggle_label(&s, None), "Account: not signed in");
    }

    #[test]
    fn draw_without_microsoft_explains_why() {
        let l = Layout::new([427.0, 240.0]);
        let s = Store::default();
        let mut ui = Ui { open: true, ..Ui::default() };
        let draws = ui.draw(&l, &s, None, [0.0, 0.0], &|t| t.len() as f32 * 6.0);
        assert!(draws.iter().any(|d| matches!(d, Draw::Text(t, _, RED, _) if t.contains("Microsoft account"))));
    }
}
