// Mario Minigame - Complete GPU Implementation
// All game logic and rendering in a single WGSL shader

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
    kind: u32,      // 0=mario, 1=goomba, 2=koopa, 3=coin, 4=mushroom
    state: u32,     // animation state
    flags: u32,     // bit0=flip_x, bit1=alive, bit2=on_ground, bit3=is_big, bit4=is_player
    timer: u32,     // multi-purpose timer
};

struct Block {
    pos: vec2<f32>,
    kind: u32,      // 0=brick, 1=question, 2=question_empty, 3=ground
    flags: u32,     // bit0=hit, bit1=destroyed
};

struct Platform {
    x: f32,
    y: f32,
    width: f32,
    is_ground: u32,
};

// Bindings
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read_write> entities: array<Entity, 128>;
@group(0) @binding(2) var<storage, read_write> blocks: array<Block, 512>;
@group(0) @binding(3) var<storage, read_write> platforms: array<Platform, 64>;
@group(0) @binding(4) var<storage, read> sprites: array<u32, 256>;  // 16 sprites * 4 u32s each
@group(0) @binding(5) var<storage, read> palettes: array<u32, 48>;  // 12 palettes * 4 colors

//=============================================================================
// CONSTANTS
//=============================================================================

const GRAVITY: f32 = 0.35;
const MAX_FALL: f32 = 6.0;
const JUMP_VEL: f32 = -7.5;
const MOVE_SPEED: f32 = 1.5;
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
// UTILITY FUNCTIONS
//=============================================================================

fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i + vec2<f32>(0.0, 0.0)), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

fn random(seed: f32, offset: f32) -> f32 {
    return hash(vec2<f32>(seed * 12.9898, offset * 78.233 + u.time * 0.001));
}

fn aabb(a_pos: vec2<f32>, a_size: vec2<f32>, b_pos: vec2<f32>, b_size: vec2<f32>) -> bool {
    return a_pos.x < b_pos.x + b_size.x && a_pos.x + a_size.x > b_pos.x &&
           a_pos.y < b_pos.y + b_size.y && a_pos.y + a_size.y > b_pos.y;
}

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

fn draw_sprite(px: vec2<f32>, pos: vec2<f32>, sprite_id: u32, palette_id: u32, flip: bool) -> vec3<f32> {
    let local = px - pos;
    if (local.x >= 0.0 && local.x < 8.0 && local.y >= 0.0 && local.y < 8.0) {
        let ci = get_sprite_pixel(sprite_id, u32(local.x), u32(local.y), flip);
        return get_color(palette_id, ci);
    }
    return vec3<f32>(-1.0);
}

//=============================================================================
// COMPUTE SHADER - GAME LOGIC
//=============================================================================

