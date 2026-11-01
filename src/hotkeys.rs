//! keyboard shortcuts: the list of actions (shared with the settings window) + global hotkeys (Windows only).
//!
//! * the overlay is click-through and never has the focus, so the only way to control it from inside a game
//!   is a *global* hotkey (`RegisterHotKey`). It lives in the overlay process; most actions simply change
//!   `hudmon.ini` (the same file the settings window writes), so the overlay and the settings window stay in sync.
//! * show/hide is the exception: it is kept in memory only (`VISIBLE`) and is never written to the file,
//!   so a restarted overlay is always visible.
//! * bindings are strings like `ctrl+alt+f2` (see `parse_chord`) stored in `hudmon.ini` as `key_<action>=...`.
use crate::config::Config;
use std::sync::atomic::{AtomicBool, Ordering};

/// everything that can be triggered with a shortcut
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    ToggleVisible,
    Fps,
    Cpu,
    Gpu,
    Ram,
    Temps,
    Labels,
    Border,
    NextCorner,
    FontUp,
    FontDown,
    OpacityUp,
    OpacityDown,
}

/// order = index in `Config::keys`
pub const ALL: [Action; 13] = [
    Action::ToggleVisible,
    Action::Fps,
    Action::Cpu,
    Action::Gpu,
    Action::Ram,
    Action::Temps,
    Action::Labels,
    Action::Border,
    Action::NextCorner,
    Action::FontUp,
    Action::FontDown,
    Action::OpacityUp,
    Action::OpacityDown,
];

impl Action {
    /// name used in `hudmon.ini`: `key_<name>=...`
    pub fn name(self) -> &'static str {
        match self {
            Action::ToggleVisible => "visible",
            Action::Fps => "fps",
            Action::Cpu => "cpu",
            Action::Gpu => "gpu",
            Action::Ram => "ram",
            Action::Temps => "temps",
            Action::Labels => "labels",
            Action::Border => "border",
            Action::NextCorner => "corner",
            Action::FontUp => "font_up",
            Action::FontDown => "font_down",
            Action::OpacityUp => "opacity_up",
            Action::OpacityDown => "opacity_down",
        }
    }

    /// default global shortcut. Ctrl+Alt+<letter> is deliberately avoided: on many keyboard layouts
    /// (e.g. Polish) AltGr == Ctrl+Alt, so such a hotkey would swallow typing characters like ą, ć, ł.
    pub fn default_key(self) -> &'static str {
        match self {
            Action::ToggleVisible => "ctrl+alt+f1",
            Action::Fps => "ctrl+alt+f2",
            Action::Cpu => "ctrl+alt+f3",
            Action::Gpu => "ctrl+alt+f4",
            Action::Ram => "ctrl+alt+f5",
            Action::Temps => "ctrl+alt+f6",
            Action::Labels => "ctrl+alt+f7",
            Action::Border => "ctrl+alt+f8",
            Action::NextCorner => "ctrl+alt+f9",
            Action::FontUp => "ctrl+alt+plus",
            Action::FontDown => "ctrl+alt+minus",
            Action::OpacityUp => "ctrl+alt+pageup",
            Action::OpacityDown => "ctrl+alt+pagedown",
        }
    }
}

pub fn default_keys() -> Vec<String> {
    ALL.iter().map(|a| a.default_key().to_string()).collect()
}

/// index of an action by its ini name (`fps`, `font_up`, ...)
pub fn index_of(name: &str) -> Option<usize> {
    let name = name.trim().to_lowercase();
    ALL.iter().position(|a| a.name() == name)
}

/// the value of a binding that is switched off
pub const NONE: &str = "none";

/// validates a value from the ini file: `none` or a correct chord. Returns the normalized text.
pub fn normalize(v: &str) -> Option<String> {
    let v: String = v
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    if v.is_empty() || v == NONE || v == "off" {
        return Some(NONE.to_string());
    }
    parse_chord(&v).map(|_| v)
}

