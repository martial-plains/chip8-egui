//! This module provides the input system for the Chip8 emulator. It keeps
//! track of the state of all 16 keys and handles any key press requests
//! from programs.

/// A response for a requested key press by the processor.
///
/// Contains the key code of the pressed key and the register where
/// the processor should store it in.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct KeyRequestResponse {
    /// The key code of the pressed key.
    pub key_code: u8,
    /// The register where the processor should store the key code.
    pub register: usize,
}

/// Input system for the [`super::Chip8`]. Keeps track of the state of all 16 keys
/// and any key press requests from programs.
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Input {
    /// The current state of all 16 keys.
    state: [bool; 16],
    /// Whether the system is currently waiting for user input.
    waiting: bool,
    /// The register where the processor should store the key code for the next input event.
    request_reg: usize,
    /// The response to a previous key press request, if any.
    request_response: Option<KeyRequestResponse>,
}

impl Input {
    /// Creates a new [`Input`] instance with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the input state of the given key code.
    ///
    /// # Arguments
    ///
    /// * `key_code`: The key code of the key that was pressed or released.
    /// * `pressed`: A boolean indicating whether the key was pressed (true)
    ///              or released (false).
    pub fn update(&mut self, key_code: u8, pressed: bool) {
        let key_index = usize::from(key_code);
        if self.state[key_index] == pressed {
            return;
        }
        self.state[key_index] = pressed;

        if pressed && self.waiting {
            self.waiting = false;
            self.request_response = Some(KeyRequestResponse {
                key_code,
                register: self.request_reg,
            });
        }
    }

    /// Requests a single key press from the user.
    ///
    /// # Arguments
    ///
    /// * `register`: The index of the register where the key code should be stored.
    pub fn request_key_press(&mut self, register: usize) {
        self.waiting = true;
        self.request_reg = register;
    }

    /// Returns the input request response.
    ///
    /// This will be `None` if no key press was requested or if the key press
    /// was already consumed.
    pub fn request_response(&mut self) -> Option<KeyRequestResponse> {
        self.request_response.take()
    }

    /// Returns whether the input system is currently waiting for user input.
    #[must_use]
    pub fn waiting(&self) -> bool {
        self.waiting
    }

    /// Returns whether the given key is currently pressed.
    ///
    /// # Arguments
    ///
    /// * `key_code`: The key code of the key to check.
    #[must_use]
    pub fn is_key_pressed(&self, key_code: u8) -> bool {
        self.state[usize::from(key_code)]
    }
}
