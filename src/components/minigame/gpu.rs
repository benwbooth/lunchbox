//! Minimal WebGPU bootstrap - all game logic is in WGSL shaders

#![allow(dead_code)]

use super::sprites::*;

/// Pack sprite data for GPU upload
pub fn pack_sprite_atlas() -> Vec<u32> {
    let mut atlas = Vec::new();

    let sprite_list = [
        MARIO_STAND, MARIO_WALK1, MARIO_WALK2, MARIO_JUMP,
        GOOMBA, BRICK, QUESTION, GROUND,
        KOOPA_WALK, COIN, MUSHROOM, MARIO_DEAD,
        QUESTION_EMPTY, KOOPA_SHELL, BRICK_DEBRIS,
        MARIO_BIG_STAND_TOP,
    ];

    for sprite in &sprite_list {
        // Pack 8x8 sprite (2 bits per pixel) into 4 u32s
        let mut packed = [0u32; 4];
        for row in 0..8 {
            let lo = sprite[row * 2];
            let hi = sprite[row * 2 + 1];
            for col in 0..8 {
                let bit = 7 - col;
                let color_idx = ((lo >> bit) & 1) | (((hi >> bit) & 1) << 1);
                let pixel_idx = row * 8 + col;
                let word_idx = pixel_idx / 16;
                let bit_idx = (pixel_idx % 16) * 2;
                packed[word_idx] |= (color_idx as u32) << bit_idx;
            }
        }
        atlas.extend_from_slice(&packed);
    }

    // Pad to 256 u32s (16 sprites * 4 u32s = 64, but shader expects 256)
    atlas.resize(256, 0);
    atlas
}

/// Pack palette data for GPU upload
pub fn pack_palettes() -> Vec<u32> {
    let palette_list: [Palette; 12] = [
        PALETTE_MARIO,      // 0
        PALETTE_LUIGI,      // 1
        PALETTE_GOOMBA,     // 2
        PALETTE_BRICK,      // 3
        PALETTE_QUESTION,   // 4
        PALETTE_GROUND,     // 5
        PALETTE_KOOPA,      // 6
        PALETTE_COIN,       // 7
        PALETTE_PLAYER,     // 8
        PALETTE_MUSHROOM,   // 9
        PALETTE_TOAD,       // 10
        PALETTE_PRINCESS,   // 11
    ];

    let mut palettes = Vec::new();
    for pal in &palette_list {
        palettes.extend_from_slice(pal);
    }
    palettes
}

/// Uniforms struct matching WGSL layout (32 bytes)
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Uniforms {
    pub resolution: [f32; 2],
    pub time: f32,
    pub delta_time: f32,
    pub mouse: [f32; 2],
    pub mouse_click: u32,
    pub frame: u32,
}

impl Uniforms {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(&self.resolution[0].to_le_bytes());
        bytes.extend_from_slice(&self.resolution[1].to_le_bytes());
        bytes.extend_from_slice(&self.time.to_le_bytes());
        bytes.extend_from_slice(&self.delta_time.to_le_bytes());
        bytes.extend_from_slice(&self.mouse[0].to_le_bytes());
        bytes.extend_from_slice(&self.mouse[1].to_le_bytes());
        bytes.extend_from_slice(&self.mouse_click.to_le_bytes());
        bytes.extend_from_slice(&self.frame.to_le_bytes());
        bytes
    }
}

/// Convert Vec<u32> to bytes
pub fn u32_slice_to_bytes(data: &[u32]) -> Vec<u8> {
    data.iter().flat_map(|v| v.to_le_bytes()).collect()
}
