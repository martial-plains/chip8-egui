use std::path::Path;

use chip8::{graphics::Rgb, Chip8};
use eframe::Frame;

#[cfg(not(target_arch = "wasm32"))]
use crate::audio;
use crate::gui::{Chip8Message, Gui};

pub const DEFAULT_STEPS_PER_FRAME: u32 = 10;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    #[serde(skip)]
    chip8: Chip8,
    gui: Gui,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    audio: audio::System,
    steps_per_frame: u32,
    paused: bool,
    last_rom: Vec<u8>,
}

impl Default for App {
    fn default() -> Self {
        let chip8 = Chip8::new();
        #[cfg(not(target_arch = "wasm32"))]
        let audio = Self::create_audio_system(&chip8).expect("Failed to create audio::System");
        Self {
            chip8,
            #[cfg(not(target_arch = "wasm32"))]
            audio,
            steps_per_frame: DEFAULT_STEPS_PER_FRAME,
            paused: false,
            last_rom: Vec::default(),
            gui: Gui::default(),
        }
    }
}

impl eframe::App for App {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let Self { .. } = self;

        egui::CentralPanel::default().show(ctx, |_| {});

        if !self.paused {
            for _ in 0..self.steps_per_frame {
                self.chip8.step();
            }
        }

        self.update_gui(ctx, frame);

        ctx.request_repaint();
    }
}

impl App {
    /// Creates a new [`App`] instance.
    ///
    /// Called once before the first frame.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value::<App>(storage, eframe::APP_KEY).unwrap_or_default();
        }

        let mut chip8 = Chip8::new();
        let mut last_rom = Vec::new();

        if let Some(data) = Self::get_arg_rom() {
            chip8.load_rom_data(data.clone());
            last_rom = data;
        }

        #[cfg(not(target_arch = "wasm32"))]
        let audio = Self::create_audio_system(&chip8).expect("Failed to create audio::System");

        let gui = Gui::new();

        Self {
            chip8,
            #[cfg(not(target_arch = "wasm32"))]
            audio,
            steps_per_frame: DEFAULT_STEPS_PER_FRAME,
            paused: false,
            last_rom,
            gui,
        }
    }

    /// Create a new [`audio::System`] using the sound timer from the given
    /// `Chip8` instance.
    ///
    /// This will also start the audio stream. This function will only return
    /// the [`audio::System`] if it can be both created and played without errors,
    /// otherwise it returns `Err`.
    #[cfg(not(target_arch = "wasm32"))]

    fn create_audio_system(chip8: &Chip8) -> Result<audio::System, anyhow::Error> {
        let audio = audio::System::new(chip8.bus.clock.sound_timer.clone())?;
        audio.play().map(|_| audio).map_err(|e| {
            log::error!("Failed to play audio stream: {e}");
            e
        })
    }

    /// Update the [`Gui`] and handle all state-changing messages.
    fn update_gui(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        for message in self.gui.update(ctx, frame, &self.chip8) {
            match message {
                Chip8Message::LoadRom(data) => {
                    self.chip8.reset_and_load(data.clone());
                    self.last_rom = data;
                    #[cfg(not(target_arch = "wasm32"))]
                    self.reset_audio();
                }
                Chip8Message::ResetROM => {
                    self.chip8.reset_and_load(self.last_rom.clone());
                    #[cfg(not(target_arch = "wasm32"))]
                    self.reset_audio();
                }
                Chip8Message::SetForegroundColor(color) => {
                    self.chip8.bus.graphics.set_foreground_color(Rgb {
                        red: color.r(),
                        green: color.g(),
                        blue: color.b(),
                    });
                }
                Chip8Message::SetBackgroundColor(color) => {
                    self.chip8.bus.graphics.set_background_color(Rgb {
                        red: color.r(),
                        green: color.g(),
                        blue: color.b(),
                    });
                }
                Chip8Message::SetStepRate(steps) => self.steps_per_frame = steps,
                Chip8Message::SetShiftQuirk(enabled) => {
                    self.chip8.processor.shift_quirk_enabled = enabled;
                }
                Chip8Message::SetVblankWait(enabled) => {
                    self.chip8.processor.vblank_wait = enabled;
                }
                Chip8Message::UpdateKeys(key_updates) => {
                    for (key_code, pressed) in key_updates {
                        self.chip8.update_key_state(key_code, pressed);
                    }
                }
                Chip8Message::TogglePause => self.paused = !self.paused,
                Chip8Message::SaveState(path) => {
                    if let Err(e) = self.save_chip8(&path) {
                        log::error!("Failed to save Chip8 state to {}: {e}.", path.display());
                    }
                }
                Chip8Message::LoadState(path) => match Self::load_chip8(&path) {
                    Ok(chip8) => {
                        self.chip8 = chip8;
                        #[cfg(not(target_arch = "wasm32"))]
                        self.reset_audio();
                    }
                    Err(e) => {
                        log::error!("Failed to load Chip8 state from {}: {e}.", path.display());
                    }
                },
                Chip8Message::Step => self.chip8.step(),
            }
        }
    }

    /// Get the ROM data from the path provided as the first argument when
    /// run from the command line.
    fn get_arg_rom() -> Option<Vec<u8>> {
        std::env::args().nth(1).and_then(|rom_path| {
            std::fs::read(&rom_path)
                .map_err(|e| log::error!("Failed to read ROM from {rom_path}: {e}"))
                .ok()
        })
    }

    /// Load [`Chip8`] state from the given `path`.
    fn load_chip8(path: impl AsRef<Path>) -> anyhow::Result<Chip8> {
        let bytes = std::fs::read(path)?;
        let chip8 = bincode::deserialize::<Chip8>(&bytes)?;
        Ok(chip8)
    }

    /// Save [`Chip8`] state to a file specified by `path`.
    fn save_chip8(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let bytes = bincode::serialize(&self.chip8)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Reset the audio system. This should be called anytime the [`Chip8`] is reset,
    /// as the new sound timer needs to be linked to a new [`audio::System`].
    #[cfg(not(target_arch = "wasm32"))]

    fn reset_audio(&mut self) {
        match Self::create_audio_system(&self.chip8) {
            Ok(audio) => self.audio = audio,
            Err(e) => log::error!("Failed to create new audio::System: {e}"),
        }
    }
}
