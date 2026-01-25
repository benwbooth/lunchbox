//! Procedural world generation for the Mario mini-game

use super::entities::{GameWorld, Mario, Goomba, Platform, Block, BlockType};
use super::physics::GOOMBA_SPEED;

/// Generate a new game world with platforms, blocks, Marios, and Goombas
pub fn generate_world(width: i32, height: i32, tile_size: i32) -> GameWorld {
    let mut world = GameWorld::new(width, height);
    world.tile_size = tile_size;

    let tiles_wide = width / tile_size;
    let tiles_high = height / tile_size;

    // Ground level is near the bottom
    let ground_y = tiles_high - 2;

    // Generate ground platforms
    generate_ground(&mut world, tiles_wide, ground_y);

    // Generate floating platforms with bricks and question blocks
    generate_floating_platforms(&mut world, tiles_wide, ground_y, tile_size);

    // Spawn more Marios (8-15 based on screen width)
    spawn_marios(&mut world, tiles_wide, tile_size);

    // Spawn more Goombas
    spawn_goombas(&mut world, tile_size);

    world
}

/// Generate ground platforms with occasional gaps
fn generate_ground(world: &mut GameWorld, tiles_wide: i32, ground_y: i32) {
    let mut x = 0;

    while x < tiles_wide {
        // Platform length
        let length = if js_sys::Math::random() < 0.8 {
            (js_sys::Math::random() * 10.0 + 8.0) as i32 // 8-18 tiles
        } else {
            (js_sys::Math::random() * 5.0 + 4.0) as i32 // 4-9 tiles
        };

        let actual_length = length.min(tiles_wide - x);
        if actual_length > 0 {
            world.platforms.push(Platform::new(x, ground_y, actual_length, true));
        }

        x += actual_length;

        // Gap (15% chance, 2-3 tiles wide)
        if x < tiles_wide && js_sys::Math::random() < 0.15 {
            let gap = (js_sys::Math::random() * 2.0 + 2.0) as i32;
            x += gap;
        }
    }
}

/// Generate floating platforms with bricks and question blocks
fn generate_floating_platforms(world: &mut GameWorld, tiles_wide: i32, ground_y: i32, _tile_size: i32) {
    // Generate platform levels dynamically based on screen height
    // Space platforms every 3-4 tiles from near ground up to near top
    let mut levels = Vec::new();
    let mut y = ground_y - 4;
    while y > 3 {
        levels.push(y);
        y -= 3; // 3 tiles between each level
    }

    for &level_y in &levels {

        let mut x = (js_sys::Math::random() * 4.0) as i32;

        while x < tiles_wide - 3 {
            // 50% chance to place a platform at this position
            if js_sys::Math::random() < 0.5 {
                let length = (js_sys::Math::random() * 5.0 + 3.0) as i32; // 3-8 tiles
                let actual_length = length.min(tiles_wide - x);

                // Slight vertical variation
                let y_offset = (js_sys::Math::random() * 2.0 - 1.0) as i32;
                let platform_y = (level_y + y_offset).max(3);

                // Add platform for collision
                world.platforms.push(Platform::new(x, platform_y, actual_length, false));

                // Add blocks for this platform
                for tx in 0..actual_length {
                    let block_x = x + tx;
                    let block_type = if js_sys::Math::random() < 0.2 {
                        // 20% chance for question block
                        BlockType::Question
                    } else {
                        BlockType::Brick
                    };
                    world.blocks.push(Block::new(block_x, platform_y, block_type));
                }

                x += actual_length + (js_sys::Math::random() * 3.0 + 2.0) as i32;
            } else {
                x += 2;
            }
        }
    }

    // Add some standalone question blocks in the air (more for larger screens)
    let num_questions = ((tiles_wide * ground_y) / 80).max(5) as usize;
    for _ in 0..num_questions {
        let qx = (js_sys::Math::random() * (tiles_wide - 2) as f64) as i32 + 1;
        // Spawn across the full height (from y=4 to ground_y - 4)
        let qy = (js_sys::Math::random() * ((ground_y - 8) as f64) + 4.0) as i32;

        // Check if position is free
        let has_block = world.blocks.iter().any(|b| b.x == qx && b.y == qy);
        let has_platform = world.platforms.iter().any(|p| {
            qx >= p.x && qx < p.x + p.width && qy == p.y
        });

        if !has_block && !has_platform {
            world.blocks.push(Block::new(qx, qy, BlockType::Question));
            // Add invisible platform for standing
            world.platforms.push(Platform::new(qx, qy, 1, false));
        }
    }
}

/// Spawn more Mario characters
fn spawn_marios(world: &mut GameWorld, tiles_wide: i32, tile_size: i32) {
    // More Marios for larger screens: 12-25 based on screen width
    let count = ((tiles_wide as f64 / 8.0).ceil() as i32).clamp(12, 25);

    for i in 0..count {
        // Spread Marios across the level
        let x_segment = (tiles_wide * tile_size) / count;
        let base_x = i * x_segment + x_segment / 2;
        let x = (base_x as f64 + js_sys::Math::random() * 40.0 - 20.0).max(16.0);

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

        // 20% chance to start as big Mario
        if js_sys::Math::random() < 0.2 {
            mario.is_big = true;
        }

        world.marios.push(mario);
    }
}

/// Spawn more Goombas on platforms
fn spawn_goombas(world: &mut GameWorld, tile_size: i32) {
    let platform_count = world.platforms.len();

    for idx in 0..platform_count {
        let platform = &world.platforms[idx];

        // 50% chance to spawn Goombas on this platform
        if js_sys::Math::random() < 0.5 && platform.width >= 3 {
            // Spawn 1-2 Goombas per platform
            let goomba_count = if js_sys::Math::random() < 0.3 { 2 } else { 1 };

            for _ in 0..goomba_count {
                let goomba_x = ((platform.x + 1) * tile_size) as f64
                    + js_sys::Math::random() * ((platform.width - 2) * tile_size) as f64;
                let goomba_y = ((platform.y - 1) * tile_size) as f64;

                let mut goomba = Goomba::new(goomba_x, goomba_y);
                goomba.facing_right = js_sys::Math::random() > 0.5;
                goomba.vel.x = if goomba.facing_right { GOOMBA_SPEED } else { -GOOMBA_SPEED };
                world.goombas.push(goomba);
            }
        }
    }
}

/// Spawn a new Goomba at a random location
pub fn spawn_random_goomba(world: &mut GameWorld) {
    if world.platforms.is_empty() {
        return;
    }

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
    goomba.vel.x = if goomba.facing_right { GOOMBA_SPEED } else { -GOOMBA_SPEED };
    world.goombas.push(goomba);
}

/// Spawn a new Mario at a random location
pub fn spawn_random_mario(world: &mut GameWorld) {
    if world.platforms.is_empty() {
        return;
    }

    let idx = (js_sys::Math::random() * world.platforms.len() as f64) as usize;
    let platform = &world.platforms[idx];

    let tile_size = world.tile_size;
    let x = (platform.x * tile_size) as f64 + (platform.width * tile_size / 2) as f64;
    let y = ((platform.y - 2) * tile_size) as f64;

    let id = world.next_id();
    let mut mario = Mario::new(x, y, id);
    mario.facing_right = js_sys::Math::random() > 0.5;
    world.marios.push(mario);
}
