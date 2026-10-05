#![no_main]
#![no_std]

mod bongocat_frames;
mod bongocat_renderer;

// RMK generates hardware setup, storage, macros, and the display task from TOML.
#[rmk::macros::rmk_keyboard]
mod keyboard {}
