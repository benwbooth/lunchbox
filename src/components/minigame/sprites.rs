//! 8x8 pixel sprite definitions for the Mario mini-game
#![allow(dead_code)]

/// Each sprite is an 8x8 bitmap where each bit represents one pixel
/// The array has 8 bytes, each byte representing one row (top to bottom)
/// Within each byte, bit 7 is leftmost pixel, bit 0 is rightmost

// Mario standing - simple recognizable shape
pub const MARIO_STANDING: [u8; 8] = [
    0b00011100, // Hat
    0b00111110, // Hat brim
    0b00011010, // Face with eye
    0b00011110, // Face
    0b01111111, // Body with arms
    0b00111110, // Body
    0b00110110, // Legs
    0b01110111, // Feet
];

// Mario jumping - arms up
pub const MARIO_JUMP: [u8; 8] = [
    0b00011100, // Hat
    0b00111110, // Hat brim
    0b00011010, // Face with eye
    0b00011110, // Face
    0b11111111, // Arms up wide
    0b00111110, // Body
    0b00011100, // Legs together
    0b00110110, // Feet apart
];

// Mario walking frame 1
pub const MARIO_WALK1: [u8; 8] = [
    0b00011100, // Hat
    0b00111110, // Hat brim
    0b00011010, // Face with eye
    0b00011110, // Face
    0b01111110, // Body with arm
    0b00111110, // Body
    0b00110100, // One leg forward
    0b01100110, // Feet
];

// Mario walking frame 2
pub const MARIO_WALK2: [u8; 8] = [
    0b00011100, // Hat
    0b00111110, // Hat brim
    0b00011010, // Face with eye
    0b00011110, // Face
    0b01111110, // Body with arm
    0b00111110, // Body
    0b00010110, // Other leg forward
    0b01100110, // Feet
];

// Goomba enemy
pub const GOOMBA: [u8; 8] = [
    0b00111100, // Top of head
    0b01111110, // Head
    0b11011011, // Eyes (angry)
    0b11111111, // Face
    0b01111110, // Body top
    0b00111100, // Body
    0b01100110, // Feet
    0b11100111, // Feet wide
];

// Brick block
pub const BRICK: [u8; 8] = [
    0b11111111,
    0b10010010,
    0b11111111,
    0b01001001,
    0b11111111,
    0b10010010,
    0b11111111,
    0b01001001,
];

// Question block
pub const QUESTION_BLOCK: [u8; 8] = [
    0b11111111,
    0b10000001,
    0b10011001,
    0b10000101,
    0b10001001,
    0b10000001,
    0b10001001,
    0b11111111,
];

// Ground/floor tile
pub const GROUND: [u8; 8] = [
    0b11111111,
    0b11111111,
    0b11101110,
    0b11111111,
    0b11111111,
    0b01110111,
    0b11111111,
    0b11111111,
];

// Colors (RGB as u32: 0x00RRGGBB)
pub const MARIO_RED: u32 = 0x00E52521;       // Classic Mario red
pub const MARIO_SKIN: u32 = 0x00FFB27F;      // Skin tone
pub const MARIO_BROWN: u32 = 0x00723A22;     // Hair/shoes
pub const GOOMBA_BROWN: u32 = 0x008B4513;    // Goomba body
pub const GOOMBA_TAN: u32 = 0x00D2B48C;      // Goomba face
pub const BRICK_COLOR: u32 = 0x00B86B3F;     // Brick orange-brown
pub const BRICK_DARK: u32 = 0x00804020;      // Brick mortar
pub const GROUND_COLOR: u32 = 0x00D87B40;    // Ground orange
pub const GROUND_DARK: u32 = 0x00A05820;     // Ground shadow
pub const SKY_BLUE: u32 = 0x005C94FC;        // Classic SMB sky blue
pub const QUESTION_YELLOW: u32 = 0x00FFB800; // Question block
pub const HIGHLIGHT_RED: u32 = 0x00FF4444;   // Player-controlled Mario highlight
