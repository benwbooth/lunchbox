//! Physics simulation for the Mario mini-game

use super::entities::{GameWorld, Mario, Goomba, Mushroom, Debris, BlockType, MarioState};

/// Physics constants
pub const GRAVITY: f64 = 0.4;
pub const JUMP_VELOCITY: f64 = -8.5;
pub const BIG_JUMP_VELOCITY: f64 = -9.5;
pub const MOVE_SPEED: f64 = 2.0;
pub const MAX_FALL_SPEED: f64 = 7.0;
pub const GOOMBA_SPEED: f64 = 0.6;
pub const MUSHROOM_SPEED: f64 = 1.5;

/// Check if two rectangles overlap (AABB collision)
pub fn aabb_overlap(
    ax: f64, ay: f64, aw: f64, ah: f64,
    bx: f64, by: f64, bw: f64, bh: f64,
) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

/// Update Mario physics and handle block collisions
fn update_mario_physics(mario: &mut Mario, world: &GameWorld, block_hits: &mut Vec<(usize, u32)>) {
    if !mario.alive {
        // Death animation - just fall
        mario.vel.y += GRAVITY;
        mario.pos.y += mario.vel.y;
        if mario.death_timer > 0 {
            mario.death_timer -= 1;
        }
        return;
    }

    // Decrement invincibility
    if mario.invincible_timer > 0 {
        mario.invincible_timer -= 1;
    }

    // Apply gravity
    mario.vel.y += GRAVITY;
    if mario.vel.y > MAX_FALL_SPEED {
        mario.vel.y = MAX_FALL_SPEED;
    }

    // Update position
    let old_x = mario.pos.x;
    let old_y = mario.pos.y;
    mario.pos.x += mario.vel.x;
    mario.pos.y += mario.vel.y;

    mario.on_ground = false;
    let tile_size = world.tile_size;
    let (mx, my, mw, mh) = mario.hitbox();

    // Platform collision (ground)
    for platform in &world.platforms {
        let (px, py, pw, _ph) = platform.hitbox(tile_size);
        let mario_bottom = my + mh;
        let mario_prev_bottom = old_y + mh;

        // Landing on platform
        if mario.vel.y > 0.0
            && mx + mw > px && mx < px + pw
            && mario_bottom >= py && mario_prev_bottom <= py + 4.0
        {
            mario.pos.y = py - mh;
            mario.vel.y = 0.0;
            mario.on_ground = true;
        }
    }

    // Block collision
    for (idx, block) in world.blocks.iter().enumerate() {
        let (bx, by, bw, bh) = block.hitbox(tile_size);
        let (mx, my, mw, mh) = mario.hitbox();

        // Check for head bump (hitting block from below)
        if mario.vel.y < 0.0 {
            let head_y = my;
            let prev_head_y = old_y;
            let block_bottom = by + bh;

            if mx + mw > bx + 2.0 && mx < bx + bw - 2.0
                && head_y <= block_bottom && prev_head_y >= block_bottom - 4.0
            {
                mario.pos.y = block_bottom;
                mario.vel.y = 0.0;

                // Record block hit for processing
                if block.block_type == BlockType::Brick ||
                   (block.block_type == BlockType::Question && !block.hit) {
                    block_hits.push((idx, mario.id));
                }
            }
        }

        // Landing on block from above
        let (mx, my, mw, mh) = mario.hitbox();
        if mario.vel.y > 0.0 {
            let mario_bottom = my + mh;
            let mario_prev_bottom = old_y + mh;

            if mx + mw > bx && mx < bx + bw
                && mario_bottom >= by && mario_prev_bottom <= by + 4.0
            {
                mario.pos.y = by - mh;
                mario.vel.y = 0.0;
                mario.on_ground = true;
            }
        }

        // Side collision with blocks
        let (mx, my, mw, mh) = mario.hitbox();
        if aabb_overlap(mx, my + 4.0, mw, mh - 8.0, bx, by, bw, bh) {
            // Push out horizontally
            if mario.vel.x > 0.0 && old_x + mw <= bx + 2.0 {
                mario.pos.x = bx - mw;
                mario.vel.x = 0.0;
            } else if mario.vel.x < 0.0 && old_x >= bx + bw - 2.0 {
                mario.pos.x = bx + bw;
                mario.vel.x = 0.0;
            }
        }
    }

    // World boundaries (wrap horizontally)
    if mario.pos.x < -16.0 {
        mario.pos.x = world.width as f64;
    } else if mario.pos.x > world.width as f64 {
        mario.pos.x = -16.0;
    }

    // Fall off bottom - die if small, respawn if big (lose power)
    if mario.pos.y > world.height as f64 + 50.0 {
        respawn_mario(mario, world);
    }

    // Update animation state
    if mario.state != MarioState::Dead {
        if !mario.on_ground {
            mario.state = MarioState::Jumping;
        } else if mario.vel.x.abs() > 0.1 {
            mario.state = MarioState::Walking;
            mario.walk_timer += 1;
            if mario.walk_timer >= 8 {
                mario.walk_timer = 0;
                mario.walk_frame = (mario.walk_frame + 1) % 2;
            }
        } else {
            mario.state = MarioState::Standing;
            mario.walk_frame = 0;
            mario.walk_timer = 0;
        }
    }

    // Update facing direction
    if mario.vel.x > 0.1 {
        mario.facing_right = true;
    } else if mario.vel.x < -0.1 {
        mario.facing_right = false;
    }

    // Apply friction
    if mario.on_ground {
        mario.vel.x *= if mario.is_player { 0.7 } else { 0.85 };
        if mario.vel.x.abs() < 0.1 {
            mario.vel.x = 0.0;
        }
    }
}

