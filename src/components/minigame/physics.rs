//! Physics simulation for the Mario mini-game

use super::entities::{GameWorld, Mario, Goomba, MarioState};

/// Physics constants
pub const GRAVITY: f64 = 0.5;
pub const JUMP_VELOCITY: f64 = -9.0;
pub const MOVE_SPEED: f64 = 2.5;
pub const MAX_FALL_SPEED: f64 = 8.0;
pub const GOOMBA_SPEED: f64 = 0.8;

/// Check if two rectangles overlap (AABB collision)
pub fn aabb_overlap(
    ax: f64, ay: f64, aw: f64, ah: f64,
    bx: f64, by: f64, bw: f64, bh: f64,
) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

/// Update a single Mario's physics
fn update_mario_physics(mario: &mut Mario, world: &GameWorld) {
    // Apply gravity
    mario.vel.y += GRAVITY;
    if mario.vel.y > MAX_FALL_SPEED {
        mario.vel.y = MAX_FALL_SPEED;
    }

    // Update position
    mario.pos.x += mario.vel.x;
    mario.pos.y += mario.vel.y;

    // Track if we're on ground this frame
    mario.on_ground = false;

    // Platform collision (only when falling)
    let (mx, my, mw, mh) = mario.hitbox();
    let tile_size = world.tile_size;

    for platform in &world.platforms {
        let (px, py, pw, _ph) = platform.hitbox(tile_size);

        // Check if Mario's feet are at platform level
        let mario_bottom = my + mh;
        let mario_prev_bottom = mario_bottom - mario.vel.y;

        // Only collide if:
        // 1. Falling (vel.y > 0)
        // 2. Horizontally overlapping
        // 3. Feet are at or below platform top
        // 4. Were above platform top before
        if mario.vel.y > 0.0
            && mx + mw > px && mx < px + pw
            && mario_bottom >= py && mario_prev_bottom <= py + 2.0
        {
            mario.pos.y = py - mh;
            mario.vel.y = 0.0;
            mario.on_ground = true;
        }
    }

    // World boundaries (wrap horizontally)
    if mario.pos.x < -16.0 {
        mario.pos.x = world.width as f64;
    } else if mario.pos.x > world.width as f64 {
        mario.pos.x = -16.0;
    }

    // If fallen off bottom, respawn at top
    if mario.pos.y > world.height as f64 + 32.0 {
        mario.pos.y = -32.0;
        // Find a random platform to spawn above
        if !world.platforms.is_empty() {
            let idx = (js_sys::Math::random() * world.platforms.len() as f64) as usize;
            let plat = &world.platforms[idx % world.platforms.len()];
            mario.pos.x = (plat.x * tile_size) as f64 + (plat.width * tile_size / 2) as f64;
        }
    }

    // Update animation state
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

    // Update facing direction
    if mario.vel.x > 0.1 {
        mario.facing_right = true;
    } else if mario.vel.x < -0.1 {
        mario.facing_right = false;
    }

    // Apply friction when on ground and not actively moving
    if mario.on_ground && !mario.is_player {
        mario.vel.x *= 0.9;
        if mario.vel.x.abs() < 0.1 {
            mario.vel.x = 0.0;
        }
    } else if mario.on_ground && mario.is_player {
        // Player friction is higher for more responsive controls
        mario.vel.x *= 0.7;
    }
}

/// Update a single Goomba's physics
fn update_goomba_physics(goomba: &mut Goomba, world: &GameWorld) {
    if !goomba.alive {
        // Squished animation countdown
        if goomba.squish_timer > 0 {
            goomba.squish_timer -= 1;
        }
        return;
    }

    // Apply gravity
    goomba.vel.y += GRAVITY;
    if goomba.vel.y > MAX_FALL_SPEED {
        goomba.vel.y = MAX_FALL_SPEED;
    }

    // Update position
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
            && goomba_bottom >= py && goomba_prev_bottom <= py + 2.0
        {
            goomba.pos.y = py - gh;
            goomba.vel.y = 0.0;

            // Check if at edge of platform - turn around
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

    // World boundaries - wrap or turn around
    if goomba.pos.x < 0.0 {
        goomba.vel.x = GOOMBA_SPEED;
        goomba.facing_right = true;
    } else if goomba.pos.x > world.width as f64 - 16.0 {
        goomba.vel.x = -GOOMBA_SPEED;
        goomba.facing_right = false;
    }

    // If fallen off, mark as dead
    if goomba.pos.y > world.height as f64 + 32.0 {
        goomba.alive = false;
    }

    // Walk animation
    goomba.walk_timer += 1;
    if goomba.walk_timer >= 12 {
        goomba.walk_timer = 0;
        goomba.walk_frame = (goomba.walk_frame + 1) % 2;
    }
}

/// Check for Mario stomping on Goomba
fn check_stomp_collisions(world: &mut GameWorld) {
    // Collect stomps to process
    let mut stomps: Vec<(usize, usize)> = Vec::new();

    for (mi, mario) in world.marios.iter().enumerate() {
        // Only check if falling
        if mario.vel.y <= 0.0 {
            continue;
        }

        let (_, _, _, feet_h) = mario.feet_hitbox();
        let feet_y = mario.pos.y + 16.0 - feet_h;

        for (gi, goomba) in world.goombas.iter().enumerate() {
            if !goomba.alive {
                continue;
            }

            let (gx, gy, gw, _) = goomba.head_hitbox();

            // Check if Mario's feet overlap Goomba's head
            let (fx, _, fw, fh) = mario.feet_hitbox();
            if aabb_overlap(fx, feet_y, fw, fh, gx, gy, gw, 8.0) {
                stomps.push((mi, gi));
            }
        }
    }

    // Process stomps
    for (mi, gi) in stomps {
        // Kill Goomba
        world.goombas[gi].alive = false;
        world.goombas[gi].squish_timer = 30; // Show squished sprite briefly

        // Bounce Mario
        world.marios[mi].vel.y = JUMP_VELOCITY * 0.6;
    }
}

/// Run one physics update tick
pub fn update(world: &mut GameWorld) {
    // Update all Marios
    let platforms_snapshot = world.platforms.clone();
    let world_width = world.width;
    let world_height = world.height;
    let tile_size = world.tile_size;

    // Create a temporary world view for physics
    let temp_world = GameWorld {
        platforms: platforms_snapshot,
        marios: Vec::new(),
        goombas: Vec::new(),
        width: world_width,
        height: world_height,
        tile_size,
        player_mario_id: None,
        next_mario_id: 0,
        goomba_spawn_timer: 0,
    };

    for mario in &mut world.marios {
        update_mario_physics(mario, &temp_world);
    }

    for goomba in &mut world.goombas {
        update_goomba_physics(goomba, &temp_world);
    }

    // Check stomp collisions
    check_stomp_collisions(world);

    // Remove dead Goombas whose squish timer expired
    world.goombas.retain(|g| g.alive || g.squish_timer > 0);
}

/// Apply player input to the player-controlled Mario
pub fn apply_player_input(world: &mut GameWorld, left: bool, right: bool, jump: bool) {
    if let Some(mario) = world.player_mario_mut() {
        // Horizontal movement
        if left && !right {
            mario.vel.x = -MOVE_SPEED;
        } else if right && !left {
            mario.vel.x = MOVE_SPEED;
        }

        // Jump (only if on ground)
        if jump && mario.on_ground {
            mario.vel.y = JUMP_VELOCITY;
            mario.on_ground = false;
        }
    }
}
