#![windows_subsystem = "windows"] // console app without console window

fn main() {
    std::panic::set_hook(Box::new(|i| {
        hudmon::metrics::fps::log(&format!("PANIC (hudmon): {i}"))
    }));
    hudmon::settings::run();
}
