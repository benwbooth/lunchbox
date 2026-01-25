//! Main Mario mini-game component with canvas rendering

use leptos::prelude::*;
use leptos::html;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, KeyboardEvent};
use std::cell::RefCell;
use std::rc::Rc;

use super::sprites::*;
use super::entities::{GameWorld, MarioState, BlockType};
use super::physics;
use super::world;
use super::ai;

/// Render scale (2x pixel size)
const SCALE: f64 = 2.0;

/// Target FPS
const TARGET_FPS: f64 = 60.0;
const FRAME_TIME: f64 = 1000.0 / TARGET_FPS;

/// Spawn intervals
const GOOMBA_RESPAWN_INTERVAL: u32 = 240;
const MAX_GOOMBAS: usize = 20;

/// Input state for player controls
#[derive(Clone, Default)]
struct InputState {
    left: bool,
    right: bool,
    jump: bool,
}

/// Draw a 4-color sprite (NES style) to the canvas
fn draw_sprite_4color(
    ctx: &CanvasRenderingContext2d,
    sprite: &Sprite,
    x: f64,
    y: f64,
    palette: &Palette,
    flip_x: bool,
    scale: f64,
) {
    for row in 0..8 {
        let lo = sprite[row * 2];
        let hi = sprite[row * 2 + 1];

        for col in 0..8 {
            let bit = 7 - col;
            let color_idx = ((lo >> bit) & 1) | (((hi >> bit) & 1) << 1);

            if color_idx == 0 {
                continue; // Transparent
            }

            let color = palette[color_idx as usize];
            let r = ((color >> 16) & 0xFF) as u8;
            let g = ((color >> 8) & 0xFF) as u8;
            let b = (color & 0xFF) as u8;

            ctx.set_fill_style_str(&format!("rgb({},{},{})", r, g, b));

            let px = if flip_x {
                x + (7 - col) as f64 * scale
            } else {
                x + col as f64 * scale
            };
            let py = y + row as f64 * scale;
            ctx.fill_rect(px, py, scale, scale);
        }
    }
}

