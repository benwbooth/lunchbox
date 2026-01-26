// Mario Minigame - Compute Shader
// Game logic update shader with full entity-entity collision

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
    kind: u32,      // 0=mario, 1=goomba, 2=koopa, 3=coin, 4=mushroom, 5=debris
    state: u32,     // For koopa: 0=walk, 1=shell, 2=moving_shell. For debris: lifetime
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
const SHELL_SPEED: f32 = 4.0;

const ENTITY_COUNT: u32 = 128u;
const BLOCK_COUNT: u32 = 512u;
const PLATFORM_COUNT: u32 = 512u;

// Entity kinds
const KIND_MARIO: u32 = 0u;
const KIND_GOOMBA: u32 = 1u;
const KIND_KOOPA: u32 = 2u;
const KIND_COIN: u32 = 3u;
const KIND_MUSHROOM: u32 = 4u;
const KIND_DEBRIS: u32 = 5u;

// Koopa states
const KOOPA_WALK: u32 = 0u;
const KOOPA_SHELL: u32 = 1u;
const KOOPA_SHELL_MOVING: u32 = 2u;

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

// Check AABB collision between two entities
fn entities_collide(a_pos: vec2<f32>, b_pos: vec2<f32>) -> bool {
    return abs(a_pos.x - b_pos.x) < 7.0 && abs(a_pos.y - b_pos.y) < 7.0;
}

// Check if A is stomping B (A is above B and falling)
fn is_stomping(a_pos: vec2<f32>, a_vel_y: f32, b_pos: vec2<f32>) -> bool {
    return a_vel_y > 0.0 &&
           a_pos.y + 8.0 > b_pos.y && a_pos.y + 4.0 < b_pos.y &&
           abs(a_pos.x - b_pos.x) < 6.0;
}

// Find free debris slot
fn find_debris_slot() -> u32 {
    for (var i = 100u; i < ENTITY_COUNT; i = i + 1u) {
        if ((entities[i].flags & FLAG_ALIVE) == 0u) {
            return i;
        }
    }
    return 0xFFFFFFFFu;
}

//=============================================================================
// COMPUTE SHADER - GAME LOGIC
//=============================================================================