@compute @workgroup_size(64)
fn update(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;

    // Initialize on first frame
    if (u.frame == 0u) {
        // Initialize platforms
        if (idx < PLATFORM_COUNT) {
            let ground_y = floor(u.resolution.y / TILE) - 2.0;
            if (idx < 8u) {
                // Ground platforms
                let w = 8.0 + random(f32(idx), 1.0) * 12.0;
                platforms[idx].x = f32(idx) * 20.0;
                platforms[idx].y = ground_y;
                platforms[idx].width = w;
                platforms[idx].is_ground = 1u;
            } else {
                // Floating platforms
                let level = f32(idx % 4u);
                platforms[idx].x = random(f32(idx), 2.0) * (u.resolution.x / TILE - 10.0);
                platforms[idx].y = ground_y - 4.0 - level * 4.0;
                platforms[idx].width = 3.0 + random(f32(idx), 3.0) * 5.0;
                platforms[idx].is_ground = 0u;
            }
        }

        // Initialize blocks on floating platforms
        if (idx < BLOCK_COUNT) {
            let plat_idx = idx / 8u;
            if (plat_idx >= 8u && plat_idx < PLATFORM_COUNT) {
                let plat = platforms[plat_idx];
                let block_offset = f32(idx % 8u);
                if (block_offset < plat.width) {
                    blocks[idx].pos = vec2<f32>((plat.x + block_offset) * TILE, plat.y * TILE);
                    blocks[idx].kind = select(0u, 1u, random(f32(idx), 4.0) < 0.2);
                    blocks[idx].flags = 0u;
                } else {
                    blocks[idx].flags = 2u; // destroyed/unused
                }
            } else {
                blocks[idx].flags = 2u;
            }
        }

        // Initialize entities
        if (idx < ENTITY_COUNT) {
            var e: Entity;
            e.flags = FLAG_ALIVE;

            if (idx < 20u) {
                // Marios
                e.kind = KIND_MARIO;
                e.pos = vec2<f32>(
                    random(f32(idx), 5.0) * u.resolution.x,
                    random(f32(idx), 6.0) * u.resolution.y * 0.5
                );
                e.vel = vec2<f32>(select(-MOVE_SPEED, MOVE_SPEED, random(f32(idx), 7.0) > 0.5) * 0.5, 0.0);
                if (idx == 0u) { e.flags = e.flags | FLAG_PLAYER; }
                // Random character palette (Mario, Luigi, Toad, Princess colors)
                e.state = u32(random(f32(idx), 20.0) * 4.0);
            } else if (idx < 35u) {
                // Goombas
                e.kind = KIND_GOOMBA;
                e.pos = vec2<f32>(
                    random(f32(idx), 8.0) * u.resolution.x,
                    random(f32(idx), 9.0) * u.resolution.y * 0.3
                );
                e.vel = vec2<f32>(select(-0.5, 0.5, random(f32(idx), 10.0) > 0.5), 0.0);
            } else if (idx < 42u) {
                // Koopas
                e.kind = KIND_KOOPA;
                e.pos = vec2<f32>(
                    random(f32(idx), 11.0) * u.resolution.x,
                    random(f32(idx), 12.0) * u.resolution.y * 0.3
                );
                e.vel = vec2<f32>(select(-0.4, 0.4, random(f32(idx), 13.0) > 0.5), 0.0);
            } else {
                // Coins
                e.kind = KIND_COIN;
                e.pos = vec2<f32>(
                    random(f32(idx), 14.0) * u.resolution.x,
                    random(f32(idx), 15.0) * u.resolution.y * 0.5 + 50.0
                );
                e.vel = vec2<f32>(0.0, 0.0);
            }

            entities[idx] = e;
        }
        return;
    }

    // Update entities
    if (idx < ENTITY_COUNT) {
        var e = entities[idx];

        if ((e.flags & FLAG_ALIVE) == 0u) {
            entities[idx] = e;
            return;
        }

        let is_player = (e.flags & FLAG_PLAYER) != 0u;
        let on_ground = (e.flags & FLAG_GROUND) != 0u;

        // Gravity
        e.vel.y = min(e.vel.y + GRAVITY, MAX_FALL);

        // Update position
        let old_y = e.pos.y;
        e.pos = e.pos + e.vel;

        // Clear ground flag
        e.flags = e.flags & ~FLAG_GROUND;

        // Platform collision
        for (var i = 0u; i < PLATFORM_COUNT; i = i + 1u) {
            let p = platforms[i];
            let px = p.x * TILE;
            let py = p.y * TILE;
            let pw = p.width * TILE;

            if (e.vel.y > 0.0 &&
                e.pos.x + 8.0 > px && e.pos.x < px + pw &&
                e.pos.y + 8.0 >= py && old_y + 8.0 <= py + 4.0) {
                e.pos.y = py - 8.0;
                e.vel.y = 0.0;
                e.flags = e.flags | FLAG_GROUND;
            }
        }

        // Block collision
        for (var i = 0u; i < BLOCK_COUNT; i = i + 1u) {
            let b = blocks[i];
            if ((b.flags & 2u) != 0u) { continue; }

            if (e.vel.y > 0.0 &&
                e.pos.x + 8.0 > b.pos.x + 2.0 && e.pos.x < b.pos.x + 6.0 &&
                e.pos.y + 8.0 >= b.pos.y && old_y + 8.0 <= b.pos.y + 4.0) {
                e.pos.y = b.pos.y - 8.0;
                e.vel.y = 0.0;
                e.flags = e.flags | FLAG_GROUND;
            }
        }

        // Screen wrap
        if (e.pos.x < -8.0) { e.pos.x = u.resolution.x; }
        if (e.pos.x > u.resolution.x) { e.pos.x = -8.0; }

        // Fall respawn
        if (e.pos.y > u.resolution.y + 32.0) {
            e.pos.y = -16.0;
            e.pos.x = random(f32(idx) + u.time, 16.0) * u.resolution.x;
            e.vel.y = 0.0;
        }

        // AI for non-player entities
        if (!is_player && (e.flags & FLAG_GROUND) != 0u) {
            // Random direction changes
            if (random(f32(idx) + u.time, 17.0) < 0.01) {
                e.vel.x = -e.vel.x;
            }

            // Random jumps (Mario only)
            if (e.kind == KIND_MARIO && random(f32(idx) + u.time, 18.0) < 0.008) {
                e.vel.y = JUMP_VEL * 0.8;
                e.flags = e.flags & ~FLAG_GROUND;
            }

            // Edge detection - reverse at screen edges
            if (e.pos.x < 16.0) { e.vel.x = abs(e.vel.x); }
            if (e.pos.x > u.resolution.x - 24.0) { e.vel.x = -abs(e.vel.x); }
        }

        // Update facing direction
        if (e.vel.x > 0.1) { e.flags = e.flags & ~FLAG_FLIP; }
        else if (e.vel.x < -0.1) { e.flags = e.flags | FLAG_FLIP; }

        // Friction
        if ((e.flags & FLAG_GROUND) != 0u && !is_player) {
            // Keep constant speed for AI
        } else if ((e.flags & FLAG_GROUND) != 0u) {
            e.vel.x = e.vel.x * 0.8;
        }

        // Animation timer
        e.timer = e.timer + 1u;

        entities[idx] = e;
    }
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