/// Draw the game world to the canvas
fn render(ctx: &CanvasRenderingContext2d, world: &GameWorld, width: u32, height: u32) {
    // Clear with black background
    ctx.set_fill_style_str("#000000");
    ctx.fill_rect(0.0, 0.0, width as f64, height as f64);

    let tile_size = world.tile_size as f64;

    // Draw ground platforms
    for platform in &world.platforms {
        if platform.is_ground {
            let (px, py, pw, _) = platform.hitbox(world.tile_size);
            for tx in 0..platform.width {
                draw_sprite_4color(
                    ctx,
                    &GROUND,
                    px + tx as f64 * tile_size,
                    py,
                    &PALETTE_GROUND,
                    false,
                    SCALE,
                );
            }
        }
    }

    // Draw blocks (bricks and question blocks)
    for block in &world.blocks {
        let (bx, by, _, _) = block.hitbox(world.tile_size);
        let (sprite, palette) = match block.block_type {
            BlockType::Brick => (&BRICK, &PALETTE_BRICK),
            BlockType::Question => (&QUESTION, &PALETTE_QUESTION),
            BlockType::QuestionEmpty => (&QUESTION_EMPTY, &PALETTE_BRICK),
            BlockType::Ground => (&GROUND, &PALETTE_GROUND),
        };
        draw_sprite_4color(ctx, sprite, bx, by, palette, false, SCALE);
    }

    // Draw debris
    for debris in &world.debris {
        if debris.alive {
            // Draw a small brick piece with rotation (simplified)
            ctx.save();
            ctx.translate(debris.pos.x + 4.0, debris.pos.y + 4.0).ok();
            ctx.rotate(debris.rotation).ok();
            draw_sprite_4color(ctx, &BRICK_DEBRIS, -4.0, -4.0, &PALETTE_BRICK, false, SCALE);
            ctx.restore();
        }
    }

    // Draw mushrooms
    for mushroom in &world.mushrooms {
        if mushroom.active {
            draw_sprite_4color(
                ctx,
                &MUSHROOM,
                mushroom.pos.x,
                mushroom.pos.y,
                &PALETTE_MUSHROOM,
                false,
                SCALE,
            );
        }
    }

    // Draw Goombas
    for goomba in &world.goombas {
        if goomba.alive {
            draw_sprite_4color(
                ctx,
                &GOOMBA,
                goomba.pos.x,
                goomba.pos.y,
                &PALETTE_GOOMBA,
                !goomba.facing_right,
                SCALE,
            );
        } else if goomba.squish_timer > 0 {
            // Draw squished Goomba
            ctx.set_fill_style_str(&format!("rgb({},{},{})",
                ((NES_BROWN >> 16) & 0xFF) as u8,
                ((NES_BROWN >> 8) & 0xFF) as u8,
                (NES_BROWN & 0xFF) as u8
            ));
            ctx.fill_rect(goomba.pos.x, goomba.pos.y + 12.0, 16.0, 4.0);
        }
    }

    // Draw Marios
    for mario in &world.marios {
        // Skip if dead and off screen
        if mario.state == MarioState::Dead && mario.pos.y > height as f64 {
            continue;
        }

        // Blink when invincible
        if mario.invincible_timer > 0 && (mario.invincible_timer / 4) % 2 == 0 {
            continue;
        }

        let palette = if mario.is_player {
            &PALETTE_PLAYER
        } else {
            &PALETTE_MARIO
        };

        if mario.state == MarioState::Dead {
            draw_sprite_4color(ctx, &MARIO_DEAD, mario.pos.x, mario.pos.y, palette, false, SCALE);
        } else if mario.is_big {
            // Draw big Mario (2 sprites stacked)
            let (top, bot) = match mario.state {
                MarioState::Standing => (&MARIO_BIG_STAND_TOP, &MARIO_BIG_STAND_BOT),
                MarioState::Walking => {
                    if mario.walk_frame == 0 {
                        (&MARIO_BIG_WALK_TOP, &MARIO_BIG_WALK_BOT)
                    } else {
                        (&MARIO_BIG_STAND_TOP, &MARIO_BIG_STAND_BOT)
                    }
                }
                MarioState::Jumping => (&MARIO_BIG_WALK_TOP, &MARIO_BIG_WALK_BOT),
                MarioState::Dead => (&MARIO_BIG_STAND_TOP, &MARIO_BIG_STAND_BOT),
            };
            draw_sprite_4color(ctx, top, mario.pos.x, mario.pos.y, palette, !mario.facing_right, SCALE);
            draw_sprite_4color(ctx, bot, mario.pos.x, mario.pos.y + 16.0, palette, !mario.facing_right, SCALE);
        } else {
            // Draw small Mario
            let sprite = match mario.state {
                MarioState::Standing => &MARIO_STAND,
                MarioState::Walking => {
                    if mario.walk_frame == 0 { &MARIO_WALK1 } else { &MARIO_WALK2 }
                }
                MarioState::Jumping => &MARIO_JUMP,
                MarioState::Dead => &MARIO_DEAD,
            };
            draw_sprite_4color(ctx, sprite, mario.pos.x, mario.pos.y, palette, !mario.facing_right, SCALE);
        }

        // Draw player indicator
        if mario.is_player && mario.alive {
            ctx.set_fill_style_str("#ffffff");
            ctx.fill_rect(mario.pos.x + 6.0, mario.pos.y - 8.0, 4.0, 4.0);
        }
    }
}

