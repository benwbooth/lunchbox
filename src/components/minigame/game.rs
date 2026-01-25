//! Main Mario mini-game component with canvas rendering

use leptos::prelude::*;
use leptos::html;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, KeyboardEvent};
use std::cell::RefCell;
use std::rc::Rc;

use super::sprites::*;
use super::entities::{GameWorld, MarioState};
use super::physics;
use super::world;
use super::ai;

/// Render scale (pixels per sprite pixel)
const SCALE: i32 = 2;

/// Target FPS
const TARGET_FPS: f64 = 60.0;
const FRAME_TIME: f64 = 1000.0 / TARGET_FPS;

/// Goomba respawn interval (frames)
const GOOMBA_RESPAWN_INTERVAL: u32 = 300; // ~5 seconds at 60fps
const MAX_GOOMBAS: usize = 12;

/// Input state for player controls
#[derive(Clone, Default)]
struct InputState {
    left: bool,
    right: bool,
    jump: bool,
}

/// Draw an 8x8 sprite to the canvas at the given position
fn draw_sprite(
    ctx: &CanvasRenderingContext2d,
    sprite: &[u8; 8],
    x: f64,
    y: f64,
    color: u32,
    flip_x: bool,
) {
    let r = ((color >> 16) & 0xFF) as u8;
    let g = ((color >> 8) & 0xFF) as u8;
    let b = (color & 0xFF) as u8;
    let color_str = format!("rgb({},{},{})", r, g, b);

    ctx.set_fill_style_str(&color_str);

    for (row_idx, row) in sprite.iter().enumerate() {
        for bit in 0..8 {
            let bit_pos = if flip_x { bit } else { 7 - bit };
            if (row >> bit_pos) & 1 == 1 {
                let px = if flip_x {
                    x + (7 - bit) as f64 * SCALE as f64
                } else {
                    x + bit as f64 * SCALE as f64
                };
                let py = y + row_idx as f64 * SCALE as f64;
                ctx.fill_rect(px, py, SCALE as f64, SCALE as f64);
            }
        }
    }
}

