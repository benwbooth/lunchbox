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
        // STRICT 8x8 TILE GRID - everything aligns to tile boundaries
        let screen_tiles_x = u32(floor(u.resolution.x / TILE));
        let screen_tiles_y = u32(floor(u.resolution.y / TILE));

        // Platform layout: rows of platforms every 4 tiles vertically
        // Each row has platforms with gaps for jumping
        let row_spacing = 4u;  // Vertical tiles between platform rows
        let num_rows = screen_tiles_y / row_spacing;

        // Initialize platforms on strict grid
        if (idx < PLATFORM_COUNT) {
            // Calculate which row and segment this platform belongs to
            let platforms_per_row = 16u;  // Platform segments per row
            let row = idx / platforms_per_row;
            let seg = idx % platforms_per_row;

            if (row < num_rows) {
                let y_tile = row * row_spacing + 2u;  // Start 2 tiles from top

                // Segment width in tiles
                let seg_width = screen_tiles_x / platforms_per_row;

                // Create platform with gaps - alternate pattern
                // Even rows: platforms on even segments
                // Odd rows: platforms on odd segments (staggered)
                let has_platform = ((row + seg) % 2u) == 0u;

                if (has_platform && y_tile < screen_tiles_y - 2u) {
                    platforms[idx].x = f32(seg * seg_width);
                    platforms[idx].y = f32(y_tile);
                    platforms[idx].width = f32(seg_width);
                    platforms[idx].is_ground = select(0u, 1u, row == num_rows - 1u);
                } else {
                    platforms[idx].width = 0.0;  // No platform here (gap)
                }
            } else {
                platforms[idx].width = 0.0;
            }
        }

        // Initialize blocks on strict grid - fill gaps between platforms
        if (idx < BLOCK_COUNT) {
            let blocks_per_row = 32u;
            let row = idx / blocks_per_row;
            let col = idx % blocks_per_row;

            let block_row_spacing = 4u;  // Same as platform spacing
            let num_block_rows = screen_tiles_y / block_row_spacing;

            if (row < num_block_rows) {
                let y_tile = row * block_row_spacing + 2u;
                let x_tile = col * (screen_tiles_x / blocks_per_row);

                // Place blocks in a checkerboard pattern offset from platforms
                let has_block = ((row + col) % 3u) == 1u;  // Every 3rd position

                if (has_block && y_tile < screen_tiles_y - 2u) {
                    blocks[idx].pos = vec2<f32>(f32(x_tile) * TILE, f32(y_tile) * TILE);
                    blocks[idx].kind = select(0u, 1u, (row + col) % 5u == 0u);  // Some question blocks
                    blocks[idx].flags = 0u;
                } else {
                    blocks[idx].flags = 2u;  // No block here
                }
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