/// The Mario Mini-Game component
#[component]
pub fn MarioMinigame() -> impl IntoView {
    let canvas_ref = NodeRef::<html::Canvas>::new();
    let game_world: Rc<RefCell<Option<GameWorld>>> = Rc::new(RefCell::new(None));
    let input_state: Rc<RefCell<InputState>> = Rc::new(RefCell::new(InputState::default()));
    let animation_frame_id: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
    let last_frame_time: Rc<RefCell<f64>> = Rc::new(RefCell::new(0.0));
    let initialized: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    let game_world_init = game_world.clone();
    let animation_frame_id_init = animation_frame_id.clone();
    let last_frame_time_init = last_frame_time.clone();
    let input_state_loop = input_state.clone();
    let initialized_clone = initialized.clone();

    Effect::new(move || {
        // Prevent double initialization
        if *initialized_clone.borrow() {
            return;
        }

        let Some(canvas) = canvas_ref.get() else { return };
        let canvas: HtmlCanvasElement = canvas.into();

        // Get window dimensions for full-screen canvas
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };

        // Use window inner dimensions for proper sizing
        let width = window.inner_width().ok()
            .and_then(|v| v.as_f64())
            .map(|w| w as u32)
            .unwrap_or(800);
        let height = window.inner_height().ok()
            .and_then(|v| v.as_f64())
            .map(|h| h as u32)
            .unwrap_or(600);

        // Try to get container size first, fall back to window
        let parent = canvas.parent_element();
        let (width, height) = if let Some(p) = parent {
            let pw = p.client_width();
            let ph = p.client_height();
            if pw > 100 && ph > 100 {
                (pw as u32, ph as u32)
            } else {
                (width, height)
            }
        } else {
            (width, height)
        };

        if width < 100 || height < 100 {
            return; // Container not ready yet
        }

        canvas.set_width(width);
        canvas.set_height(height);

        let ctx = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok());

        let Some(ctx) = ctx else { return };

        ctx.set_image_smoothing_enabled(false);

        *initialized_clone.borrow_mut() = true;

        // Generate world with 16-pixel tiles
        let tile_size = 16;
        let world = world::generate_world(width as i32, height as i32, tile_size);
        *game_world_init.borrow_mut() = Some(world);

        // Start game loop
        let game_world_loop = game_world_init.clone();
        let animation_frame_id_loop = animation_frame_id_init.clone();
        let last_frame_time_loop = last_frame_time_init.clone();
        let input_loop = input_state_loop.clone();

        let game_loop_ref: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
        let game_loop_ref_inner = game_loop_ref.clone();

        let closure = Closure::new(move |timestamp: f64| {
            let mut last_time = last_frame_time_loop.borrow_mut();

            let elapsed = timestamp - *last_time;
            if elapsed < FRAME_TIME {
                if let Some(window) = web_sys::window() {
                    if let Some(ref closure) = *game_loop_ref_inner.borrow() {
                        let id = window.request_animation_frame(closure.as_ref().unchecked_ref()).ok();
                        *animation_frame_id_loop.borrow_mut() = id;
                    }
                }
                return;
            }
            *last_time = timestamp;

            if let Some(ref mut world) = *game_world_loop.borrow_mut() {
                // Apply player input
                let input = input_loop.borrow();
                physics::apply_player_input(world, input.left, input.right, input.jump);

                // Update AI
                ai::update_ai(world);

                // Update physics
                physics::update(world);

                // Spawn new Goombas periodically
                world.spawn_timer += 1;
                if world.spawn_timer >= GOOMBA_RESPAWN_INTERVAL && world.goombas.len() < MAX_GOOMBAS {
                    world::spawn_random_goomba(world);
                    world.spawn_timer = 0;
                }

                // Render
                render(&ctx, world, width, height);
            }

            if let Some(window) = web_sys::window() {
                if let Some(ref closure) = *game_loop_ref_inner.borrow() {
                    let id = window.request_animation_frame(closure.as_ref().unchecked_ref()).ok();
                    *animation_frame_id_loop.borrow_mut() = id;
                }
            }
        });

        *game_loop_ref.borrow_mut() = Some(closure);

        if let Some(window) = web_sys::window() {
            let id = {
                let borrow = game_loop_ref.borrow();
                if let Some(ref closure) = *borrow {
                    window.request_animation_frame(closure.as_ref().unchecked_ref()).ok()
                } else {
                    None
                }
            };
            *animation_frame_id_init.borrow_mut() = id;
        }

        std::mem::forget(game_loop_ref);
    });

    // Keyboard input handlers
    let input_keydown = input_state.clone();
    let input_keyup = input_state.clone();

    let on_keydown = move |ev: KeyboardEvent| {
        let key = ev.key();
        let mut input = input_keydown.borrow_mut();

        match key.as_str() {
            "ArrowLeft" | "a" | "A" => {
                input.left = true;
                ev.prevent_default();
            }
            "ArrowRight" | "d" | "D" => {
                input.right = true;
                ev.prevent_default();
            }
            "ArrowUp" | "w" | "W" | " " => {
                input.jump = true;
                ev.prevent_default();
            }
            _ => {}
        }
    };

    let on_keyup = move |ev: KeyboardEvent| {
        let key = ev.key();
        let mut input = input_keyup.borrow_mut();

        match key.as_str() {
            "ArrowLeft" | "a" | "A" => input.left = false,
            "ArrowRight" | "d" | "D" => input.right = false,
            "ArrowUp" | "w" | "W" | " " => input.jump = false,
            _ => {}
        }
    };

    // Click to select Mario
    let game_world_click = game_world.clone();
    let on_click = move |ev: web_sys::MouseEvent| {
        let target = ev.target().and_then(|t| t.dyn_into::<HtmlCanvasElement>().ok());
        let Some(canvas) = target else { return };

        let rect = canvas.get_bounding_client_rect();
        let click_x = ev.client_x() as f64 - rect.left();
        let click_y = ev.client_y() as f64 - rect.top();

        if let Some(ref mut world) = *game_world_click.borrow_mut() {
            for mario in &world.marios {
                if !mario.alive {
                    continue;
                }
                let (mx, my, mw, mh) = mario.hitbox();
                if click_x >= mx && click_x <= mx + mw && click_y >= my && click_y <= my + mh {
                    world.set_player(mario.id);
                    break;
                }
            }
        }
    };

    view! {
        <div
            class="minigame-container"
            tabindex="0"
            on:keydown=on_keydown
            on:keyup=on_keyup
        >
            <canvas
                class="minigame-canvas"
                node_ref=canvas_ref
                on:click=on_click
            />
            <div class="minigame-overlay">
                <p>"Select a platform to view games."</p>
            </div>
        </div>
    }
}
