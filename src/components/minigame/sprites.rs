//! 8x8 pixel sprite definitions using NES-style 4-color palettes
#![allow(dead_code)]

/// Each sprite is an 8x8 bitmap with 2 bits per pixel (4 colors: 0=transparent, 1-3=palette colors)
/// Stored as 16 bytes: 2 bytes per row (low bits first, then high bits)
/// This mimics NES PPU tile format

/// Sprite data: 8 rows, each row has 8 pixels with 2 bits each
/// Format: [row0_lo, row0_hi, row1_lo, row1_hi, ...]
pub type Sprite = [u8; 16];

/// Color palette: [transparent, color1, color2, color3]
pub type Palette = [u32; 4];

// NES Color palette (full 64-color palette)
// Dark colors (row 0x00)
pub const NES_GRAY_DARK: u32 = 0x626262;
pub const NES_BLUE_DARK: u32 = 0x002389;
pub const NES_BLUE_INDIGO: u32 = 0x1412AD;
pub const NES_PURPLE_DARK: u32 = 0x3B00A4;
pub const NES_MAGENTA_DARK: u32 = 0x5D0074;
pub const NES_RED_DARK: u32 = 0x6F0032;
pub const NES_RED_BROWN: u32 = 0x6C0700;
pub const NES_BROWN_DARK: u32 = 0x561B00;
pub const NES_OLIVE: u32 = 0x333500;
pub const NES_GREEN_DARK: u32 = 0x0C4800;
pub const NES_GREEN_FOREST: u32 = 0x005200;
pub const NES_TEAL_DARK: u32 = 0x004C18;
pub const NES_CYAN_DARK: u32 = 0x003E5B;
pub const NES_BLACK: u32 = 0x000000;

// Medium colors (row 0x10)
pub const NES_GRAY: u32 = 0xABABAB;
pub const NES_BLUE: u32 = 0x0058F8;
pub const NES_BLUE_BRIGHT: u32 = 0x3545FF;
pub const NES_BLUE_PURPLE: u32 = 0x6F35FF;
pub const NES_PURPLE: u32 = 0x9D28E0;
pub const NES_MAGENTA: u32 = 0xB72890;
pub const NES_RED_ORANGE: u32 = 0xB63725;
pub const NES_ORANGE_BROWN: u32 = 0x994F00;
pub const NES_YELLOW_GREEN: u32 = 0x6C6D00;
pub const NES_GREEN: u32 = 0x00A800;
pub const NES_GREEN_BRIGHT: u32 = 0x0C9300;
pub const NES_SEA_GREEN: u32 = 0x008F4B;
pub const NES_CYAN: u32 = 0x007DA8;

// Bright colors (row 0x20)
pub const NES_WHITE: u32 = 0xFFFFFF;
pub const NES_SKY_BLUE: u32 = 0x49AEFF;
pub const NES_BLUE_LIGHT: u32 = 0x7D9AFF;
pub const NES_LAVENDER: u32 = 0xB588FF;
pub const NES_PURPLE_LIGHT: u32 = 0xE479FF;
pub const NES_PINK: u32 = 0xFF77CC;
pub const NES_SALMON: u32 = 0xFF8577;
pub const NES_ORANGE: u32 = 0xFC9838;
pub const NES_YELLOW: u32 = 0xFCE4A0;
pub const NES_LIME: u32 = 0x8CD252;
pub const NES_GREEN_LIGHT: u32 = 0x5DE07B;
pub const NES_MINT: u32 = 0x49DEA3;
pub const NES_CYAN_LIGHT: u32 = 0x49D5D1;
pub const NES_GRAY_MEDIUM: u32 = 0x424242;

// Pale/pastel colors (row 0x30)
pub const NES_PALE_BLUE: u32 = 0xB3E0FF;
pub const NES_PALE_LAVENDER: u32 = 0xC8D4FF;
pub const NES_PALE_PURPLE: u32 = 0xDFCCFF;
pub const NES_PALE_PINK: u32 = 0xF2C5FF;
pub const NES_PINK_LIGHT: u32 = 0xFFC4EA;
pub const NES_PALE_SALMON: u32 = 0xFFC8C3;
pub const NES_PALE_ORANGE: u32 = 0xF9D5A6;
pub const NES_PALE_YELLOW: u32 = 0xE8E29B;
pub const NES_PALE_LIME: u32 = 0xCEED9C;
pub const NES_PALE_GREEN: u32 = 0xB6F4AB;
pub const NES_PALE_MINT: u32 = 0xA8F3C5;
pub const NES_PALE_CYAN: u32 = 0xA8EFEB;
pub const NES_GRAY_LIGHT: u32 = 0xA8A8A8;

