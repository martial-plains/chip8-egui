#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    // Log to stdout (if you run with `RUST_LOG=debug`).

    use env_logger::{Builder, Target};
    let mut builder = Builder::from_default_env();

    builder.target(Target::Stdout);
    builder.init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Chip8",
        native_options,
        Box::new(|cc| Box::new(chip8_ui::App::new(cc))),
    )
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.

    use log::Level;
    console_error_panic_hook::set_once();

    #[cfg(debug_assertions)]
    console_log::init_with_level(Level::Debug);

    #[cfg(not(debug_assertions))]
    console_log::init_with_level(Level::Info);

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|cc| Box::new(chip8_ui::App::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
