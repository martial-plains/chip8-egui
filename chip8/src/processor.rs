//! This module contains the implementation of the Chip8 central processing
//! unit (CPU). The CPU executes the instructions stored in the memory of the
//! Chip8 computer.

use std::collections::VecDeque;

use crate::graphics;

use super::Bus;

/// The maximum amount of instructions that should be stored
/// in the [`Cpu`]'s buffer of instructions.
const INSTRUCTION_BUFFER_LENGTH: usize = 100;

/// The default starting address for the [`Cpu`].
/// For most Chip8 programs, 0x200 should be
const STARTING_PC: usize = 0x200;

/// Describes how the program counter should be updated after
/// executing an instruction.
enum ProgramCounterUpdate {
    /// Go directly to the next instruction (pc + 2)
    Next,

    /// Skip the next instruction (pc + 4).
    SkipNext,

    /// Jump to the given address.
    Jump(usize),
}

/// This structs contains information about an instruction in a computer program.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Instruction {
    /// An unsigned integer representing the memory address where the instruction is located.
    pub address: usize,

    /// An unsigned integer representing the opcode of the instruction.
    pub opcode: usize,

    /// A string representing a display-friendly explanation of what the instruction does.
    pub display: String,
}

/// This struct represents the central processing unit of a computer.
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Cpu {
    /// An array of 16 unsigned 8-bit integers representing the Vx registers.
    pub v: [u8; 16],

    /// An unsigned integer representing the index register.
    pub i: usize,

    /// An unsigned integer representing the program counter.
    pub pc: usize,

    /// An unsigned integer representing the stack pointer.
    pub sp: usize,

    /// An array of 16 unsigned integers representing the stack memory.
    pub stack: [usize; 16],

    /// A boolean indicating whether the shift quirk is enabled. This affects
    /// the behavior of certain instructions.
    pub shift_quirk_enabled: bool,

    /// A boolean indicating whether the processor should wait for the vertical
    /// blank interrupt before drawing a sprite.
    pub vblank_wait: bool,

    /// A string representing a display-friendly explanation of what the
    /// current opcode is doing.
    pub display: String,

    /// A [`VecDeque`] of [`Instruction`] instances representing the last
    /// `INSTRUCTION_BUFFER_LENGTH` instructions that the [`Cpu`] has
    /// executed.
    pub instructions: VecDeque<Instruction>,
}