// Game-specific colors (Super Mario Bros)
pub const NES_MARIO_RED: u32 = 0xE45C10;
pub const NES_BRICK_BROWN: u32 = 0x8C4B14;
pub const NES_SKIN: u32 = 0xFCBCA0;
pub const NES_BROWN: u32 = 0x503000;

// Palettes
pub const PALETTE_MARIO: Palette = [0, NES_MARIO_RED, NES_SKIN, NES_BRICK_BROWN];
pub const PALETTE_MARIO_BIG: Palette = [0, NES_MARIO_RED, NES_SKIN, NES_BRICK_BROWN];
pub const PALETTE_LUIGI: Palette = [0, NES_GREEN, NES_SKIN, NES_BRICK_BROWN];
pub const PALETTE_TOAD: Palette = [0, NES_MARIO_RED, NES_WHITE, NES_SKIN];
pub const PALETTE_PRINCESS: Palette = [0, NES_PINK, NES_YELLOW, NES_SKIN];
pub const PALETTE_GOOMBA: Palette = [0, NES_BRICK_BROWN, NES_SKIN, NES_BLACK];
pub const PALETTE_BRICK: Palette = [0, NES_ORANGE, NES_BRICK_BROWN, NES_BROWN];
pub const PALETTE_QUESTION: Palette = [0, NES_YELLOW, NES_ORANGE, NES_BLACK];
pub const PALETTE_GROUND: Palette = [0, NES_ORANGE, NES_BRICK_BROWN, NES_BROWN];
pub const PALETTE_MUSHROOM: Palette = [0, NES_MARIO_RED, NES_WHITE, NES_SKIN];
pub const PALETTE_PLAYER: Palette = [0, NES_WHITE, NES_SKIN, NES_MARIO_RED];
pub const PALETTE_PLAYER_LUIGI: Palette = [0, NES_WHITE, NES_SKIN, NES_GREEN];
pub const PALETTE_PLAYER_TOAD: Palette = [0, NES_WHITE, NES_SKIN, NES_MARIO_RED];
pub const PALETTE_PLAYER_PRINCESS: Palette = [0, NES_WHITE, NES_SKIN, NES_PINK];
pub const PALETTE_KOOPA: Palette = [0, NES_GREEN, NES_SKIN, NES_WHITE];
pub const PALETTE_COIN: Palette = [0, NES_YELLOW, NES_ORANGE, NES_BROWN];
pub const PALETTE_STAR: Palette = [0, NES_YELLOW, NES_WHITE, NES_BLACK];
pub const PALETTE_FIREBALL: Palette = [0, NES_ORANGE, NES_YELLOW, NES_WHITE];

/// Helper to create sprite from visual representation
/// Each char: '.' = 0 (transparent), '1' = color 1, '2' = color 2, '3' = color 3
pub const fn sprite_from_str(s: &[u8; 64]) -> Sprite {
    let mut result = [0u8; 16];
    let mut row = 0;
    while row < 8 {
        let mut lo = 0u8;
        let mut hi = 0u8;
        let mut col = 0;
        while col < 8 {
            let ch = s[row * 8 + col];
            let val = match ch {
                b'.' | b'0' => 0,
                b'1' => 1,
                b'2' => 2,
                b'3' => 3,
                _ => 0,
            };
            let bit = 7 - col;
            if val & 1 != 0 { lo |= 1 << bit; }
            if val & 2 != 0 { hi |= 1 << bit; }
            col += 1;
        }
        result[row * 2] = lo;
        result[row * 2 + 1] = hi;
        row += 1;
    }
    result
}

// Mario standing (small)
pub const MARIO_STAND: Sprite = sprite_from_str(b"\
...111..\
..1111..\
..3221..\
.32123..\
.332233.\
..1111..\
..1331..\
..33.33.");

// Mario walking frame 1
pub const MARIO_WALK1: Sprite = sprite_from_str(b"\
...111..\
..1111..\
..3221..\
.32123..\
.332233.\
..1111..\
.13..31.\
.33..33.");

// Mario walking frame 2
pub const MARIO_WALK2: Sprite = sprite_from_str(b"\
...111..\
..1111..\
..3221..\
.32123..\
.332233.\
..1111..\
..3113..\
.33..33.");

