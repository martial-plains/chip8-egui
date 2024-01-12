#![warn(rust_2018_idioms)]
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

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(chip8_ui::App::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
