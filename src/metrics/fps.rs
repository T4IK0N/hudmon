//! fps without injecting into the game process is not possible without system support, so we use ready-made solutions:
//! * windows: PresentMon
//! * linux:   MangoHud log
use super::{Provider, Snapshot};
use std::{
    collections::{HashMap, VecDeque},
    fs,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime},
};

pub enum FpsSource {
    None,
    /// path to PresentMon.exe + args
    PresentMon(String, Vec<String>),
    /// dir with logs of MangoHud
    MangoHud(PathBuf),
}

type Frames = Arc<Mutex<HashMap<String, VecDeque<Instant>>>>;

pub struct FpsProvider {
    source: FpsSource,
    frames: Frames,
}

impl FpsProvider {
    pub fn new(source: FpsSource) -> Self {
        let frames: Frames = Arc::default();
        if let FpsSource::PresentMon(exe, args) = &source {
            spawn_presentmon(exe.clone(), args.clone(), frames.clone());
        }
        Self { source, frames }
    }
}

/// diagnostic log next to the exe (the application has no console).
pub fn log(msg: &str) {
    use std::io::Write;
    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("hudmon.log")))
        .unwrap_or_else(|| "hudmon.log".into());
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{msg}");
    }
}

fn spawn_presentmon(exe: String, args: Vec<String>, frames: Frames) {
    thread::spawn(move || {
        let mut cmd = Command::new(&exe);
        cmd.args(&args).stdout(Stdio::piped()).stdin(Stdio::null());
        match fs::File::create(std::env::temp_dir().join("hudmon-presentmon-stderr.log")) {
            Ok(f) => cmd.stderr(f),
            Err(_) => cmd.stderr(Stdio::null()),
        };
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                log(&format!("Nie udało się uruchomić '{exe}': {e}"));
                return;
            }
        };
        log(&format!("Uruchomiono PresentMon: {exe} {}", args.join(" ")));
        let reader = BufReader::new(child.stdout.take().unwrap());
        let mut app_i: Option<usize> = None;
        let mut rows = 0u64;
        for line in reader.lines().map_while(Result::ok) {
            let cols: Vec<&str> = line.split(',').collect();
            let Some(a) = app_i else {
                // we are looking for the CSV header
                // we only need the column with the process name
                app_i = cols.iter().position(|c| {
                    let c = c.trim().to_lowercase();
                    c == "application" || c == "processname"
                });
                if app_i.is_some() {
                    log(&format!("Nagłówek CSV: {line}"));
                }
                continue;
            };
            if let Some(app) = cols.get(a) {
                let app = app.trim();
                if app.is_empty() || app.eq_ignore_ascii_case("dwm.exe") {
                    continue;
                }
                rows += 1;
                frames
                    .lock()
                    .unwrap()
                    .entry(app.to_string())
                    .or_default()
                    .push_back(Instant::now());
            }
        }
        let _ = child.wait();
        log(&format!(
            "PresentMon zakończył działanie po {rows} klatkach. Jeśli 0 – uruchom hudmon jako administrator (szczegóły: %TEMP%\\hudmon-presentmon-stderr.log)."
        ));
    });
}

impl Provider for FpsProvider {
    fn sample(&mut self, out: &mut Snapshot) {
        out.fps = match &self.source {
            FpsSource::None => None,
            FpsSource::PresentMon(..) => {
                let mut map = self.frames.lock().unwrap();
                let cutoff = Instant::now() - Duration::from_secs(1);
                let mut best = 0usize;
                for q in map.values_mut() {
                    while q.front().is_some_and(|t| *t < cutoff) {
                        q.pop_front();
                    }
                    best = best.max(q.len());
                }
                map.retain(|_, q| !q.is_empty());
                (best > 0).then_some(best as f32)
            }
            FpsSource::MangoHud(dir) => mangohud_fps(dir),
        };
    }
}

/// reads the last line of the newest `.csv` log (first column = fps).
fn mangohud_fps(dir: &PathBuf) -> Option<f32> {
    let (path, modified) = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "csv"))
        .filter_map(|e| Some((e.path(), e.metadata().ok()?.modified().ok()?)))
        .max_by_key(|(_, m)| *m)?;
    if SystemTime::now().duration_since(modified).ok()? > Duration::from_secs(3) {
        return None; // log inactive
    }
    let mut f = fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    f.seek(SeekFrom::Start(len.saturating_sub(256))).ok()?;
    let mut tail = String::new();
    f.read_to_string(&mut tail).ok()?;
    let last = tail.lines().rev().find(|l| !l.trim().is_empty())?;
    last.split(',').next()?.trim().parse().ok()
}
