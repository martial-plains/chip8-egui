#![feature(cfg_match)]
#![warn(clippy::pedantic, clippy::nursery)]

mod app;
pub use app::App;
#[cfg(not(target_arch = "wasm32"))]
pub mod audio;
pub mod gui;
