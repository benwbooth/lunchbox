// Mario Minigame - Compute Shader
// Split into init (once) and update (every frame) for optimal performance

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
    // Dynamic grid dimensions (calculated from screen size)
    grid_width: u32,      // Spatial grid width in tiles
    grid_height: u32,     // Spatial grid height in tiles
    grid_size: u32,       // grid_width * grid_height
    egrid_width: u32,     // Entity grid width in cells
    egrid_height: u32,    // Entity grid height in cells
    egrid_cells: u32,     // egrid_width * egrid_height
    egrid_size: u32,      // egrid_cells * EGRID_SLOTS
    block_count: u32,     // Active block count for this screen size
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

// Bindings - read_write for compute
// Arrays use runtime sizing - actual sizes passed via uniforms and buffer creation
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read_write> entities_in: array<Entity, 256>;
@group(0) @binding(2) var<storage, read_write> entities_out: array<Entity, 256>;
@group(0) @binding(3) var<storage, read_write> blocks: array<Block>;           // Runtime sized
@group(0) @binding(4) var<storage, read_write> spatial_grid: array<atomic<u32>>;  // Runtime sized
@group(0) @binding(5) var<storage, read_write> entity_grid: array<u32>;        // Runtime sized
@group(0) @binding(6) var<storage, read_write> entity_cell_counts: array<atomic<u32>>;  // Runtime sized

//=============================================================================
// CONSTANTS
//=============================================================================

// Physics constants
const GRAVITY: f32 = 0.04;
const MAX_FALL: f32 = 1.5;
const JUMP_VEL: f32 = -2.0;
const MOVE_SPEED: f32 = 0.3;
const TILE: f32 = 8.0;
const SHELL_SPEED: f32 = 0.4;

// Fixed counts (entities are always 256)
const ENTITY_COUNT: u32 = 256u;

// Entity grid cell size (fixed, dimensions are dynamic via uniforms)
const ECELL_SIZE: f32 = 64.0;   // Entity cell size in pixels
const EGRID_SLOTS: u32 = 4u;    // Max entities per cell

// Note: GRID_WIDTH, GRID_HEIGHT, u.grid_size, EGRID_WIDTH, EGRID_HEIGHT, u.egrid_cells,
// u.egrid_size, u.block_count are now passed via uniforms (u.grid_width, etc.)

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

// Goomba states
const GOOMBA_WALK: u32 = 0u;
const GOOMBA_FLAT: u32 = 1u;  // Squished, waiting to disappear

// Flags
const FLAG_FLIP: u32 = 1u;
const FLAG_ALIVE: u32 = 2u;
const FLAG_GROUND: u32 = 4u;
const FLAG_BIG: u32 = 8u;
const FLAG_PLAYER: u32 = 16u;
const FLAG_DYING: u32 = 32u;  // Death animation in progress

//=============================================================================
// UTILITY FUNCTIONS
//=============================================================================