/// Respawn a Mario at a random platform
fn respawn_mario(mario: &mut Mario, world: &GameWorld) {
    mario.pos.y = -32.0;
    mario.vel.y = 0.0;
    mario.is_big = false;

    if !world.platforms.is_empty() {
        let idx = (js_sys::Math::random() * world.platforms.len() as f64) as usize;
        let plat = &world.platforms[idx % world.platforms.len()];
        mario.pos.x = (plat.x * world.tile_size) as f64 +
                      (plat.width * world.tile_size / 2) as f64;
    }
}

/// Update Goomba physics
fn update_goomba_physics(goomba: &mut Goomba, world: &GameWorld) {
    if !goomba.alive {
        if goomba.squish_timer > 0 {
            goomba.squish_timer -= 1;
        }
        return;
    }

    goomba.vel.y += GRAVITY;
    if goomba.vel.y > MAX_FALL_SPEED {
        goomba.vel.y = MAX_FALL_SPEED;
    }

    goomba.pos.x += goomba.vel.x;
    goomba.pos.y += goomba.vel.y;

    let (gx, gy, gw, gh) = goomba.hitbox();
    let tile_size = world.tile_size;

    // Platform collision
    for platform in &world.platforms {
        let (px, py, pw, _ph) = platform.hitbox(tile_size);
        let goomba_bottom = gy + gh;
        let goomba_prev_bottom = goomba_bottom - goomba.vel.y;

        if goomba.vel.y > 0.0
            && gx + gw > px && gx < px + pw
            && goomba_bottom >= py && goomba_prev_bottom <= py + 4.0
        {
            goomba.pos.y = py - gh;
            goomba.vel.y = 0.0;

            // Turn at edges
            let goomba_center = goomba.pos.x + gw / 2.0;
            if goomba_center < px + 8.0 {
                goomba.vel.x = GOOMBA_SPEED;
                goomba.facing_right = true;
            } else if goomba_center > px + pw - 8.0 {
                goomba.vel.x = -GOOMBA_SPEED;
                goomba.facing_right = false;
            }
        }
    }

    // Block collision
    for block in &world.blocks {
        let (bx, by, bw, bh) = block.hitbox(tile_size);
        let (gx, gy, gw, gh) = goomba.hitbox();

        // Side collision
        if aabb_overlap(gx, gy + 4.0, gw, gh - 8.0, bx, by, bw, bh) {
            if goomba.vel.x > 0.0 {
                goomba.vel.x = -GOOMBA_SPEED;
                goomba.facing_right = false;
            } else {
                goomba.vel.x = GOOMBA_SPEED;
                goomba.facing_right = true;
            }
        }
    }

    // World boundaries
    if goomba.pos.x < 0.0 {
        goomba.vel.x = GOOMBA_SPEED;
        goomba.facing_right = true;
    } else if goomba.pos.x > world.width as f64 - 16.0 {
        goomba.vel.x = -GOOMBA_SPEED;
        goomba.facing_right = false;
    }

    if goomba.pos.y > world.height as f64 + 32.0 {
        goomba.alive = false;
    }

    goomba.walk_timer += 1;
    if goomba.walk_timer >= 12 {
        goomba.walk_timer = 0;
        goomba.walk_frame = (goomba.walk_frame + 1) % 2;
    }
}

