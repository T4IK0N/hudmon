//! settings window (`hudmon.exe`) – in Polish or English. Changes are saved immediately to hudmon.ini,
//! and a running overlay (`hudmon-overlay.exe`) reloads the file in ~0.5 s
//! the window and the preview area have a fixed size; an overlay that is too
use crate::{
    config::{self, Config, Lang},
    hotkeys::{self, Action},
    instance,
    metrics::Snapshot,
    render::{self, fill_rect, put, text, DIM, LABEL, VALUE},
};
use std::{
    num::NonZeroU32,
    rc::Rc,
    time::{Duration, Instant},
};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyEvent, MouseButton, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::WindowBuilder,
};

const WIN_W: usize = 400;
const WIN_H: usize = 708;
const PREVIEW: (usize, usize, usize, usize) = (16, 560, 368, 132); // x, y, width, height
const HK_ROW_Y: i32 = 82; // first row on the hotkeys page
const BG: u32 = 0x181818;
const LIGHT: u32 = 0x808080;
const COLORS: [u32; 8] = [
    0x0C0C0C, 0x2B2B2B, 0x000040, 0x003020, 0x402000, 0x300030, 0x400000, 0x202830,
];

// texts (font 8x8 has only ascii, so polish without diacritics)
#[derive(Clone, Copy, PartialEq)]
enum Status {
    None,
    Started,
    MissingFile,
    NotRunning,
    Closed,
    Denied,
}

/// which screen of the settings window is shown
#[derive(Clone, Copy, PartialEq)]
enum Page {
    Main,
    Hotkeys,
}

/// message on the hotkeys page (index into `T::hk_msgs`)
#[derive(Clone, Copy, PartialEq)]
enum HkMsg {
    None,
    Capture,
    NeedModifier,
    Taken,
    AltGr,
}

/// state of the window that is not part of the saved settings
#[derive(Clone, Copy, PartialEq)]
struct Ui {
    page: Page,
    capture: Option<usize>, // index of the action whose shortcut is being recorded
    msg: HkMsg,
}