impl Cpu {
    /// Create a new [`Cpu`] instance. This is similar to [`Cpu::default`],
    /// with the exception that the program counter is set to `STARTING_PC`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pc: STARTING_PC,
            sp: 0,
            v: [0; 16],
            i: 0,
            stack: [0; 16],
            shift_quirk_enabled: false,
            vblank_wait: false,
            display: String::new(),
            instructions: VecDeque::new(),
        }
    }

    /// Execute one processor cycle. This will fetch, decode, and execute the next
    /// opcode from memory. Note that if the processor is currently waiting on
    /// input from the user, no instructions will be executed.
    pub fn cycle(&mut self, bus: &mut Bus) {
        if bus.input.waiting() {
            return;
        } else if let Some(request) = bus.input.request_response() {
            self.v[request.register] = request.key_code;
        }

        if self.pc >= 4096 {
            return;
        }
        // get the next two bytes and combine into one two-byte instruction
        let opcode = (usize::from(bus.memory[self.pc]) << 8) | usize::from(bus.memory[self.pc + 1]);

        let (pc_update, display) = self.process_opcode(opcode, bus);

        // push new instruction
        let instruction = Instruction {
            address: self.pc,
            opcode,
            display,
        };
        self.push_instruction(instruction);

        match pc_update {
            ProgramCounterUpdate::Next => self.pc += 2,
            ProgramCounterUpdate::SkipNext => self.pc += 4,
            ProgramCounterUpdate::Jump(addr) => self.pc = addr,
        }
    }

    /// Push an instruction to the instruction buffer. This will
    /// remove the last instruction in the list if the length has exceeded
    /// the [`INSTRUCTION_BUFFER_LENGTH`].
    fn push_instruction(&mut self, instruction: Instruction) {
        self.instructions.push_front(instruction);
        if self.instructions.len() > INSTRUCTION_BUFFER_LENGTH {
            self.instructions.pop_back();
        }
    }

    /// Process a single opcode. This will apply any state changing effects of the
    /// instructions onto the given [`Bus`].
    fn process_opcode(&mut self, opcode: usize, bus: &mut Bus) -> (ProgramCounterUpdate, String) {
        // define some commonly used variables
        let x = (opcode & 0x0F00) >> 8;
        let y = (opcode & 0x00F0) >> 4;
        let nn = u8::try_from(opcode & 0x00FF).unwrap();
        let nnn = opcode & 0x0FFF;

        match (opcode & 0xF000) >> 12 {
            // 0___
            0x0 => match opcode & 0x000F {
                // 00E0
                0x0000 => Self::op_00e0(bus),

                // 00EE
                0x000E => self.op_00ee(),

                // invalid
                _ => {
                    log::error!("Invalid 0x0___ instruction: {opcode:X}");
                    let display = "Invalid instruction".into();
                    (ProgramCounterUpdate::Next, display)
                }
            },

            // 1nnn
            0x1 => Self::op_1nnn(nnn),

            // 2nnn
            0x2 => self.op_2nnn(nnn),

            // 3xnn
            0x3 => self.op_3xnn(x, nn),

            // 4Xnn
            0x4 => self.op_4xnn(x, nn),

            // 5xy0
            0x5 => self.op_5xy0(x, y),

            // 6xnn
            0x6 => self.op_6xnn(x, nn),

            // 7xnn
            0x7 => self.op_7xnn(x, nn),

            // 8___
            0x8 => match opcode & 0x000F {
                // 8xy0
                0x0 => self.op_8xy0(x, y),

                // 8xy1
                0x1 => self.op_8xy1(x, y),

                // 8xy2
                0x2 => self.op_8xy2(x, y),

                // 8xy3
                0x3 => self.op_8xy3(x, y),

                // 8xy4
                0x4 => self.op_8xy4(x, y),

                // 8xy5
                0x5 => self.op_8xy5(x, y),

                // 8xy6
                0x6 => self.op_8xy6(x, y),

                // 8xy7
                0x7 => self.op_8xy7(y, x),

                // 8xyE
                0xE => self.op_8xye(x, y),

                // invalid
                _ => {
                    let display = "Invalid instruction".into();
                    log::error!("Invalid 8XY_ instruction: {opcode:X}");
                    (ProgramCounterUpdate::Next, display)
                }
            },

            // 9xy0
            9 => self.op_9xy0(x, y),

            // Annn
            0xA => self.op_annn(nnn),

            // Bnnn
            0xB => self.op_bnnn(nnn),

            // Cxnn
            0xC => self.op_cxnn(x, nn),

            // Dxyn
            0xD => self.op_dxyn(bus, opcode, x, y),

            // E___
            0xE => match opcode & 0x000F {
                // Ex9E
                0x000E => self.op_ex9e(bus, x),

                // ExA1
                0x0001 => self.op_exa1(bus, x),

                // invalid
                _ => {
                    let display = "Invalid instruction".into();
                    log::error!("Invalid EX__ instruction: {opcode:X}");
                    (ProgramCounterUpdate::Next, display)
                }
            },

            // F___
            0xF => match opcode & 0x00FF {
                // Fx07
                0x0007 => self.op_fx07(bus, x),

                // Fx0A
                0x000A => Self::op_fx0a(bus, x),

                // Fx15
                0x0015 => self.op_fx15(bus, x),

                // Fx18
                0x0018 => self.op_fx18(bus, x),

                // Fx1E
                0x001E => self.op_fx1e(x),

                // Fx29
                0x0029 => self.op_fx29(x),

                // Fx33
                0x0033 => self.op_fx33(bus, x),

                // Fx55
                0x0055 => self.op_fx55(x, bus),

                // Fx65
                0x0065 => self.op_fx65(x, bus),

                // invalid
                _ => {
                    let display = "Invalid instruction".into();
                    log::error!("Invalid FX__ instruction: {opcode:X}");
                    (ProgramCounterUpdate::Next, display)
                }
            },

            // invalid
            _ => {
                let display = "Invalid instruction".into();
                log::error!("Unknown opcode: {opcode:X}");
                (ProgramCounterUpdate::Next, display)
            }
        }
    }

    fn op_fx65(&mut self, x: usize, bus: &mut Bus) -> (ProgramCounterUpdate, String) {
        let display = format!("Read memory at I into V0 to V{x:X}");
        for i in 0..=x {
            self.v[i] = bus.memory[self.i];
            self.i += 1;
        }
        (ProgramCounterUpdate::Next, display)
    }

    fn op_fx55(&mut self, x: usize, bus: &mut Bus) -> (ProgramCounterUpdate, String) {
        let display = format!("Store V0 to V{x:X} starting at I");
        for i in 0..=x {
            bus.memory[self.i] = self.v[i];
            self.i += 1;
        }
        (ProgramCounterUpdate::Next, display)
    }

    fn op_fx33(&mut self, bus: &mut Bus, x: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Store BCD of {} starting at I", self.v[x]);
        bus.memory[self.i] = (self.v[x] / 100) % 10;
        bus.memory[self.i + 1] = (self.v[x] / 10) % 10;
        bus.memory[self.i + 2] = self.v[x] % 10;
        (ProgramCounterUpdate::Next, display)
    }

    fn op_fx29(&mut self, x: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Set I to addr of sprite digit {}", self.v[x]);
        self.i = 5 * usize::from(self.v[x]);
        (ProgramCounterUpdate::Next, display)
    }

    fn op_fx1e(&mut self, x: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Set I to I + V{x:X}");
        self.i += usize::from(self.v[x]);
        (ProgramCounterUpdate::Next, display)
    }

    fn op_fx18(&mut self, bus: &mut Bus, x: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Set sound timer to V{x:X} ({})", self.v[x]);
        (*bus.clock.sound_timer).store(self.v[x], std::sync::atomic::Ordering::SeqCst);
        (ProgramCounterUpdate::Next, display)
    }

    fn op_fx15(&mut self, bus: &mut Bus, x: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Set delay timer to V{x:X} ({})", self.v[x]);
        bus.clock.delay_timer = self.v[x];
        (ProgramCounterUpdate::Next, display)
    }

    fn op_fx07(&mut self, bus: &mut Bus, x: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Set V{x:X} to delay timer ({})", bus.clock.delay_timer);
        self.v[x] = bus.clock.delay_timer;
        (ProgramCounterUpdate::Next, display)
    }

    fn op_exa1(&mut self, bus: &mut Bus, x: usize) -> (ProgramCounterUpdate, String) {
        let not_pressed = !bus.input.is_key_pressed(self.v[x]);
        let display = format!(
            "Skip next instr if key code {:#X} not pressed ({not_pressed})",
            self.v[x]
        );
        if not_pressed {
            (ProgramCounterUpdate::SkipNext, display)
        } else {
            (ProgramCounterUpdate::Next, display)
        }
    }

    fn op_ex9e(&mut self, bus: &mut Bus, x: usize) -> (ProgramCounterUpdate, String) {
        let pressed = bus.input.is_key_pressed(self.v[x]);
        let display = format!("Skip instr if key {:#X} pressed ({pressed})", self.v[x]);
        if pressed {
            (ProgramCounterUpdate::SkipNext, display)
        } else {
            (ProgramCounterUpdate::Next, display)
        }
    }

    fn op_dxyn(
        &mut self,
        bus: &mut Bus,
        opcode: usize,
        x: usize,
        y: usize,
    ) -> (ProgramCounterUpdate, String) {
        if self.vblank_wait {
            // spin wait for vblank
            loop {
                bus.clock.update();
                if bus.clock.vblank_interrupt {
                    break;
                }
            }
        }
        let n = opcode & 0xF;
        let x = usize::from(self.v[x]) % graphics::WIDTH;
        let y = usize::from(self.v[y]) % graphics::HEIGHT;
        let display = format!(
            "Draw {n} byte sprite from addr {:#06X} at point ({x}, {y})",
            self.i
        );
        let mut collision = false;
        for i in 0..n {
            let data = bus.memory[self.i + i];
            collision |= bus.graphics.draw_byte(x, y + i, data);
        }
        self.v[0xF] = collision.into();
        (ProgramCounterUpdate::Next, display)
    }

    fn op_cxnn(&mut self, x: usize, nn: u8) -> (ProgramCounterUpdate, String) {
        let mut buf = [0u8; 1];
        getrandom::getrandom(&mut buf).unwrap();
        let display = format!("Set V{x:X} to {} [rand] AND {nn:#X}", buf[0]);
        self.v[x] = buf[0] & nn;
        (ProgramCounterUpdate::Next, display)
    }

    fn op_bnnn(&mut self, nnn: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Jump to {nnn:#06X} + {:#06X}", self.v[0]);
        (
            ProgramCounterUpdate::Jump(nnn + usize::from(self.v[0])),
            display,
        )
    }

    fn op_annn(&mut self, nnn: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Set I register to {nnn:#06X}");
        self.i = nnn;
        (ProgramCounterUpdate::Next, display)
    }

    fn op_9xy0(&mut self, x: usize, y: usize) -> (ProgramCounterUpdate, String) {
        let display = format!(
            "If V{x:X} ({}) != V{y:X} ({}), skip next instr",
            self.v[x], self.v[y]
        );
        if self.v[x] == self.v[y] {
            (ProgramCounterUpdate::Next, display)
        } else {
            (ProgramCounterUpdate::SkipNext, display)
        }
    }

    fn op_8xye(&mut self, x: usize, y: usize) -> (ProgramCounterUpdate, String) {
        if self.shift_quirk_enabled {
            self.v[x] = self.v[y];
        }
        let overflow = (self.v[x] & 0x80) >> 7;
        let display = format!("V{x:X} shifted one left, VF = {overflow}");
        self.v[x] <<= 1;
        self.v[0xF] = overflow;
        (ProgramCounterUpdate::Next, display)
    }

    fn op_8xy7(&mut self, y: usize, x: usize) -> (ProgramCounterUpdate, String) {
        let (result, overflow) = self.v[y].overflowing_sub(self.v[x]);
        let display = format!(
            "Set V{x:X} to ({} - {}), VF = {}",
            self.v[y],
            self.v[x],
            u8::from(!overflow)
        );
        self.v[x] = result;
        self.v[0xF] = u8::from(!overflow);
        (ProgramCounterUpdate::Next, display)
    }

    fn op_8xy6(&mut self, x: usize, y: usize) -> (ProgramCounterUpdate, String) {
        if self.shift_quirk_enabled {
            self.v[x] = self.v[y];
        }
        let overflow = self.v[x] & 1;
        let display = format!("V{x:X} shifted one right, VF = {overflow}");
        self.v[x] >>= 1;
        self.v[0xF] = overflow;
        (ProgramCounterUpdate::Next, display)
    }

    fn op_8xy5(&mut self, x: usize, y: usize) -> (ProgramCounterUpdate, String) {
        let (result, overflow) = self.v[x].overflowing_sub(self.v[y]);
        let display = format!(
            "Set V{x:X} to ({} - {}), VF = {}",
            self.v[x],
            self.v[y],
            u8::from(!overflow)
        );
        self.v[x] = result;
        self.v[0xF] = u8::from(!overflow);
        (ProgramCounterUpdate::Next, display)
    }

    fn op_8xy4(&mut self, x: usize, y: usize) -> (ProgramCounterUpdate, String) {
        let (result, overflow) = self.v[x].overflowing_add(self.v[y]);
        let display = format!(
            "Set V{x:X} to ({} + {}), VF = {}",
            self.v[x],
            self.v[y],
            u8::from(overflow)
        );
        self.v[x] = result;
        self.v[0xF] = u8::from(overflow);
        (ProgramCounterUpdate::Next, display)
    }

    fn op_8xy3(&mut self, x: usize, y: usize) -> (ProgramCounterUpdate, String) {
        let display = format!(
            "Set V{x:X} to V{x:X} XOR V{y:X} ({:2X} XOR {:2X})",
            self.v[x], self.v[y]
        );
        self.v[x] ^= self.v[y];
        self.v[0xF] = 0;
        (ProgramCounterUpdate::Next, display)
    }

    fn op_8xy2(&mut self, x: usize, y: usize) -> (ProgramCounterUpdate, String) {
        let display = format!(
            "Set V{x:X} to V{x:X} AND V{y:X} ({:2X} AND {:2X})",
            self.v[x], self.v[y]
        );
        self.v[x] &= self.v[y];
        self.v[0xF] = 0;
        (ProgramCounterUpdate::Next, display)
    }

    fn op_8xy1(&mut self, x: usize, y: usize) -> (ProgramCounterUpdate, String) {
        let display = format!(
            "Set V{x:X} to V{x:X} OR V{y:X} ({:2X} OR {:2X})",
            self.v[x], self.v[y]
        );
        self.v[x] |= self.v[y];
        self.v[0xF] = 0;
        (ProgramCounterUpdate::Next, display)
    }

    fn op_8xy0(&mut self, x: usize, y: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Set V{x:X} to V{y:X} ({})", self.v[y]);
        self.v[x] = self.v[y];
        (ProgramCounterUpdate::Next, display)
    }

    fn op_7xnn(&mut self, x: usize, nn: u8) -> (ProgramCounterUpdate, String) {
        let display = format!("Add {nn} to V{x:X}");
        self.v[x] = self.v[x].wrapping_add(nn);
        (ProgramCounterUpdate::Next, display)
    }

    fn op_6xnn(&mut self, x: usize, nn: u8) -> (ProgramCounterUpdate, String) {
        let display = format!("Set V{x:X} to {nn}");
        self.v[x] = nn;
        (ProgramCounterUpdate::Next, display)
    }

    fn op_5xy0(&mut self, x: usize, y: usize) -> (ProgramCounterUpdate, String) {
        let display = format!(
            "If V{x:X} ({}) == V{y:X} ({}), skip next instr",
            self.v[x], self.v[y]
        );
        if self.v[x] == self.v[y] {
            (ProgramCounterUpdate::SkipNext, display)
        } else {
            (ProgramCounterUpdate::Next, display)
        }
    }

    fn op_4xnn(&mut self, x: usize, nn: u8) -> (ProgramCounterUpdate, String) {
        let display = format!("If V{x:X} ({}) != {nn}, skip next instr", self.v[x]);
        if self.v[x] == nn {
            (ProgramCounterUpdate::Next, display)
        } else {
            (ProgramCounterUpdate::SkipNext, display)
        }
    }

    fn op_3xnn(&mut self, x: usize, nn: u8) -> (ProgramCounterUpdate, String) {
        let display = format!("If V{x:X} ({}) == {nn}, skip next instr", self.v[x]);
        if self.v[x] == nn {
            (ProgramCounterUpdate::SkipNext, display)
        } else {
            (ProgramCounterUpdate::Next, display)
        }
    }

    fn op_2nnn(&mut self, nnn: usize) -> (ProgramCounterUpdate, String) {
        self.stack[self.sp] = self.pc + 2;
        self.sp += 1;
        let display = format!("Call subroutine at {nnn:#06X}");
        (ProgramCounterUpdate::Jump(nnn), display)
    }

    fn op_00e0(bus: &mut Bus) -> (ProgramCounterUpdate, String) {
        bus.graphics.clear();
        let display = "Clear the screen".into();
        (ProgramCounterUpdate::Next, display)
    }

    fn op_00ee(&mut self) -> (ProgramCounterUpdate, String) {
        self.sp -= 1;
        let display = format!("Return to addr {:#06X}", self.stack[self.sp]);
        (ProgramCounterUpdate::Jump(self.stack[self.sp]), display)
    }

    fn op_1nnn(nnn: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Jump to addr {nnn:#06X}");
        (ProgramCounterUpdate::Jump(nnn), display)
    }

    fn op_fx0a(bus: &mut Bus, x: usize) -> (ProgramCounterUpdate, String) {
        let display = format!("Store next key press in V{x:X}");
        bus.input.request_key_press(x);
        (ProgramCounterUpdate::Next, display)
    }
}