// Mario jumping
pub const MARIO_JUMP: Sprite = sprite_from_str(b"\
...111..\
..1111..\
..3221..\
.32123..\
1332233.\
.111111.\
..3113..\
.33..33.");

// Big Mario standing (top half)
pub const MARIO_BIG_STAND_TOP: Sprite = sprite_from_str(b"\
...111..\
..11111.\
..3322..\
.322122.\
.3221222\
..3223..\
...111..\
..11111.");

// Big Mario standing (bottom half)
pub const MARIO_BIG_STAND_BOT: Sprite = sprite_from_str(b"\
..11111.\
.111111.\
.1133311\
..3333..\
..3333..\
..33.33.\
..33.33.\
.333.333");

// Big Mario walking top
pub const MARIO_BIG_WALK_TOP: Sprite = sprite_from_str(b"\
...111..\
..11111.\
..3322..\
.322122.\
.3221222\
..3223..\
...111..\
..11111.");

// Big Mario walking bottom
pub const MARIO_BIG_WALK_BOT: Sprite = sprite_from_str(b"\
..11111.\
.1111111\
.1133311\
..3333..\
..3.33..\
..33.33.\
.33...33\
333...33");

// Goomba
pub const GOOMBA: Sprite = sprite_from_str(b"\
..1111..\
.111111.\
12211221\
11111111\
.111111.\
..2222..\
.22..22.\
22....22");

// Brick block - clean 8x8 with two bricks per row, offset pattern
// 1=highlight, 2=brick face, 3=dark mortar
pub const BRICK: Sprite = sprite_from_str(b"\
22221222\
22221222\
22221222\
33333333\
12222122\
12222122\
12222122\
33333333");

// Question block (frame 1) - clear ? shape
// 1=yellow(border), 2=orange(background), 3=black(? shape)
pub const QUESTION: Sprite = sprite_from_str(b"\
11111111\
12333321\
12222321\
12233321\
12232221\
12222221\
12233221\
11111111");

// Question block (hit/empty)
pub const QUESTION_EMPTY: Sprite = sprite_from_str(b"\
33333333\
31111113\
31333313\
31333313\
31333313\
31333313\
31111113\
33333333");

// Ground block - earthy texture with dark bottom
// 1=highlight/light, 2=main color, 3=dark/shadow
pub const GROUND: Sprite = sprite_from_str(b"\
11111111\
12212212\
22122212\
12212122\
22122122\
12222212\
22222222\
33333333");

// Mushroom
pub const MUSHROOM: Sprite = sprite_from_str(b"\
..1111..\
.111111.\
11211211\
12222221\
11111111\
..3333..\
.322223.\
.333333.");

// Brick debris (small piece)
pub const BRICK_DEBRIS: Sprite = sprite_from_str(b"\
........\
........\
..12....\
.1231...\
.1211...\
..11....\
........\
........");

// Death sprite (Mario falling)
pub const MARIO_DEAD: Sprite = sprite_from_str(b"\
..3223..\
.322223.\
..1111..\
.111111.\
..1111..\
...11...\
..1111..\
..3..3..");

// Koopa (green turtle) walking - uses PALETTE_KOOPA
// 1=green(shell), 2=tan(skin), 3=white(eyes/belly)
pub const KOOPA_WALK: Sprite = sprite_from_str(b"\
..111...\
.11111..\
.13311..\
.11111..\
..333...\
..222...\
.22.22..\
.22.22..");

// Koopa shell (when stomped)
pub const KOOPA_SHELL: Sprite = sprite_from_str(b"\
........\
..1111..\
.111111.\
11333311\
12222221\
13333331\
........\
........");

// Coin sprite - uses PALETTE_COIN
// 1=yellow, 2=orange(shading), 3=dark(outline)
pub const COIN: Sprite = sprite_from_str(b"\
..3333..\
.311113.\
31222213\
31222213\
31222213\
31222213\
.311113.\
..3333..");

// Fire flower power-up
pub const FIRE_FLOWER: Sprite = sprite_from_str(b"\
...11...\
..1221..\
.122221.\
..1221..\
...33...\
..3333..\
..3..3..\
...33...");

// Invincibility star
pub const STAR: Sprite = sprite_from_str(b"\
...11...\
..1111..\
.111111.\
11111111\
.113331.\
..13.31.\
..1...1.\
.1....1.");

// Fireball projectile
pub const FIREBALL: Sprite = sprite_from_str(b"\
........\
...11...\
..1221..\
.122221.\
.122221.\
..1221..\
...11...\
........");
