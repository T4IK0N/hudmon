//! apperance settings, the `hudmon.ini` file (key=value) is reloaded live by the overlay
use std::{fs, path::PathBuf, time::SystemTime};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Lang {
    Pl,
    En,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Config {
    pub fps: bool,
    pub cpu: bool,
    pub gpu: bool,
    pub ram: bool,
    pub temps: bool,
    pub labels: bool, // "FPS 144" vs "144"
    pub border: bool,
    pub font: u32,       // font size 1x..6x; 1x = 16 px (font 8x8 scalable x2)
    pub bg_color: u32,   // 0xRRGGBB
    pub bg_opacity: u32, // 0..=100 (%)
    pub corner: u8,      // 0 TL, 1 TR, 2 BL, 3 BR
    pub lang: Lang,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fps: true,
            cpu: true,
            gpu: true,
            ram: true,
            temps: true,
            labels: true,
            border: true,
            font: 1,
            bg_color: 0x0C0C0C,
            bg_opacity: 100,
            corner: 0,
            lang: Lang::Pl,
        }
    }
}

pub fn path() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("hudmon")
        .join("hudmon.ini")
}

pub fn mtime() -> Option<SystemTime> {
    fs::metadata(path()).ok()?.modified().ok()
}

impl Config {
    pub fn px_scale(&self) -> usize {
        self.font as usize + 1
    }

    pub fn load() -> Self {
        let mut c = Self::default();
        let Ok(text) = fs::read_to_string(path()) else {
            return c;
        };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            let b = || matches!(v.to_lowercase().as_str(), "1" | "true" | "on" | "yes");
            match k {
                "fps" => c.fps = b(),
                "cpu" => c.cpu = b(),
                "gpu" => c.gpu = b(),
                "ram" => c.ram = b(),
                "temps" => c.temps = b(),
                "labels" => c.labels = b(),
                "border" => c.border = b(),
                "font" => c.font = v.parse().unwrap_or(1).clamp(1, 6),
                "bg_opacity" => c.bg_opacity = v.parse().unwrap_or(100).min(100),
                "bg_color" => {
                    if let Ok(x) = u32::from_str_radix(v.trim_start_matches('#'), 16) {
                        c.bg_color = x & 0xFFFFFF;
                    }
                }
                "corner" => c.corner = v.parse().unwrap_or(0).min(3),
                "lang" => {
                    c.lang = if v.eq_ignore_ascii_case("en") {
                        Lang::En
                    } else {
                        Lang::Pl
                    }
                }
                _ => {}
            }
        }
        c
    }

    pub fn save(&self) {
        let p = path();
        if let Some(d) = p.parent() {
            let _ = fs::create_dir_all(d);
        }
        let on = |b: bool| if b { 1 } else { 0 };
        let text = format!(
            "# hudmon - ustawienia (mozna edytowac recznie; bg_color = hex RRGGBB)\nfps={}\ncpu={}\ngpu={}\nram={}\ntemps={}\nlabels={}\nborder={}\nfont={}\nbg_color={:06X}\nbg_opacity={}\ncorner={}\nlang={}\n",
            on(self.fps), on(self.cpu), on(self.gpu), on(self.ram), on(self.temps), on(self.labels),
            on(self.border), self.font, self.bg_color, self.bg_opacity, self.corner,
            if self.lang == Lang::En { "en" } else { "pl" }
        );
        let tmp = p.with_extension("tmp");
        if fs::write(&tmp, text).is_ok() {
            let _ = fs::rename(&tmp, &p);
        }
    }
}
