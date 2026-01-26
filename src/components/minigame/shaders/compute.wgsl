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
        // FULL SCREEN COVERAGE with wide platforms and interesting layout
        let screen_tiles_x = u32(floor(u.resolution.x / TILE));
        let screen_tiles_y = u32(floor(u.resolution.y / TILE));

        // Initialize platforms - wide platforms covering whole screen
        if (idx < PLATFORM_COUNT) {
            // Layout: 32 rows of 16 platforms each = 512 total
            let platforms_per_row = 16u;
            let row = idx / platforms_per_row;
            let col = idx % platforms_per_row;

            let total_rows = 32u;
            let row_height = screen_tiles_y / total_rows;

            if (row < total_rows) {
                let y_tile = row * row_height;

                // Random seed for this platform
                let seed = f32(idx);

                // Vary the pattern by row type
                let row_type = row % 4u;

                // Calculate x position and width based on row type
                var x_tile: u32;
                var width: u32;
                var has_platform: bool = true;

                if (row_type == 0u) {
                    // Long platforms spanning most of width with small gaps
                    let section_width = screen_tiles_x / 4u;
                    x_tile = col * (screen_tiles_x / platforms_per_row);
                    width = 8u + u32(random(seed, 1.0) * 8.0);  // 8-16 tiles wide
                    has_platform = random(seed, 2.0) < 0.85;
                } else if (row_type == 1u) {
                    // Medium platforms, more spread out
                    x_tile = col * (screen_tiles_x / platforms_per_row) + u32(random(seed, 3.0) * 4.0);
                    width = 6u + u32(random(seed, 4.0) * 10.0);  // 6-16 tiles wide
                    has_platform = random(seed, 5.0) < 0.75;
                } else if (row_type == 2u) {
                    // Staggered platforms
                    let offset = select(0u, screen_tiles_x / 32u, col % 2u == 1u);
                    x_tile = col * (screen_tiles_x / platforms_per_row) + offset;
                    width = 5u + u32(random(seed, 6.0) * 12.0);  // 5-17 tiles wide
                    has_platform = random(seed, 7.0) < 0.8;
                } else {
                    // Varied size platforms
                    x_tile = col * (screen_tiles_x / platforms_per_row);
                    width = 4u + u32(random(seed, 8.0) * 14.0);  // 4-18 tiles wide
                    has_platform = random(seed, 9.0) < 0.7;
                }

                // Ensure ground row is always filled
                let is_ground = row >= total_rows - 2u;
                if (is_ground) {
                    has_platform = true;
                    width = max(width, screen_tiles_x / platforms_per_row + 2u);
                }

                // Clamp to screen bounds
                if (x_tile >= screen_tiles_x) { x_tile = screen_tiles_x - 1u; }
                if (x_tile + width > screen_tiles_x) { width = screen_tiles_x - x_tile; }

                if (has_platform && width > 0u) {
                    platforms[idx].x = f32(x_tile);
                    platforms[idx].y = f32(y_tile);
                    platforms[idx].width = f32(width);
                    platforms[idx].is_ground = select(0u, 1u, is_ground);
                } else {
                    platforms[idx].width = 0.0;
                }
            } else {
                platforms[idx].width = 0.0;
            }
        }

        // Initialize blocks scattered across the level
        if (idx < BLOCK_COUNT) {
            let seed = f32(idx + 1000u);
            let screen_tiles_xf = f32(screen_tiles_x);
            let screen_tiles_yf = f32(screen_tiles_y);

            // Random position on grid
            let x_tile = u32(random(seed, 10.0) * screen_tiles_xf);
            let y_tile = u32(random(seed, 11.0) * (screen_tiles_yf - 4.0)) + 2u;

            // 60% chance to place a block
            if (random(seed, 12.0) < 0.6) {
                blocks[idx].pos = vec2<f32>(f32(x_tile) * TILE, f32(y_tile) * TILE);
                blocks[idx].kind = select(0u, 1u, random(seed, 13.0) < 0.3);  // 30% question blocks
                blocks[idx].flags = 0u;
            } else {
                blocks[idx].flags = 2u;
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
