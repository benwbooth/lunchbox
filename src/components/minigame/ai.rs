//! AI behavior for auto-controlled Marios

use super::entities::GameWorld;
use super::physics::{MOVE_SPEED, JUMP_VELOCITY};

/// Update AI for all non-player Marios
pub fn update_ai(world: &mut GameWorld) {
    let player_id = world.player_mario_id;

    // Collect data needed for AI decisions
    let goomba_positions: Vec<(f64, f64, bool)> = world.goombas
        .iter()
        .filter(|g| g.alive)
        .map(|g| (g.pos.x, g.pos.y, g.alive))
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
        if Some(mario.id) == player_id {
            continue;
        }

        // Decrement cooldowns
        if mario.ai_jump_cooldown > 0 {
            mario.ai_jump_cooldown -= 1;
        }
        if mario.ai_direction_timer > 0 {
            mario.ai_direction_timer -= 1;
        }

        // Find nearest alive Goomba
        let nearest_goomba = goomba_positions
            .iter()
            .filter(|(_, _, alive)| *alive)
            .map(|(gx, gy, _)| {
                let dx = gx - mario.pos.x;
                let dy = gy - mario.pos.y;
                let dist = (dx * dx + dy * dy).sqrt();
                (dist, *gx, *gy)
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // AI decision making
        match nearest_goomba {
            Some((dist, gx, gy)) if dist < 200.0 => {
                // Goomba nearby - chase and stomp!
                ai_chase_goomba(mario, gx, gy, &platform_data);
            }
            _ => {
                // No nearby Goomba - wander and explore
                ai_wander(mario, &platform_data, world_width);
            }
        }
    }
}

/// AI behavior: chase and stomp a Goomba
fn ai_chase_goomba(mario: &mut super::entities::Mario, gx: f64, gy: f64, platforms: &[(f64, f64, f64)]) {
    let dx = gx - mario.pos.x;
    let dy = gy - mario.pos.y;

    // Move toward Goomba horizontally
    if dx.abs() > 4.0 {
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
        // Jump if Goomba is at same level or slightly below, and we're close
        if dx.abs() < 60.0 && dy >= -20.0 && dy < 80.0 {
            mario.vel.y = JUMP_VELOCITY;
            mario.on_ground = false;
            mario.ai_jump_cooldown = 30;
        }
        // Jump to reach Goomba on higher platform
        else if dy < -20.0 && dx.abs() < 100.0 {
            mario.vel.y = JUMP_VELOCITY;
            mario.on_ground = false;
            mario.ai_jump_cooldown = 45;
        }
    }

    // Check for platform edge - don't walk off while on ground
    if mario.on_ground {
        check_platform_edge(mario, platforms);
    }
}

/// AI behavior: wander around looking for action
fn ai_wander(mario: &mut super::entities::Mario, platforms: &[(f64, f64, f64)], world_width: f64) {
    // Pick a random direction to move if timer expired
    if mario.ai_direction_timer == 0 {
        mario.ai_direction_timer = (js_sys::Math::random() * 60.0 + 30.0) as u8;

        // 70% chance to pick a direction, 30% to stand still
        if js_sys::Math::random() < 0.7 {
            if js_sys::Math::random() > 0.5 {
                mario.facing_right = true;
            } else {
                mario.facing_right = false;
            }
        }
    }

    // Move in facing direction
    if mario.ai_direction_timer > 10 {
        if mario.facing_right {
            mario.vel.x = MOVE_SPEED * 0.6;
        } else {
            mario.vel.x = -MOVE_SPEED * 0.6;
        }
    }

    // Occasionally jump while wandering (exploration)
    if mario.on_ground && mario.ai_jump_cooldown == 0 && js_sys::Math::random() < 0.02 {
        mario.vel.y = JUMP_VELOCITY * 0.9;
        mario.on_ground = false;
        mario.ai_jump_cooldown = 60;
    }

    // Check for platform edge
    if mario.on_ground {
        check_platform_edge(mario, platforms);
    }

    // Turn around at world edges
    if mario.pos.x < 20.0 {
        mario.facing_right = true;
        mario.ai_direction_timer = 30;
    } else if mario.pos.x > world_width - 40.0 {
        mario.facing_right = false;
        mario.ai_direction_timer = 30;
    }
}

/// Check if Mario is near platform edge and should turn around or jump
fn check_platform_edge(mario: &mut super::entities::Mario, platforms: &[(f64, f64, f64)]) {
    let mario_center = mario.pos.x + 8.0;
    let mario_bottom = mario.pos.y + 16.0;
    let look_ahead = if mario.facing_right { 20.0 } else { -20.0 };
    let future_x = mario_center + look_ahead;

    // Check if there's a platform beneath where we'd be
    let has_ground_ahead = platforms.iter().any(|(px, py, pw)| {
        future_x >= *px && future_x <= *px + *pw
            && *py >= mario_bottom - 4.0 && *py <= mario_bottom + 16.0
    });

    if !has_ground_ahead {
        // Either turn around or jump to a nearby platform
        let can_jump_to_platform = platforms.iter().any(|(px, py, pw)| {
            // Check for platform above and ahead
            let platform_center = *px + *pw / 2.0;
            let dx = (platform_center - mario_center).abs();
            let dy = *py - mario_bottom;
            dx < 80.0 && dy < 0.0 && dy > -120.0
        });

        if can_jump_to_platform && mario.ai_jump_cooldown == 0 && js_sys::Math::random() < 0.5 {
            // Jump to the platform
            mario.vel.y = JUMP_VELOCITY;
            mario.on_ground = false;
            mario.ai_jump_cooldown = 45;
        } else {
            // Turn around
            mario.facing_right = !mario.facing_right;
            mario.ai_direction_timer = 20;
        }
    }
}
