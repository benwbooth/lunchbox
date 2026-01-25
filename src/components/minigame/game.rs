//! Main Mario mini-game component with canvas rendering
//! Uses ImageData for efficient batched pixel rendering

use leptos::prelude::*;
use leptos::html;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, ImageData, KeyboardEvent};
use std::cell::RefCell;
use std::rc::Rc;

use super::sprites::*;
use super::entities::{GameWorld, MarioState, BlockType, KoopaState, CharacterType};
use super::physics;
use super::world;
use super::ai;

/// Tile size in pixels (matches 8x8 sprites)
const TILE_SIZE: i32 = 8;

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

/// Pixel buffer for efficient rendering
struct PixelBuffer {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl PixelBuffer {
    fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        Self {
            data: vec![0; size],
            width,
            height,
        }
    }

    fn clear(&mut self) {
        self.data.fill(0);
    }

    #[inline]
    fn set_pixel(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = ((y as u32 * self.width + x as u32) * 4) as usize;
        self.data[idx] = r;
        self.data[idx + 1] = g;
        self.data[idx + 2] = b;
        self.data[idx + 3] = 255;
    }

    fn to_image_data(&self) -> Result<ImageData, JsValue> {
        let clamped = wasm_bindgen::Clamped(&self.data[..]);
        ImageData::new_with_u8_clamped_array_and_sh(clamped, self.width, self.height)
    }
}

/// Draw a 4-color sprite to the pixel buffer
#[inline]
fn draw_sprite_to_buffer(
    buffer: &mut PixelBuffer,
    sprite: &Sprite,
    x: i32,
    y: i32,
    palette: &Palette,
    flip_x: bool,
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

            let px = if flip_x {
                x + (7 - col) as i32
            } else {
                x + col as i32
            };
            let py = y + row as i32;
            buffer.set_pixel(px, py, r, g, b);
        }
    }
}

