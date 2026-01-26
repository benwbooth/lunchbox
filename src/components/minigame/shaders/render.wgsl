// Mario Minigame - Render Shader
// Fragment shader for sprite rendering (separate module to avoid binding conflicts)

//=============================================================================
// DATA STRUCTURES
//=============================================================================

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    delta_time: f32,
    mouse: vec2<f32>,
    mouse_click: u32,
    frame: u32,
};

struct Entity {
    pos: vec2<f32>,
    vel: vec2<f32>,
    kind: u32,
    state: u32,
    flags: u32,
    timer: u32,
};

struct Block {
    pos: vec2<f32>,
    kind: u32,
    flags: u32,
};

struct Platform {
    x: f32,
    y: f32,
    width: f32,
    is_ground: u32,
};

// Bindings - read-only for fragment shader (same binding indices, separate shader module)
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read> entities: array<Entity, 128>;
@group(0) @binding(2) var<storage, read> blocks: array<Block, 512>;
@group(0) @binding(3) var<storage, read> platforms: array<Platform, 64>;
@group(0) @binding(4) var<storage, read> sprites: array<u32, 256>;
@group(0) @binding(5) var<storage, read> palettes: array<u32, 48>;

//=============================================================================
// CONSTANTS
//=============================================================================

const TILE: f32 = 8.0;

const ENTITY_COUNT: u32 = 48u;
const BLOCK_COUNT: u32 = 256u;
const PLATFORM_COUNT: u32 = 32u;

// Entity kinds
const KIND_MARIO: u32 = 0u;
const KIND_GOOMBA: u32 = 1u;
const KIND_KOOPA: u32 = 2u;
const KIND_COIN: u32 = 3u;
const KIND_MUSHROOM: u32 = 4u;

// Sprite indices
const SPR_MARIO_STAND: u32 = 0u;
const SPR_MARIO_WALK1: u32 = 1u;
const SPR_MARIO_WALK2: u32 = 2u;
const SPR_MARIO_JUMP: u32 = 3u;
const SPR_GOOMBA: u32 = 4u;
const SPR_BRICK: u32 = 5u;
const SPR_QUESTION: u32 = 6u;
const SPR_GROUND: u32 = 7u;
const SPR_KOOPA: u32 = 8u;
const SPR_COIN: u32 = 9u;
const SPR_MUSHROOM: u32 = 10u;

// Palette indices
const PAL_MARIO: u32 = 0u;
const PAL_LUIGI: u32 = 1u;
const PAL_GOOMBA: u32 = 2u;
const PAL_BRICK: u32 = 3u;
const PAL_QUESTION: u32 = 4u;
const PAL_GROUND: u32 = 5u;
const PAL_KOOPA: u32 = 6u;
const PAL_COIN: u32 = 7u;
const PAL_PLAYER: u32 = 8u;

// Flags
const FLAG_FLIP: u32 = 1u;
const FLAG_ALIVE: u32 = 2u;
const FLAG_GROUND: u32 = 4u;
const FLAG_BIG: u32 = 8u;
const FLAG_PLAYER: u32 = 16u;

//=============================================================================
// SPRITE RENDERING
//=============================================================================

fn get_sprite_pixel(sprite_id: u32, x: u32, y: u32, flip: bool) -> u32 {
    let px = select(x, 7u - x, flip);
    let idx = y * 8u + px;
    let word_idx = sprite_id * 4u + idx / 16u;
    let bit_idx = (idx % 16u) * 2u;
    return (sprites[word_idx] >> bit_idx) & 3u;
}

fn get_color(palette_id: u32, color_idx: u32) -> vec3<f32> {
    if (color_idx == 0u) { return vec3<f32>(-1.0); }
    let packed = palettes[palette_id * 4u + color_idx];
    return vec3<f32>(
        f32((packed >> 16u) & 0xFFu) / 255.0,
        f32((packed >> 8u) & 0xFFu) / 255.0,
        f32(packed & 0xFFu) / 255.0
    );
}

fn draw_sprite(pixel: vec2<f32>, pos: vec2<f32>, sprite_id: u32, palette_id: u32, flip: bool) -> vec3<f32> {
    if (pixel.x >= pos.x && pixel.x < pos.x + TILE &&
        pixel.y >= pos.y && pixel.y < pos.y + TILE) {
        let lx = u32(pixel.x - pos.x);
        let ly = u32(pixel.y - pos.y);
        return get_color(palette_id, get_sprite_pixel(sprite_id, lx, ly, flip));
    }
    return vec3<f32>(-1.0);
}

//=============================================================================
// VERTEX SHADER
//=============================================================================

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOut {
    var out: VertexOut;
    let x = f32(i32(vi) - 1) * 2.0;
    let y = f32(i32(vi & 1u) * 2 - 1) * 2.0;
    out.pos = vec4<f32>(x, -y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (y + 1.0) * 0.5);
    return out;
}