@compute @workgroup_size(64)
fn update(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;

    // Initialize on first frame
    if (u.frame == 0u) {
        let screen_tiles_x = u32(floor(u.resolution.x / TILE));
        let screen_tiles_y = u32(floor(u.resolution.y / TILE));

        // Initialize platforms
        if (idx < PLATFORM_COUNT) {
            let platforms_per_row = 16u;
            let row = idx / platforms_per_row;
            let col = idx % platforms_per_row;
            let total_rows = 32u;

            if (row < total_rows) {
                let seed = f32(idx);
                let is_ground = row >= total_rows - 2u;
                var y_tile: u32;
                if (is_ground) {
                    y_tile = screen_tiles_y - 1u - (total_rows - 1u - row);
                } else {
                    let available_height = screen_tiles_y - 2u;
                    y_tile = (row * available_height) / (total_rows - 2u);
                }

                let row_type = row % 4u;
                var x_tile: u32;
                var width: u32;
                var has_platform: bool = true;

                if (row_type == 0u) {
                    x_tile = col * (screen_tiles_x / platforms_per_row);
                    width = 8u + u32(random(seed, 1.0) * 8.0);
                    has_platform = random(seed, 2.0) < 0.85;
                } else if (row_type == 1u) {
                    x_tile = col * (screen_tiles_x / platforms_per_row) + u32(random(seed, 3.0) * 4.0);
                    width = 6u + u32(random(seed, 4.0) * 10.0);
                    has_platform = random(seed, 5.0) < 0.75;
                } else if (row_type == 2u) {
                    let offset = select(0u, screen_tiles_x / 32u, col % 2u == 1u);
                    x_tile = col * (screen_tiles_x / platforms_per_row) + offset;
                    width = 5u + u32(random(seed, 6.0) * 12.0);
                    has_platform = random(seed, 7.0) < 0.8;
                } else {
                    x_tile = col * (screen_tiles_x / platforms_per_row);
                    width = 4u + u32(random(seed, 8.0) * 14.0);
                    has_platform = random(seed, 9.0) < 0.7;
                }

                if (is_ground) {
                    has_platform = true;
                    width = max(width, screen_tiles_x / platforms_per_row + 2u);
                }

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

        // Initialize blocks
        if (idx < BLOCK_COUNT) {
            let seed = f32(idx + 1000u);
            let screen_tiles_xf = f32(screen_tiles_x);
            let screen_tiles_yf = f32(screen_tiles_y);
            let x_tile = u32(random(seed, 10.0) * screen_tiles_xf);
            let y_tile = u32(random(seed, 11.0) * (screen_tiles_yf - 4.0)) + 2u;

            if (random(seed, 12.0) < 0.6) {
                blocks[idx].pos = vec2<f32>(f32(x_tile) * TILE, f32(y_tile) * TILE);
                blocks[idx].kind = select(0u, 1u, random(seed, 13.0) < 0.3);
                blocks[idx].flags = 0u;
            } else {
                blocks[idx].flags = 2u;
            }
        }

        // Initialize entities
        if (idx < ENTITY_COUNT) {
            var e: Entity;
            e.flags = FLAG_ALIVE;
            e.state = 0u;

            if (idx < 40u) {
                // Marios/Luigis
                e.kind = KIND_MARIO;
                e.pos = vec2<f32>(
                    random(f32(idx), 5.0) * u.resolution.x,
                    random(f32(idx), 6.0) * u.resolution.y * 0.9
                );
                e.vel = vec2<f32>(select(-MOVE_SPEED, MOVE_SPEED, random(f32(idx), 7.0) > 0.5) * 0.5, 0.0);
                if (idx == 0u) { e.flags = e.flags | FLAG_PLAYER; }
                e.state = select(0u, 1u, random(f32(idx), 20.0) < 0.5); // 0=Mario, 1=Luigi
            } else if (idx < 70u) {
                // Goombas
                e.kind = KIND_GOOMBA;
                e.pos = vec2<f32>(
                    random(f32(idx), 8.0) * u.resolution.x,
                    random(f32(idx), 9.0) * u.resolution.y * 0.9
                );
                e.vel = vec2<f32>(select(-0.5, 0.5, random(f32(idx), 10.0) > 0.5), 0.0);
            } else if (idx < 90u) {
                // Koopas
                e.kind = KIND_KOOPA;
                e.pos = vec2<f32>(
                    random(f32(idx), 11.0) * u.resolution.x,
                    random(f32(idx), 12.0) * u.resolution.y * 0.8
                );
                e.vel = vec2<f32>(select(-0.4, 0.4, random(f32(idx), 13.0) > 0.5), 0.0);
                e.state = KOOPA_WALK;
            } else if (idx < 100u) {
                // Coins
                e.kind = KIND_COIN;
                e.pos = vec2<f32>(
                    random(f32(idx), 14.0) * u.resolution.x,
                    random(f32(idx), 15.0) * u.resolution.y * 0.85
                );
                e.vel = vec2<f32>(0.0, 0.0);
            } else {
                // Reserve slots for debris (start inactive)
                e.kind = KIND_DEBRIS;
                e.flags = 0u; // Not alive
            }

            entities[idx] = e;
        }
        return;
    }

    // Update entities
    if (idx < ENTITY_COUNT) {
        var e = entities[idx];

        // Handle debris lifetime
        if (e.kind == KIND_DEBRIS && (e.flags & FLAG_ALIVE) != 0u) {
            e.vel.y = min(e.vel.y + GRAVITY, MAX_FALL);
            e.pos = e.pos + e.vel;
            e.state = e.state + 1u;
            // Debris disappears after falling off screen or timeout
            if (e.pos.y > u.resolution.y + 16.0 || e.state > 120u) {
                e.flags = e.flags & ~FLAG_ALIVE;
            }
            entities[idx] = e;
            return;
        }

        if ((e.flags & FLAG_ALIVE) == 0u) {
            entities[idx] = e;
            return;
        }

        let is_player = (e.flags & FLAG_PLAYER) != 0u;
        let is_koopa_shell = e.kind == KIND_KOOPA && e.state >= KOOPA_SHELL;
        let is_moving_shell = e.kind == KIND_KOOPA && e.state == KOOPA_SHELL_MOVING;

        // Gravity (not for stationary shells or coins)
        if (e.kind != KIND_COIN && !(is_koopa_shell && e.state == KOOPA_SHELL)) {
            e.vel.y = min(e.vel.y + GRAVITY, MAX_FALL);
        }

        // Update position
        let old_pos = e.pos;
        let old_y = e.pos.y;
        e.pos = e.pos + e.vel;

        // Clear ground flag
        e.flags = e.flags & ~FLAG_GROUND;

        // Platform collision
        for (var i = 0u; i < PLATFORM_COUNT; i = i + 1u) {
            let p = platforms[i];
            if (p.width <= 0.0) { continue; }
            let px = p.x * TILE;
            let py = p.y * TILE;
            let pw = p.width * TILE;

            if (e.vel.y > 0.0 &&
                e.pos.x + 7.0 > px && e.pos.x + 1.0 < px + pw &&
                e.pos.y + 8.0 >= py && old_y + 8.0 <= py + 4.0) {
                e.pos.y = py - 8.0;
                e.vel.y = 0.0;
                e.flags = e.flags | FLAG_GROUND;
            }
        }

        // Block collision (landing on top + hitting from below)
        for (var i = 0u; i < BLOCK_COUNT; i = i + 1u) {
            var b = blocks[i];
            if ((b.flags & 2u) != 0u) { continue; }

            // Landing on top
            if (e.vel.y > 0.0 &&
                e.pos.x + 7.0 > b.pos.x && e.pos.x + 1.0 < b.pos.x + 8.0 &&
                e.pos.y + 8.0 >= b.pos.y && old_y + 8.0 <= b.pos.y + 4.0) {
                e.pos.y = b.pos.y - 8.0;
                e.vel.y = 0.0;
                e.flags = e.flags | FLAG_GROUND;
            }

            // Mario hitting block from below
            if (e.kind == KIND_MARIO && e.vel.y < 0.0 &&
                e.pos.x + 7.0 > b.pos.x && e.pos.x + 1.0 < b.pos.x + 8.0 &&
                e.pos.y <= b.pos.y + 8.0 && old_y >= b.pos.y + 4.0) {

                e.vel.y = 1.0; // Bounce down
                e.pos.y = b.pos.y + 9.0;

                if (b.kind == 0u) {
                    // Brick - destroy it and spawn 4 debris pieces
                    blocks[i].flags = blocks[i].flags | 2u; // Mark destroyed

                    // Spawn debris (find free slots in entity pool)
                    for (var d = 0u; d < 4u; d = d + 1u) {
                        for (var slot = 100u; slot < ENTITY_COUNT; slot = slot + 1u) {
                            if ((entities[slot].flags & FLAG_ALIVE) == 0u) {
                                var debris: Entity;
                                debris.kind = KIND_DEBRIS;
                                debris.flags = FLAG_ALIVE;
                                debris.pos = b.pos + vec2<f32>(f32(d % 2u) * 4.0, f32(d / 2u) * 4.0);
                                debris.vel = vec2<f32>(
                                    select(-2.0, 2.0, d % 2u == 1u) + random(f32(slot), 30.0),
                                    -4.0 - random(f32(slot), 31.0) * 3.0
                                );
                                debris.state = 0u;
                                debris.timer = 0u;
                                entities[slot] = debris;
                                break;
                            }
                        }
                    }
                } else if (b.kind == 1u) {
                    // Question block - mark as hit (empty)
                    blocks[i].kind = 2u;
                    blocks[i].flags = blocks[i].flags | 1u;
                }
            }
        }

        // Entity-Entity collision detection
        for (var j = 0u; j < ENTITY_COUNT; j = j + 1u) {
            if (j == idx) { continue; }
            var other = entities[j];
            if ((other.flags & FLAG_ALIVE) == 0u) { continue; }
            if (other.kind == KIND_COIN || other.kind == KIND_DEBRIS) { continue; }

            let dx = e.pos.x - other.pos.x;
            let dy = e.pos.y - other.pos.y;
            let collides = abs(dx) < 7.0 && abs(dy) < 7.0;

            if (!collides) { continue; }

            // Check if we're stomping (our feet hitting their head)
            let stomping = e.vel.y > 0.0 && dy < -2.0 && dy > -10.0 && abs(dx) < 6.0;

            // GOOMBA-GOOMBA: both reverse direction
            if (e.kind == KIND_GOOMBA && other.kind == KIND_GOOMBA) {
                e.vel.x = -e.vel.x;
                e.pos.x = e.pos.x + sign(dx) * 2.0;
            }

            // KOOPA-KOOPA: both reverse (if walking)
            if (e.kind == KIND_KOOPA && other.kind == KIND_KOOPA &&
                e.state == KOOPA_WALK && other.state == KOOPA_WALK) {
                e.vel.x = -e.vel.x;
                e.pos.x = e.pos.x + sign(dx) * 2.0;
            }

            // GOOMBA-KOOPA: both reverse (if koopa walking)
            if (e.kind == KIND_GOOMBA && other.kind == KIND_KOOPA && other.state == KOOPA_WALK) {
                e.vel.x = -e.vel.x;
                e.pos.x = e.pos.x + sign(dx) * 2.0;
            }
            if (e.kind == KIND_KOOPA && e.state == KOOPA_WALK && other.kind == KIND_GOOMBA) {
                e.vel.x = -e.vel.x;
                e.pos.x = e.pos.x + sign(dx) * 2.0;
            }

            // MARIO interactions
            if (e.kind == KIND_MARIO) {
                // MARIO stomps GOOMBA
                if (other.kind == KIND_GOOMBA && stomping) {
                    entities[j].flags = entities[j].flags & ~FLAG_ALIVE; // Kill goomba
                    e.vel.y = JUMP_VEL * 0.5; // Bounce
                }
                // GOOMBA hits MARIO (not stomping)
                else if (other.kind == KIND_GOOMBA && !stomping) {
                    e.flags = e.flags & ~FLAG_ALIVE; // Mario dies
                    e.vel.y = JUMP_VEL;
                }

                // MARIO stomps KOOPA (walking) -> becomes shell
                if (other.kind == KIND_KOOPA && other.state == KOOPA_WALK && stomping) {
                    entities[j].state = KOOPA_SHELL;
                    entities[j].vel.x = 0.0;
                    e.vel.y = JUMP_VEL * 0.5;
                }
                // KOOPA (walking) hits MARIO -> Mario dies
                else if (other.kind == KIND_KOOPA && other.state == KOOPA_WALK && !stomping) {
                    e.flags = e.flags & ~FLAG_ALIVE;
                    e.vel.y = JUMP_VEL;
                }

                // MARIO touches stationary SHELL -> kick it
                if (other.kind == KIND_KOOPA && other.state == KOOPA_SHELL) {
                    entities[j].state = KOOPA_SHELL_MOVING;
                    entities[j].vel.x = select(-SHELL_SPEED, SHELL_SPEED, dx < 0.0);
                    e.pos.x = e.pos.x + sign(dx) * 4.0; // Push away
                }

                // MOVING SHELL hits MARIO -> Mario dies
                if (other.kind == KIND_KOOPA && other.state == KOOPA_SHELL_MOVING) {
                    e.flags = e.flags & ~FLAG_ALIVE;
                    e.vel.y = JUMP_VEL;
                }

                // MARIO stomps another MARIO/LUIGI
                if (other.kind == KIND_MARIO && stomping && (other.flags & FLAG_PLAYER) == 0u) {
                    entities[j].flags = entities[j].flags & ~FLAG_ALIVE;
                    e.vel.y = JUMP_VEL * 0.5;
                }
                // MARIO collides with MARIO (not stomping) - push apart
                else if (other.kind == KIND_MARIO && !stomping) {
                    e.pos.x = e.pos.x + sign(dx) * 1.0;
                }
            }

            // Moving shell kills goombas and other koopas
            if (e.kind == KIND_KOOPA && e.state == KOOPA_SHELL_MOVING) {
                if (other.kind == KIND_GOOMBA) {
                    entities[j].flags = entities[j].flags & ~FLAG_ALIVE;
                }
                if (other.kind == KIND_KOOPA && j != idx) {
                    entities[j].flags = entities[j].flags & ~FLAG_ALIVE;
                }
            }
        }

        // Screen wrap
        if (e.pos.x < -8.0) { e.pos.x = u.resolution.x; }
        if (e.pos.x > u.resolution.x) { e.pos.x = -8.0; }

        // Fall respawn (for Marios and enemies, not debris)
        if (e.pos.y > u.resolution.y + 32.0 && e.kind != KIND_DEBRIS) {
            if (e.kind == KIND_MARIO || e.kind == KIND_GOOMBA || e.kind == KIND_KOOPA) {
                e.pos.y = -16.0;
                e.pos.x = random(f32(idx) + u.time, 16.0) * u.resolution.x;
                e.vel.y = 0.0;
                e.flags = e.flags | FLAG_ALIVE;
                if (e.kind == KIND_KOOPA) {
                    e.state = KOOPA_WALK;
                    e.vel.x = select(-0.4, 0.4, random(f32(idx) + u.time, 17.0) > 0.5);
                }
            }
        }

        // AI for non-player entities
        let on_ground = (e.flags & FLAG_GROUND) != 0u;
        if (!is_player && on_ground && (e.flags & FLAG_ALIVE) != 0u) {
            // Random direction changes (not for shells)
            if (!is_koopa_shell && random(f32(idx) + u.time, 17.0) < 0.01) {
                e.vel.x = -e.vel.x;
            }

            // Random jumps (Mario AI only)
            if (e.kind == KIND_MARIO && random(f32(idx) + u.time, 18.0) < 0.012) {
                e.vel.y = JUMP_VEL * 0.8;
                e.flags = e.flags & ~FLAG_GROUND;
            }

            // Edge detection
            if (e.pos.x < 16.0) { e.vel.x = abs(e.vel.x); }
            if (e.pos.x > u.resolution.x - 24.0) { e.vel.x = -abs(e.vel.x); }

            // Shells bounce off edges
            if (is_moving_shell) {
                if (e.pos.x < 8.0 || e.pos.x > u.resolution.x - 16.0) {
                    e.vel.x = -e.vel.x;
                }
            }
        }

        // Update facing direction
        if (e.vel.x > 0.1) { e.flags = e.flags & ~FLAG_FLIP; }
        else if (e.vel.x < -0.1) { e.flags = e.flags | FLAG_FLIP; }

        // Friction for player on ground
        if (on_ground && is_player) {
            e.vel.x = e.vel.x * 0.85;
        }

        // Animation timer
        e.timer = e.timer + 1u;

        entities[idx] = e;
    }
}