/// Draw the game world to the pixel buffer and render to canvas
fn render(ctx: &CanvasRenderingContext2d, buffer: &mut PixelBuffer, world: &GameWorld, width: u32, height: u32) {
    // Clear buffer (black background, alpha = 255)
    for i in 0..buffer.data.len() / 4 {
        let idx = i * 4;
        buffer.data[idx] = 0;     // R
        buffer.data[idx + 1] = 0; // G
        buffer.data[idx + 2] = 0; // B
        buffer.data[idx + 3] = 255; // A
    }

    let tile_size = world.tile_size;

    // Draw ground platforms
    for platform in &world.platforms {
        if platform.is_ground {
            for tx in 0..platform.width {
                let px = (platform.x + tx) * tile_size;
                let py = platform.y * tile_size;
                draw_sprite_to_buffer(buffer, &GROUND, px, py, &PALETTE_GROUND, false);
            }
        }
    }

    // Draw blocks (bricks and question blocks)
    for block in &world.blocks {
        let bx = block.x * tile_size;
        let by = block.y * tile_size;
        let (sprite, palette) = match block.block_type {
            BlockType::Brick => (&BRICK, &PALETTE_BRICK),
            BlockType::Question => (&QUESTION, &PALETTE_QUESTION),
            BlockType::QuestionEmpty => (&QUESTION_EMPTY, &PALETTE_BRICK),
            BlockType::Ground => (&GROUND, &PALETTE_GROUND),
        };
        draw_sprite_to_buffer(buffer, sprite, bx, by, palette, false);
    }

    // Draw debris (no rotation in buffer mode, just position)
    for debris in &world.debris {
        if debris.alive {
            draw_sprite_to_buffer(buffer, &BRICK_DEBRIS, debris.pos.x as i32, debris.pos.y as i32, &PALETTE_BRICK, false);
        }
    }

    // Draw mushrooms
    for mushroom in &world.mushrooms {
        if mushroom.active {
            draw_sprite_to_buffer(buffer, &MUSHROOM, mushroom.pos.x as i32, mushroom.pos.y as i32, &PALETTE_MUSHROOM, false);
        }
    }

    // Draw Goombas
    for goomba in &world.goombas {
        if goomba.alive {
            draw_sprite_to_buffer(buffer, &GOOMBA, goomba.pos.x as i32, goomba.pos.y as i32, &PALETTE_GOOMBA, !goomba.facing_right);
        } else if goomba.squish_timer > 0 {
            // Draw squished Goomba as a flat rectangle
            let x = goomba.pos.x as i32;
            let y = goomba.pos.y as i32 + 6;
            let r = ((NES_BROWN >> 16) & 0xFF) as u8;
            let g = ((NES_BROWN >> 8) & 0xFF) as u8;
            let b = (NES_BROWN & 0xFF) as u8;
            for dx in 0..8 {
                for dy in 0..2 {
                    buffer.set_pixel(x + dx, y + dy, r, g, b);
                }
            }
        }
    }

    // Draw Koopas
    for koopa in &world.koopas {
        if koopa.alive {
            let kx = koopa.pos.x as i32;
            let ky = koopa.pos.y as i32;

            match koopa.state {
                KoopaState::Walking => {
                    // Koopa is taller when walking (12px), draw head above body
                    draw_sprite_to_buffer(buffer, &KOOPA_WALK, kx, ky + 4, &PALETTE_KOOPA, !koopa.facing_right);
                }
                KoopaState::Shell | KoopaState::ShellMoving => {
                    draw_sprite_to_buffer(buffer, &KOOPA_SHELL, kx, ky, &PALETTE_KOOPA, false);
                }
            }
        }
    }

    // Draw Coins
    for coin in &world.coins {
        if !coin.collected {
            draw_sprite_to_buffer(buffer, &COIN, coin.pos.x as i32, coin.pos.y as i32, &PALETTE_COIN, false);
        }
    }

    // Draw Marios (and other characters)
    for mario in &world.marios {
        // Skip if dead and off screen
        if mario.state == MarioState::Dead && mario.pos.y > height as f64 {
            continue;
        }

        // Blink when invincible
        if mario.invincible_timer > 0 && (mario.invincible_timer / 4) % 2 == 0 {
            continue;
        }

        // Get palette based on character type and player status
        let palette = if mario.is_player {
            match mario.character_type {
                CharacterType::Mario => &PALETTE_PLAYER,
                CharacterType::Luigi => &PALETTE_PLAYER_LUIGI,
                CharacterType::Toad => &PALETTE_PLAYER_TOAD,
                CharacterType::Princess => &PALETTE_PLAYER_PRINCESS,
            }
        } else {
            match mario.character_type {
                CharacterType::Mario => &PALETTE_MARIO,
                CharacterType::Luigi => &PALETTE_LUIGI,
                CharacterType::Toad => &PALETTE_TOAD,
                CharacterType::Princess => &PALETTE_PRINCESS,
            }
        };

        let mx = mario.pos.x as i32;
        let my = mario.pos.y as i32;

        if mario.state == MarioState::Dead {
            draw_sprite_to_buffer(buffer, &MARIO_DEAD, mx, my, palette, false);
        } else if mario.is_big {
            // Draw big character (2 sprites stacked)
            let (top, bot) = match mario.character_type {
                CharacterType::Mario | CharacterType::Luigi => {
                    match mario.state {
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
                    }
                }
                CharacterType::Toad => (&TOAD_BIG_STAND_TOP, &TOAD_BIG_STAND_BOT),
                CharacterType::Princess => (&PRINCESS_BIG_STAND_TOP, &PRINCESS_BIG_STAND_BOT),
            };
            draw_sprite_to_buffer(buffer, top, mx, my, palette, !mario.facing_right);
            draw_sprite_to_buffer(buffer, bot, mx, my + 8, palette, !mario.facing_right);
        } else {
            // Draw small character
            let sprite = match mario.character_type {
                CharacterType::Mario | CharacterType::Luigi => {
                    match mario.state {
                        MarioState::Standing => &MARIO_STAND,
                        MarioState::Walking => {
                            if mario.walk_frame == 0 { &MARIO_WALK1 } else { &MARIO_WALK2 }
                        }
                        MarioState::Jumping => &MARIO_JUMP,
                        MarioState::Dead => &MARIO_DEAD,
                    }
                }
                CharacterType::Toad => {
                    match mario.state {
                        MarioState::Standing => &TOAD_STAND,
                        MarioState::Walking => &TOAD_WALK1,
                        MarioState::Jumping => &TOAD_JUMP,
                        MarioState::Dead => &MARIO_DEAD,
                    }
                }
                CharacterType::Princess => {
                    match mario.state {
                        MarioState::Standing => &PRINCESS_STAND,
                        MarioState::Walking => &PRINCESS_WALK1,
                        MarioState::Jumping => &PRINCESS_JUMP,
                        MarioState::Dead => &MARIO_DEAD,
                    }
                }
            };
            draw_sprite_to_buffer(buffer, sprite, mx, my, palette, !mario.facing_right);
        }

        // Draw player indicator
        if mario.is_player && mario.alive {
            for dx in 0..2 {
                for dy in 0..2 {
                    buffer.set_pixel(mx + 3 + dx, my - 4 + dy, 255, 255, 255);
                }
            }
        }
    }

    // Put the image data to canvas
    if let Ok(image_data) = buffer.to_image_data() {
        ctx.put_image_data(&image_data, 0.0, 0.0).ok();
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
        let canvas: HtmlCanvasElement = canvas.clone().into();

        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };

        // Clone references for the delayed initialization closure
        let canvas_clone = canvas.clone();
        let game_world_init_clone = game_world_init.clone();
        let animation_frame_id_init_clone = animation_frame_id_init.clone();
        let last_frame_time_init_clone = last_frame_time_init.clone();
        let input_state_loop_clone = input_state_loop.clone();
        let initialized_inner = initialized_clone.clone();

        // Use requestAnimationFrame to ensure layout is computed
        let init_closure = Closure::once(move || {
            if *initialized_inner.borrow() {
                return;
            }

            let window = match web_sys::window() {
                Some(w) => w,
                None => return,
            };

            // Use window dimensions directly - most reliable approach
            // Subtract sidebar width (~240px) and toolbar height (~52px)
            let window_width = window.inner_width().ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1200.0);
            let window_height = window.inner_height().ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(800.0);

            // Calculate available space (window minus sidebar and toolbar)
            let sidebar_width = 240.0;
            let toolbar_height = 52.0;

            let width = ((window_width - sidebar_width).max(400.0)) as u32;
            let height = ((window_height - toolbar_height).max(300.0)) as u32;

            canvas_clone.set_width(width);
            canvas_clone.set_height(height);

            let ctx = canvas_clone
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok());

            let Some(ctx) = ctx else { return };

            ctx.set_image_smoothing_enabled(false);

            *initialized_inner.borrow_mut() = true;

            // Generate world with 8-pixel tiles (matches sprite size)
            let world = world::generate_world(width as i32, height as i32, TILE_SIZE);
            *game_world_init_clone.borrow_mut() = Some(world);

            // Create pixel buffer for efficient rendering
            let pixel_buffer: Rc<RefCell<PixelBuffer>> = Rc::new(RefCell::new(PixelBuffer::new(width, height)));

            // Start game loop
            let game_world_loop = game_world_init_clone.clone();
            let animation_frame_id_loop = animation_frame_id_init_clone.clone();
            let animation_frame_id_outer = animation_frame_id_init_clone.clone();
            let last_frame_time_loop = last_frame_time_init_clone.clone();
            let input_loop = input_state_loop_clone.clone();
            let buffer_loop = pixel_buffer.clone();

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
                    let input = input_loop.borrow();
                    physics::apply_player_input(world, input.left, input.right, input.jump);
                    ai::update_ai(world);
                    physics::update(world);

                    world.spawn_timer += 1;
                    if world.spawn_timer >= GOOMBA_RESPAWN_INTERVAL && world.goombas.len() < MAX_GOOMBAS {
                        world::spawn_random_goomba(world);
                        world.spawn_timer = 0;
                    }

                    render(&ctx, &mut buffer_loop.borrow_mut(), world, width, height);
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
                *animation_frame_id_outer.borrow_mut() = id;
            }

            std::mem::forget(game_loop_ref);
        });

        // Schedule initialization after layout
        window.request_animation_frame(init_closure.as_ref().unchecked_ref()).ok();
        init_closure.forget();
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
