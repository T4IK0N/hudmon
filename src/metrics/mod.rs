//! data layer, the ui only sees `snapshot`.
pub mod fps;
pub mod gpu;
pub mod lhm;
pub mod system;

#[derive(Default, Clone)]
pub struct Snapshot {
    pub cpu: f32,
    pub ram_used: u64,
    pub ram_total: u64,
    pub gpu: Option<f32>,
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub fps: Option<f32>,
}

pub trait Provider {
    fn sample(&mut self, out: &mut Snapshot);
}
