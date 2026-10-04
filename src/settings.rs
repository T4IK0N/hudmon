//! settings window (`hudmon.exe`) – in Polish or English. Changes are saved immediately to hudmon.ini,
//! and a running overlay (`hudmon-overlay.exe`) reloads the file in ~0.5 s
//! the window and the preview area have a fixed size; an overlay that is too
use crate::{
    config::{Config, Lang},
    instance,
    metrics::Snapshot,
    render::{self, fill_rect, put, text, DIM, LABEL, VALUE},
};
use std::{num::NonZeroU32, rc::Rc};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const WIN_W: usize = 400;
const WIN_H: usize = 708;
const PREVIEW: (usize, usize, usize, usize) = (16, 560, 368, 132); // x, y, width, height
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
}

struct Wd {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    act: Act,
}

fn widgets() -> Vec<Wd> {
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
    Changed,
    StartOverlay,
    CloseOverlay,
}

fn click(cfg: &mut Config, mx: i32, my: i32) -> Click {
    let before = cfg.clone();
    for wd in widgets() {
        if mx >= wd.x && mx < wd.x + wd.w && my >= wd.y && my < wd.y + wd.h {
            match wd.act {
                Act::StartOverlay => return Click::StartOverlay,
                Act::CloseOverlay => return Click::CloseOverlay,
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
            }
            break;
        }
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

fn draw_ui(buf: &mut [u32], cfg: &Config, status: Status) {
    let t = tr(cfg.lang);
    buf.fill(0xFF00_0000 | BG);
    text(buf, WIN_W, 16, 14, t.title, LABEL, 2);
    for wd in widgets() {
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

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);
            let Event::WindowEvent { event, .. } = event else {
                return;
            };
            match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::CursorMoved { position, .. } => {
                    mouse = (position.x as i32, position.y as i32)
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } => match click(&mut cfg, mouse.0, mouse.1) {
                    Click::Changed => {
                        cfg.save();
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
                    Click::Nothing => {}
                },
                WindowEvent::RedrawRequested => {
                    let (Some(w), Some(h)) =
                        (NonZeroU32::new(WIN_W as u32), NonZeroU32::new(WIN_H as u32))
                    else {
                        return;
                    };
                    if surface.resize(w, h).is_ok() {
                        if let Ok(mut buf) = surface.buffer_mut() {
                            draw_ui(&mut buf, &cfg, status);
                            let _ = buf.present();
                        }
                    }
                }
                _ => {}
            }
        })
        .expect("run");
}
