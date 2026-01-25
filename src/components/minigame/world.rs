//! Procedural world generation for the Mario mini-game

use super::entities::{GameWorld, Mario, Goomba, Platform};

/// Generate a new game world with platforms, Marios, and Goombas
pub fn generate_world(width: i32, height: i32, tile_size: i32) -> GameWorld {
    let mut world = GameWorld::new(width, height);
    world.tile_size = tile_size;

    let tiles_wide = width / tile_size;
    let tiles_high = height / tile_size;

    // Ground level is near the bottom
    let ground_y = tiles_high - 2;

    // Generate ground with gaps
    generate_ground(&mut world, tiles_wide, ground_y);

    // Generate floating platforms at various heights
    generate_floating_platforms(&mut world, tiles_wide, ground_y);

    // Spawn Marios
    spawn_marios(&mut world, tiles_wide, tile_size);

    // Spawn initial Goombas
    spawn_goombas(&mut world, tile_size);

    world
}

/// Generate ground platforms with occasional gaps
fn generate_ground(world: &mut GameWorld, tiles_wide: i32, ground_y: i32) {
    let mut x = 0;

    while x < tiles_wide {
        // Decide platform length (60-80% chance of long platform)
        let length = if js_sys::Math::random() < 0.7 {
            (js_sys::Math::random() * 8.0 + 6.0) as i32 // 6-14 tiles
        } else {
            (js_sys::Math::random() * 4.0 + 3.0) as i32 // 3-7 tiles
        };

        let actual_length = length.min(tiles_wide - x);
        if actual_length > 0 {
            world.platforms.push(Platform::new(x, ground_y, actual_length, true));
        }

        x += actual_length;

        // Gap (20% chance, 2-4 tiles wide)
        if x < tiles_wide && js_sys::Math::random() < 0.2 {
            let gap = (js_sys::Math::random() * 3.0 + 2.0) as i32;
            x += gap;
        }
    }
}

/// Generate floating brick platforms at various heights
fn generate_floating_platforms(world: &mut GameWorld, tiles_wide: i32, ground_y: i32) {
    // Define height levels for floating platforms
    let levels = [
        ground_y - 4,  // Low platforms
        ground_y - 7,  // Medium platforms
        ground_y - 10, // High platforms
        ground_y - 13, // Very high platforms
    ];

    for &level_y in &levels {
        if level_y < 2 {
            continue; // Too close to top
        }

        let mut x = (js_sys::Math::random() * 5.0) as i32;

        while x < tiles_wide - 3 {
            // 40% chance to place a platform at this position
            if js_sys::Math::random() < 0.4 {
                let length = (js_sys::Math::random() * 4.0 + 3.0) as i32; // 3-7 tiles
                let actual_length = length.min(tiles_wide - x);

                // Slight vertical variation
                let y_offset = (js_sys::Math::random() * 2.0 - 1.0) as i32;
                let platform_y = (level_y + y_offset).max(2);

                world.platforms.push(Platform::new(x, platform_y, actual_length, false));

                x += actual_length + 3; // Gap between platforms
            } else {
                x += 2;
            }
        }
    }
}

/// Spawn Mario characters
fn spawn_marios(world: &mut GameWorld, tiles_wide: i32, tile_size: i32) {
    // Number of Marios based on screen width (3-8)
    let count = ((tiles_wide as f64 / 15.0).ceil() as i32).clamp(3, 8);

    for i in 0..count {
        // Spread Marios across the level
        let x_segment = (tiles_wide * tile_size) / count;
        let base_x = i * x_segment + x_segment / 2;
        let x = (base_x as f64 + js_sys::Math::random() * 50.0 - 25.0).max(16.0);

        // Spawn above a random platform
        let platform_idx = (js_sys::Math::random() * world.platforms.len() as f64) as usize;
        let y = if let Some(plat) = world.platforms.get(platform_idx) {
            (plat.y * tile_size - 20) as f64
        } else {
            100.0
        };

        let id = world.next_id();
        let mut mario = Mario::new(x, y, id);
        mario.facing_right = js_sys::Math::random() > 0.5;
        world.marios.push(mario);
    }
}

/// Spawn Goombas on platforms
fn spawn_goombas(world: &mut GameWorld, tile_size: i32) {
    let platform_indices: Vec<usize> = (0..world.platforms.len()).collect();

    for idx in platform_indices {
        let platform = &world.platforms[idx];

        // 35% chance to spawn a Goomba on this platform
        if js_sys::Math::random() < 0.35 && platform.width >= 3 {
            let goomba_x = ((platform.x + 1) * tile_size) as f64
                + js_sys::Math::random() * ((platform.width - 2) * tile_size) as f64;
            let goomba_y = ((platform.y - 1) * tile_size) as f64;

            let mut goomba = Goomba::new(goomba_x, goomba_y);
            goomba.facing_right = js_sys::Math::random() > 0.5;
            if goomba.facing_right {
                goomba.vel.x = super::physics::GOOMBA_SPEED;
            } else {
                goomba.vel.x = -super::physics::GOOMBA_SPEED;
            }
            world.goombas.push(goomba);
        }
    }
}

/// Spawn a new Goomba at a random location (called periodically to replenish)
pub fn spawn_random_goomba(world: &mut GameWorld) {
    if world.platforms.is_empty() {
        return;
    }

    // Pick a random platform (prefer higher ones for variety)
    let idx = (js_sys::Math::random() * world.platforms.len() as f64) as usize;
    let platform = &world.platforms[idx];

    if platform.width < 3 {
        return;
    }

    let tile_size = world.tile_size;
    let goomba_x = ((platform.x + 1) * tile_size) as f64
        + js_sys::Math::random() * ((platform.width - 2) * tile_size) as f64;
    let goomba_y = ((platform.y - 1) * tile_size) as f64;

    let mut goomba = Goomba::new(goomba_x, goomba_y);
    goomba.facing_right = js_sys::Math::random() > 0.5;
    if goomba.facing_right {
        goomba.vel.x = super::physics::GOOMBA_SPEED;
    } else {
        goomba.vel.x = -super::physics::GOOMBA_SPEED;
    }
    world.goombas.push(goomba);
}
