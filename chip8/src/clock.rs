//! This module provides a clock for updating the sound and delay timers of
//! a Chip8 emulator. The [`Clock`] struct keeps track of the current value of
//! the delay timer, the sound timer, and whether a vblank interrupt has occurred.
//!
//! The delay timer and the sound timer are decremented at a rate of 60Hz, which is
//! the frequency at which the timers are updated.

use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

/// Handles the updating of the [`super::Chip8`] sound and delay timers. The `delay_timer` and
/// the `sound_timer` are decremented by `1` at a rate of `60Hz`.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Clock {
    /// The current value of the delay timer.
    pub delay_timer: u8,
    /// The current value of the sound timer, stored in an atomic variable for thread-safety.
    #[serde(skip)]
    pub sound_timer: Arc<AtomicU8>,
    /// A flag indicating whether a vblank interrupt has occurred.
    pub vblank_interrupt: bool,
    /// The time at which the last delay timer update occurred.
    #[cfg_attr(not(target_arch = "wasm32"), serde(skip, default = "Instant::now"))]
    #[cfg(not(target_arch = "wasm32"))]
    last_delay: Instant,
    #[cfg(target_arch = "wasm32")]
    last_delay: f64,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            delay_timer: Default::default(),
            sound_timer: Arc::default(),
            #[cfg(not(target_arch = "wasm32"))]
            last_delay: Instant::now(),
            #[cfg(target_arch = "wasm32")]
            last_delay: f64::default(),
            vblank_interrupt: Default::default(),
        }
    }
}

impl Clock {
    /// The frequency (in Hz) at which the timers are updated.
    const TIMER_FREQUENCY_HZ: f64 = 60.0;

    /// Create a new `Clock`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn update(&mut self) {
        let elapsed_time = self.last_delay.elapsed().as_secs_f64();

        if elapsed_time >= 1.0 / Self::TIMER_FREQUENCY_HZ {
            self.delay_timer = self.delay_timer.saturating_sub(1);
            self.sound_timer
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                    Some(x.saturating_sub(1))
                })
                .unwrap_or_default();
            self.vblank_interrupt = true;
            self.last_delay += Duration::from_secs_f64(1.0 / Self::TIMER_FREQUENCY_HZ);
        } else {
            self.vblank_interrupt = false;
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn update(&mut self) {
        let current_time = js_sys::Date::now();
        let elapsed_time = current_time - self.last_delay;

        if elapsed_time >= 1.0 / Self::TIMER_FREQUENCY_HZ {
            self.delay_timer = self.delay_timer.saturating_sub(1);
            self.sound_timer
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                    Some(x.saturating_sub(1))
                })
                .unwrap_or_default();
            self.vblank_interrupt = true;
            self.last_delay = current_time;
        } else {
            self.vblank_interrupt = false;
        }
    }
}