/// Update mushroom physics
fn update_mushroom_physics(mushroom: &mut Mushroom, world: &GameWorld) {
    if !mushroom.active {
        return;
    }

    // Rising animation
    if mushroom.rising {
        mushroom.rise_progress += 0.5;
        mushroom.pos.y = mushroom.origin_y - mushroom.rise_progress;
        if mushroom.rise_progress >= 16.0 {
            mushroom.rising = false;
        }
        return;
    }

    mushroom.vel.y += GRAVITY;
    if mushroom.vel.y > MAX_FALL_SPEED {
        mushroom.vel.y = MAX_FALL_SPEED;
    }

    mushroom.pos.x += mushroom.vel.x;
    mushroom.pos.y += mushroom.vel.y;

    let (mx, my, mw, mh) = mushroom.hitbox();
    let tile_size = world.tile_size;

    // Platform collision
    for platform in &world.platforms {
        let (px, py, pw, _ph) = platform.hitbox(tile_size);
        let bottom = my + mh;
        let prev_bottom = bottom - mushroom.vel.y;

        if mushroom.vel.y > 0.0
            && mx + mw > px && mx < px + pw
            && bottom >= py && prev_bottom <= py + 4.0
        {
            mushroom.pos.y = py - mh;
            mushroom.vel.y = 0.0;
        }
    }

    // Block collision (bounce off sides)
    for block in &world.blocks {
        let (bx, by, bw, bh) = block.hitbox(tile_size);
        let (mx, my, mw, mh) = mushroom.hitbox();

        if aabb_overlap(mx, my + 4.0, mw, mh - 8.0, bx, by, bw, bh) {
            mushroom.vel.x = -mushroom.vel.x;
        }

        // Land on block
        if mushroom.vel.y > 0.0 {
            let bottom = my + mh;
            let prev_bottom = bottom - mushroom.vel.y;
            if mx + mw > bx && mx < bx + bw
                && bottom >= by && prev_bottom <= by + 4.0
            {
                mushroom.pos.y = by - mh;
                mushroom.vel.y = 0.0;
            }
        }
    }

    // World wrap
    if mushroom.pos.x < -16.0 {
        mushroom.pos.x = world.width as f64;
    } else if mushroom.pos.x > world.width as f64 {
        mushroom.pos.x = -16.0;
    }

    if mushroom.pos.y > world.height as f64 + 32.0 {
        mushroom.active = false;
    }
}

/// Update debris physics
fn update_debris_physics(debris: &mut Debris, world_height: i32) {
    if !debris.alive {
        return;
    }

    debris.vel.y += GRAVITY;
    debris.pos.x += debris.vel.x;
    debris.pos.y += debris.vel.y;
    debris.rotation += debris.rotation_speed;

    if debris.pos.y > world_height as f64 + 50.0 {
        debris.alive = false;
    }
}

