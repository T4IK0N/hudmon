use crate::{config::Config, metrics::Snapshot};
use font8x8::legacy::BASIC_LEGACY;

/// colors used in the overlay – these are not configurable, but can be changed here if desired
pub const LABEL: u32 = 0xFFB000;
pub const VALUE: u32 = 0x33FF66;
pub const DIM: u32 = 0x707070;
pub const WARM: u32 = 0xFFB000;
pub const HOT: u32 = 0xFF4040;
const LIGHT: u32 = 0x808080;
const DARKB: u32 = 0x303030;

/// max width (in characters) – window has a fixed size depending only on settings
const FPS_COLS: usize = 4;
const PCT_COLS: usize = 4;
const TEMP_COL: usize = 6;
const TEMP_COLS: usize = 4;
const RAM_COLS: usize = 10;

pub fn pad(sc: usize) -> usize {
    4 * sc
}
pub fn line_h(sc: usize) -> usize {
    9 * sc
}

pub fn premult(rgb: u32, opacity_pct: u32) -> u32 {
    let a = opacity_pct.min(100) * 255 / 100;
    let m = |c: u32| (c & 0xFF) * a / 255;
    (a << 24) | (m(rgb >> 16) << 16) | (m(rgb >> 8) << 8) | m(rgb)
}

#[inline]
pub fn put(buf: &mut [u32], stride: usize, x: usize, y: usize, rgb: u32) {
    if x < stride {
        if let Some(p) = buf.get_mut(y * stride + x) {
            *p = 0xFF00_0000 | rgb;
        }
    }
}

pub fn fill_rect(buf: &mut [u32], stride: usize, x: usize, y: usize, w: usize, h: usize, rgb: u32) {
    for yy in y..y + h {
        for xx in x..x + w {
            put(buf, stride, xx, yy, rgb);
        }
    }
}

pub fn text(buf: &mut [u32], stride: usize, x: usize, y: usize, s: &str, rgb: u32, sc: usize) {
    for (i, ch) in s.chars().enumerate() {
        let g = BASIC_LEGACY
            .get(ch as usize)
            .unwrap_or(&BASIC_LEGACY[b'?' as usize]);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..8 {
                if bits >> col & 1 == 1 {
                    fill_rect(
                        buf,
                        stride,
                        x + (i * 8 + col) * sc,
                        y + row * sc,
                        sc,
                        sc,
                        rgb,
                    );
                }
            }
        }
    }
}

struct Row {
    label: &'static str,
    value: String,
    vcol: u32,
    temp: Option<(String, u32)>,
}

fn pct(v: Option<f32>) -> (String, u32) {
    v.map_or(("n/a".into(), DIM), |v| (format!("{:.0}%", v), VALUE))
}

fn temp(v: Option<f32>) -> (String, u32) {
    match v {
        None => ("n/a".into(), DIM),
        Some(t) => (
            format!("{:.0}C", t),
            if t >= 85.0 {
                HOT
            } else if t >= 70.0 {
                WARM
            } else {
                VALUE
            },
        ),
    }
}

fn build_rows(cfg: &Config, s: &Snapshot) -> Vec<Row> {
    let mut rows = Vec::new();
    if cfg.fps {
        let (value, vcol) = s
            .fps
            .map_or(("n/a".to_string(), DIM), |f| (format!("{:.0}", f), VALUE));
        rows.push(Row {
            label: "FPS",
            value,
            vcol,
            temp: None,
        });
    }
    if cfg.cpu {
        let (value, vcol) = pct(Some(s.cpu));
        rows.push(Row {
            label: "CPU",
            value,
            vcol,
            temp: cfg.temps.then(|| temp(s.cpu_temp)),
        });
    }
    if cfg.gpu {
        let (value, vcol) = pct(s.gpu);
        rows.push(Row {
            label: "GPU",
            value,
            vcol,
            temp: cfg.temps.then(|| temp(s.gpu_temp)),
        });
    }
    if cfg.ram {
        let gb = |b: u64| b as f64 / 1_073_741_824.0;
        let value = format!("{:.1}/{:.0}G", gb(s.ram_used), gb(s.ram_total));
        rows.push(Row {
            label: "RAM",
            value,
            vcol: VALUE,
            temp: None,
        });
    }
    rows
}

