//! Physics simulation for the Mario mini-game

use super::entities::{GameWorld, Mario, Goomba, Koopa, KoopaState, Mushroom, Coin, Debris, BlockType, MarioState};

/// Physics constants
pub const GRAVITY: f64 = 0.4;
pub const JUMP_VELOCITY: f64 = -8.5;
pub const BIG_JUMP_VELOCITY: f64 = -9.5;
pub const MOVE_SPEED: f64 = 2.0;
pub const MAX_FALL_SPEED: f64 = 7.0;
pub const GOOMBA_SPEED: f64 = 0.6;
pub const KOOPA_SPEED: f64 = 0.5;
pub const SHELL_SPEED: f64 = 4.0;
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
    if mario.pos.x < -8.0 {
        mario.pos.x = world.width as f64;
    } else if mario.pos.x > world.width as f64 {
        mario.pos.x = -8.0;
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
    mario.is_big = false;

    // Randomly choose spawn location: top (40%), bottom/hole (40%), or sides (20%)
    let spawn_type = js_sys::Math::random();

    if spawn_type < 0.4 {
        // Spawn from top (falling down)
        mario.pos.y = -32.0;
        mario.vel.y = 0.0;

        if !world.platforms.is_empty() {
            let idx = (js_sys::Math::random() * world.platforms.len() as f64) as usize;
            let plat = &world.platforms[idx % world.platforms.len()];
            mario.pos.x = (plat.x * world.tile_size) as f64 +
                          (plat.width * world.tile_size / 2) as f64;
        }
    } else if spawn_type < 0.8 {
        // Spawn from bottom (jumping out of hole)
        mario.pos.y = world.height as f64 + 8.0;
        mario.vel.y = JUMP_VELOCITY * 1.5; // Strong upward jump

        // Random x position, preferring near gaps in ground
        let ground_platforms: Vec<_> = world.platforms.iter()
            .filter(|p| p.is_ground)
            .collect();

        if ground_platforms.len() >= 2 {
            // Spawn between two ground platforms (in a gap)
            let idx = (js_sys::Math::random() * (ground_platforms.len() - 1) as f64) as usize;
            let plat = ground_platforms[idx];
            let next_plat = ground_platforms.get(idx + 1);

            if let Some(next) = next_plat {
                let gap_x = (plat.x + plat.width) * world.tile_size;
                let gap_width = (next.x * world.tile_size) - gap_x;
                if gap_width > 0 {
                    mario.pos.x = gap_x as f64 + (gap_width as f64 / 2.0);
                } else {
                    mario.pos.x = js_sys::Math::random() * world.width as f64;
                }
            } else {
                mario.pos.x = js_sys::Math::random() * world.width as f64;
            }
        } else {
            mario.pos.x = js_sys::Math::random() * world.width as f64;
        }
    } else {
        // Spawn from side (running in)
        let from_left = js_sys::Math::random() > 0.5;
        mario.pos.x = if from_left { -8.0 } else { world.width as f64 + 8.0 };
        mario.vel.x = if from_left { MOVE_SPEED } else { -MOVE_SPEED };
        mario.facing_right = from_left;

        // Spawn at a random height above a platform
        if !world.platforms.is_empty() {
            let idx = (js_sys::Math::random() * world.platforms.len() as f64) as usize;
            let plat = &world.platforms[idx % world.platforms.len()];
            mario.pos.y = ((plat.y - 2) * world.tile_size) as f64;
        } else {
            mario.pos.y = world.height as f64 / 2.0;
        }
        mario.vel.y = 0.0;
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

        // Land on top of block
        if goomba.vel.y > 0.0 {
            let goomba_bottom = gy + gh;
            let goomba_prev_bottom = goomba_bottom - goomba.vel.y;
            if gx + gw > bx && gx < bx + bw
                && goomba_bottom >= by && goomba_prev_bottom <= by + 4.0
            {
                goomba.pos.y = by - gh;
                goomba.vel.y = 0.0;
                goomba.on_ground = true;
            }
        }

        // Side collision
        if aabb_overlap(gx, gy + 2.0, gw, gh - 4.0, bx, by, bw, bh) {
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
    } else if goomba.pos.x > world.width as f64 - 8.0 {
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
        if mushroom.rise_progress >= 8.0 {
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

        if aabb_overlap(mx, my + 2.0, mw, mh - 4.0, bx, by, bw, bh) {
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
    if mushroom.pos.x < -8.0 {
        mushroom.pos.x = world.width as f64;
    } else if mushroom.pos.x > world.width as f64 {
        mushroom.pos.x = -8.0;
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

/// Update Koopa physics
fn update_koopa_physics(koopa: &mut Koopa, world: &GameWorld) {
    if !koopa.alive {
        return;
    }

    // Gravity
    koopa.vel.y += GRAVITY;
    if koopa.vel.y > MAX_FALL_SPEED {
        koopa.vel.y = MAX_FALL_SPEED;
    }

    // Movement based on state
    match koopa.state {
        KoopaState::Walking => {
            koopa.vel.x = if koopa.facing_right { KOOPA_SPEED } else { -KOOPA_SPEED };
        }
        KoopaState::Shell => {
            koopa.vel.x = 0.0;
            koopa.shell_timer = koopa.shell_timer.saturating_add(1);
            // Pop back up after a while
            if koopa.shell_timer > 180 {
                koopa.state = KoopaState::Walking;
                koopa.shell_timer = 0;
            }
        }
        KoopaState::ShellMoving => {
            // Shell keeps moving at high speed
        }
    }

    koopa.pos.x += koopa.vel.x;
    koopa.pos.y += koopa.vel.y;

    let (kx, ky, kw, kh) = koopa.hitbox();
    let tile_size = world.tile_size;
    koopa.on_ground = false;

    // Platform collision
    for platform in &world.platforms {
        let (px, py, pw, _ph) = platform.hitbox(tile_size);
        let koopa_bottom = ky + kh;
        let koopa_prev_bottom = koopa_bottom - koopa.vel.y;

        if koopa.vel.y > 0.0
            && kx + kw > px && kx < px + pw
            && koopa_bottom >= py && koopa_prev_bottom <= py + 4.0
        {
            koopa.pos.y = py - kh;
            koopa.vel.y = 0.0;
            koopa.on_ground = true;

            // Turn at edges (only when walking)
            if koopa.state == KoopaState::Walking {
                let koopa_center = koopa.pos.x + kw / 2.0;
                if koopa_center < px + 8.0 {
                    koopa.facing_right = true;
                } else if koopa_center > px + pw - 8.0 {
                    koopa.facing_right = false;
                }
            }
        }
    }

    // Block collision
    for block in &world.blocks {
        let (bx, by, bw, bh) = block.hitbox(tile_size);
        let (kx, ky, kw, kh) = koopa.hitbox();

        // Land on top of block
        if koopa.vel.y > 0.0 {
            let koopa_bottom = ky + kh;
            let koopa_prev_bottom = koopa_bottom - koopa.vel.y;
            if kx + kw > bx && kx < bx + bw
                && koopa_bottom >= by && koopa_prev_bottom <= by + 4.0
            {
                koopa.pos.y = by - kh;
                koopa.vel.y = 0.0;
                koopa.on_ground = true;
            }
        }

        // Side collision - bounce shell, turn walker
        if aabb_overlap(kx, ky + 2.0, kw, kh - 4.0, bx, by, bw, bh) {
            if koopa.state == KoopaState::ShellMoving {
                koopa.vel.x = -koopa.vel.x;
            } else {
                koopa.facing_right = !koopa.facing_right;
            }
        }
    }

    // World boundaries
    if koopa.pos.x < 0.0 {
        if koopa.state == KoopaState::ShellMoving {
            koopa.vel.x = SHELL_SPEED;
        } else {
            koopa.facing_right = true;
        }
    } else if koopa.pos.x > world.width as f64 - 8.0 {
        if koopa.state == KoopaState::ShellMoving {
            koopa.vel.x = -SHELL_SPEED;
        } else {
            koopa.facing_right = false;
        }
    }

    if koopa.pos.y > world.height as f64 + 32.0 {
        koopa.alive = false;
    }

    // Walk animation
    if koopa.state == KoopaState::Walking {
        koopa.walk_timer += 1;
        if koopa.walk_timer >= 12 {
            koopa.walk_timer = 0;
            koopa.walk_frame = (koopa.walk_frame + 1) % 2;
        }
    }
}

/// Update Coin physics
fn update_coin_physics(coin: &mut Coin, world: &GameWorld) {
    if coin.collected {
        return;
    }

    // Gravity
    coin.vel.y += GRAVITY;
    if coin.vel.y > MAX_FALL_SPEED {
        coin.vel.y = MAX_FALL_SPEED;
    }

    coin.pos.x += coin.vel.x;
    coin.pos.y += coin.vel.y;

    let (cx, cy, cw, ch) = coin.hitbox();
    let tile_size = world.tile_size;
    coin.on_ground = false;

    // Platform collision
    for platform in &world.platforms {
        let (px, py, pw, _ph) = platform.hitbox(tile_size);
        let coin_bottom = cy + ch;
        let coin_prev_bottom = coin_bottom - coin.vel.y;

        if coin.vel.y > 0.0
            && cx + cw > px && cx < px + pw
            && coin_bottom >= py && coin_prev_bottom <= py + 4.0
        {
            coin.pos.y = py - ch;
            coin.vel.y = 0.0;
            coin.on_ground = true;
        }
    }

    // Block collision
    for block in &world.blocks {
        let (bx, by, bw, bh) = block.hitbox(tile_size);
        let (cx, cy, cw, ch) = coin.hitbox();

        if coin.vel.y > 0.0 {
            let coin_bottom = cy + ch;
            let coin_prev_bottom = coin_bottom - coin.vel.y;
            if cx + cw > bx && cx < bx + bw
                && coin_bottom >= by && coin_prev_bottom <= by + 4.0
            {
                coin.pos.y = by - ch;
                coin.vel.y = 0.0;
                coin.on_ground = true;
            }
        }
    }

    if coin.pos.y > world.height as f64 + 32.0 {
        coin.collected = true; // Remove it
    }
}

/// Handle Goomba-Goomba collisions (bounce off each other)
fn check_goomba_collisions(world: &mut GameWorld) {
    let len = world.goombas.len();
    for i in 0..len {
        if !world.goombas[i].alive {
            continue;
        }
        for j in (i + 1)..len {
            if !world.goombas[j].alive {
                continue;
            }

            let (ax, ay, aw, ah) = world.goombas[i].hitbox();
            let (bx, by, bw, bh) = world.goombas[j].hitbox();

            if aabb_overlap(ax, ay, aw, ah, bx, by, bw, bh) {
                // Bounce off each other - reverse directions
                let a_center = ax + aw / 2.0;
                let b_center = bx + bw / 2.0;

                if a_center < b_center {
                    // A is to the left, A goes left, B goes right
                    world.goombas[i].vel.x = -GOOMBA_SPEED;
                    world.goombas[i].facing_right = false;
                    world.goombas[j].vel.x = GOOMBA_SPEED;
                    world.goombas[j].facing_right = true;
                } else {
                    world.goombas[i].vel.x = GOOMBA_SPEED;
                    world.goombas[i].facing_right = true;
                    world.goombas[j].vel.x = -GOOMBA_SPEED;
                    world.goombas[j].facing_right = false;
                }

                // Push apart
                let overlap = (aw / 2.0 + bw / 2.0) - (b_center - a_center).abs();
                if overlap > 0.0 {
                    world.goombas[i].pos.x -= overlap / 2.0;
                    world.goombas[j].pos.x += overlap / 2.0;
                }
            }
        }
    }
}

/// Check for Mario interacting with Koopas
fn check_koopa_collisions(world: &mut GameWorld) {
    let mut koopa_stomps: Vec<usize> = Vec::new();
    let mut shell_kicks: Vec<(usize, bool)> = Vec::new(); // (koopa_idx, kick_right)
    let mut mario_hits: Vec<usize> = Vec::new();

    for (mi, mario) in world.marios.iter().enumerate() {
        if !mario.alive || mario.state == MarioState::Dead {
            continue;
        }

        for (ki, koopa) in world.koopas.iter().enumerate() {
            if !koopa.alive {
                continue;
            }

            let (mx, my, mw, mh) = mario.hitbox();
            let (kx, ky, kw, kh) = koopa.hitbox();

            if aabb_overlap(mx, my, mw, mh, kx, ky, kw, kh) {
                match koopa.state {
                    KoopaState::Walking => {
                        // Stomp or get hit
                        if mario.vel.y > 0.0 && my + mh - 4.0 < ky + 4.0 {
                            koopa_stomps.push(ki);
                        } else if mario.invincible_timer == 0 {
                            mario_hits.push(mi);
                        }
                    }
                    KoopaState::Shell => {
                        // Kick the shell
                        let kick_right = mx + mw / 2.0 < kx + kw / 2.0;
                        shell_kicks.push((ki, kick_right));
                    }
                    KoopaState::ShellMoving => {
                        // Moving shell kills Mario (unless stomping)
                        if mario.vel.y > 0.0 && my + mh - 4.0 < ky + 4.0 {
                            // Stop the shell
                            koopa_stomps.push(ki);
                        } else if mario.invincible_timer == 0 {
                            mario_hits.push(mi);
                        }
                    }
                }
            }
        }
    }

    // Process stomps (turn into shell or stop shell)
    for ki in koopa_stomps {
        let koopa = &mut world.koopas[ki];
        match koopa.state {
            KoopaState::Walking => {
                koopa.state = KoopaState::Shell;
                koopa.shell_timer = 0;
                koopa.pos.y += 4.0; // Adjust for shorter shell
            }
            KoopaState::ShellMoving => {
                koopa.state = KoopaState::Shell;
                koopa.vel.x = 0.0;
                koopa.shell_timer = 0;
            }
            _ => {}
        }
    }

    // Process shell kicks
    for (ki, kick_right) in shell_kicks {
        let koopa = &mut world.koopas[ki];
        koopa.state = KoopaState::ShellMoving;
        koopa.vel.x = if kick_right { SHELL_SPEED } else { -SHELL_SPEED };
    }

    // Process Mario hits
    for mi in mario_hits {
        let mario = &mut world.marios[mi];
        if mario.is_big {
            mario.is_big = false;
            mario.invincible_timer = 90;
            mario.pos.y += 8.0;
        } else {
            mario.alive = false;
            mario.state = MarioState::Dead;
            mario.vel.y = JUMP_VELOCITY;
            mario.death_timer = 60;
        }
    }

    // Shell kills Goombas and other Koopas
    for koopa in &world.koopas {
        if koopa.state != KoopaState::ShellMoving {
            continue;
        }
        let (kx, ky, kw, kh) = koopa.hitbox();

        for goomba in &mut world.goombas {
            if !goomba.alive {
                continue;
            }
            let (gx, gy, gw, gh) = goomba.hitbox();
            if aabb_overlap(kx, ky, kw, kh, gx, gy, gw, gh) {
                goomba.alive = false;
                goomba.squish_timer = 30;
            }
        }
    }

    // Shell-shell collision
    let koopa_count = world.koopas.len();
    for i in 0..koopa_count {
        if world.koopas[i].state != KoopaState::ShellMoving {
            continue;
        }
        for j in 0..koopa_count {
            if i == j || !world.koopas[j].alive {
                continue;
            }
            let (ax, ay, aw, ah) = world.koopas[i].hitbox();
            let (bx, by, bw, bh) = world.koopas[j].hitbox();
            if aabb_overlap(ax, ay, aw, ah, bx, by, bw, bh) {
                world.koopas[j].alive = false;
            }
        }
    }
}

/// Check for Mario collecting coins
fn check_coin_collisions(world: &mut GameWorld) {
    for mario in &world.marios {
        if !mario.alive {
            continue;
        }
        let (mx, my, mw, mh) = mario.hitbox();

        for coin in &mut world.coins {
            if coin.collected {
                continue;
            }
            let (cx, cy, cw, ch) = coin.hitbox();
            if aabb_overlap(mx, my, mw, mh, cx, cy, cw, ch) {
                coin.collected = true;
            }
        }
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
            stomped_mario.pos.y += 8.0; // Adjust position for smaller hitbox
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
            mario.pos.y += 8.0;
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
                    mario.pos.y -= 8.0; // Grow upward
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
                    world.debris.push(Debris::new(bx, by, -1.5, -4.0));
                    world.debris.push(Debris::new(bx + 4.0, by, 1.5, -4.0));
                    world.debris.push(Debris::new(bx, by + 4.0, -1.5, -3.0));
                    world.debris.push(Debris::new(bx + 4.0, by + 4.0, 1.5, -3.0));

                    blocks_to_remove.push(block_idx);
                } else {
                    // Bump animation
                    block.bump_offset = -2.0;
                }
            }
            BlockType::Question => {
                if !block.hit {
                    block.hit = true;
                    block.block_type = BlockType::QuestionEmpty;
                    block.bump_offset = -2.0;

                    // Spawn mushroom
                    let mx = (block.x * world.tile_size) as f64;
                    let my = (block.y * world.tile_size - 8) as f64;
                    mushrooms_to_spawn.push((mx, my));
                }
            }
            _ => {}
        }
    }

    // Spawn mushrooms
    for (x, y) in mushrooms_to_spawn {
        let mut mushroom = Mushroom::new(x, y + 8.0);
        mushroom.origin_y = y + 8.0;
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

    for koopa in &mut world.koopas {
        update_koopa_physics(koopa, &world_snapshot);
    }

    for mushroom in &mut world.mushrooms {
        update_mushroom_physics(mushroom, &world_snapshot);
    }

    for coin in &mut world.coins {
        update_coin_physics(coin, &world_snapshot);
    }

    let world_height = world.height;
    for debris in &mut world.debris {
        update_debris_physics(debris, world_height);
    }

    // Process collisions
    check_stomp_collisions(world);
    check_koopa_collisions(world);
    check_goomba_collisions(world);
    check_mushroom_collisions(world);
    check_coin_collisions(world);
    process_block_hits(world, block_hits);
    update_block_animations(world);

    // Cleanup dead entities
    world.goombas.retain(|g| g.alive || g.squish_timer > 0);
    world.koopas.retain(|k| k.alive);
    world.mushrooms.retain(|m| m.active);
    world.coins.retain(|c| !c.collected);
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