/// Check for Mario stomping on Goomba or other Mario
fn check_stomp_collisions(world: &mut GameWorld) {
    let mut mario_stomps: Vec<(usize, usize)> = Vec::new(); // (stomper, stomped)
    let mut goomba_stomps: Vec<(usize, usize)> = Vec::new(); // (mario, goomba)
    let mut mario_hits: Vec<(usize, usize)> = Vec::new(); // (mario, goomba) side hit

    // Check Mario-Goomba collisions
    for (mi, mario) in world.marios.iter().enumerate() {
        if !mario.alive || mario.state == MarioState::Dead {
            continue;
        }

        for (gi, goomba) in world.goombas.iter().enumerate() {
            if !goomba.alive {
                continue;
            }

            let (mx, my, mw, mh) = mario.hitbox();
            let (gx, gy, gw, gh) = goomba.hitbox();

            if aabb_overlap(mx, my, mw, mh, gx, gy, gw, gh) {
                // Check if stomping (falling and feet above goomba head)
                if mario.vel.y > 0.0 && my + mh - 8.0 < gy + 8.0 {
                    goomba_stomps.push((mi, gi));
                } else if mario.invincible_timer == 0 {
                    mario_hits.push((mi, gi));
                }
            }
        }
    }

    // Check Mario-Mario collisions
    let mario_count = world.marios.len();
    for i in 0..mario_count {
        if !world.marios[i].alive {
            continue;
        }
        for j in (i + 1)..mario_count {
            if !world.marios[j].alive {
                continue;
            }

            let (ax, ay, aw, ah) = world.marios[i].hitbox();
            let (bx, by, bw, bh) = world.marios[j].hitbox();

            if aabb_overlap(ax, ay, aw, ah, bx, by, bw, bh) {
                // Check if i is stomping j
                if world.marios[i].vel.y > 0.0 && ay + ah - 8.0 < by + 8.0 {
                    mario_stomps.push((i, j));
                }
                // Check if j is stomping i
                else if world.marios[j].vel.y > 0.0 && by + bh - 8.0 < ay + 8.0 {
                    mario_stomps.push((j, i));
                }
                // Side collision - push apart
                else {
                    let overlap_x = ((ax + aw / 2.0) - (bx + bw / 2.0)).abs();
                    if overlap_x < (aw + bw) / 2.0 {
                        let push = ((aw + bw) / 2.0 - overlap_x) / 2.0 + 0.5;
                        if ax < bx {
                            world.marios[i].pos.x -= push;
                            world.marios[j].pos.x += push;
                            world.marios[i].vel.x = -1.0;
                            world.marios[j].vel.x = 1.0;
                        } else {
                            world.marios[i].pos.x += push;
                            world.marios[j].pos.x -= push;
                            world.marios[i].vel.x = 1.0;
                            world.marios[j].vel.x = -1.0;
                        }
                    }
                }
            }
        }
    }

    // Process goomba stomps
    for (mi, gi) in goomba_stomps {
        world.goombas[gi].alive = false;
        world.goombas[gi].squish_timer = 30;
        world.marios[mi].vel.y = JUMP_VELOCITY * 0.5;
    }

    // Process mario stomps
    for (stomper, stomped) in mario_stomps {
        let stomped_mario = &mut world.marios[stomped];
        if stomped_mario.is_big {
            // Shrink to small
            stomped_mario.is_big = false;
            stomped_mario.invincible_timer = 90;
            stomped_mario.pos.y += 16.0; // Adjust position for smaller hitbox
        } else {
            // Die
            stomped_mario.alive = false;
            stomped_mario.state = MarioState::Dead;
            stomped_mario.vel.y = JUMP_VELOCITY;
            stomped_mario.death_timer = 60;
        }
        world.marios[stomper].vel.y = JUMP_VELOCITY * 0.5;
    }

    // Process mario-goomba side hits
    for (mi, _gi) in mario_hits {
        let mario = &mut world.marios[mi];
        if mario.is_big {
            mario.is_big = false;
            mario.invincible_timer = 90;
            mario.pos.y += 16.0;
        } else {
            mario.alive = false;
            mario.state = MarioState::Dead;
            mario.vel.y = JUMP_VELOCITY;
            mario.death_timer = 60;
        }
    }
}

/// Check for Mario collecting mushroom
fn check_mushroom_collisions(world: &mut GameWorld) {
    let mut collected: Vec<usize> = Vec::new();

    for (mi, mushroom) in world.mushrooms.iter().enumerate() {
        if !mushroom.active || mushroom.rising {
            continue;
        }

        let (mx, my, mw, mh) = mushroom.hitbox();

        for mario in &mut world.marios {
            if !mario.alive {
                continue;
            }

            let (px, py, pw, ph) = mario.hitbox();
            if aabb_overlap(px, py, pw, ph, mx, my, mw, mh) {
                if !mario.is_big {
                    mario.is_big = true;
                    mario.pos.y -= 16.0; // Grow upward
                }
                collected.push(mi);
                break;
            }
        }
    }

    for idx in collected.into_iter().rev() {
        world.mushrooms[idx].active = false;
    }
}