/// Draw the game world to the canvas
fn render(ctx: &CanvasRenderingContext2d, world: &GameWorld, width: u32, height: u32) {
    // Clear with sky blue
    let r = ((SKY_BLUE >> 16) & 0xFF) as u8;
    let g = ((SKY_BLUE >> 8) & 0xFF) as u8;
    let b = (SKY_BLUE & 0xFF) as u8;
    ctx.set_fill_style_str(&format!("rgb({},{},{})", r, g, b));
    ctx.fill_rect(0.0, 0.0, width as f64, height as f64);

    let tile_size = world.tile_size;

    // Draw platforms
    for platform in &world.platforms {
        let (px, py, _pw, _ph) = platform.hitbox(tile_size);

        if platform.is_ground {
            // Draw ground tiles
            for tx in 0..platform.width {
                draw_sprite(
                    ctx,
                    &GROUND,
                    px + (tx * tile_size) as f64,
                    py,
                    GROUND_COLOR,
                    false,
                );
            }
        } else {
            // Draw brick tiles
            for tx in 0..platform.width {
                draw_sprite(
                    ctx,
                    &BRICK,
                    px + (tx * tile_size) as f64,
                    py,
                    BRICK_COLOR,
                    false,
                );
            }
        }
    }

    // Draw Goombas
    for goomba in &world.goombas {
        if goomba.alive {
            draw_sprite(
                ctx,
                &GOOMBA,
                goomba.pos.x,
                goomba.pos.y,
                GOOMBA_BROWN,
                !goomba.facing_right,
            );
        } else if goomba.squish_timer > 0 {
            // Draw squished Goomba (just bottom half)
            ctx.set_fill_style_str(&format!("rgb({},{},{})",
                ((GOOMBA_BROWN >> 16) & 0xFF) as u8,
                ((GOOMBA_BROWN >> 8) & 0xFF) as u8,
                (GOOMBA_BROWN & 0xFF) as u8
            ));
            ctx.fill_rect(goomba.pos.x, goomba.pos.y + 12.0, 16.0, 4.0);
        }
    }

    // Draw Marios
    for mario in &world.marios {
        let sprite = match mario.state {
            MarioState::Standing => &MARIO_STANDING,
            MarioState::Jumping => &MARIO_JUMP,
            MarioState::Walking => {
                if mario.walk_frame == 0 { &MARIO_WALK1 } else { &MARIO_WALK2 }
            }
        };

        // Use brighter color for player-controlled Mario
        let color = if mario.is_player { HIGHLIGHT_RED } else { MARIO_RED };

        draw_sprite(
            ctx,
            sprite,
            mario.pos.x,
            mario.pos.y,
            color,
            !mario.facing_right,
        );

        // Draw a small indicator above player Mario
        if mario.is_player {
            ctx.set_fill_style_str("#ffffff");
            ctx.fill_rect(mario.pos.x + 6.0, mario.pos.y - 6.0, 4.0, 4.0);
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

    // Initialize game when canvas is ready
    let game_world_init = game_world.clone();
    let animation_frame_id_init = animation_frame_id.clone();
    let last_frame_time_init = last_frame_time.clone();
    let input_state_loop = input_state.clone();

    Effect::new(move || {
        let Some(canvas) = canvas_ref.get() else { return };
        let canvas: HtmlCanvasElement = canvas.into();

        // Set canvas size to match container
        let parent = canvas.parent_element();
        let (width, height) = if let Some(p) = parent {
            (p.client_width() as u32, p.client_height() as u32)
        } else {
            (800, 600)
        };

        canvas.set_width(width);
        canvas.set_height(height);

        // Get 2D context
        let ctx = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok());

        let Some(ctx) = ctx else { return };

        // Disable image smoothing for crisp pixels
        ctx.set_image_smoothing_enabled(false);

        // Generate world
        let tile_size = 8 * SCALE;
        let world = world::generate_world(width as i32, height as i32, tile_size);
        *game_world_init.borrow_mut() = Some(world);

        // Start game loop
        let game_world_loop = game_world_init.clone();
        let animation_frame_id_loop = animation_frame_id_init.clone();
        let last_frame_time_loop = last_frame_time_init.clone();
        let input_loop = input_state_loop.clone();

        // Set up the game loop using a self-referential closure pattern
        let game_loop_ref: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
        let game_loop_ref_inner = game_loop_ref.clone();

        let closure = Closure::new(move |timestamp: f64| {
            let mut last_time = last_frame_time_loop.borrow_mut();

            // Frame rate limiting
            let elapsed = timestamp - *last_time;
            if elapsed < FRAME_TIME {
                // Request next frame
                if let Some(window) = web_sys::window() {
                    if let Some(ref closure) = *game_loop_ref_inner.borrow() {
                        let id = window.request_animation_frame(closure.as_ref().unchecked_ref()).ok();
                        *animation_frame_id_loop.borrow_mut() = id;
                    }
                }
                return;
            }
            *last_time = timestamp;

            // Update game state
            if let Some(ref mut world) = *game_world_loop.borrow_mut() {
                // Apply player input
                let input = input_loop.borrow();
                physics::apply_player_input(world, input.left, input.right, input.jump);

                // Update AI
                ai::update_ai(world);

                // Update physics
                physics::update(world);

                // Spawn new Goombas periodically
                world.goomba_spawn_timer += 1;
                if world.goomba_spawn_timer >= GOOMBA_RESPAWN_INTERVAL
                    && world.goombas.len() < MAX_GOOMBAS
                {
                    world::spawn_random_goomba(world);
                    world.goomba_spawn_timer = 0;
                }

                // Render
                render(&ctx, world, width, height);
            }

            // Request next frame
            if let Some(window) = web_sys::window() {
                if let Some(ref closure) = *game_loop_ref_inner.borrow() {
                    let id = window.request_animation_frame(closure.as_ref().unchecked_ref()).ok();
                    *animation_frame_id_loop.borrow_mut() = id;
                }
            }
        });

        // Store the closure in our reference cell and start the loop
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

        // Leak the Rc so the closure stays alive for the lifetime of the page
        // The closure will naturally stop when the canvas context becomes invalid
        std::mem::forget(game_loop_ref);
    });

    // Handle keyboard input
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

    // Handle click to select Mario
    let game_world_click = game_world.clone();
    let on_click = move |ev: web_sys::MouseEvent| {
        let target = ev.target().and_then(|t| t.dyn_into::<HtmlCanvasElement>().ok());
        let Some(canvas) = target else { return };

        let rect = canvas.get_bounding_client_rect();
        let click_x = ev.client_x() as f64 - rect.left();
        let click_y = ev.client_y() as f64 - rect.top();

        if let Some(ref mut world) = *game_world_click.borrow_mut() {
            // Find Mario under click
            for mario in &world.marios {
                let (mx, my, mw, mh) = mario.hitbox();
                if click_x >= mx && click_x <= mx + mw
                    && click_y >= my && click_y <= my + mh
                {
                    world.set_player(mario.id);
                    break;
                }
            }
        }
    };

    // Note: Animation frame cleanup is handled automatically by browser when canvas is removed

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
