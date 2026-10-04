//! hudmon - shared code between overlay (`hudmon`) and settings window (`hudmon-settings`).
pub mod config;
pub mod instance;
pub mod metrics;
pub mod overlay;
#[cfg(windows)]
pub mod present_win;
pub mod render;
pub mod settings;
