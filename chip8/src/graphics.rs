//! This module provides a simple graphics buffer implementation with a fixed resolution of 64x32 pixels.

use std::mem;

/// The height of the graphics buffer in pixels. This is a constant value
/// set to 32.
pub const HEIGHT: usize = 32;
/// The width of the graphics buffer in pixels. This is a constant value set
/// to 64.
pub const WIDTH: usize = 64;
/// The total number of pixels in the graphics buffer. This is calculated
/// as the product of [`WIDTH`] and [`HEIGHT`].
pub const PIXEL_COUNT: usize = WIDTH * HEIGHT;
/// The default foreground color for the graphics buffer. This is an [`Rgb`]
/// struct with the value `[255, 255, 255]`, representing white.
pub const DEFAULT_FOREGROUND: Rgb = Rgb {
    red: 255,
    green: 255,
    blue: 255,
};
/// The default background color for the graphics buffer. This is an [`Rgb`]
/// struct with the value `[0, 0, 0]`, representing black.
pub const DEFAULT_BACKGROUND: Rgb = Rgb {
    red: 0,
    green: 0,
    blue: 0,
};

/// A struct representing an RGB color with 8 bits per channel. This struct
/// holds 3 fields of [`u8`] values representing the red, green, and blue
/// channels of the color.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    /// Red color
    pub red: u8,
    /// Green color
    pub green: u8,
    /// Blue color
    pub blue: u8,
}

impl Rgb {
    /// Converts struct to an array of 3 [`u8`]
    #[must_use]
    pub fn as_array(&self) -> [u8; 3] {
        [self.red, self.green, self.blue]
    }

    /// Converts array into [`Rgb`]
    #[must_use]
    pub fn from_array(array: [u8; 3]) -> Self {
        Self {
            red: array[0],
            green: array[1],
            blue: array[2],
        }
    }
}

/// A struct representing the graphics buffer. This struct holds a 2D array
/// of [`Rgb`] colors representing the graphics buffer, as well as foreground
/// and background colors. The buffer supports drawing single bytes (8 pixels)
/// with a given position and data, and keeps track of collisions between
/// active pixels.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct Buffer {
    #[serde(with = "serde_big_array::BigArray")]
    vram: [Rgb; PIXEL_COUNT],
    /// An [`Rgb`] value that represents the color used for drawing active pixels.
    pub foreground_rgb: Rgb,
    /// An [`Rgb`] value that represents the color used for drawing inactive
    /// pixels (i.e., the background color).
    pub background_rgb: Rgb,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            vram: [DEFAULT_BACKGROUND; PIXEL_COUNT],
            foreground_rgb: DEFAULT_FOREGROUND,
            background_rgb: DEFAULT_BACKGROUND,
        }
    }
}

impl Buffer {
    /// Creates a new [`Buffer`] instance with the default background and
    /// foreground colors.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws a byte (8 pixels) with the given position and data. Returns a
    /// [`bool`] indicating whether any active pixels in the byte collided
    /// with active pixels already present in the buffer.
    pub fn draw_byte(&mut self, x: usize, y: usize, data: u8) -> bool {
        if y >= PIXEL_COUNT / WIDTH {
            return false;
        }

        let max_x = (WIDTH - x).min(8);
        let bitmasks: [u8; 8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

        let mut collision = false;

        for (b, &mask) in bitmasks.iter().enumerate().take(max_x) {
            let pos = (WIDTH * y) + x + b;
            let new_pixel_active = (data & mask) != 0;
            let old_pixel_active = self.vram[pos] == self.foreground_rgb;
            if new_pixel_active && old_pixel_active {
                collision = true;
            }
            self.vram[pos] = if new_pixel_active ^ old_pixel_active {
                self.foreground_rgb
            } else {
                self.background_rgb
            };
        }
        collision
    }

    /// Sets the foreground color of the buffer to the given [`Rgb`]
    /// value, and updates the colors of all active foreground pixels in the
    /// buffer accordingly.
    #[inline]
    pub fn set_foreground_color(&mut self, foreground: Rgb) {
        let old_color = mem::replace(&mut self.foreground_rgb, foreground);

        for color in &mut self.vram {
            if *color == old_color {
                *color = foreground;
            }
        }
    }

    /// Sets the background color of the buffer to the given [`Rgb`]
    /// value,and updates the colors of all background pixels in the buffer
    /// accordingly.
    #[inline]
    pub fn set_background_color(&mut self, background: Rgb) {
        let old_color = mem::replace(&mut self.background_rgb, background);

        for color in &mut self.vram {
            if *color == old_color {
                *color = background;
            }
        }
    }

    /// Returns the graphics buffer as a flat array of [`Rgb`] values.
    #[must_use]
    pub fn as_rgb8(&self) -> [u8; PIXEL_COUNT * 3] {
        let mut data = [0; PIXEL_COUNT * 3];
        for (i, pixel) in self.vram.iter().enumerate() {
            let offset = i * 3;
            data[offset] = pixel.red;
            data[offset + 1] = pixel.green;
            data[offset + 2] = pixel.blue;
        }
        data
    }

    /// Clears the graphics buffer by setting all pixels to the current background color.
    #[inline]
    pub fn clear(&mut self) {
        self.vram = [self.background_rgb; PIXEL_COUNT];
    }
}
