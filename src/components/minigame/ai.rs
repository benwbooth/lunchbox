//! AI behavior for auto-controlled Marios

use super::entities::{GameWorld, MarioState};
use super::physics::{MOVE_SPEED, JUMP_VELOCITY, BIG_JUMP_VELOCITY};

/// Update AI for all non-player Marios
pub fn update_ai(world: &mut GameWorld) {
    let player_id = world.player_mario_id;

    // Collect data for AI decisions
    let goomba_positions: Vec<(f64, f64)> = world.goombas
        .iter()
        .filter(|g| g.alive)
        .map(|g| (g.pos.x, g.pos.y))
        .collect();

    let mushroom_positions: Vec<(f64, f64)> = world.mushrooms
        .iter()
        .filter(|m| m.active && !m.rising)
        .map(|m| (m.pos.x, m.pos.y))
        .collect();

    let mario_positions: Vec<(u32, f64, f64, bool)> = world.marios
        .iter()
        .filter(|m| m.alive)
        .map(|m| (m.id, m.pos.x, m.pos.y, m.is_big))
        .collect();

    let platform_data: Vec<(f64, f64, f64)> = world.platforms
        .iter()
        .map(|p| {
            let (px, py, pw, _) = p.hitbox(world.tile_size);
            (px, py, pw)
        })
        .collect();

    let world_width = world.width as f64;

    for mario in &mut world.marios {
        // Skip player-controlled Mario
        if Some(mario.id) == player_id || !mario.alive || mario.state == MarioState::Dead {
            continue;
        }

        // Decrement cooldowns
        if mario.ai_jump_cooldown > 0 {
            mario.ai_jump_cooldown -= 1;
        }
        if mario.ai_direction_timer > 0 {
            mario.ai_direction_timer -= 1;
        }

        // Find nearest mushroom (highest priority if not big)
        let nearest_mushroom = if !mario.is_big {
            mushroom_positions
                .iter()
                .map(|(mx, my)| {
                    let dx = mx - mario.pos.x;
                    let dy = my - mario.pos.y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    (dist, *mx, *my)
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        } else {
            None
        };

        // Find nearest Goomba
        let nearest_goomba = goomba_positions
            .iter()
            .map(|(gx, gy)| {
                let dx = gx - mario.pos.x;
                let dy = gy - mario.pos.y;
                let dist = (dx * dx + dy * dy).sqrt();
                (dist, *gx, *gy)
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Find nearest other Mario (to stomp or avoid)
        let nearest_mario = mario_positions
            .iter()
            .filter(|(id, _, _, _)| *id != mario.id)
            .map(|(_, mx, my, is_big)| {
                let dx = mx - mario.pos.x;
                let dy = my - mario.pos.y;
                let dist = (dx * dx + dy * dy).sqrt();
                (dist, *mx, *my, *is_big)
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // AI decision making priority:
        // 1. Get mushroom if small and nearby
        // 2. Stomp enemy Mario or Goomba
        // 3. Wander
        if let Some((dist, mx, my)) = nearest_mushroom {
            if dist < 150.0 {
                ai_chase_target(mario, mx, my, &platform_data, world_width);
                continue;
            }
        }

        // Check if we can stomp a nearby target
        let can_stomp_mario = nearest_mario
            .as_ref()
            .map(|(d, _, my, is_big)| *d < 120.0 && (*my > mario.pos.y || !is_big))
            .unwrap_or(false);

        let can_stomp_goomba = nearest_goomba
            .as_ref()
            .map(|(d, _, _)| *d < 150.0)
            .unwrap_or(false);

        if can_stomp_mario {
            if let Some((_, mx, my, _)) = nearest_mario {
                ai_chase_target(mario, mx, my, &platform_data, world_width);
                continue;
            }
        }

        if can_stomp_goomba {
            if let Some((_, gx, gy)) = nearest_goomba {
                ai_chase_target(mario, gx, gy, &platform_data, world_width);
                continue;
            }
        }

        // Wander
        ai_wander(mario, &platform_data, world_width);
    }
}

/// AI behavior: chase a target (goomba, mushroom, or mario)
fn ai_chase_target(
    mario: &mut super::entities::Mario,
    target_x: f64,
    target_y: f64,
    platforms: &[(f64, f64, f64)],
    world_width: f64,
) {
    let dx = target_x - mario.pos.x;
    let dy = target_y - mario.pos.y;

    // Move toward target
    if dx.abs() > 8.0 {
        if dx > 0.0 {
            mario.vel.x = MOVE_SPEED * 0.9;
            mario.facing_right = true;
        } else {
            mario.vel.x = -MOVE_SPEED * 0.9;
            mario.facing_right = false;
        }
    }

    // Jump logic
    if mario.on_ground && mario.ai_jump_cooldown == 0 {
        let jump_vel = if mario.is_big { BIG_JUMP_VELOCITY } else { JUMP_VELOCITY };

        // Jump to stomp if close and target is at same level or below
        if dx.abs() < 50.0 && dy >= -10.0 && dy < 60.0 {
            mario.vel.y = jump_vel;
            mario.on_ground = false;
            mario.ai_jump_cooldown = 25;
        }
        // Jump to reach target on higher platform
        else if dy < -20.0 && dx.abs() < 80.0 {
            mario.vel.y = jump_vel;
            mario.on_ground = false;
            mario.ai_jump_cooldown = 40;
        }
    }

    if mario.on_ground {
        check_platform_edge(mario, platforms, world_width);
    }
}

/// AI behavior: wander around
fn ai_wander(
    mario: &mut super::entities::Mario,
    platforms: &[(f64, f64, f64)],
    world_width: f64,
) {
    // Pick direction if timer expired
    if mario.ai_direction_timer == 0 {
        mario.ai_direction_timer = (js_sys::Math::random() * 90.0 + 30.0) as u8;

        if js_sys::Math::random() < 0.7 {
            mario.facing_right = js_sys::Math::random() > 0.5;
        }
    }

    // Move in facing direction
    if mario.ai_direction_timer > 15 {
        let speed = MOVE_SPEED * 0.5;
        mario.vel.x = if mario.facing_right { speed } else { -speed };
    }

    // Occasionally jump
    if mario.on_ground && mario.ai_jump_cooldown == 0 && js_sys::Math::random() < 0.015 {
        let jump_vel = if mario.is_big { BIG_JUMP_VELOCITY } else { JUMP_VELOCITY };
        mario.vel.y = jump_vel * 0.85;
        mario.on_ground = false;
        mario.ai_jump_cooldown = 50;
    }

    if mario.on_ground {
        check_platform_edge(mario, platforms, world_width);
    }

    // Turn at world edges
    if mario.pos.x < 30.0 {
        mario.facing_right = true;
        mario.ai_direction_timer = 30;
    } else if mario.pos.x > world_width - 50.0 {
        mario.facing_right = false;
        mario.ai_direction_timer = 30;
    }
}

/// Check platform edges and decide whether to turn or jump
fn check_platform_edge(
    mario: &mut super::entities::Mario,
    platforms: &[(f64, f64, f64)],
    _world_width: f64,
) {
    let mario_center = mario.pos.x + 8.0;
    let mario_bottom = mario.pos.y + mario.sprite_height();
    let look_ahead = if mario.facing_right { 24.0 } else { -24.0 };
    let future_x = mario_center + look_ahead;

    let has_ground_ahead = platforms.iter().any(|(px, py, pw)| {
        future_x >= *px && future_x <= *px + *pw
            && *py >= mario_bottom - 4.0 && *py <= mario_bottom + 8.0
    });

    if !has_ground_ahead {
        let can_jump = platforms.iter().any(|(px, py, pw)| {
            let platform_center = *px + *pw / 2.0;
            let dx = (platform_center - mario_center).abs();
            let dy = *py - mario_bottom;
            dx < 70.0 && dy < 0.0 && dy > -100.0
        });

        let jump_vel = if mario.is_big { BIG_JUMP_VELOCITY } else { JUMP_VELOCITY };

        if can_jump && mario.ai_jump_cooldown == 0 && js_sys::Math::random() < 0.4 {
            mario.vel.y = jump_vel;
            mario.on_ground = false;
            mario.ai_jump_cooldown = 40;
        } else {
            mario.facing_right = !mario.facing_right;
            mario.ai_direction_timer = 25;
        }
    }
}