//=============================================================================
// FRAGMENT SHADER - RENDERING
//=============================================================================

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let px = in.uv * u.resolution;

    // Background - dark blue gradient
    var col = mix(vec3<f32>(0.0, 0.0, 0.1), vec3<f32>(0.0, 0.0, 0.0), in.uv.y);

    // Draw platforms (ground)
    for (var i = 0u; i < PLATFORM_COUNT; i = i + 1u) {
        let p = platforms[i];
        if (p.is_ground == 0u) { continue; }

        let plat_pos = vec2<f32>(p.x * TILE, p.y * TILE);
        let plat_size = vec2<f32>(p.width * TILE, TILE);

        if (px.x >= plat_pos.x && px.x < plat_pos.x + plat_size.x &&
            px.y >= plat_pos.y && px.y < plat_pos.y + plat_size.y) {
            let lx = u32(px.x - plat_pos.x) % 8u;
            let ly = u32(px.y - plat_pos.y) % 8u;
            let c = get_color(PAL_GROUND, get_sprite_pixel(SPR_GROUND, lx, ly, false));
            if (c.r >= 0.0) { col = c; }
        }
    }

    // Draw blocks
    for (var i = 0u; i < BLOCK_COUNT; i = i + 1u) {
        let b = blocks[i];
        if ((b.flags & 2u) != 0u) { continue; }

        if (px.x >= b.pos.x && px.x < b.pos.x + TILE &&
            px.y >= b.pos.y && px.y < b.pos.y + TILE) {
            let lx = u32(px.x - b.pos.x);
            let ly = u32(px.y - b.pos.y);
            var spr = SPR_BRICK;
            var pal = PAL_BRICK;
            if (b.kind == 1u) { spr = SPR_QUESTION; pal = PAL_QUESTION; }
            let c = get_color(pal, get_sprite_pixel(spr, lx, ly, false));
            if (c.r >= 0.0) { col = c; }
        }
    }

    // Draw entities (back to front by kind)
    // Coins first
    for (var i = 0u; i < ENTITY_COUNT; i = i + 1u) {
        let e = entities[i];
        if ((e.flags & FLAG_ALIVE) == 0u || e.kind != KIND_COIN) { continue; }
        let c = draw_sprite(px, e.pos, SPR_COIN, PAL_COIN, false);
        if (c.r >= 0.0) { col = c; }
    }

    // Goombas
    for (var i = 0u; i < ENTITY_COUNT; i = i + 1u) {
        let e = entities[i];
        if ((e.flags & FLAG_ALIVE) == 0u || e.kind != KIND_GOOMBA) { continue; }
        let flip = (e.flags & FLAG_FLIP) != 0u;
        let c = draw_sprite(px, e.pos, SPR_GOOMBA, PAL_GOOMBA, flip);
        if (c.r >= 0.0) { col = c; }
    }

    // Koopas
    for (var i = 0u; i < ENTITY_COUNT; i = i + 1u) {
        let e = entities[i];
        if ((e.flags & FLAG_ALIVE) == 0u || e.kind != KIND_KOOPA) { continue; }
        let flip = (e.flags & FLAG_FLIP) != 0u;
        let c = draw_sprite(px, e.pos, SPR_KOOPA, PAL_KOOPA, flip);
        if (c.r >= 0.0) { col = c; }
    }

    // Marios last (on top)
    for (var i = 0u; i < ENTITY_COUNT; i = i + 1u) {
        let e = entities[i];
        if ((e.flags & FLAG_ALIVE) == 0u || e.kind != KIND_MARIO) { continue; }

        let flip = (e.flags & FLAG_FLIP) != 0u;
        let on_ground = (e.flags & FLAG_GROUND) != 0u;
        let is_player = (e.flags & FLAG_PLAYER) != 0u;

        // Select sprite based on state
        var spr = SPR_MARIO_STAND;
        if (!on_ground) {
            spr = SPR_MARIO_JUMP;
        } else if (abs(e.vel.x) > 0.3) {
            spr = select(SPR_MARIO_WALK1, SPR_MARIO_WALK2, (e.timer / 8u) % 2u == 1u);
        }

        // Select palette - use character variant stored in state, or player palette
        var pal = PAL_MARIO;
        if (is_player) {
            pal = PAL_PLAYER;
        } else if (e.state == 1u) {
            pal = PAL_LUIGI;
        }

        let c = draw_sprite(px, e.pos, spr, pal, flip);
        if (c.r >= 0.0) { col = c; }

        // Player indicator (white dot above head)
        if (is_player) {
            let indicator_pos = e.pos + vec2<f32>(3.0, -4.0);
            if (px.x >= indicator_pos.x && px.x < indicator_pos.x + 2.0 &&
                px.y >= indicator_pos.y && px.y < indicator_pos.y + 2.0) {
                col = vec3<f32>(1.0, 1.0, 1.0);
            }
        }
    }

    return vec4<f32>(col, 1.0);
}
