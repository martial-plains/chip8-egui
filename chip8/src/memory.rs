//! The `memory` module provides a struct and some associated functions to
//! represent the memory of a Chip8 system. The memory is represented as an
//! array of 8-bit unsigned integers ([`u8`]), with a size of 4096 bytes.

use std::ops::{Index, IndexMut};

/// The total size of the Chip8 memory.
const MEMORY_SIZE: usize = 4096;

/// The size of the interpreter. This is used to determine where the program memory should start.
const INTERPRETER_SIZE: usize = 512;

/// Built-in Chip8 font data. This is stored in the interpreter's memory.
const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

/// The [`Memory`] struct represents the memory of a Chip8 system. It contains
/// a fixed-size array of [`u8`] values that can be accessed using the [`Index`]
/// and [`IndexMut`] traits.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Memory {
    #[serde(with = "serde_big_array::BigArray")]
    memory: [u8; MEMORY_SIZE],
}

impl Default for Memory {
    fn default() -> Self {
        let mut memory = [0; MEMORY_SIZE];
        memory[..80].clone_from_slice(&FONT);
        Self { memory }
    }
}

impl Index<usize> for Memory {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.memory[index]
    }
}

impl IndexMut<usize> for Memory {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.memory[index]
    }
}

impl Memory {
    /// Creates a new [`Memory`] object filled with zeroes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads the ROM bytes from `data`. If this is smaller than the program
    /// size (`MEMORY_SIZE - INTERPRETER_SIZE`), then the remaining memory will
    /// be filled with zeroes.
    pub fn load_rom(&mut self, mut data: Vec<u8>) {
        data.resize(MEMORY_SIZE - INTERPRETER_SIZE, 0);
        self.memory[INTERPRETER_SIZE..=0xFFF].clone_from_slice(&data);
    }
}
