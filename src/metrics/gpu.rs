//! GPU: nvml (nvidia, windows+linux) - sysfs (amd/intel, linux).
//! windows + amd/intel: unimplemented (look for: README: PDH "GPU Engine").
use super::{Provider, Snapshot};
use nvml_wrapper::Nvml;
#[cfg(target_os = "linux")]
use std::{fs, path::PathBuf};

enum Backend {
    Nvml(Nvml),
    #[cfg(target_os = "linux")]
    Sysfs(PathBuf),
    None,
}

pub struct GpuProvider {
    backend: Backend,
}

impl GpuProvider {
    pub fn new() -> Self {
        // NVML is loading dynamically, so it will fail if the driver is not installed
        if let Ok(nvml) = Nvml::init() {
            if nvml.device_by_index(0).is_ok() {
                return Self {
                    backend: Backend::Nvml(nvml),
                };
            }
        }
        #[cfg(target_os = "linux")]
        if let Ok(dir) = fs::read_dir("/sys/class/drm") {
            let mut cards: Vec<_> = dir.flatten().map(|e| e.path()).collect();
            cards.sort();
            for c in cards {
                let p = c.join("device/gpu_busy_percent");
                if p.exists() {
                    return Self {
                        backend: Backend::Sysfs(p),
                    };
                }
            }
        }
        Self {
            backend: Backend::None,
        }
    }
}

impl Provider for GpuProvider {
    fn sample(&mut self, out: &mut Snapshot) {
        out.gpu_temp = match &self.backend {
            Backend::Nvml(n) => n
                .device_by_index(0)
                .and_then(|d| {
                    d.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                })
                .ok()
                .map(|t| t as f32),
            #[cfg(target_os = "linux")]
            Backend::Sysfs(p) => hwmon_temp(p),
            Backend::None => None,
        };
        out.gpu = match &self.backend {
            Backend::Nvml(n) => n
                .device_by_index(0)
                .and_then(|d| d.utilization_rates())
                .ok()
                .map(|u| u.gpu as f32),
            #[cfg(target_os = "linux")]
            Backend::Sysfs(p) => fs::read_to_string(p)
                .ok()
                .and_then(|s| s.trim().parse().ok()),
            Backend::None => None,
        };
    }
}

#[cfg(target_os = "linux")]
fn hwmon_temp(busy_path: &PathBuf) -> Option<f32> {
    let dir = busy_path.parent()?.join("hwmon");
    for e in fs::read_dir(dir).ok()?.flatten() {
        if let Some(v) = fs::read_to_string(e.path().join("temp1_input"))
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
        {
            return Some(v / 1000.0);
        }
    }
    None
}
