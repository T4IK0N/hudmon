//! overlay window: always-on-top, click-through, with transparent background and live settings reload
use crate::{
    config::{self, Config},
    metrics::{
        fps::{FpsProvider, FpsSource},
        gpu::GpuProvider,
        lhm::LhmProvider,
        system::SystemProvider,
        Provider, Snapshot,
    },
    render,
};
use std::{
    rc::Rc,
    time::{Duration, Instant, SystemTime},
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder, WindowLevel},
};

#[cfg(windows)]
use crate::present_win::LayeredPresenter as Presenter;

#[cfg(not(windows))]
struct Presenter(softbuffer::Surface<Rc<Window>, Rc<Window>>);

#[cfg(not(windows))]
impl Presenter {
    fn present(&mut self, pixels: &[u32], w: usize, h: usize, _x: i32, _y: i32) {
        use std::num::NonZeroU32;
        let (Some(nw), Some(nh)) = (NonZeroU32::new(w as u32), NonZeroU32::new(h as u32)) else {
            return;
        };
        if self.0.resize(nw, nh).is_ok() {
            if let Ok(mut buf) = self.0.buffer_mut() {
                if buf.len() == pixels.len() {
                    buf.copy_from_slice(pixels);
                }
                let _ = buf.present();
            }
        }
    }
}

struct Overlay {
    window: Rc<Window>,
    presenter: Presenter,
    cfg: Config,
    cfg_time: Option<SystemTime>,
    snap: Snapshot,
    providers: Vec<Box<dyn Provider>>,
    pix: Vec<u32>,
    geo: (i32, i32, usize, usize),
}

/// size and position of the window (corner of the monitor selected in settings)
fn geometry(window: &Window, cfg: &Config) -> (i32, i32, usize, usize) {
    let (w, h) = render::measure(cfg);
    let mon = window
        .current_monitor()
        .or_else(|| window.primary_monitor());
    let (mx, my, mw, mh) = mon.map_or((0, 0, 1920, 1080), |m| {
        let (p, s) = (m.position(), m.size());
        (p.x, p.y, s.width as i32, s.height as i32)
    });
    let margin = 16;
    let x = if cfg.corner & 1 == 1 {
        mx + mw - w as i32 - margin
    } else {
        mx + margin
    };
    let y = if cfg.corner & 2 == 2 {
        my + mh - h as i32 - margin
    } else {
        my + margin
    };
    (x, y, w, h)
}

impl Overlay {
    fn tick(&mut self) {
        let t = config::mtime();
        if t != self.cfg_time {
            self.cfg_time = t;
            self.cfg = Config::load();
        }
        for p in self.providers.iter_mut() {
            p.sample(&mut self.snap);
        }
        self.frame();
    }

    fn frame(&mut self) {
        let geo = geometry(&self.window, &self.cfg);
        if geo != self.geo {
            self.geo = geo;
            #[cfg(not(windows))]
            {
                let _ = self
                    .window
                    .request_inner_size(PhysicalSize::new(geo.2 as u32, geo.3 as u32));
                self.window
                    .set_outer_position(winit::dpi::PhysicalPosition::new(geo.0, geo.1));
            }
        }
        let (x, y, w, h) = geo;
        self.pix.resize(w * h, 0);
        render::draw(&mut self.pix, w, h, &self.cfg, &self.snap);
        self.presenter.present(&self.pix, w, h, x, y);
    }
}

/// linux: on wayland window cannot set its own position or have alpha channel
/// (compositor puts it in the middle, background is opaque), therefore, when X11 (XWayland) is available, we force it
fn new_event_loop() -> EventLoop<()> {
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        if std::env::var("DISPLAY").map_or(false, |d| !d.is_empty()) {
            match EventLoopBuilder::new().with_x11().build() {
                Ok(el) => return el,
                Err(e) => crate::metrics::fps::log(&format!(
                    "X11 niedostepny ({e}) - uzywam domyslnego backendu"
                )),
            }
        } else if std::env::var("WAYLAND_DISPLAY").is_ok() {
            crate::metrics::fps::log(
                "Czysty Wayland (brak XWayland): brak pozycjonowania okna i przezroczystosci",
            );
        }
    }
    EventLoop::new().expect("event loop")
}

pub fn run(interval: Duration, fps: FpsSource, lhm_port: u16) {
    // one copy at the time
    let r = crate::instance::kill_others();
    if r.failed > 0 {
        crate::metrics::fps::log("Inna kopia hudmon dziala z wyzszymi uprawnieniami - zamknij ja (Menedzer zadan) lub uruchom jako administrator.");
        return;
    }
    let mut providers: Vec<Box<dyn Provider>> = vec![
        Box::new(SystemProvider::new()),
        Box::new(GpuProvider::new()),
        Box::new(FpsProvider::new(fps)),
    ];
    if cfg!(windows) {
        providers.push(Box::new(LhmProvider::new(lhm_port)));
    }
    crate::metrics::fps::log("providers ok");

    let cfg = Config::load();
    let (w, h) = render::measure(&cfg);
    let event_loop = new_event_loop();
    let mut wb = WindowBuilder::new()
        .with_title("hudmon")
        .with_decorations(false)
        .with_resizable(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_inner_size(PhysicalSize::new(w as u32, h as u32));
    #[cfg(not(windows))]
    {
        wb = wb.with_transparent(true); // ARGB window
    }
    #[cfg(windows)]
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        wb = wb.with_skip_taskbar(true);
    }
    let window = Rc::new(wb.build(&event_loop).expect("window"));
    crate::metrics::fps::log("window ok");
    let _ = window.set_cursor_hittest(false); // click-through (Windows, X11; not Wayland)

    #[cfg(windows)]
    let presenter = {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        match window.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(h)) => Presenter::new(h.hwnd.get()),
            _ => panic!("brak uchwytu okna Win32"),
        }
    };
    #[cfg(not(windows))]
    let presenter = {
        let context = softbuffer::Context::new(window.clone()).expect("context");
        Presenter(softbuffer::Surface::new(&context, window.clone()).expect("surface"))
    };

    let mut ov = Overlay {
        window,
        presenter,
        cfg_time: config::mtime(),
        cfg,
        snap: Snapshot::default(),
        providers,
        pix: Vec::new(),
        geo: (i32::MIN, 0, 0, 0),
    };

    event_loop
        .run(move |event, elwt| match event {
            Event::NewEvents(StartCause::Init | StartCause::ResumeTimeReached { .. }) => {
                ov.tick();
                elwt.set_control_flow(ControlFlow::WaitUntil(Instant::now() + interval));
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => ov.frame(),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => elwt.exit(),
            _ => {}
        })
        .expect("run");
}