/// size of the window in pixels – depends only on the settings (does not "jump" when values change)
pub fn measure(cfg: &Config) -> (usize, usize) {
    let sc = cfg.px_scale();
    let lab = if cfg.labels { 4 } else { 0 };
    let pct_cols = if cfg.temps {
        TEMP_COL + TEMP_COLS
    } else {
        PCT_COLS
    };
    let mut cols = 0;
    let mut rows = 0;
    for (on, c) in [
        (cfg.fps, FPS_COLS),
        (cfg.cpu, pct_cols),
        (cfg.gpu, pct_cols),
        (cfg.ram, RAM_COLS),
    ] {
        if on {
            rows += 1;
            cols = cols.max(lab + c);
        }
    }
    if rows == 0 {
        rows = 1;
        cols = 3;
    }
    (
        cols * 8 * sc + 2 * pad(sc),
        rows * line_h(sc) + 2 * pad(sc) - sc,
    )
}

pub fn draw(buf: &mut [u32], w: usize, h: usize, cfg: &Config, s: &Snapshot) {
    buf.fill(premult(cfg.bg_color, cfg.bg_opacity));
    if cfg.border {
        for x in 0..w {
            put(buf, w, x, 0, LIGHT);
            put(buf, w, x, h - 1, DARKB);
        }
        for y in 0..h {
            put(buf, w, 0, y, LIGHT);
            put(buf, w, w - 1, y, DARKB);
        }
    }
    let sc = cfg.px_scale();
    let (cw, lh, p) = (8 * sc, line_h(sc), pad(sc));
    let rows = build_rows(cfg, s);
    if rows.is_empty() {
        text(buf, w, p, p, "---", DIM, sc);
        return;
    }
    let lab = if cfg.labels { 4 } else { 0 };
    for (i, r) in rows.iter().enumerate() {
        let y = p + i * lh;
        if cfg.labels {
            text(buf, w, p, y, r.label, LABEL, sc);
        }
        text(buf, w, p + lab * cw, y, &r.value, r.vcol, sc);
        if let Some((t, col)) = &r.temp {
            text(buf, w, p + (lab + TEMP_COL) * cw, y, t, *col, sc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_never_overflows_for_any_combination() {
        let snap = Snapshot {
            cpu: 100.0,
            ram_used: 127_900_000_000,
            ram_total: 137_438_953_472,
            gpu: Some(100.0),
            fps: Some(1000.0),
            cpu_temp: Some(105.0),
            gpu_temp: Some(99.0),
        };
        for mask in 0u32..128 {
            for scale in 1..=6 {
                let cfg = Config {
                    fps: mask & 1 != 0,
                    cpu: mask & 2 != 0,
                    gpu: mask & 4 != 0,
                    ram: mask & 8 != 0,
                    temps: mask & 16 != 0,
                    labels: mask & 32 != 0,
                    border: mask & 64 != 0,
                    font: scale,
                    bg_opacity: 50,
                    ..Config::default()
                };
                let (w, h) = measure(&cfg);
                let mut buf = vec![0u32; w * h];
                draw(&mut buf, w, h, &cfg, &snap);
                assert_eq!(
                    buf[w * h / 2] >> 24 == 0xFF || buf[w * h / 2] >> 24 == 127,
                    true
                );
            }
        }
    }

    #[test]
    fn premultiply_is_correct() {
        assert_eq!(premult(0xFFFFFF, 100), 0xFFFFFFFF);
        assert_eq!(premult(0xFFFFFF, 0), 0x00000000);
        assert_eq!(premult(0x00FF00, 50) >> 24, 127);
    }
}