/// applies the action to the config. Returns true when something changed.
/// (`ToggleVisible` does not touch the config - see `VISIBLE`)
pub fn apply(a: Action, c: &mut Config) -> bool {
    let before = c.clone();
    match a {
        Action::ToggleVisible => {}
        Action::Fps => c.fps = !c.fps,
        Action::Cpu => c.cpu = !c.cpu,
        Action::Gpu => c.gpu = !c.gpu,
        Action::Ram => c.ram = !c.ram,
        Action::Temps => c.temps = !c.temps,
        Action::Labels => c.labels = !c.labels,
        Action::Border => c.border = !c.border,
        Action::NextCorner => c.corner = (c.corner + 1) % 4,
        Action::FontUp => c.font = (c.font + 1).min(6),
        Action::FontDown => c.font = c.font.saturating_sub(1).max(1),
        Action::OpacityUp => c.bg_opacity = (c.bg_opacity + 10).min(100),
        Action::OpacityDown => c.bg_opacity = c.bg_opacity.saturating_sub(10),
    }
    *c != before
}

/// overlay visibility (not saved to the file)
static VISIBLE: AtomicBool = AtomicBool::new(true);

pub fn visible() -> bool {
    VISIBLE.load(Ordering::Relaxed)
}

/// global shortcut: Windows virtual-key code + modifiers
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Chord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub vk: u32,
}

/// `"ctrl+alt+f2"` -> `Chord`. At least one modifier is required (a bare letter as a global hotkey
/// would block that key in every program). Key names: a-z, 0-9, f1-f24, numpad0-9, up/down/left/right,
/// pageup/pagedown, home/end, insert/delete, space, plus, minus.
pub fn parse_chord(s: &str) -> Option<Chord> {
    let mut c = Chord {
        ctrl: false,
        alt: false,
        shift: false,
        win: false,
        vk: 0,
    };
    for part in s.split('+') {
        match part.trim().to_lowercase().as_str() {
            "ctrl" | "control" => c.ctrl = true,
            "alt" => c.alt = true,
            "shift" => c.shift = true,
            "win" | "super" | "meta" => c.win = true,
            key => {
                if c.vk != 0 {
                    return None; // two non-modifier keys
                }
                c.vk = key_vk(key)?;
            }
        }
    }
    (c.vk != 0 && (c.ctrl || c.alt || c.shift || c.win)).then_some(c)
}

fn key_vk(k: &str) -> Option<u32> {
    let b = k.as_bytes();
    if b.len() == 1 {
        return match b[0] {
            b'a'..=b'z' => Some(0x41 + (b[0] - b'a') as u32),
            b'0'..=b'9' => Some(0x30 + (b[0] - b'0') as u32),
            _ => None,
        };
    }
    if let Some(n) = k.strip_prefix('f').and_then(|n| n.parse::<u32>().ok()) {
        return (1..=24).contains(&n).then(|| 0x70 + n - 1);
    }
    if let Some(n) = k.strip_prefix("numpad").and_then(|n| n.parse::<u32>().ok()) {
        return (n <= 9).then(|| 0x60 + n);
    }
    Some(match k {
        "up" => 0x26,
        "down" => 0x28,
        "left" => 0x25,
        "right" => 0x27,
        "pageup" | "pgup" => 0x21,
        "pagedown" | "pgdn" => 0x22,
        "home" => 0x24,
        "end" => 0x23,
        "insert" => 0x2D,
        "delete" | "del" => 0x2E,
        "space" => 0x20,
        "plus" => 0xBB,  // VK_OEM_PLUS
        "minus" => 0xBD, // VK_OEM_MINUS
        _ => return None,
    })
}

/// runs the action requested by a hotkey (called from the hotkey thread)
#[cfg(windows)]
fn trigger(a: Action) {
    if a == Action::ToggleVisible {
        VISIBLE.fetch_xor(true, Ordering::Relaxed);
        return;
    }
    let mut c = Config::load();
    if apply(a, &mut c) {
        c.save(); // the overlay notices the new file within ~0.5 s
    }
}

