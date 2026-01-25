//! Mario mini-game module - plays in the background when no platform is selected
//!
//! Uses WebGPU for rendering - all game logic runs in WGSL shaders.

mod sprites;
mod gpu;
mod game;

pub use game::MarioMinigame;