struct T {
    title: &'static str,
    window_title: &'static str,
    toggles: [&'static str; 7], // FPS, CPU, GPU, RAM, temperatures, labels, frame
    font: &'static str,
    opacity: &'static str,
    color: &'static str,
    position: &'static str,
    corners: [&'static str; 4],
    overlay: &'static str,
    start: &'static str,
    close: &'static str,
    preview: &'static str,
    status: [&'static str; 6],
    hk_open: &'static str,
    hk_title: &'static str,
    hk_back: &'static str,
    hk_master: &'static str,
    hk_names: [&'static str; 13], // order = `hotkeys::ALL`
    hk_none: &'static str,
    hk_reset: &'static str,
    hk_apply: &'static str,
    hk_msgs: [&'static str; 5], // order = `HkMsg`
    hk_help: [&'static str; 4],
}

fn tr(lang: Lang) -> T {
    match lang {
        Lang::Pl => T {
            title: "USTAWIENIA HUDMON",
            window_title: "hudmon - ustawienia",
            toggles: [
                "FPS",
                "CPU",
                "GPU",
                "RAM",
                "Temperatury",
                "Napisy (FPS, CPU...)",
                "Ramka",
            ],
            font: "Czcionka",
            opacity: "Krycie tla",
            color: "Kolor tla",
            position: "Pozycja na ekranie",
            corners: ["LG", "PG", "LD", "PD"],
            overlay: "Overlay",
            start: "Uruchom",
            close: "Zamknij",
            preview: "Podglad",
            status: [
                "",
                "Uruchomiono overlay",
                "Brak pliku overlay",
                "Overlay nie dziala",
                "Zamknieto overlay",
                "Brak uprawnien (admin)",
            ],
            hk_open: "Skroty >",
            hk_title: "SKROTY",
            hk_back: "Wroc",
            hk_master: "Skroty globalne",
            hk_names: [
                "Widocznosc",
                "FPS",
                "CPU",
                "GPU",
                "RAM",
                "Temp.",
                "Napisy",
                "Ramka",
                "Naroznik",
                "Czcionka +",
                "Czcionka -",
                "Krycie +",
                "Krycie -",
            ],
            hk_none: "wylaczony",
            hk_reset: "Domyslne",
            hk_apply: "Zastosuj",
            hk_msgs: [
                "",
                "Nacisnij skrot...",
                "Dodaj Ctrl/Alt/Shift",
                "Skrot jest juz uzyty",
                "Uwaga: Ctrl+Alt+litera",
            ],
            hk_help: [
                "Kliknij skrot, potem nacisnij nowa kombinacje.",
                "Esc anuluje, Del lub x wylacza skrot.",
                "Zmiany po restarcie overlay (Zastosuj).",
                "Ctrl+Alt+litera blokuje polskie znaki (AltGr).",
            ],
        },
        Lang::En => T {
            title: "HUDMON SETTINGS",
            window_title: "hudmon - settings",
            toggles: [
                "FPS",
                "CPU",
                "GPU",
                "RAM",
                "Temperatures",
                "Labels (FPS, CPU...)",
                "Border",
            ],
            font: "Font size",
            opacity: "BG opacity",
            color: "BG color",
            position: "Screen position",
            corners: ["TL", "TR", "BL", "BR"],
            overlay: "Overlay",
            start: "Start",
            close: "Close",
            preview: "Preview",
            status: [
                "",
                "Overlay started",
                "Overlay file missing",
                "Overlay not running",
                "Overlay closed",
                "Access denied (admin)",
            ],
            hk_open: "Hotkeys >",
            hk_title: "HOTKEYS",
            hk_back: "Back",
            hk_master: "Global hotkeys",
            hk_names: [
                "Show/hide",
                "FPS",
                "CPU",
                "GPU",
                "RAM",
                "Temp.",
                "Labels",
                "Border",
                "Corner",
                "Font +",
                "Font -",
                "Opacity +",
                "Opacity -",
            ],
            hk_none: "disabled",
            hk_reset: "Defaults",
            hk_apply: "Apply",
            hk_msgs: [
                "",
                "Press a shortcut...",
                "Add Ctrl/Alt/Shift",
                "Shortcut already used",
                "Warn: Ctrl+Alt+letter",
            ],
            hk_help: [
                "Click a shortcut, then press new keys.",
                "Esc cancels, Del or x disables the shortcut.",
                "Changes apply after overlay restart (Apply).",
                "Ctrl+Alt+letter blocks AltGr characters.",
            ],
        },
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Act {
    Fps,
    Cpu,
    Gpu,
    Ram,
    Temps,
    Labels,
    Border,
    Scale(i32),
    Opacity(i32),
    Color(u32),
    Corner(u8),
    StartOverlay,
    CloseOverlay,
    SetLang(Lang),
    OpenHotkeys,
    Back,
    HkMaster,
    HkEdit(usize),
    HkClear(usize),
    HkReset,
    HkApply,
}

struct Wd {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    act: Act,
}

/// widgets of the hotkeys page: Back, master switch, a row per action (shortcut + disable button), Defaults, Apply
fn hotkey_widgets() -> Vec<Wd> {
    let mut v = vec![
        Wd {
            x: 300,
            y: 8,
            w: 84,
            h: 28,
            act: Act::Back,
        },
        Wd {
            x: 16,
            y: 46,
            w: 368,
            h: 24,
            act: Act::HkMaster,
        },
    ];
    for i in 0..hotkeys::ALL.len() {
        let y = HK_ROW_Y + i as i32 * 30;
        v.push(Wd {
            x: 196,
            y,
            w: 148,
            h: 28,
            act: Act::HkEdit(i),
        });
        v.push(Wd {
            x: 348,
            y,
            w: 36,
            h: 28,
            act: Act::HkClear(i),
        });
    }
    v.push(Wd {
        x: 16,
        y: 484,
        w: 144,
        h: 28,
        act: Act::HkReset,
    });
    v.push(Wd {
        x: 168,
        y: 484,
        w: 144,
        h: 28,
        act: Act::HkApply,
    });
    v
}

fn widgets(page: Page) -> Vec<Wd> {
    if page == Page::Hotkeys {
        return hotkey_widgets();
    }
    let mut v = Vec::new();
    let toggles = [
        Act::Fps,
        Act::Cpu,
        Act::Gpu,
        Act::Ram,
        Act::Temps,
        Act::Labels,
        Act::Border,
    ];
    for (i, a) in toggles.iter().enumerate() {
        v.push(Wd {
            x: 16,
            y: 46 + i as i32 * 26,
            w: 368,
            h: 24,
            act: *a,
        });
    }
    for (row, (m, p)) in [
        (Act::Scale(-1), Act::Scale(1)),
        (Act::Opacity(-10), Act::Opacity(10)),
    ]
    .iter()
    .enumerate()
    {
        let y = 240 + row as i32 * 34;
        v.push(Wd {
            x: 192,
            y,
            w: 32,
            h: 28,
            act: *m,
        });
        v.push(Wd {
            x: 304,
            y,
            w: 32,
            h: 28,
            act: *p,
        });
    }
    for (i, c) in COLORS.iter().enumerate() {
        v.push(Wd {
            x: 16 + i as i32 * 44,
            y: 336,
            w: 36,
            h: 36,
            act: Act::Color(*c),
        });
    }
    for i in 0..4u8 {
        v.push(Wd {
            x: 16 + i as i32 * 72,
            y: 408,
            w: 64,
            h: 28,
            act: Act::Corner(i),
        });
    }
    v.push(Wd {
        x: 16,
        y: 474,
        w: 136,
        h: 28,
        act: Act::StartOverlay,
    });
    v.push(Wd {
        x: 160,
        y: 474,
        w: 136,
        h: 28,
        act: Act::CloseOverlay,
    });
    v.push(Wd {
        x: 300,
        y: 8,
        w: 40,
        h: 28,
        act: Act::SetLang(Lang::Pl),
    });
    v.push(Wd {
        x: 344,
        y: 8,
        w: 40,
        h: 28,
        act: Act::SetLang(Lang::En),
    });
    if cfg!(windows) {
        // global hotkeys exist on Windows only
        v.push(Wd {
            x: 232,
            y: 446,
            w: 152,
            h: 24,
            act: Act::OpenHotkeys,
        });
    }
    v
}

fn toggle_state(cfg: &Config, a: Act) -> (bool, usize) {
    match a {
        Act::Fps => (cfg.fps, 0),
        Act::Cpu => (cfg.cpu, 1),
        Act::Gpu => (cfg.gpu, 2),
        Act::Ram => (cfg.ram, 3),
        Act::Temps => (cfg.temps, 4),
        Act::Labels => (cfg.labels, 5),
        _ => (cfg.border, 6),
    }
}

enum Click {
    Nothing,
    Redraw, // only the window state changed (page, recording), nothing to save
    Changed,
    StartOverlay,
    CloseOverlay,
    ApplyHotkeys,
}

fn click(cfg: &mut Config, ui: &mut Ui, mx: i32, my: i32) -> Click {
    let (before, ui_before) = (cfg.clone(), *ui);
    for wd in widgets(ui.page) {
        if mx >= wd.x && mx < wd.x + wd.w && my >= wd.y && my < wd.y + wd.h {
            if !matches!(wd.act, Act::HkEdit(_)) {
                ui.capture = None; // any other click ends the recording
                ui.msg = HkMsg::None;
            }
            match wd.act {
                Act::StartOverlay => return Click::StartOverlay,
                Act::CloseOverlay => return Click::CloseOverlay,
                Act::HkApply => return Click::ApplyHotkeys,
                Act::Fps => cfg.fps = !cfg.fps,
                Act::Cpu => cfg.cpu = !cfg.cpu,
                Act::Gpu => cfg.gpu = !cfg.gpu,
                Act::Ram => cfg.ram = !cfg.ram,
                Act::Temps => cfg.temps = !cfg.temps,
                Act::Labels => cfg.labels = !cfg.labels,
                Act::Border => cfg.border = !cfg.border,
                Act::Scale(d) => cfg.font = (cfg.font as i32 + d).clamp(1, 6) as u32,
                Act::Opacity(d) => {
                    cfg.bg_opacity = (cfg.bg_opacity as i32 + d).clamp(0, 100) as u32
                }
                Act::Color(c) => cfg.bg_color = c,
                Act::Corner(n) => cfg.corner = n,
                Act::SetLang(l) => cfg.lang = l,
                Act::OpenHotkeys => ui.page = Page::Hotkeys,
                Act::Back => ui.page = Page::Main,
                Act::HkMaster => cfg.hotkeys = !cfg.hotkeys,
                Act::HkEdit(i) => {
                    if ui.capture == Some(i) {
                        ui.capture = None;
                        ui.msg = HkMsg::None;
                    } else {
                        ui.capture = Some(i);
                        ui.msg = HkMsg::Capture;
                    }
                }
                Act::HkClear(i) => cfg.keys[i] = hotkeys::NONE.to_string(),
                Act::HkReset => cfg.keys = hotkeys::default_keys(),
            }
            break;
        }
    }
    if *cfg != before {
        Click::Changed
    } else if *ui != ui_before {
        Click::Redraw
    } else {
        Click::Nothing
    }
}

/// name of a physical key in the notation of `hotkeys::parse_chord` (`None` for modifiers and unsupported keys)
fn code_name(code: KeyCode) -> Option<&'static str> {
    use KeyCode::*;
    Some(match code {
        KeyA => "a",
        KeyB => "b",
        KeyC => "c",
        KeyD => "d",
        KeyE => "e",
        KeyF => "f",
        KeyG => "g",
        KeyH => "h",
        KeyI => "i",
        KeyJ => "j",
        KeyK => "k",
        KeyL => "l",
        KeyM => "m",
        KeyN => "n",
        KeyO => "o",
        KeyP => "p",
        KeyQ => "q",
        KeyR => "r",
        KeyS => "s",
        KeyT => "t",
        KeyU => "u",
        KeyV => "v",
        KeyW => "w",
        KeyX => "x",
        KeyY => "y",
        KeyZ => "z",
        Digit0 => "0",
        Numpad0 => "numpad0",
        Digit1 => "1",
        Numpad1 => "numpad1",
        Digit2 => "2",
        Numpad2 => "numpad2",
        Digit3 => "3",
        Numpad3 => "numpad3",
        Digit4 => "4",
        Numpad4 => "numpad4",
        Digit5 => "5",
        Numpad5 => "numpad5",
        Digit6 => "6",
        Numpad6 => "numpad6",
        Digit7 => "7",
        Numpad7 => "numpad7",
        Digit8 => "8",
        Numpad8 => "numpad8",
        Digit9 => "9",
        Numpad9 => "numpad9",
        F1 => "f1",
        F2 => "f2",
        F3 => "f3",
        F4 => "f4",
        F5 => "f5",
        F6 => "f6",
        F7 => "f7",
        F8 => "f8",
        F9 => "f9",
        F10 => "f10",
        F11 => "f11",
        F12 => "f12",
        F13 => "f13",
        F14 => "f14",
        F15 => "f15",
        F16 => "f16",
        F17 => "f17",
        F18 => "f18",
        F19 => "f19",
        F20 => "f20",
        F21 => "f21",
        F22 => "f22",
        F23 => "f23",
        F24 => "f24",
        ArrowUp => "up",
        ArrowDown => "down",
        ArrowLeft => "left",
        ArrowRight => "right",
        PageUp => "pageup",
        PageDown => "pagedown",
        Home => "home",
        End => "end",
        Insert => "insert",
        Delete => "delete",
        Space => "space",
        Equal => "plus",
        Minus => "minus",
        _ => return None,
    })
}

/// hotkeys page: records the shortcut of the selected row (modifiers + key); Esc cancels, Del/Backspace disables
fn hotkey_key(cfg: &mut Config, ui: &mut Ui, code: KeyCode, mods: ModifiersState) -> Click {
    let Some(i) = ui.capture else {
        if code == KeyCode::Escape {
            ui.page = Page::Main;
            ui.msg = HkMsg::None;
            return Click::Redraw;
        }
        return Click::Nothing;
    };
    let any_mod = mods.control_key() || mods.alt_key() || mods.shift_key() || mods.super_key();
    match code {
        KeyCode::Escape => {
            ui.capture = None;
            ui.msg = HkMsg::None;
            return Click::Redraw;
        }
        KeyCode::Delete | KeyCode::Backspace if !any_mod => {
            cfg.keys[i] = hotkeys::NONE.to_string();
            ui.capture = None;
            ui.msg = HkMsg::None;
            return Click::Changed;
        }
        _ => {}
    }
    let Some(name) = code_name(code) else {
        return Click::Nothing; // only a modifier so far - keep waiting
    };
    let Some(chord) = hotkeys::compose(
        mods.control_key(),
        mods.alt_key(),
        mods.shift_key(),
        mods.super_key(),
        name,
    ) else {
        ui.msg = HkMsg::NeedModifier;
        return Click::Redraw;
    };
    if hotkeys::taken(&cfg.keys, i, &chord) {
        ui.msg = HkMsg::Taken;
        return Click::Redraw;
    }
    ui.msg = if hotkeys::altgr_risky(&chord) {
        HkMsg::AltGr
    } else {
        HkMsg::None
    };
    ui.capture = None;
    cfg.keys[i] = chord;
    Click::Changed
}

/// keys inside the settings window (the window must have the focus; the global hotkeys are in `hotkeys.rs`)
/// 1-7 toggles, Up/Down font, Left/Right opacity, C corner, L language, K hotkeys page (Windows),
/// Enter start, Delete close
fn key_press(cfg: &mut Config, ui: &mut Ui, code: KeyCode, mods: ModifiersState) -> Click {
    if ui.page == Page::Hotkeys {
        return hotkey_key(cfg, ui, code, mods);
    }
    let before = cfg.clone();
    let action = match code {
        KeyCode::Enter | KeyCode::NumpadEnter => return Click::StartOverlay,
        KeyCode::Delete => return Click::CloseOverlay,
        KeyCode::KeyK if cfg!(windows) => {
            ui.page = Page::Hotkeys;
            return Click::Redraw;
        }
        KeyCode::KeyL => {
            cfg.lang = if cfg.lang == Lang::Pl {
                Lang::En
            } else {
                Lang::Pl
            };
            None
        }
        KeyCode::Digit1 | KeyCode::Numpad1 => Some(Action::Fps),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some(Action::Cpu),
        KeyCode::Digit3 | KeyCode::Numpad3 => Some(Action::Gpu),
        KeyCode::Digit4 | KeyCode::Numpad4 => Some(Action::Ram),
        KeyCode::Digit5 | KeyCode::Numpad5 => Some(Action::Temps),
        KeyCode::Digit6 | KeyCode::Numpad6 => Some(Action::Labels),
        KeyCode::Digit7 | KeyCode::Numpad7 => Some(Action::Border),
        KeyCode::KeyC => Some(Action::NextCorner),
        KeyCode::ArrowUp => Some(Action::FontUp),
        KeyCode::ArrowDown => Some(Action::FontDown),
        KeyCode::ArrowRight => Some(Action::OpacityUp),
        KeyCode::ArrowLeft => Some(Action::OpacityDown),
        _ => None,
    };
    if let Some(a) = action {
        hotkeys::apply(a, cfg);
    }
    if *cfg != before {
        Click::Changed
    } else {
        Click::Nothing
    }
}

fn frame(buf: &mut [u32], x: i32, y: i32, w: i32, h: i32, border: u32, fill: u32) {
    fill_rect(
        buf, WIN_W, x as usize, y as usize, w as usize, h as usize, border,
    );
    fill_rect(
        buf,
        WIN_W,
        x as usize + 1,
        y as usize + 1,
        w as usize - 2,
        h as usize - 2,
        fill,
    );
}

fn draw_ui(buf: &mut [u32], cfg: &Config, status: Status, ui: Ui) {
    if ui.page == Page::Hotkeys {
        draw_hotkeys(buf, cfg, status, ui);
        return;
    }
    let t = tr(cfg.lang);
    buf.fill(0xFF00_0000 | BG);
    text(buf, WIN_W, 16, 14, t.title, LABEL, 2);
    for wd in widgets(Page::Main) {
        let (x, y) = (wd.x, wd.y);
        match wd.act {
            Act::Scale(d) | Act::Opacity(d) => {
                frame(buf, x, y, wd.w, wd.h, LIGHT, 0x303030);
                text(
                    buf,
                    WIN_W,
                    x as usize + 8,
                    y as usize + 6,
                    if d < 0 { "-" } else { "+" },
                    VALUE,
                    2,
                );
            }
            Act::Color(c) => {
                let sel = cfg.bg_color == c;
                frame(buf, x, y, wd.w, wd.h, if sel { VALUE } else { LIGHT }, c);
                if sel {
                    frame(buf, x + 1, y + 1, wd.w - 2, wd.h - 2, VALUE, c);
                }
            }
            Act::Corner(n) => {
                let sel = cfg.corner == n;
                frame(
                    buf,
                    x,
                    y,
                    wd.w,
                    wd.h,
                    if sel { VALUE } else { LIGHT },
                    if sel { 0x205030 } else { 0x303030 },
                );
                text(
                    buf,
                    WIN_W,
                    x as usize + 16,
                    y as usize + 6,
                    t.corners[n as usize],
                    if sel { VALUE } else { DIM },
                    2,
                );
            }
            Act::StartOverlay => {
                frame(buf, x, y, wd.w, wd.h, LIGHT, 0x205030);
                text(
                    buf,
                    WIN_W,
                    x as usize + 8,
                    y as usize + 6,
                    t.start,
                    VALUE,
                    2,
                );
            }
            Act::CloseOverlay => {
                frame(buf, x, y, wd.w, wd.h, LIGHT, 0x402020);
                text(
                    buf,
                    WIN_W,
                    x as usize + 8,
                    y as usize + 6,
                    t.close,
                    0xFF8080,
                    2,
                );
            }
            Act::SetLang(l) => {
                let sel = cfg.lang == l;
                frame(
                    buf,
                    x,
                    y,
                    wd.w,
                    wd.h,
                    if sel { VALUE } else { LIGHT },
                    if sel { 0x205030 } else { 0x303030 },
                );
                text(
                    buf,
                    WIN_W,
                    x as usize + 4,
                    y as usize + 6,
                    if l == Lang::Pl { "PL" } else { "EN" },
                    if sel { VALUE } else { DIM },
                    2,
                );
            }
            Act::OpenHotkeys => {
                frame(buf, x, y, wd.w, wd.h, LIGHT, 0x303030);
                text(
                    buf,
                    WIN_W,
                    x as usize + 4,
                    y as usize + 4,
                    t.hk_open,
                    VALUE,
                    2,
                );
            }
            Act::Back
            | Act::HkMaster
            | Act::HkEdit(_)
            | Act::HkClear(_)
            | Act::HkReset
            | Act::HkApply => {} // widgets of the hotkeys page
            a => {
                let (on, i) = toggle_state(cfg, a);
                frame(buf, x, y + 2, 20, 20, LIGHT, 0x000000);
                if on {
                    fill_rect(buf, WIN_W, x as usize + 5, y as usize + 7, 10, 10, VALUE);
                }
                text(
                    buf,
                    WIN_W,
                    x as usize + 32,
                    y as usize + 4,
                    t.toggles[i],
                    if on { VALUE } else { DIM },
                    2,
                );
            }
        }
    }
    text(buf, WIN_W, 16, 244, t.font, LABEL, 2);
    text(buf, WIN_W, 232, 244, &format!("{}x", cfg.font), VALUE, 2);
    text(buf, WIN_W, 16, 278, t.opacity, LABEL, 2);
    text(
        buf,
        WIN_W,
        232,
        278,
        &format!("{}%", cfg.bg_opacity),
        VALUE,
        2,
    );
    text(buf, WIN_W, 16, 314, t.color, LABEL, 2);
    text(buf, WIN_W, 16, 386, t.position, LABEL, 2);
    text(buf, WIN_W, 16, 452, t.overlay, LABEL, 2);
    text(buf, WIN_W, 16, 510, t.status[status as usize], DIM, 2);
    text(buf, WIN_W, 16, 538, t.preview, LABEL, 2);

    let (px, py, pw, ph) = PREVIEW;
    let snap = Snapshot {
        cpu: 23.0,
        ram_used: 8_700_000_000,
        ram_total: 17_179_869_184,
        gpu: Some(45.0),
        fps: Some(144.0),
        cpu_temp: Some(65.0),
        gpu_temp: Some(58.0),
    };
    let (w, h) = render::measure(cfg);
    let mut ov = vec![0u32; w * h];
    render::draw(&mut ov, w, h, cfg, &snap);
    let f = ((pw - 16) as f32 / w as f32)
        .min((ph - 16) as f32 / h as f32)
        .min(1.0);
    let (dw, dh) = ((w as f32 * f) as usize, (h as f32 * f) as usize);
    for y in 0..ph {
        for x in 0..pw {
            let chk = if (x / 8 + y / 8) % 2 == 0 {
                0x505050u32
            } else {
                0x383838
            };
            let (ox, oy) = (x as isize - 8, y as isize - 8);
            let rgb = if ox >= 0 && oy >= 0 && (ox as usize) < dw && (oy as usize) < dh {
                let (sx, sy) = (
                    ((ox as f32 / f) as usize).min(w - 1),
                    ((oy as f32 / f) as usize).min(h - 1),
                );
                let p = ov[sy * w + sx];
                let a = p >> 24;
                let ch =
                    |s: u32| (((p >> s) & 0xFF) + ((chk >> s) & 0xFF) * (255 - a) / 255).min(255);
                (ch(16) << 16) | (ch(8) << 8) | ch(0)
            } else {
                chk
            };
            put(buf, WIN_W, px + x, py + y, rgb);
        }
    }
}

fn draw_hotkeys(buf: &mut [u32], cfg: &Config, status: Status, ui: Ui) {
    let t = tr(cfg.lang);
    buf.fill(0xFF00_0000 | BG);
    text(buf, WIN_W, 16, 14, t.hk_title, LABEL, 2);
    for wd in widgets(Page::Hotkeys) {
        let (x, y) = (wd.x, wd.y);
        match wd.act {
            Act::Back => {
                frame(buf, x, y, wd.w, wd.h, LIGHT, 0x303030);
                text(
                    buf,
                    WIN_W,
                    x as usize + 10,
                    y as usize + 6,
                    t.hk_back,
                    VALUE,
                    2,
                );
            }
            Act::HkMaster => {
                let on = cfg.hotkeys;
                frame(buf, x, y + 2, 20, 20, LIGHT, 0x000000);
                if on {
                    fill_rect(buf, WIN_W, x as usize + 5, y as usize + 7, 10, 10, VALUE);
                }
                text(
                    buf,
                    WIN_W,
                    x as usize + 32,
                    y as usize + 4,
                    t.hk_master,
                    if on { VALUE } else { DIM },
                    2,
                );
            }
            Act::HkEdit(i) => {
                let rec = ui.capture == Some(i);
                frame(
                    buf,
                    x,
                    y,
                    wd.w,
                    wd.h,
                    if rec { VALUE } else { LIGHT },
                    if rec { 0x205030 } else { 0x303030 },
                );
                let key = &cfg.keys[i];
                let (label, col) = if rec {
                    ("...".to_string(), VALUE)
                } else if key == hotkeys::NONE {
                    (t.hk_none.to_string(), DIM)
                } else {
                    (
                        hotkeys::pretty(key, 17),
                        if cfg.hotkeys { VALUE } else { DIM },
                    )
                };
                text(buf, WIN_W, x as usize + 6, y as usize + 10, &label, col, 1);
            }
            Act::HkClear(_) => {
                frame(buf, x, y, wd.w, wd.h, LIGHT, 0x402020);
                text(
                    buf,
                    WIN_W,
                    x as usize + 10,
                    y as usize + 6,
                    "x",
                    0xFF8080,
                    2,
                );
            }
            Act::HkReset => {
                frame(buf, x, y, wd.w, wd.h, LIGHT, 0x303030);
                text(
                    buf,
                    WIN_W,
                    x as usize + 8,
                    y as usize + 6,
                    t.hk_reset,
                    VALUE,
                    2,
                );
            }
            Act::HkApply => {
                frame(buf, x, y, wd.w, wd.h, LIGHT, 0x205030);
                text(
                    buf,
                    WIN_W,
                    x as usize + 8,
                    y as usize + 6,
                    t.hk_apply,
                    VALUE,
                    2,
                );
            }
            _ => {}
        }
    }
    for (i, name) in t.hk_names.iter().enumerate() {
        let y = HK_ROW_Y as usize + i * 30 + 6;
        text(
            buf,
            WIN_W,
            16,
            y,
            name,
            if cfg.hotkeys { LABEL } else { DIM },
            2,
        );
    }
    let msg = if ui.msg != HkMsg::None {
        t.hk_msgs[ui.msg as usize]
    } else {
        t.status[status as usize]
    };
    text(buf, WIN_W, 16, 524, msg, VALUE, 2);
    for (i, line) in t.hk_help.iter().enumerate() {
        text(buf, WIN_W, 16, 560 + i * 16, line, DIM, 1);
    }
}

pub fn run() {
    let mut cfg = Config::load();
    let event_loop = EventLoop::new().expect("event loop");
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut wb = WindowBuilder::new()
        .with_title(tr(cfg.lang).window_title)
        .with_inner_size(PhysicalSize::new(WIN_W as u32, WIN_H as u32))
        .with_resizable(false);

    #[cfg(windows)]
    {
        use winit::{platform::windows::IconExtWindows, window::Icon};

        wb = wb.with_window_icon(Icon::from_resource(1, None).ok());
    }

    #[cfg(target_os = "linux")]
    {
        use winit::window::Icon;

        let icon = image::load_from_memory_with_format(
            include_bytes!("../assets/icon.ico"),
            image::ImageFormat::Ico,
        )
        .ok()
        .and_then(|img| {
            let rgba = img.into_rgba8();
            let (width, height) = rgba.dimensions();

            Icon::from_rgba(rgba.into_raw(), width, height).ok()
        });

        wb = wb.with_window_icon(icon);
    }

    #[cfg(target_os = "linux")]
    {
        use winit::platform::wayland::WindowBuilderExtWayland;
        wb = wb.with_name("hudmon", "hudmon");
    }

    let window = Rc::new(wb.build(&event_loop).expect("window"));
    let context = softbuffer::Context::new(window.clone()).expect("context");
    let mut surface = softbuffer::Surface::new(&context, window.clone()).expect("surface");
    let mut mouse = (0i32, 0i32);
    let mut status = Status::None;
    let mut ui = Ui {
        page: Page::Main,
        capture: None,
        msg: HkMsg::None,
    };
    let mut mods = ModifiersState::default();
    let mut cfg_time = config::mtime();

    event_loop
        .run(move |event, elwt| {
            // wake up twice a second: the file can be changed by a global hotkey of the overlay
            elwt.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(500),
            ));
            let event = match event {
                Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                    let t = config::mtime();
                    if t != cfg_time {
                        cfg_time = t;
                        let new = Config::load();
                        if new != cfg {
                            cfg = new;
                            window.set_title(tr(cfg.lang).window_title);
                            window.request_redraw();
                        }
                    }
                    return;
                }
                Event::WindowEvent { event, .. } => event,
                _ => return,
            };
            let res = match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                    return;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    mouse = (position.x as i32, position.y as i32);
                    return;
                }
                WindowEvent::ModifiersChanged(m) => {
                    mods = m.state();
                    return;
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } => click(&mut cfg, &mut ui, mouse.0, mouse.1),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(code),
                            state: ElementState::Pressed,
                            repeat,
                            ..
                        },
                    ..
                } => {
                    // holding a key repeats only the sliders (arrows), never toggles or Start/Close
                    let arrow = matches!(
                        code,
                        KeyCode::ArrowUp
                            | KeyCode::ArrowDown
                            | KeyCode::ArrowLeft
                            | KeyCode::ArrowRight
                    );
                    if repeat && !arrow {
                        return;
                    }
                    key_press(&mut cfg, &mut ui, code, mods)
                }
                WindowEvent::RedrawRequested => {
                    let (Some(w), Some(h)) =
                        (NonZeroU32::new(WIN_W as u32), NonZeroU32::new(WIN_H as u32))
                    else {
                        return;
                    };
                    if surface.resize(w, h).is_ok() {
                        if let Ok(mut buf) = surface.buffer_mut() {
                            draw_ui(&mut buf, &cfg, status, ui);
                            let _ = buf.present();
                        }
                    }
                    return;
                }
                _ => return,
            };
            match res {
                Click::Changed => {
                    cfg.save();
                    cfg_time = config::mtime();
                    window.set_title(tr(cfg.lang).window_title);
                    window.request_redraw();
                }
                Click::StartOverlay => {
                    status = if instance::start_overlay() {
                        Status::Started
                    } else {
                        Status::MissingFile
                    };
                    window.request_redraw();
                }
                Click::CloseOverlay => {
                    let r = instance::kill_others();
                    status = match (r.killed, r.failed) {
                        (0, 0) => Status::NotRunning,
                        (_, 0) => Status::Closed,
                        _ => Status::Denied,
                    };
                    window.request_redraw();
                }
                Click::Redraw => window.request_redraw(),
                Click::ApplyHotkeys => {
                    // the overlay reads its hotkeys only at start: restart it if it is running
                    let r = instance::kill_others();
                    status = if r.failed > 0 {
                        Status::Denied
                    } else if r.killed == 0 {
                        Status::NotRunning
                    } else if instance::start_overlay() {
                        Status::Started
                    } else {
                        Status::MissingFile
                    };
                    ui.msg = HkMsg::None;
                    window.request_redraw();
                }
                Click::Nothing => {}
            }
        })
        .expect("run");
}
