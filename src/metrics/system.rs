use super::{Provider, Snapshot};
use std::time::{Duration, Instant};
use sysinfo::System;

#[cfg(not(windows))]
mod cpu_temp {
    use sysinfo::Components;
    pub struct Reader(Components);
    impl Reader {
        pub fn new() -> Self {
            Self(Components::new_with_refreshed_list())
        }
        pub fn read(&mut self) -> Option<f32> {
            self.0.refresh();
            const KEYS: [&str; 5] = ["package", "tctl", "tdie", "cpu", "core"];
            self.0
                .iter()
                .filter(|c| KEYS.iter().any(|k| c.label().to_lowercase().contains(k)))
                .map(|c| c.temperature())
                .filter(|t| t.is_finite() && *t > 0.0 && *t < 150.0)
                .fold(None, |m: Option<f32>, t| Some(m.map_or(t, |m| m.max(t))))
        }
    }
}

#[cfg(windows)]
mod cpu_temp {
    pub struct Reader;
    impl Reader {
        pub fn new() -> Self {
            Self
        }
        pub fn read(&mut self) -> Option<f32> {
            None
        }
    }
}

pub struct SystemProvider {
    sys: System,
    temps: cpu_temp::Reader,
    last_temp: Option<Instant>,
    temp: Option<f32>,
}

impl SystemProvider {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu();
        Self {
            sys,
            temps: cpu_temp::Reader::new(),
            last_temp: None,
            temp: None,
        }
    }
}

impl Provider for SystemProvider {
    fn sample(&mut self, out: &mut Snapshot) {
        self.sys.refresh_cpu();
        self.sys.refresh_memory();
        out.cpu = self.sys.global_cpu_info().cpu_usage();
        out.ram_used = self.sys.used_memory();
        out.ram_total = self.sys.total_memory();
        if self
            .last_temp
            .map_or(true, |t| t.elapsed() >= Duration::from_secs(2))
        {
            self.temp = self.temps.read();
            self.last_temp = Some(Instant::now());
        }
        out.cpu_temp = self.temp;
    }
}
