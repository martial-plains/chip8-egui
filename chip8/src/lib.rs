//! The Chip8 Emulator Rust crate is also highly performant, featuring
//! optimized code that leverages the latest Rust language features and
//! compiler optimizations. This ensures that the emulator runs smoothly and
//! efficiently on modern hardware, even when running demanding Chip8 games.

use crate::processor::Cpu;

pub mod clock;
pub mod graphics;
pub mod input;
pub mod memory;
pub mod processor;

/// The [`Bus`] struct contains fields for different components of a computer system
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Bus {
    /// An instance of the [`clock::Clock`] struct, which represents the system
    /// clock of the computer. This is used to synchronize the different
    /// components of the system and ensure that they operate at the same speed.
    pub clock: clock::Clock,

    /// An instance of the [`graphics::Buffer`] struct, which represents the
    /// display buffer of the computer. This is used to store the contents
    /// of the screen and update it as necessary.
    pub graphics: graphics::Buffer,

    /// An instance of the [`input::Input`] struct, which represents the
    /// input devices of the computer. This is used to handle user input, such
    /// as keyboard and mouse events.
    pub input: input::Input,

    /// An instance of the [`memory::Memory`] struct, which represents the
    /// memory of the computer. This is used to store the instructions and
    /// data that the processor needs to execute.
    pub memory: memory::Memory,
}

/// The [`Chip8`] struct represents a computer system that uses the Chip-8 virtual machine.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Chip8 {
    /// An instance of the [`Cpu`] struct, which represents the CPU of
    /// the system. This is responsible for executing the instructions in
    /// memory.
    pub processor: Cpu,

    /// An instance of the [`Bus`] struct, which represents the different
    /// components of the system. This is used to connect the CPU to the other
    /// components of the system and facilitate communication between them.
    pub bus: Bus,
}

impl Chip8 {
    /// Creates a new instance of the [`Chip8`] struct with a new [`Cpu`] instance and
    /// the default values for the `Bus` struct's fields.
    ///
    /// # Returns
    ///
    /// The newly created instance of the [`Chip8`] struct.
    #[must_use]
    pub fn new() -> Self {
        Self {
            processor: Cpu::new(),
            ..Default::default()
        }
    }

    /// Executes one instruction cycle of the Chip-8 CPU by updating the system clock and
    /// calling the `cycle` method of the [`Cpu`] struct to execute the current instruction.
    pub fn step(&mut self) {
        self.bus.clock.update();
        self.processor.cycle(&mut self.bus);
    }

    /// Loads the given [`Vec<u8>`] of ROM data into the memory of the [`Bus`] struct. This
    /// method is called to load a Chip-8 ROM into the memory before executing it.
    ///
    /// # Arguments
    ///
    /// * `data`: A [`Vec<u8>`] of ROM data to load into the memory.
    pub fn load_rom_data(&mut self, data: Vec<u8>) {
        self.bus.memory.load_rom(data);
    }

    /// Updates the state of a key on the input device. Takes in a [`u8`] representing the
    /// key code and a boolean `pressed` indicating whether the key is pressed or released.
    /// This method is called to handle keyboard input events.
    ///
    /// # Arguments
    ///
    /// * `key_code`: A [`u8`] representing the key code of the pressed or released key.
    /// * `pressed`: A boolean indicating whether the key is pressed ([`true`]) or released ([`false`]).
    pub fn update_key_state(&mut self, key_code: u8, pressed: bool) {
        self.bus.input.update(key_code, pressed);
    }

    /// Resets the state of the Chip8 system by clearing the display buffer of the [`Bus`]
    /// struct and creating a new [`Bus`] instance with the same graphics buffer as the
    /// previous [`Bus`] instance. It also creates a new [`Cpu`] instance with the same
    /// shift quirk and vblank wait settings as the previous [`Cpu`] instance.
    pub fn reset(&mut self) {
        self.bus.graphics.clear();
        self.bus = Bus {
            graphics: self.bus.graphics,
            ..Default::default()
        };

        let shift_quirk_enabled = self.processor.shift_quirk_enabled;
        let vblank_wait = self.processor.vblank_wait;
        self.processor = Cpu::new();
        self.processor.shift_quirk_enabled = shift_quirk_enabled;
        self.processor.vblank_wait = vblank_wait;
    }

    /// The `reset_and_load` method is a convenience method that resets the
    /// state of the Chip8 system using the `reset` method and then loads the given
    /// ROM data into the system using the `load_rom_data` method. This method is used to
    /// quickly reset the system to its initial state and load a new ROM for execution.
    ///
    /// # Arguments
    ///
    /// * `data` - A [`Vec<u8>`] representing the ROM data to load into the memory.
    pub fn reset_and_load(&mut self, data: Vec<u8>) {
        self.reset();
        self.load_rom_data(data);
    }
}
