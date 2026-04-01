//! Mario mini-game module - plays in the background when no platform is selected
//!
//! Uses WebGPU for rendering - all game logic runs in WGSL shaders.

mod game;
mod gpu;
mod sprites;

pub use game::MarioMinigame;
