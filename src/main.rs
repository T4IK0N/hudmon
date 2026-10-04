#![cfg_attr(windows, windows_subsystem = "windows")] // without console window on windows

use hudmon::{
    metrics::fps::{log, FpsSource},
    overlay,
};
use std::{path::PathBuf, time::Duration};

fn main() {
    std::panic::set_hook(Box::new(|i| log(&format!("PANIC: {i}"))));

    let mut interval = Duration::from_millis(500);
    let mut fps = FpsSource::None;
    let mut lhm_port: u16 = 8085;
    let mut pm_exe: Option<String> = None;
    let mut pm_args: Vec<String> = vec!["--output_stdout".into(), "--stop_existing_session".into()];
    let mut a = std::env::args().skip(1);
    while let Some(arg) = a.next() {
        match arg.as_str() {
            "--interval" => {
                interval = Duration::from_millis(
                    a.next()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(500)
                        .max(200),
                )
            }
            "--lhm-port" => lhm_port = a.next().and_then(|v| v.parse().ok()).unwrap_or(8085), // Windows
            "--presentmon" => pm_exe = a.next(), // Windows
            "--presentmon-args" => {
                pm_args = a
                    .next()
                    .unwrap_or_default()
                    .split_whitespace()
                    .map(String::from)
                    .collect()
            }
            "--mangohud-dir" => {
                fps = FpsSource::MangoHud(PathBuf::from(a.next().unwrap_or_default()))
            } // Linux
            "-h" | "--help" => {
                println!("hudmon-overlay [--interval ms]\n       [--presentmon PresentMon.exe [--presentmon-args \"...\"]] [--lhm-port N]\n       [--mangohud-dir KATALOG_LOGOW]\nWyglad i uruchamianie: hudmon");
                return;
            }
            _ => {}
        }
    }

    log("--- start ---");

    // windows: if there is no path provided, we look for PresentMon*.exe next to hudmon.exe / in the current directory
    if pm_exe.is_none() && cfg!(windows) && matches!(fps, FpsSource::None) {
        let dirs = [
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf())),
            std::env::current_dir().ok(),
        ];
        'find: for d in dirs.into_iter().flatten() {
            for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
                let n = e.file_name().to_string_lossy().to_lowercase();
                if n.starts_with("presentmon") && n.ends_with(".exe") {
                    pm_exe = Some(e.path().to_string_lossy().into_owned());
                    break 'find;
                }
            }
        }
    }
    if let Some(exe) = pm_exe {
        fps = FpsSource::PresentMon(exe, pm_args);
    } else if cfg!(windows) && matches!(fps, FpsSource::None) {
        log("Nie znaleziono PresentMon*.exe obok hudmon.exe – FPS wyłączony.");
    }

    overlay::run(interval, fps, lhm_port);
}