/// Process block hits (break bricks, spawn mushrooms)
fn process_block_hits(world: &mut GameWorld, block_hits: Vec<(usize, u32)>) {
    let mut blocks_to_remove: Vec<usize> = Vec::new();
    let mut mushrooms_to_spawn: Vec<(f64, f64)> = Vec::new();

    for (block_idx, mario_id) in block_hits {
        let mario = world.marios.iter().find(|m| m.id == mario_id);
        let is_big = mario.map(|m| m.is_big).unwrap_or(false);

        let block = &mut world.blocks[block_idx];

        match block.block_type {
            BlockType::Brick => {
                if is_big {
                    // Break the brick
                    let bx = (block.x * world.tile_size) as f64;
                    let by = (block.y * world.tile_size) as f64;

                    // Spawn 4 debris pieces
                    world.debris.push(Debris::new(bx, by, -2.0, -6.0));
                    world.debris.push(Debris::new(bx + 8.0, by, 2.0, -6.0));
                    world.debris.push(Debris::new(bx, by + 8.0, -2.0, -4.0));
                    world.debris.push(Debris::new(bx + 8.0, by + 8.0, 2.0, -4.0));

                    blocks_to_remove.push(block_idx);
                } else {
                    // Bump animation
                    block.bump_offset = -4.0;
                }
            }
            BlockType::Question => {
                if !block.hit {
                    block.hit = true;
                    block.block_type = BlockType::QuestionEmpty;
                    block.bump_offset = -4.0;

                    // Spawn mushroom
                    let mx = (block.x * world.tile_size) as f64;
                    let my = (block.y * world.tile_size - 16) as f64;
                    mushrooms_to_spawn.push((mx, my));
                }
            }
            _ => {}
        }
    }

    // Spawn mushrooms
    for (x, y) in mushrooms_to_spawn {
        let mut mushroom = Mushroom::new(x, y + 16.0);
        mushroom.origin_y = y + 16.0;
        world.mushrooms.push(mushroom);
    }

    // Remove broken blocks (in reverse order to maintain indices)
    blocks_to_remove.sort();
    for idx in blocks_to_remove.into_iter().rev() {
        world.blocks.remove(idx);
    }
}

/// Update block bump animations
fn update_block_animations(world: &mut GameWorld) {
    for block in &mut world.blocks {
        if block.bump_offset < 0.0 {
            block.bump_offset += 1.0;
            if block.bump_offset > 0.0 {
                block.bump_offset = 0.0;
            }
        }
    }
}

/// Run one physics update tick
pub fn update(world: &mut GameWorld) {
    let mut block_hits: Vec<(usize, u32)> = Vec::new();

    // Update all entities
    let world_snapshot = world.clone();

    for mario in &mut world.marios {
        update_mario_physics(mario, &world_snapshot, &mut block_hits);
    }

    for goomba in &mut world.goombas {
        update_goomba_physics(goomba, &world_snapshot);
    }

    for mushroom in &mut world.mushrooms {
        update_mushroom_physics(mushroom, &world_snapshot);
    }

    let world_height = world.height;
    for debris in &mut world.debris {
        update_debris_physics(debris, world_height);
    }

    // Process collisions
    check_stomp_collisions(world);
    check_mushroom_collisions(world);
    process_block_hits(world, block_hits);
    update_block_animations(world);

    // Cleanup dead entities
    world.goombas.retain(|g| g.alive || g.squish_timer > 0);
    world.mushrooms.retain(|m| m.active);
    world.debris.retain(|d| d.alive);

    // Respawn dead marios after animation
    let world_height = world.height;
    let platforms_clone = world.platforms.clone();
    let tile_size = world.tile_size;

    for mario in &mut world.marios {
        if !mario.alive && mario.death_timer == 0 && mario.pos.y > world_height as f64 {
            mario.alive = true;
            mario.state = MarioState::Standing;
            mario.is_big = false;
            mario.pos.y = -32.0;
            mario.vel.y = 0.0;

            if !platforms_clone.is_empty() {
                let idx = (js_sys::Math::random() * platforms_clone.len() as f64) as usize;
                let plat = &platforms_clone[idx % platforms_clone.len()];
                mario.pos.x = (plat.x * tile_size) as f64 + (plat.width * tile_size / 2) as f64;
            }
        }
    }
}

/// Apply player input
pub fn apply_player_input(world: &mut GameWorld, left: bool, right: bool, jump: bool) {
    if let Some(mario) = world.player_mario_mut() {
        if mario.state == MarioState::Dead {
            return;
        }

        if left && !right {
            mario.vel.x = -MOVE_SPEED;
        } else if right && !left {
            mario.vel.x = MOVE_SPEED;
        }

        if jump && mario.on_ground {
            mario.vel.y = if mario.is_big { BIG_JUMP_VELOCITY } else { JUMP_VELOCITY };
            mario.on_ground = false;
        }
    }
}