// PCG hash - fast integer hash function (much faster than sin())
fn pcg_hash(input: u32) -> u32 {
    let state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn random(seed: f32, offset: f32) -> f32 {
    let s = u32(seed * 12.9898 + offset * 78.233);
    return f32(pcg_hash(s)) / 4294967295.0;
}

fn random_with_time(seed: f32, offset: f32) -> f32 {
    let s = u32(seed * 12.9898 + offset * 78.233 + u.time * 100.0);
    return f32(pcg_hash(s)) / 4294967295.0;
}

// Spatial grid helpers - O(1) block lookup
fn grid_index(tile_x: u32, tile_y: u32) -> u32 {
    return tile_y * u.grid_width + tile_x;
}

fn get_block_at_tile(tile_x: u32, tile_y: u32) -> u32 {
    if (tile_x >= u.grid_width || tile_y >= u.grid_height) {
        return 0xFFFFFFFFu;
    }
    let idx = grid_index(tile_x, tile_y);
    if (idx >= u.grid_size) {
        return 0xFFFFFFFFu;
    }
    return atomicLoad(&spatial_grid[idx]);
}

fn pos_to_tile_x(x: f32) -> u32 {
    return u32(max(0.0, x / TILE));
}

fn pos_to_tile_y(y: f32) -> u32 {
    return u32(max(0.0, y / TILE));
}

// Entity grid helpers - 64px cells for entity-entity collision
fn ecell_index(cx: u32, cy: u32) -> u32 {
    return cy * u.egrid_width + cx;
}

fn pos_to_ecell_x(x: f32) -> u32 {
    return u32(max(0.0, x / ECELL_SIZE));
}

fn pos_to_ecell_y(y: f32) -> u32 {
    return u32(max(0.0, y / ECELL_SIZE));
}

// Register entity in grid cell (finds empty slot)
fn register_entity_in_cell(ent_idx: u32, cx: u32, cy: u32) {
    if (cx >= u.egrid_width || cy >= u.egrid_height) { return; }
    let cell = ecell_index(cx, cy);
    if (cell >= u.egrid_cells) { return; }
    let slot = atomicAdd(&entity_cell_counts[cell], 1u);
    if (slot < EGRID_SLOTS) {
        let grid_idx = cell * EGRID_SLOTS + slot;
        if (grid_idx < u.egrid_size) {
            entity_grid[grid_idx] = ent_idx;
        }
    }
    // Cell full - entity won't collide with others in this cell (acceptable tradeoff)
}

//=============================================================================
// CLEAR SHADER - Run FIRST during init to clear all grids
// Must complete before init_populate runs
//=============================================================================

@compute @workgroup_size(64)
fn init_clear(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;

    // Clear spatial grid - each thread clears multiple entries
    for (var i = 0u; i < 32u; i = i + 1u) {
        let grid_idx = idx * 32u + i;
        if (grid_idx < u.grid_size) {
            atomicStore(&spatial_grid[grid_idx], 0xFFFFFFFFu);
        }
    }

    // Clear entity grid counts (one entry per thread)
    if (idx < u.egrid_cells) {
        atomicStore(&entity_cell_counts[idx], 0u);
    }
    // Clear entity grid list (debug safety; counts gate reads)
    if (idx < u.egrid_size) {
        entity_grid[idx] = 0xFFFFFFFFu;
    }

}

//=============================================================================
// INIT POPULATE SHADER - Run SECOND to generate level and spawn entities
// Grids must be cleared first by init_clear
//=============================================================================

// Atomic helper - try to register block at tile, return true if successful
fn try_register_block(tile_x: u32, tile_y: u32, block_idx: u32) -> bool {
    if (tile_x >= u.grid_width || tile_y >= u.grid_height) { return false; }
    let grid_idx = grid_index(tile_x, tile_y);
    if (grid_idx >= u.grid_size) { return false; }
    // Use atomic to prevent race conditions - only place if currently empty
    let result = atomicCompareExchangeWeak(&spatial_grid[grid_idx], 0xFFFFFFFFu, block_idx);
    return result.exchanged;
}

@compute @workgroup_size(64)
fn init_populate(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;

    // Initialize blocks
    let screen_tiles_x = u32(floor(u.resolution.x / TILE));
    let screen_tiles_y = u32(floor(u.resolution.y / TILE));

    if (idx < u.block_count) {
        let blocks_per_row = screen_tiles_x;
        let ground_end = blocks_per_row;

        if (idx < ground_end) {
            // Ground layer - bottom row with occasional pits
            let ground_col = idx;
            let pit_zone = ground_col / 20u;
            let pit_pos = ground_col % 20u;
            let has_pit = (pit_zone % 3u) == 1u;
            let in_pit = has_pit && pit_pos >= 8u && pit_pos < 12u;

            if (!in_pit) {
                let y_tile = screen_tiles_y - 1u;
                // Try to register atomically - ground blocks have priority
                if (try_register_block(ground_col, y_tile, idx)) {
                    blocks[idx].pos = vec2<f32>(f32(ground_col) * TILE, f32(y_tile) * TILE);
                    blocks[idx].kind = 3u;
                    blocks[idx].flags = 0u;
                } else {
                    blocks[idx].pos = vec2<f32>(-100.0, -100.0);
                    blocks[idx].flags = 2u;
                }
            } else {
                blocks[idx].pos = vec2<f32>(-100.0, -100.0);
                blocks[idx].flags = 2u;
            }
        } else {
            // Dense platform generation - fill entire screen using all available blocks
            let block_idx = idx - ground_end;

            // Calculate row and column directly from block index
            let col = block_idx % screen_tiles_x;
            let row_idx = block_idx / screen_tiles_x;

            // Platform rows spaced 3 tiles apart, from top to bottom
            let row_spacing = 3u;
            let num_rows = max((screen_tiles_y - 4u) / row_spacing, 1u);
            let row = row_idx % num_rows;

            // Y position for this row (from top to near-bottom)
            let base_y = 2u + row * row_spacing;

            // Row seed for consistent patterns per row
            let row_seed = random(f32(row), 123.0);
            let row_type = u32(row_seed * 4.0);  // 0-3

            var place_block = false;
            var final_x = col;
            var final_y = base_y;

            if (row_type == 0u) {
                // Horizontal platforms - 50% fill with larger gaps
                let gap_chance = random(f32(col * 17u + row * 31u), 99.0);
                place_block = gap_chance > 0.50;
            } else if (row_type == 1u) {
                // Stairs going up-right (wide steps) - every other segment
                let step = col / 4u;
                let y_offset = step % row_spacing;
                final_y = base_y + y_offset;
                let segment = col / 16u;
                place_block = final_y < screen_tiles_y - 2u && (segment % 2u) == 0u;
            } else if (row_type == 2u) {
                // Stairs going down-right (wide steps) - every other segment
                let step = col / 4u;
                let y_offset = step % row_spacing;
                if (base_y >= y_offset) {
                    final_y = base_y - y_offset;
                }
                let segment = col / 16u;
                place_block = final_y > 1u && (segment % 2u) == 1u;
            } else {
                // Floating platforms with big gaps
                let segment = col / 20u;
                let pos_in_seg = col % 20u;
                place_block = pos_in_seg < 10u;  // 10 blocks, 10 gap
            }

            // Bounds check and place - use atomic to prevent overlapping blocks
            if (place_block && final_x < screen_tiles_x && final_y > 1u && final_y < screen_tiles_y - 2u) {
                if (try_register_block(final_x, final_y, idx)) {
                    blocks[idx].pos = vec2<f32>(f32(final_x) * TILE, f32(final_y) * TILE);
                    blocks[idx].kind = select(0u, 1u, random(f32(idx), 10.0) < 0.10);
                    blocks[idx].flags = 0u;
                } else {
                    // Tile already occupied by another block
                    blocks[idx].pos = vec2<f32>(-100.0, -100.0);
                    blocks[idx].flags = 2u;
                }
            } else {
                blocks[idx].pos = vec2<f32>(-100.0, -100.0);
                blocks[idx].flags = 2u;
            }
        }
    }

    // Initialize entities (256 total)
    if (idx < ENTITY_COUNT) {
        var e: Entity;
        e.flags = FLAG_ALIVE;
        e.state = 0u;
        e.timer = 0u;

        if (idx < 100u) {
            // Marios/Luigis (100) - all same speed
            e.kind = KIND_MARIO;
            e.pos = vec2<f32>(
                random(f32(idx), 5.0) * u.resolution.x,
                random(f32(idx), 6.0) * u.resolution.y * 0.8
            );
            e.vel = vec2<f32>(select(-MOVE_SPEED, MOVE_SPEED, random(f32(idx), 7.0) > 0.5), 0.0);
            if (idx == 0u) { e.flags = e.flags | FLAG_PLAYER; }
            e.state = select(0u, 1u, random(f32(idx), 20.0) < 0.5);
        } else if (idx < 180u) {
            // Goombas (80)
            e.kind = KIND_GOOMBA;
            e.pos = vec2<f32>(
                random(f32(idx), 8.0) * u.resolution.x,
                random(f32(idx), 9.0) * u.resolution.y * 0.8
            );
            e.vel = vec2<f32>(select(-0.2, 0.2, random(f32(idx), 10.0) > 0.5), 0.0);
        } else if (idx < 220u) {
            // Koopas (40)
            e.kind = KIND_KOOPA;
            e.pos = vec2<f32>(
                random(f32(idx), 11.0) * u.resolution.x,
                random(f32(idx), 12.0) * u.resolution.y * 0.7
            );
            e.vel = vec2<f32>(select(-0.15, 0.15, random(f32(idx), 13.0) > 0.5), 0.0);
            e.state = KOOPA_WALK;
        } else {
            // Reserve slots for debris (36 slots)
            e.kind = KIND_DEBRIS;
            e.flags = 0u; // Not alive
        }

        entities_out[idx] = e;
    }
}

//=============================================================================
// FRAME PREP SHADER - Clear entity grid and check block destruction
// Runs with 128 workgroups (8192 threads) - all threads do useful work
//=============================================================================

@compute @workgroup_size(64)
fn frame_prep(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;

    // Clear entity grid counts (one entry per thread)
    if (idx < u.egrid_cells) {
        atomicStore(&entity_cell_counts[idx], 0u);
    }

    // Check blocks for destruction - one block per thread (8192 blocks, 8192 threads)
    if (idx < u.block_count) {
        var b = blocks[idx];
        // Skip already destroyed or invalid blocks (fast early exit)
        if ((b.flags & 2u) != 0u || b.kind == 3u || (b.pos.x <= 0.0 && b.pos.y <= 0.0)) {
            return;
        }

        // Check if this block was marked for destruction via spatial grid
        let tile_x = pos_to_tile_x(b.pos.x);
        let tile_y = pos_to_tile_y(b.pos.y);
        let grid_idx = grid_index(tile_x, tile_y);

        if (grid_idx >= u.grid_size) { return; }

        let grid_val = atomicLoad(&spatial_grid[grid_idx]);
        // Check destruction flag (high bit set, and index matches us)
        if ((grid_val & 0x80000000u) == 0u || (grid_val & 0x7FFFFFFFu) != idx) {
            return;
        }

        if (b.kind == 0u) {
            // Brick - destroy and spawn debris
            b.flags = b.flags | 2u;
            blocks[idx] = b;
            atomicStore(&spatial_grid[grid_idx], 0xFFFFFFFFu);

            // Spawn debris (simplified - just 2 pieces for performance)
            for (var d = 0u; d < 2u; d = d + 1u) {
                for (var slot = 220u; slot < ENTITY_COUNT; slot = slot + 1u) {
                    if ((entities_in[slot].flags & (FLAG_ALIVE | FLAG_DYING)) == 0u) {
                        var debris: Entity;
                        debris.kind = KIND_DEBRIS;
                        debris.flags = FLAG_ALIVE;
                        debris.pos = b.pos + vec2<f32>(f32(d) * 4.0, 0.0);
                        debris.vel = vec2<f32>(
                            select(-0.5, 0.5, d == 1u),
                            -1.0 - random_with_time(f32(slot), 31.0) * 0.5
                        );
                        debris.state = 0u;
                        debris.timer = 0u;
                        entities_in[slot] = debris;
                        break;
                    }
                }
            }
        } else if (b.kind == 1u) {
            // Question block - mark as empty
            b.kind = 2u;
            blocks[idx] = b;
            atomicStore(&spatial_grid[grid_idx], idx);
        }
    }
}

//=============================================================================
// UPDATE POSITIONS SHADER - Physics and block collisions
// Builds entity grid for collision pass
//=============================================================================

@compute @workgroup_size(64)
fn update_positions(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;

    // Only process entities (exactly 256 threads dispatched)
    if (idx >= ENTITY_COUNT) {
        return;
    }

    var e = entities_in[idx];

    // Handle debris lifetime
    if (e.kind == KIND_DEBRIS && (e.flags & FLAG_ALIVE) != 0u) {
        e.vel.y = min(e.vel.y + GRAVITY, MAX_FALL);
        e.pos = e.pos + e.vel;
        e.state = e.state + 1u;
        // Debris disappears after falling off screen or timeout
        if (e.pos.y > u.resolution.y + 16.0 || e.state > 120u) {
            e.flags = e.flags & ~FLAG_ALIVE;
        }
        entities_out[idx] = e;
        return;
    }

    // Handle dying entities FIRST - just fall with no collisions until off screen
    if ((e.flags & FLAG_DYING) != 0u) {
        // Apply gravity and fall (no horizontal movement change, no collisions)
        e.vel.y = min(e.vel.y + GRAVITY, MAX_FALL);
        e.pos.y = e.pos.y + e.vel.y;
        // Keep slight horizontal drift from death impact
        e.pos.x = e.pos.x + e.vel.x * 0.98;  // Slow down horizontal
        e.vel.x = e.vel.x * 0.98;

        // Only respawn after falling completely off screen (no timeout!)
        if (e.pos.y > u.resolution.y + 32.0) {
            e.flags = FLAG_ALIVE;  // Clear dying, set alive
            e.timer = 0u;
            // Spawn from top
            e.pos.x = random_with_time(f32(idx), 60.0) * u.resolution.x;
            e.pos.y = -16.0;
            e.vel.y = 0.0;
            e.vel.x = select(-MOVE_SPEED, MOVE_SPEED, random_with_time(f32(idx), 61.0) > 0.5);
            if (e.kind == KIND_KOOPA) {
                e.state = KOOPA_WALK;
            } else if (e.kind == KIND_GOOMBA) {
                e.state = GOOMBA_WALK;
            }
        }
        entities_out[idx] = e;
        return;
    }

    if ((e.flags & FLAG_ALIVE) == 0u) {
        entities_out[idx] = e;
        return;
    }

    let is_koopa_shell = e.kind == KIND_KOOPA && e.state >= KOOPA_SHELL;

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

    // Block collision using swept check - scan tiles from old to new position
    let ent_left = e.pos.x + 1.0;
    let ent_right = e.pos.x + 7.0;

    // Scan Y tiles from old position to new position (capped to prevent slow iteration)
    let y_start = min(old_y, e.pos.y);
    let y_end = max(old_y + 8.0, e.pos.y + 8.0);
    let tile_y_start = pos_to_tile_y(y_start);
    let tile_y_end = min(pos_to_tile_y(y_end) + 1u, tile_y_start + 3u);  // Max 3 tiles vertically
    let tile_x_min = pos_to_tile_x(e.pos.x);
    let tile_x_max = min(pos_to_tile_x(e.pos.x + 7.0) + 1u, tile_x_min + 2u);  // Max 2 tiles horizontally

    // Check tiles in the swept area (limited iteration)
    for (var ty = tile_y_start; ty <= tile_y_end; ty = ty + 1u) {
        for (var tx = tile_x_min; tx <= tile_x_max; tx = tx + 1u) {
            let block_idx = get_block_at_tile(tx, ty);
            if (block_idx == 0xFFFFFFFFu) { continue; }
            if (block_idx >= u.block_count) { continue; }

            var b = blocks[block_idx];
            if ((b.flags & 2u) != 0u) { continue; }

            let block_left = b.pos.x;
            let block_right = b.pos.x + 8.0;
            let block_top = b.pos.y;
            let block_bottom = b.pos.y + 8.0;

            // Check X overlap with current position
            let x_overlap = ent_right > block_left && ent_left < block_right;
            if (!x_overlap) { continue; }

            // Falling down - check if we crossed this block's top
            if (e.vel.y > 0.0 && old_y + 8.0 <= block_top && e.pos.y + 8.0 > block_top) {
                e.pos.y = block_top - 8.0;
                e.vel.y = 0.0;
                e.flags = e.flags | FLAG_GROUND;
            }

            // Jumping up - check if we crossed this block's bottom
            if (e.vel.y < 0.0 && old_y >= block_bottom && e.pos.y < block_bottom) {
                e.pos.y = block_bottom;
                e.vel.y = 2.0;
                // Any entity hitting block from below can break it (not just Mario)
                let destroy_grid_idx = grid_index(tx, ty);
                if (destroy_grid_idx < u.grid_size) {
                    atomicStore(&spatial_grid[destroy_grid_idx], block_idx | 0x80000000u);
                }
            }

            // Side collision - check current overlap
            let ent_top = e.pos.y;
            let ent_bottom = e.pos.y + 8.0;
            let y_overlap = ent_bottom > block_top + 2.0 && ent_top < block_bottom - 2.0;
            if (y_overlap) {
                if (e.vel.x > 0.0 && ent_right > block_left && old_pos.x + 7.0 <= block_left) {
                    e.pos.x = block_left - 7.0;
                    e.vel.x = -e.vel.x;
                }
                if (e.vel.x < 0.0 && ent_left < block_right && old_pos.x + 1.0 >= block_right) {
                    e.pos.x = block_right - 1.0;
                    e.vel.x = -e.vel.x;
                }
            }
        }
    }

    // Floor collision using spatial grid - O(1) lookup
    if (e.vel.y > 0.0 && (e.flags & FLAG_GROUND) == 0u) {
        let ent_bottom = e.pos.y + 8.0;
        let check_y = pos_to_tile_y(ent_bottom);
        let check_x_left = pos_to_tile_x(e.pos.x + 1.0);
        let check_x_right = pos_to_tile_x(e.pos.x + 6.0);

        // Check tiles directly below entity's feet
        for (var tx = check_x_left; tx <= check_x_right; tx = tx + 1u) {
            let block_idx = get_block_at_tile(tx, check_y);
            if (block_idx != 0xFFFFFFFFu && block_idx < u.block_count) {
                let b = blocks[block_idx];
                if ((b.flags & 2u) == 0u) {
                    let block_top = b.pos.y;
                    if (old_y + 8.0 <= block_top && ent_bottom > block_top) {
                        e.pos.y = block_top - 8.0;
                        e.vel.y = 0.0;
                        e.flags = e.flags | FLAG_GROUND;
                        break;
                    }
                }
            }
        }
    }

    // Register this entity in the spatial grid for collision detection (skip coins/debris)
    if (e.kind != KIND_COIN && e.kind != KIND_DEBRIS) {
        let my_ecx = pos_to_ecell_x(e.pos.x);
        let my_ecy = pos_to_ecell_y(e.pos.y);
        register_entity_in_cell(idx, my_ecx, my_ecy);
    }

    entities_out[idx] = e;
}

//=============================================================================
// RESOLVE COLLISIONS SHADER - Entity interactions + AI/input
//=============================================================================

@compute @workgroup_size(64)
fn resolve_collisions(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;

    if (idx >= ENTITY_COUNT) {
        return;
    }

    var e = entities_in[idx];

    if ((e.flags & (FLAG_ALIVE | FLAG_DYING)) == 0u) {
        entities_out[idx] = e;
        return;
    }

    if (e.kind == KIND_DEBRIS) {
        entities_out[idx] = e;
        return;
    }

    // Dying entities: just update and skip all collision processing
    if ((e.flags & FLAG_DYING) != 0u) {
        entities_out[idx] = e;
        return;
    }

    // Handle flat goombas - they stay flat for 1 second (real time) then respawn
    // timer stores the time when goomba became flat (as centiseconds)
    if (e.kind == KIND_GOOMBA && e.state == GOOMBA_FLAT) {
        let flat_time = f32(e.timer) / 100.0;  // Convert back to seconds
        if (u.time - flat_time > 1.0) {
            // Respawn as a new goomba from the top
            e.state = GOOMBA_WALK;
            e.timer = 0u;
            e.pos.y = -16.0;
            e.pos.x = random_with_time(f32(idx), 80.0) * u.resolution.x;
            e.vel.x = select(-0.2, 0.2, random_with_time(f32(idx), 81.0) > 0.5);
            e.vel.y = 0.0;
        }
        entities_out[idx] = e;
        return;  // Flat goombas don't participate in collision
    }

    let is_player = (e.flags & FLAG_PLAYER) != 0u;
    let is_koopa_shell = e.kind == KIND_KOOPA && e.state >= KOOPA_SHELL;
    let is_moving_shell = e.kind == KIND_KOOPA && e.state == KOOPA_SHELL_MOVING;

    // Entity-Entity collision - check current cell and neighbors (9 cells)
    let my_ecx = pos_to_ecell_x(e.pos.x);
    let my_ecy = pos_to_ecell_y(e.pos.y);

    for (var cdy = -1; cdy <= 1; cdy = cdy + 1) {
        for (var cdx = -1; cdx <= 1; cdx = cdx + 1) {
            let cx = u32(max(0, i32(my_ecx) + cdx));
            let cy = u32(max(0, i32(my_ecy) + cdy));
            if (cx >= u.egrid_width || cy >= u.egrid_height) { continue; }

            let cell = ecell_index(cx, cy);
            let cell_base = cell * EGRID_SLOTS;
            let cell_count = min(atomicLoad(&entity_cell_counts[cell]), EGRID_SLOTS);
            for (var slot = 0u; slot < cell_count; slot = slot + 1u) {
                let grid_idx = cell_base + slot;
                if (grid_idx >= u.egrid_size) { continue; }
                let j = entity_grid[grid_idx];
                if (j == idx) { continue; }
                if (j >= ENTITY_COUNT) { continue; }

                var other = entities_in[j];
                if ((other.flags & FLAG_ALIVE) == 0u) { continue; }
                if (other.kind == KIND_COIN || other.kind == KIND_DEBRIS) { continue; }
                // Skip flat goombas - they're effectively dead
                if (other.kind == KIND_GOOMBA && other.state == GOOMBA_FLAT) { continue; }
                // Skip dying entities
                if ((other.flags & FLAG_DYING) != 0u) { continue; }

                let dx = e.pos.x - other.pos.x;
                let dy = e.pos.y - other.pos.y;
                let adx = abs(dx);
                let ady = abs(dy);
                if (adx >= 7.0 || ady >= 7.0) { continue; }

                // Calculate overlap and push apart FIRST
                let overlap_x = 7.0 - adx;
                let overlap_y = 7.0 - ady;
                let push_dir_x = select(-1.0, 1.0, dx > 0.0);
                let push_dir_y = select(-1.0, 1.0, dy > 0.0);

                // Check stomping (falling onto another entity)
                let stomping = e.vel.y > 0.0 && dy < -2.0 && adx < 6.0;
                let being_stomped = other.vel.y > 0.0 && dy > 2.0 && adx < 6.0;

                // MARIO interactions
                if (e.kind == KIND_MARIO && (e.flags & FLAG_DYING) == 0u) {
                    if (other.kind == KIND_GOOMBA && other.state != GOOMBA_FLAT) {
                        if (stomping) {
                            e.vel.y = JUMP_VEL * 0.5;
                            e.pos.y = other.pos.y - 8.0;  // Place on top
                        } else {
                            e.flags = (e.flags & ~FLAG_ALIVE) | FLAG_DYING;
                            e.vel.y = JUMP_VEL;
                            e.vel.x = push_dir_x * 0.5;
                            e.timer = 0u;
                        }
                    } else if (other.kind == KIND_KOOPA) {
                        if (other.state == KOOPA_WALK || other.state == KOOPA_SHELL_MOVING) {
                            if (stomping) {
                                e.vel.y = JUMP_VEL * 0.5;
                                e.pos.y = other.pos.y - 8.0;
                            } else {
                                e.flags = (e.flags & ~FLAG_ALIVE) | FLAG_DYING;
                                e.vel.y = JUMP_VEL;
                                e.vel.x = push_dir_x * 0.5;
                                e.timer = 0u;
                            }
                        } else if (other.state == KOOPA_SHELL) {
                            // Can kick stationary shell
                            e.pos.x = e.pos.x + push_dir_x * overlap_x * 0.5;
                        }
                    } else if (other.kind == KIND_MARIO) {
                        if (being_stomped) {
                            e.flags = (e.flags & ~FLAG_ALIVE) | FLAG_DYING;
                            e.vel.y = JUMP_VEL * 0.5;
                            e.vel.x = push_dir_x * 0.3;
                            e.timer = 0u;
                        } else if (stomping) {
                            e.vel.y = JUMP_VEL * 0.5;
                            e.pos.y = other.pos.y - 8.0;
                        } else {
                            // Push apart - no overlap allowed
                            e.pos.x = e.pos.x + push_dir_x * overlap_x * 0.6;
                            e.vel.x = push_dir_x * MOVE_SPEED;
                        }
                    }
                }
                // GOOMBA interactions
                else if (e.kind == KIND_GOOMBA && (e.flags & FLAG_DYING) == 0u && e.state != GOOMBA_FLAT) {
                    if (being_stomped && other.kind == KIND_MARIO) {
                        e.state = GOOMBA_FLAT;
                        e.vel.x = 0.0;
                        e.vel.y = 0.0;
                        e.timer = u32(u.time * 100.0);  // Store time when flattened (centiseconds)
                    } else if (other.kind == KIND_GOOMBA && other.state != GOOMBA_FLAT) {
                        // Push apart and reverse
                        e.pos.x = e.pos.x + push_dir_x * overlap_x * 0.5;
                        e.vel.x = push_dir_x * abs(e.vel.x);
                    } else if (other.kind == KIND_KOOPA && other.state == KOOPA_WALK) {
                        e.pos.x = e.pos.x + push_dir_x * overlap_x * 0.5;
                        e.vel.x = push_dir_x * abs(e.vel.x);
                    } else if (other.kind == KIND_KOOPA && other.state == KOOPA_SHELL_MOVING) {
                        e.flags = (e.flags & ~FLAG_ALIVE) | FLAG_DYING;
                        e.vel.y = JUMP_VEL * 0.3;
                        e.vel.x = push_dir_x * 0.5;
                    } else if (other.kind == KIND_MARIO) {
                        // Push apart from mario
                        e.pos.x = e.pos.x + push_dir_x * overlap_x * 0.5;
                    }
                }
                // KOOPA interactions
                else if (e.kind == KIND_KOOPA && (e.flags & FLAG_DYING) == 0u) {
                    if (e.state == KOOPA_WALK) {
                        if (being_stomped && other.kind == KIND_MARIO) {
                            e.state = KOOPA_SHELL;
                            e.vel.x = 0.0;
                        } else if (other.kind == KIND_GOOMBA || (other.kind == KIND_KOOPA && other.state == KOOPA_WALK)) {
                            e.pos.x = e.pos.x + push_dir_x * overlap_x * 0.5;
                            e.vel.x = push_dir_x * abs(e.vel.x);
                        } else if (other.kind == KIND_KOOPA && other.state == KOOPA_SHELL_MOVING) {
                            e.flags = (e.flags & ~FLAG_ALIVE) | FLAG_DYING;
                            e.vel.y = JUMP_VEL * 0.3;
                            e.vel.x = push_dir_x * 0.5;
                        } else if (other.kind == KIND_MARIO) {
                            e.pos.x = e.pos.x + push_dir_x * overlap_x * 0.5;
                        }
                    } else if (e.state == KOOPA_SHELL) {
                        if (other.kind == KIND_MARIO) {
                            e.state = KOOPA_SHELL_MOVING;
                            e.vel.x = select(-SHELL_SPEED, SHELL_SPEED, dx < 0.0);
                        }
                    } else if (e.state == KOOPA_SHELL_MOVING) {
                        if (being_stomped && other.kind == KIND_MARIO) {
                            e.state = KOOPA_SHELL;
                            e.vel.x = 0.0;
                        } else if (other.kind == KIND_KOOPA && other.state == KOOPA_SHELL_MOVING) {
                            // Two moving shells collide - both die
                            e.flags = (e.flags & ~FLAG_ALIVE) | FLAG_DYING;
                            e.vel.y = JUMP_VEL * 0.3;
                            e.vel.x = push_dir_x * 0.5;
                        } else if (other.kind == KIND_KOOPA && other.state == KOOPA_SHELL) {
                            // Moving shell hits stationary shell - bounce off
                            e.vel.x = -e.vel.x;
                            e.pos.x = e.pos.x + push_dir_x * overlap_x;
                        }
                    }
                }
            } // slot loop
        } // dx loop
    } // dy loop

    // Screen wrap - entities continue off edges
    if (e.pos.x < -16.0) { e.pos.x = u.resolution.x + 8.0; }
    if (e.pos.x > u.resolution.x + 16.0) { e.pos.x = -8.0; }

    // Respawn when fallen off bottom (only for alive, non-dying entities)
    if (e.pos.y > u.resolution.y + 32.0 && e.kind != KIND_DEBRIS && e.kind != KIND_COIN) {
        // Spawn from top
        e.pos.y = -16.0;
        e.pos.x = random_with_time(f32(idx), 51.0) * u.resolution.x;
        e.vel.y = 0.0;

        if (e.kind == KIND_MARIO) {
            e.vel.x = select(-MOVE_SPEED, MOVE_SPEED, random_with_time(f32(idx), 54.0) > 0.5);
        } else if (e.kind == KIND_GOOMBA) {
            e.vel.x = select(-0.2, 0.2, random_with_time(f32(idx), 56.0) > 0.5);
        } else if (e.kind == KIND_KOOPA) {
            e.state = KOOPA_WALK;
            e.vel.x = select(-0.15, 0.15, random_with_time(f32(idx), 57.0) > 0.5);
        }
    }

    // Player control using keyboard input
    let on_ground = (e.flags & FLAG_GROUND) != 0u;
    if (is_player && (e.flags & FLAG_ALIVE) != 0u) {
        let input_left = (u.mouse_click & 1u) != 0u;
        let input_right = (u.mouse_click & 2u) != 0u;
        let input_jump = (u.mouse_click & 4u) != 0u;

        // Horizontal movement
        if (input_left) {
            e.vel.x = -MOVE_SPEED;
        } else if (input_right) {
            e.vel.x = MOVE_SPEED;
        }

        // Jump when on ground
        if (input_jump && on_ground) {
            e.vel.y = JUMP_VEL;
            e.flags = e.flags & ~FLAG_GROUND;
        }
    }

    // AI for non-player entities
    if (!is_player && on_ground && (e.flags & FLAG_ALIVE) != 0u) {
        // Random direction changes (not for shells)
        if (!is_koopa_shell && random_with_time(f32(idx), 17.0) < 0.01) {
            e.vel.x = -e.vel.x;
        }

        // Random jumps (Mario AI only)
        if (e.kind == KIND_MARIO && random_with_time(f32(idx), 18.0) < 0.012) {
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

    entities_out[idx] = e;
}