/// registers the global hotkeys and handles them on a separate thread.
/// Bindings are read once, at start (changing `key_*` in the ini needs an overlay restart).
#[cfg(windows)]
pub fn spawn(cfg: &Config) {
    use crate::metrics::fps::log;
    use std::{mem::zeroed, ptr::null_mut};
    use windows_sys::Win32::UI::{
        Input::KeyboardAndMouse::RegisterHotKey,
        WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY},
    };

    let binds: Vec<(Action, String)> = ALL.iter().copied().zip(cfg.keys.iter().cloned()).collect();
    std::thread::spawn(move || unsafe {
        let mut active: Vec<Action> = Vec::new(); // hotkey id = index in `active` + 1
        for (a, text) in &binds {
            let Some(ch) = parse_chord(text) else {
                continue; // "none"
            };
            let mods = (ch.alt as u32)
                | (ch.ctrl as u32) << 1
                | (ch.shift as u32) << 2
                | (ch.win as u32) << 3
                | 0x4000; // MOD_NOREPEAT
            if RegisterHotKey(null_mut(), active.len() as i32 + 1, mods, ch.vk) == 0 {
                log(&format!(
                    "Skrot '{text}' ({}) jest zajety przez inny program - pomijam.",
                    a.name()
                ));
            } else {
                active.push(*a);
            }
        }
        log(&format!(
            "Skroty klawiszowe: zarejestrowano {}.",
            active.len()
        ));
        if active.is_empty() {
            return;
        }
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            if msg.message == WM_HOTKEY {
                if let Some(a) = active.get((msg.wParam as usize).wrapping_sub(1)) {
                    trigger(*a);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_are_valid_and_unique() {
        let keys = default_keys();
        let chords: Vec<_> = keys.iter().map(|k| parse_chord(k).expect(k)).collect();
        for (i, a) in chords.iter().enumerate() {
            for b in &chords[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn parses_chords() {
        let c = parse_chord("Ctrl + Alt + F2").unwrap();
        assert!(c.ctrl && c.alt && !c.shift && !c.win);
        assert_eq!(c.vk, 0x71);
        assert_eq!(parse_chord("shift+a").unwrap().vk, 0x41);
        assert_eq!(parse_chord("win+5").unwrap().vk, 0x35);
        assert_eq!(parse_chord("ctrl+numpad3").unwrap().vk, 0x63);
    }

    #[test]
    fn rejects_bad_chords() {
        assert!(parse_chord("f2").is_none(), "needs a modifier");
        assert!(parse_chord("ctrl+alt").is_none(), "needs a key");
        assert!(parse_chord("ctrl+a+b").is_none(), "two keys");
        assert!(parse_chord("ctrl+f25").is_none());
        assert!(parse_chord("ctrl+ą").is_none());
        assert!(parse_chord("").is_none());
    }

    #[test]
    fn normalize_values() {
        assert_eq!(normalize(" CTRL+Alt+F3 ").as_deref(), Some("ctrl+alt+f3"));
        assert_eq!(normalize("none").as_deref(), Some("none"));
        assert_eq!(normalize("").as_deref(), Some("none"));
        assert_eq!(normalize("blah"), None);
    }

    #[test]
    fn index_of_names() {
        for (i, a) in ALL.iter().enumerate() {
            assert_eq!(index_of(a.name()), Some(i));
        }
        assert_eq!(index_of("nope"), None);
    }

    #[test]
    fn apply_changes_and_clamps() {
        let mut c = Config::default();
        assert!(apply(Action::Fps, &mut c) && !c.fps);
        assert!(apply(Action::Fps, &mut c) && c.fps);
        assert!(!apply(Action::ToggleVisible, &mut c));

        c.font = 6;
        assert!(!apply(Action::FontUp, &mut c));
        c.font = 1;
        assert!(!apply(Action::FontDown, &mut c));
        assert!(apply(Action::FontUp, &mut c) && c.font == 2);

        c.bg_opacity = 95;
        assert!(apply(Action::OpacityUp, &mut c) && c.bg_opacity == 100);
        assert!(!apply(Action::OpacityUp, &mut c));
        c.bg_opacity = 5;
        assert!(apply(Action::OpacityDown, &mut c) && c.bg_opacity == 0);
        assert!(!apply(Action::OpacityDown, &mut c));

        c.corner = 3;
        assert!(apply(Action::NextCorner, &mut c) && c.corner == 0);
    }
}
