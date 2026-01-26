// Mario Minigame - Compute Shader
// Game logic update shader (separate module to avoid binding conflicts)

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

// Bindings - read_write for compute
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read_write> entities: array<Entity, 128>;
@group(0) @binding(2) var<storage, read_write> blocks: array<Block, 512>;
@group(0) @binding(3) var<storage, read_write> platforms: array<Platform, 512>;
@group(0) @binding(4) var<storage, read> sprites: array<u32, 256>;
@group(0) @binding(5) var<storage, read> palettes: array<u32, 48>;

//=============================================================================
// CONSTANTS
//=============================================================================

const GRAVITY: f32 = 0.35;
const MAX_FALL: f32 = 6.0;
const JUMP_VEL: f32 = -7.5;
const MOVE_SPEED: f32 = 1.5;
const TILE: f32 = 8.0;

const ENTITY_COUNT: u32 = 128u;   // More entities
const BLOCK_COUNT: u32 = 512u;
const PLATFORM_COUNT: u32 = 512u; // Many more platforms for dense coverage

// Entity kinds
const KIND_MARIO: u32 = 0u;
const KIND_GOOMBA: u32 = 1u;
const KIND_KOOPA: u32 = 2u;
const KIND_COIN: u32 = 3u;
const KIND_MUSHROOM: u32 = 4u;

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

fn random(seed: f32, offset: f32) -> f32 {
    return hash(vec2<f32>(seed * 12.9898, offset * 78.233 + u.time * 0.001));
}

//=============================================================================
// COMPUTE SHADER - GAME LOGIC
//=============================================================================

@compute @workgroup_size(64)
fn update(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;

    // Initialize on first frame
    if (u.frame == 0u) {
        // Initialize platforms - dense grid so Mario can always jump to another
        if (idx < PLATFORM_COUNT) {
            let screen_tiles_x = floor(u.resolution.x / TILE);
            let screen_tiles_y = floor(u.resolution.y / TILE);
            let ground_y = screen_tiles_y - 2.0;

            // Jump distance is roughly 4-6 tiles horizontal, 3-4 tiles vertical
            // So we want platforms spaced about 5 tiles apart horizontally, 3 tiles vertically
            let jump_h = 5.0;  // Horizontal jump distance
            let jump_v = 3.0;  // Vertical jump distance

            let cols = u32(screen_tiles_x / jump_h) + 1u;
            let rows = u32(screen_tiles_y / jump_v);

            if (idx < 12u) {
                // Ground platforms - full coverage with small gaps
                let segment_width = screen_tiles_x / 12.0;
                platforms[idx].x = f32(idx) * segment_width;
                platforms[idx].y = ground_y;
                platforms[idx].width = segment_width - 1.0;
                platforms[idx].is_ground = 1u;
            } else {
                // Floating platforms in a dense grid pattern
                let grid_idx = idx - 12u;
                let col = grid_idx % cols;
                let row = grid_idx / cols;

                if (row < rows - 1u) {  // Don't place platforms at ground level
                    // Base position on grid
                    let base_x = f32(col) * jump_h;
                    let base_y = f32(row) * jump_v + 4.0;  // Start 4 tiles from top

                    // Add some randomness but keep within jump range
                    let rand_x = (random(f32(idx), 1.0) - 0.5) * 2.0;  // -1 to 1
                    let rand_y = (random(f32(idx), 2.0) - 0.5) * 1.5;  // -0.75 to 0.75

                    // Stagger odd rows
                    let stagger = select(0.0, jump_h * 0.5, row % 2u == 1u);

                    platforms[idx].x = base_x + stagger + rand_x;
                    platforms[idx].y = base_y + rand_y;
                    platforms[idx].width = 3.0 + random(f32(idx), 3.0) * 3.0;  // 3-6 tiles wide
                    platforms[idx].is_ground = 0u;

                    // Make sure platform stays on screen
                    if (platforms[idx].x < 0.0) {
                        platforms[idx].x = 0.0;
                    }
                    if (platforms[idx].x + platforms[idx].width > screen_tiles_x) {
                        platforms[idx].x = screen_tiles_x - platforms[idx].width;
                    }
                } else {
                    // Unused platform
                    platforms[idx].width = 0.0;
                }
            }
        }

        // Initialize blocks - some on platforms, some floating
        if (idx < BLOCK_COUNT) {
            // Scatter blocks across the screen
            let screen_tiles_x = floor(u.resolution.x / TILE);
            let screen_tiles_y = floor(u.resolution.y / TILE);

            // Place blocks in a loose grid with randomness
            let block_cols = 16u;
            let block_rows = 12u;
            let col = idx % block_cols;
            let row = idx / block_cols;

            if (row < block_rows) {
                let col_spacing = screen_tiles_x / f32(block_cols);
                let row_spacing = (screen_tiles_y - 8.0) / f32(block_rows);

                blocks[idx].pos = vec2<f32>(
                    (f32(col) * col_spacing + random(f32(idx), 30.0) * 2.0) * TILE,
                    (f32(row) * row_spacing + 4.0 + random(f32(idx), 31.0) * 2.0) * TILE
                );
                blocks[idx].kind = select(0u, 1u, random(f32(idx), 4.0) < 0.3);  // 30% question blocks
                blocks[idx].flags = 0u;
            } else {
                blocks[idx].flags = 2u; // unused
            }
        }

        // Initialize entities spread across the screen
        if (idx < ENTITY_COUNT) {
            var e: Entity;
            e.flags = FLAG_ALIVE;

            if (idx < 40u) {
                // Marios - many of them spread across screen
                e.kind = KIND_MARIO;
                e.pos = vec2<f32>(
                    random(f32(idx), 5.0) * u.resolution.x,
                    random(f32(idx), 6.0) * u.resolution.y * 0.9
                );
                e.vel = vec2<f32>(select(-MOVE_SPEED, MOVE_SPEED, random(f32(idx), 7.0) > 0.5) * 0.5, 0.0);
                if (idx == 0u) { e.flags = e.flags | FLAG_PLAYER; }
                e.state = u32(random(f32(idx), 20.0) * 4.0);
            } else if (idx < 80u) {
                // Goombas - many spread across screen
                e.kind = KIND_GOOMBA;
                e.pos = vec2<f32>(
                    random(f32(idx), 8.0) * u.resolution.x,
                    random(f32(idx), 9.0) * u.resolution.y * 0.9
                );
                e.vel = vec2<f32>(select(-0.5, 0.5, random(f32(idx), 10.0) > 0.5), 0.0);
            } else if (idx < 100u) {
                // Koopas
                e.kind = KIND_KOOPA;
                e.pos = vec2<f32>(
                    random(f32(idx), 11.0) * u.resolution.x,
                    random(f32(idx), 12.0) * u.resolution.y * 0.8
                );
                e.vel = vec2<f32>(select(-0.4, 0.4, random(f32(idx), 13.0) > 0.5), 0.0);
            } else {
                // Coins - spread across screen
                e.kind = KIND_COIN;
                e.pos = vec2<f32>(
                    random(f32(idx), 14.0) * u.resolution.x,
                    random(f32(idx), 15.0) * u.resolution.y * 0.85
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
