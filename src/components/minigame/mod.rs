//! Mario mini-game module - plays in the background when no platform is selected
//!
//! Uses WebGPU for rendering - all game logic runs in WGSL shaders.

mod sprites;
mod gpu;

// These modules are only needed for wasm32 target (WebGPU)
#[cfg(target_arch = "wasm32")]
mod game;

#[cfg(target_arch = "wasm32")]
pub use game::MarioMinigame;

// Provide a stub component for non-wasm32 targets (IDE analysis)
#[cfg(not(target_arch = "wasm32"))]
mod game_stub {
    use leptos::prelude::*;

    #[component]
    pub fn MarioMinigame() -> impl IntoView {
        view! { <div>"WebGPU minigame (wasm32 only)"</div> }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use game_stub::MarioMinigame;
