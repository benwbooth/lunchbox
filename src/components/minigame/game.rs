//! WebGPU-powered Mario mini-game using raw web-sys APIs
//! All game logic runs in WGSL shaders - Rust only bootstraps WebGPU

use leptos::prelude::*;
use leptos::html;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, KeyboardEvent,
    GpuDevice, GpuQueue, GpuCanvasContext,
    GpuBuffer, GpuBindGroup, GpuComputePipeline, GpuRenderPipeline,
};
use std::cell::RefCell;
use std::rc::Rc;
use js_sys::{Object, Reflect, Uint8Array, Function, Promise};

use super::gpu::{pack_sprite_atlas, pack_palettes, Uniforms, u32_slice_to_bytes};

const COMPUTE_SHADER: &str = include_str!("shaders/compute.wgsl");
const RENDER_SHADER: &str = include_str!("shaders/render.wgsl");
// Fixed constants
const WORKGROUP_SIZE: u32 = 64;
const TILE_PX: u32 = 8;
const ENTITY_COUNT: u32 = 256;
const ECELL_PX: u32 = 64;  // Entity grid cell size in pixels
const EGRID_SLOTS: u32 = 16;  // Max entities per cell
const MAX_WORLD_WIDTH: u32 = 4096;
const MAX_WORLD_HEIGHT: u32 = 4096;
const MIN_ZOOM: f64 = 1.0;
const MAX_ZOOM: f64 = 5.0;

/// Calculate dynamic grid dimensions from screen size
fn calculate_grid_dimensions(width: u32, height: u32) -> (u32, u32, u32, u32, u32, u32, u32, u32) {
    // Spatial grid for block collision (8px tiles)
    let grid_width = (width / TILE_PX) + 1;
    let grid_height = (height / TILE_PX) + 1;
    let grid_size = grid_width * grid_height;

    // Entity grid for entity-entity collision (64px cells)
    let egrid_width = (width / ECELL_PX) + 1;
    let egrid_height = (height / ECELL_PX) + 1;
    let egrid_cells = egrid_width * egrid_height;
    let egrid_size = egrid_cells * EGRID_SLOTS;

    // Block count: enough for ground + dense platforms
    // Ground takes 1 row, platforms fill most of screen
    let tiles_x = width / TILE_PX;
    let tiles_y = height / TILE_PX;
    let block_count = tiles_x * tiles_y;  // Max 1 block per tile

    (grid_width, grid_height, grid_size, egrid_width, egrid_height, egrid_cells, egrid_size, block_count)
}

fn canvas_viewport_size(canvas_ref: &NodeRef<html::Canvas>) -> Option<(f64, f64)> {
    let canvas = canvas_ref.get()?;
    let canvas: HtmlCanvasElement = canvas.into();

    let w = if canvas.client_width() > 0 {
        canvas.client_width() as f64
    } else {
        canvas.width() as f64
    };
    let h = if canvas.client_height() > 0 {
        canvas.client_height() as f64
    } else {
        canvas.height() as f64
    };

    if w > 0.0 && h > 0.0 {
        Some((w, h))
    } else {
        None
    }
}

fn clamp_pan_to_zoom(pan: (f64, f64), zoom: f64, viewport: (f64, f64)) -> (f64, f64) {
    let (vw, vh) = viewport;
    let max_pan_x = ((zoom - 1.0) * vw * 0.5).max(0.0);
    let max_pan_y = ((zoom - 1.0) * vh * 0.5).max(0.0);
    (
        pan.0.clamp(-max_pan_x, max_pan_x),
        pan.1.clamp(-max_pan_y, max_pan_y),
    )
}

/// The Mario Mini-Game component (WebGPU)
#[component]
pub fn MarioMinigame(
    zoom_level: ReadSignal<f64>,
    set_zoom_level: WriteSignal<f64>,
) -> impl IntoView {
    let canvas_ref = NodeRef::<html::Canvas>::new();
    let initialized = Rc::new(RefCell::new(false));
    let gpu_state: Rc<RefCell<Option<GpuState>>> = Rc::new(RefCell::new(None));
    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    let (fps, set_fps) = signal(0u32);

    // Pan offset state for drag-to-pan
    let (pan_offset, set_pan_offset) = signal((0.0f64, 0.0f64));
    let (is_panning, set_is_panning) = signal(false);
    let (last_mouse_pos, set_last_mouse_pos) = signal((0i32, 0i32));

    // Pinch-to-zoom state
    let (last_pinch_distance, set_last_pinch_distance) = signal::<Option<f64>>(None);

    // Keep zoom/pan in bounds even when zoom comes from shared external state.
    {
        let canvas_ref = canvas_ref.clone();
        let set_zoom_level = set_zoom_level;
        let set_pan_offset = set_pan_offset;
        Effect::new(move || {
            let zoom = zoom_level.get();
            let pan = pan_offset.get();

            let clamped_zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
            let clamped_pan = if let Some(viewport) = canvas_viewport_size(&canvas_ref) {
                clamp_pan_to_zoom(pan, clamped_zoom, viewport)
            } else {
                pan
            };

            if (clamped_zoom - zoom).abs() > 0.0001 {
                set_zoom_level.set(clamped_zoom);
            }
            if (clamped_pan.0 - pan.0).abs() > 0.01 || (clamped_pan.1 - pan.1).abs() > 0.01 {
                set_pan_offset.set(clamped_pan);
            }
        });
    }

    let initialized_clone = initialized.clone();
    let gpu_state_init = gpu_state.clone();

    Effect::new(move || {
        if *initialized_clone.borrow() {
            return;
        }

        let Some(canvas) = canvas_ref.get() else { return };
        let canvas: HtmlCanvasElement = canvas.clone().into();

        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };

        let initialized_inner = initialized_clone.clone();
        let gpu_state_inner = gpu_state_init.clone();
        let canvas_clone = canvas.clone();
        let set_error = set_error_msg.clone();

        // Use requestAnimationFrame to ensure layout is computed
        let init_closure = Closure::once(move || {
            if *initialized_inner.borrow() {
                return;
            }
            *initialized_inner.borrow_mut() = true;

            // Spawn async initialization
            wasm_bindgen_futures::spawn_local(async move {
                match GpuState::new(&canvas_clone).await {
                    Ok(state) => {
                        *gpu_state_inner.borrow_mut() = Some(state);
                        start_game_loop(gpu_state_inner, move |fps| set_fps.set(fps));
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("WebGPU init failed: {}", e).into());
                        set_error.set(Some(e));
                    }
                }
            });
        });

        window.request_animation_frame(init_closure.as_ref().unchecked_ref()).ok();
        init_closure.forget();
    });

    // Keyboard input handlers
    let gpu_state_keydown = gpu_state.clone();
    let gpu_state_keyup = gpu_state.clone();

    let on_keydown = move |ev: KeyboardEvent| {
        if let Some(ref mut state) = *gpu_state_keydown.borrow_mut() {
            let key = ev.key();
            match key.as_str() {
                "ArrowLeft" | "a" | "A" => state.input_left = true,
                "ArrowRight" | "d" | "D" => state.input_right = true,
                "ArrowUp" | "w" | "W" | " " => state.input_jump = true,
                _ => return,
            }
            ev.prevent_default();
        }
    };

    let on_keyup = move |ev: KeyboardEvent| {
        if let Some(ref mut state) = *gpu_state_keyup.borrow_mut() {
            let key = ev.key();
            match key.as_str() {
                "ArrowLeft" | "a" | "A" => state.input_left = false,
                "ArrowRight" | "d" | "D" => state.input_right = false,
                "ArrowUp" | "w" | "W" | " " => state.input_jump = false,
                _ => {}
            }
        }
    };

    // Left-click drag-to-pan handlers (or middle-click)
    let on_mousedown = move |ev: web_sys::MouseEvent| {
        // Left button (0) or middle button (1)
        if ev.button() == 0 || ev.button() == 1 {
            set_is_panning.set(true);
            set_last_mouse_pos.set((ev.client_x(), ev.client_y()));
            ev.prevent_default();
        }
    };

    let on_mousemove = {
        let canvas_ref = canvas_ref.clone();
        move |ev: web_sys::MouseEvent| {
        if is_panning.get() {
            let (lx, ly) = last_mouse_pos.get();
            let dx = ev.client_x() - lx;
            let dy = ev.client_y() - ly;
            let (px, py) = pan_offset.get();
            let next_pan = (px + dx as f64, py + dy as f64);
            let zoom = zoom_level.get().clamp(MIN_ZOOM, MAX_ZOOM);
            let clamped_pan = if let Some(viewport) = canvas_viewport_size(&canvas_ref) {
                clamp_pan_to_zoom(next_pan, zoom, viewport)
            } else {
                next_pan
            };
            set_pan_offset.set(clamped_pan);
            set_last_mouse_pos.set((ev.client_x(), ev.client_y()));
        }
    }
    };

    let on_mouseup = move |_: web_sys::MouseEvent| {
        set_is_panning.set(false);
    };

    // Touch handlers for pinch-to-zoom
    let on_touchstart = move |ev: web_sys::TouchEvent| {
        let touches = ev.touches();
        if touches.length() == 2 {
            if let (Some(t1), Some(t2)) = (touches.get(0), touches.get(1)) {
                let dx = (t2.client_x() - t1.client_x()) as f64;
                let dy = (t2.client_y() - t1.client_y()) as f64;
                let dist = (dx * dx + dy * dy).sqrt();
                set_last_pinch_distance.set(Some(dist));
                ev.stop_propagation();
                ev.prevent_default();
            }
        }
    };

    let on_touchmove = {
        let canvas_ref = canvas_ref.clone();
        move |ev: web_sys::TouchEvent| {
        let touches = ev.touches();
        if touches.length() == 2 {
            if let Some(last_dist) = last_pinch_distance.get() {
                if let (Some(t1), Some(t2)) = (touches.get(0), touches.get(1)) {
                    let dx = (t2.client_x() - t1.client_x()) as f64;
                    let dy = (t2.client_y() - t1.client_y()) as f64;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if last_dist > 0.0 {
                        // Make pinch noticeably more responsive.
                        let raw_scale = dist / last_dist;
                        let scale = (1.0 + (raw_scale - 1.0) * 3.5).clamp(0.55, 1.45);
                        let target_zoom = zoom_level.get() * scale;
                        let new_zoom = target_zoom.clamp(MIN_ZOOM, MAX_ZOOM);
                        let pan = pan_offset.get();
                        let clamped_pan = if let Some(viewport) = canvas_viewport_size(&canvas_ref) {
                            clamp_pan_to_zoom(pan, new_zoom, viewport)
                        } else {
                            pan
                        };
                        set_zoom_level.set(new_zoom);
                        set_pan_offset.set(clamped_pan);
                    }
                    set_last_pinch_distance.set(Some(dist));
                    ev.stop_propagation();
                    ev.prevent_default();
                }
            }
        }
    }
    };

    let on_touchend = move |ev: web_sys::TouchEvent| {
        if ev.touches().length() < 2 {
            set_last_pinch_distance.set(None);
            ev.stop_propagation();
        }
    };

    // Mouse wheel zoom
    let on_wheel = {
        let canvas_ref = canvas_ref.clone();
        move |ev: web_sys::WheelEvent| {
        let delta = ev.delta_y();
        // Trackpad pinch often arrives as ctrl+wheel with small deltas.
        // Keep normal wheel comfortable, but make pinch much faster.
        let sensitivity = if ev.ctrl_key() { 0.0075 } else { 0.0015 };
        let zoom_factor = (-delta * sensitivity).exp();
        let target_zoom = zoom_level.get() * zoom_factor;
        let new_zoom = target_zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let pan = pan_offset.get();
        let clamped_pan = if let Some(viewport) = canvas_viewport_size(&canvas_ref) {
            clamp_pan_to_zoom(pan, new_zoom, viewport)
        } else {
            pan
        };
        set_zoom_level.set(new_zoom);
        set_pan_offset.set(clamped_pan);
        ev.stop_propagation();
        ev.prevent_default();
    }
    };

    view! {
        <div
            class="minigame-container"
            tabindex="0"
            on:keydown=on_keydown
            on:keyup=on_keyup
            on:mousedown=on_mousedown
            on:mousemove=on_mousemove
            on:mouseup=on_mouseup
            on:mouseleave=move |_| set_is_panning.set(false)
            on:touchstart=on_touchstart
            on:touchmove=on_touchmove
            on:touchend=on_touchend
            on:touchcancel=move |_| set_last_pinch_distance.set(None)
            on:wheel=on_wheel
            style="touch-action: none;"
        >
            <div
                class="minigame-zoom-wrapper"
                style:transform=move || {
                    let z = zoom_level.get().clamp(MIN_ZOOM, MAX_ZOOM);
                    let (px, py) = pan_offset.get();
                    format!("translate({}px, {}px) scale({})", px, py, z)
                }
                style:transform-origin="center center"
                style="width: 100%; height: 100%;"
            >
                <canvas
                    class="minigame-canvas"
                    node_ref=canvas_ref
                />
                <div class="minigame-overlay">
                    {move || {
                        if let Some(err) = error_msg.get() {
                            view! {
                                <p style="color: #ff6b6b">"WebGPU not available"</p>
                                <p style="font-size: 12px; opacity: 0.7">{err}</p>
                            }.into_any()
                        } else {
                            view! {
                                <p>"Select a platform to view games."</p>
                            }.into_any()
                        }
                    }}
                </div>
                <div class="minigame-fps" style="position: absolute; top: 8px; right: 8px; color: #0f0; font-family: monospace; font-size: 14px; text-shadow: 1px 1px 0 #000;">
                    {move || format!("FPS: {}", fps.get())}
                </div>
            </div>
        </div>
    }
}

fn compute_canvas_size(window: &web_sys::Window) -> (u32, u32) {
    let window_width = window.inner_width().ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1200.0);
    let window_height = window.inner_height().ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);

    let sidebar_width = 240.0;
    let toolbar_height = 52.0;

    let max_width = MAX_WORLD_WIDTH as f64;
    let max_height = MAX_WORLD_HEIGHT as f64;

    let mut width = ((window_width - sidebar_width).max(400.0))
        .min(max_width) as u32;
    let mut height = ((window_height - toolbar_height).max(300.0))
        .min(max_height) as u32;

    // Align to tile size to keep the grid stable.
    width = width.saturating_sub(width % TILE_PX).max(TILE_PX);
    height = height.saturating_sub(height % TILE_PX).max(TILE_PX);

    (width, height)
}

async fn log_adapter_info(adapter: &web_sys::GpuAdapter, device: &GpuDevice) {
    if let Some(info) = request_adapter_info(adapter).await {
        web_sys::console::log_2(&"WebGPU adapter info".into(), &info);
    }

    // Log adapter/device limits if present
    let adapter_val: JsValue = adapter.clone().into();
    if let Ok(limits) = Reflect::get(&adapter_val, &"limits".into()) {
        if !limits.is_undefined() && !limits.is_null() {
            web_sys::console::log_2(&"WebGPU adapter.limits".into(), &limits);
        }
    }
    let device_val: JsValue = device.clone().into();
    if let Ok(limits) = Reflect::get(&device_val, &"limits".into()) {
        if !limits.is_undefined() && !limits.is_null() {
            web_sys::console::log_2(&"WebGPU device.limits".into(), &limits);
        }
    }
}

async fn request_adapter_info(adapter: &web_sys::GpuAdapter) -> Option<JsValue> {
    let adapter_val: JsValue = adapter.clone().into();

    // Try requestAdapterInfo() if available
    if let Ok(func_val) = Reflect::get(&adapter_val, &"requestAdapterInfo".into()) {
        if func_val.is_function() {
            let func: Function = func_val.unchecked_into();
            if let Ok(promise_val) = func.call0(&adapter_val) {
                if let Ok(promise) = promise_val.dyn_into::<Promise>() {
                    if let Ok(info) = wasm_bindgen_futures::JsFuture::from(promise).await {
                        return Some(info);
                    }
                }
            }
        }
    }

    // Fallback: adapter.info (not universally supported yet)
    if let Ok(info) = Reflect::get(&adapter_val, &"info".into()) {
        if !info.is_undefined() && !info.is_null() {
            return Some(info);
        }
    }

    None
}

fn js_string_field(obj: &JsValue, key: &str) -> Option<String> {
    Reflect::get(obj, &key.into()).ok()?.as_string()
}

fn js_bool_field(obj: &JsValue, key: &str) -> Option<bool> {
    Reflect::get(obj, &key.into()).ok()?.as_bool()
}

async fn detect_software_adapter(adapter: &web_sys::GpuAdapter) -> Option<String> {
    let adapter_val: JsValue = adapter.clone().into();
    let mut is_fallback = js_bool_field(&adapter_val, "isFallbackAdapter").unwrap_or(false);
    let mut reason_bits: Vec<String> = Vec::new();

    if let Some(info) = request_adapter_info(adapter).await {
        if let Some(fallback) = js_bool_field(&info, "isFallbackAdapter") {
            is_fallback = is_fallback || fallback;
            if fallback {
                reason_bits.push("isFallbackAdapter=true".to_string());
            }
        }

        let arch = js_string_field(&info, "architecture").unwrap_or_default();
        let desc = js_string_field(&info, "description").unwrap_or_default();
        let vendor = js_string_field(&info, "vendor").unwrap_or_default();
        let dev = js_string_field(&info, "device").unwrap_or_default();
        let typ = js_string_field(&info, "type").unwrap_or_default();

        let arch_l = arch.to_lowercase();
        let desc_l = desc.to_lowercase();
        let typ_l = typ.to_lowercase();

        let is_swiftshader = arch_l.contains("swiftshader") || desc_l.contains("swiftshader") || typ_l == "cpu";
        if is_swiftshader {
            reason_bits.push("swiftshader/cpu adapter".to_string());
        }

        if is_fallback || is_swiftshader {
            let mut details = Vec::new();
            if !vendor.is_empty() { details.push(format!("vendor={}", vendor)); }
            if !dev.is_empty() { details.push(format!("device={}", dev)); }
            if !arch.is_empty() { details.push(format!("arch={}", arch)); }
            if !desc.is_empty() { details.push(format!("desc={}", desc)); }
            if !typ.is_empty() { details.push(format!("type={}", typ)); }
            if !details.is_empty() {
                reason_bits.push(details.join(", "));
            }
        }
    }

    if is_fallback {
        if reason_bits.is_empty() {
            return Some("fallback adapter detected".to_string());
        }
        return Some(reason_bits.join("; "));
    }

    None
}

/// WebGPU state using raw web-sys types
struct GpuState {
    device: GpuDevice,
    queue: GpuQueue,
    context: GpuCanvasContext,
    preferred_format: web_sys::GpuTextureFormat,
    // Five compute pipelines: clear + populate (once) and frame_prep + update_positions + resolve (every frame)
    clear_pipeline: GpuComputePipeline,
    populate_pipeline: GpuComputePipeline,
    frame_prep_pipeline: GpuComputePipeline,
    update_positions_pipeline: GpuComputePipeline,
    resolve_collisions_pipeline: GpuComputePipeline,
    // Two render pipelines for multi-pass rendering
    block_pipeline: GpuRenderPipeline,
    entity_pipeline: GpuRenderPipeline,
    uniform_buffer: GpuBuffer,
    entity_buffer_a: GpuBuffer,
    entity_buffer_b: GpuBuffer,
    entity_counts_buffer: GpuBuffer,
    block_buffer: GpuBuffer,
    compute_bind_group_a_to_b: GpuBindGroup,
    compute_bind_group_b_to_a: GpuBindGroup,
    render_bind_group: GpuBindGroup,
    uniforms: Uniforms,
    uniform_bytes: [u8; 64],
    frame: u32,
    start_time: f64,
    width: u32,
    height: u32,
    // Track whether we need to run init (on startup and resize)
    needs_init: bool,
    input_left: bool,
    input_right: bool,
    input_jump: bool,
    // Reference to canvas for resize detection
    canvas: HtmlCanvasElement,
    // Dynamic grid dimensions calculated from screen size
    grid_width: u32,
    grid_height: u32,
    grid_size: u32,
    egrid_width: u32,
    egrid_height: u32,
    egrid_cells: u32,
    egrid_size: u32,
    block_count: u32,
}

impl GpuState {
    async fn new(canvas: &HtmlCanvasElement) -> Result<Self, String> {
        let window = web_sys::window().ok_or("No window")?;
        let navigator = window.navigator();

        // Check for WebGPU support
        let gpu = navigator.gpu();

        // Set canvas size to match container (capped for perf, aligned to tiles)
        let (width, height) = compute_canvas_size(&window);

        canvas.set_width(width);
        canvas.set_height(height);

        // Request adapter
        let adapter_options = web_sys::GpuRequestAdapterOptions::new();
        adapter_options.set_power_preference(web_sys::GpuPowerPreference::HighPerformance);

        let adapter_promise = gpu.request_adapter_with_options(&adapter_options);
        let adapter = wasm_bindgen_futures::JsFuture::from(adapter_promise)
            .await
            .map_err(|e| format!("Failed to get adapter: {:?}", e))?;

        let adapter: web_sys::GpuAdapter = adapter.dyn_into()
            .map_err(|_| "Failed to cast adapter")?;

        if let Some(reason) = detect_software_adapter(&adapter).await {
            return Err(format!(
                "WebGPU is using a software adapter. {}. Enable hardware WebGPU (Vulkan) or disable software rasterizer.",
                reason
            ));
        }

        // Request device
        let device_promise = adapter.request_device();
        let device = wasm_bindgen_futures::JsFuture::from(device_promise)
            .await
            .map_err(|e| format!("Failed to get device: {:?}", e))?;

        let device: GpuDevice = device.dyn_into()
            .map_err(|_| "Failed to cast device")?;

        let queue = device.queue();

        log_adapter_info(&adapter, &device).await;

        // Get canvas context
        let context = canvas.get_context("webgpu")
            .map_err(|e| format!("Failed to get context: {:?}", e))?
            .ok_or("No WebGPU context")?
            .dyn_into::<GpuCanvasContext>()
            .map_err(|_| "Failed to cast context")?;

        // Configure canvas with preferred format
        let preferred_format = gpu.get_preferred_canvas_format();
        let canvas_config = web_sys::GpuCanvasConfiguration::new(&device, preferred_format);
        let _ = context.configure(&canvas_config);

        // Create shader modules
        let compute_shader_desc = web_sys::GpuShaderModuleDescriptor::new(COMPUTE_SHADER);
        let compute_shader = device.create_shader_module(&compute_shader_desc);

        let render_shader_desc = web_sys::GpuShaderModuleDescriptor::new(RENDER_SHADER);
        let render_shader = device.create_shader_module(&render_shader_desc);

        // Calculate dynamic grid dimensions based on screen size
        let (grid_width, grid_height, grid_size, egrid_width, egrid_height, egrid_cells, egrid_size, block_count) =
            calculate_grid_dimensions(width, height);

        // Create buffers with sizes based on screen dimensions
        let uniform_buffer = create_buffer(&device, 64, gpu_buffer_usage_uniform() | gpu_buffer_usage_copy_dst());
        let entity_buffer_a = create_buffer(&device, ENTITY_COUNT * 32, gpu_buffer_usage_storage() | gpu_buffer_usage_copy_dst());
        let entity_buffer_b = create_buffer(&device, ENTITY_COUNT * 32, gpu_buffer_usage_storage() | gpu_buffer_usage_copy_dst());
        let block_buffer = create_buffer(&device, block_count * 16, gpu_buffer_usage_storage() | gpu_buffer_usage_copy_dst());
        // Spatial grid for block collision
        let spatial_grid_buffer = create_buffer(&device, grid_size * 4, gpu_buffer_usage_storage() | gpu_buffer_usage_copy_dst());
        // Entity grid for entity-entity collision
        let entity_grid_buffer = create_buffer(&device, egrid_size * 4, gpu_buffer_usage_storage() | gpu_buffer_usage_copy_dst());
        let entity_counts_buffer = create_buffer(&device, egrid_cells * 4, gpu_buffer_usage_storage() | gpu_buffer_usage_copy_dst());

        // Create and upload sprite/palette buffers
        let sprite_data = pack_sprite_atlas();
        let sprite_buffer = create_buffer_with_data(&device, &u32_slice_to_bytes(&sprite_data), gpu_buffer_usage_storage());

        let palette_data = pack_palettes();
        let palette_buffer = create_buffer_with_data(&device, &u32_slice_to_bytes(&palette_data), gpu_buffer_usage_storage());

        // Create bind group layouts
        let compute_bgl = create_compute_bind_group_layout(&device);
        let render_bgl = create_render_bind_group_layout(&device);

        // Create bind groups
        let compute_bind_group_a_to_b = create_compute_bind_group(
            &device, &compute_bgl,
            &uniform_buffer,
            &entity_buffer_a, &entity_buffer_b,
            &block_buffer,
            &spatial_grid_buffer, &entity_grid_buffer, &entity_counts_buffer,
        );
        let compute_bind_group_b_to_a = create_compute_bind_group(
            &device, &compute_bgl,
            &uniform_buffer,
            &entity_buffer_b, &entity_buffer_a,
            &block_buffer,
            &spatial_grid_buffer, &entity_grid_buffer, &entity_counts_buffer,
        );
        let render_bind_group = create_render_bind_group(
            &device, &render_bgl,
            &uniform_buffer, &entity_buffer_a, &block_buffer,
            &sprite_buffer, &palette_buffer,
        );

        // Create pipeline layouts
        let compute_pipeline_layout = create_pipeline_layout(&device, &compute_bgl);
        let render_pipeline_layout = create_pipeline_layout(&device, &render_bgl);

        // Create four compute pipelines
        let clear_pipeline = create_compute_pipeline(&device, &compute_shader, &compute_pipeline_layout, "init_clear");
        let populate_pipeline = create_compute_pipeline(&device, &compute_shader, &compute_pipeline_layout, "init_populate");
        let frame_prep_pipeline = create_compute_pipeline(&device, &compute_shader, &compute_pipeline_layout, "frame_prep");
        let update_positions_pipeline = create_compute_pipeline(&device, &compute_shader, &compute_pipeline_layout, "update_positions");
        let resolve_collisions_pipeline = create_compute_pipeline(&device, &compute_shader, &compute_pipeline_layout, "resolve_collisions");

        // Create two render pipelines for multi-pass rendering
        let block_pipeline = create_render_pipeline(
            &device, &render_shader, &render_pipeline_layout, preferred_format,
            "vs_block", "fs_block"
        );
        let entity_pipeline = create_render_pipeline(
            &device, &render_shader, &render_pipeline_layout, preferred_format,
            "vs_entity", "fs_entity"
        );

        let start_time = js_sys::Date::now();

        let uniforms = Uniforms {
            resolution: [width as f32, height as f32],
            grid_width,
            grid_height,
            grid_size,
            egrid_width,
            egrid_height,
            egrid_cells,
            egrid_size,
            block_count,
            ..Default::default()
        };

        Ok(Self {
            device,
            queue,
            context,
            preferred_format,
            clear_pipeline,
            populate_pipeline,
            frame_prep_pipeline,
            update_positions_pipeline,
            resolve_collisions_pipeline,
            block_pipeline,
            entity_pipeline,
            uniform_buffer,
            entity_buffer_a,
            entity_buffer_b,
            entity_counts_buffer,
            block_buffer,
            compute_bind_group_a_to_b,
            compute_bind_group_b_to_a,
            render_bind_group,
            uniforms,
            uniform_bytes: [0u8; 64],
            frame: 0,
            start_time,
            width,
            height,
            needs_init: true,  // Run init on first frame
            input_left: false,
            input_right: false,
            input_jump: false,
            canvas: canvas.clone(),
            // Dynamic grid dimensions
            grid_width,
            grid_height,
            grid_size,
            egrid_width,
            egrid_height,
            egrid_cells,
            egrid_size,
            block_count,
        })
    }

    fn update(&mut self) {
        // Check for resize - recompute desired canvas size
        // Note: For now we just reinit with the same buffer sizes. Full buffer recreation
        // on resize would require more complex state management.
        if let Some(window) = web_sys::window() {
            let (desired_width, desired_height) = compute_canvas_size(&window);
            if desired_width != self.width || desired_height != self.height {
                // Canvas resized - trigger full reinit
                self.canvas.set_width(desired_width);
                self.canvas.set_height(desired_height);
                let canvas_config = web_sys::GpuCanvasConfiguration::new(&self.device, self.preferred_format);
                let _ = self.context.configure(&canvas_config);

                self.width = desired_width;
                self.height = desired_height;

                // Recalculate grid dimensions for the new size
                let (gw, gh, gs, ew, eh, ec, es, bc) = calculate_grid_dimensions(desired_width, desired_height);
                self.grid_width = gw;
                self.grid_height = gh;
                self.grid_size = gs;
                self.egrid_width = ew;
                self.egrid_height = eh;
                self.egrid_cells = ec;
                self.egrid_size = es;
                self.block_count = bc;

                // Update uniforms with new dimensions
                self.uniforms.resolution = [desired_width as f32, desired_height as f32];
                self.uniforms.grid_width = gw;
                self.uniforms.grid_height = gh;
                self.uniforms.grid_size = gs;
                self.uniforms.egrid_width = ew;
                self.uniforms.egrid_height = eh;
                self.uniforms.egrid_cells = ec;
                self.uniforms.egrid_size = es;
                self.uniforms.block_count = bc;

                self.needs_init = true;
                self.frame = 0;
                self.start_time = js_sys::Date::now();
            }
        }

        // Update uniforms
        let now = js_sys::Date::now();
        let time = ((now - self.start_time) / 1000.0) as f32;
        self.uniforms.time = time;
        self.uniforms.delta_time = 1.0 / 60.0;
        self.uniforms.frame = self.frame;

        // Encode input state
        let mut input_bits = 0u32;
        if self.input_left { input_bits |= 1; }
        if self.input_right { input_bits |= 2; }
        if self.input_jump { input_bits |= 4; }
        self.uniforms.mouse_click = input_bits;

        // Write uniforms to buffer
        self.uniforms.write_bytes(&mut self.uniform_bytes);
        let uniform_view = unsafe { Uint8Array::view(&self.uniform_bytes) };
        let _ = self.queue.write_buffer_with_u32_and_buffer_source(&self.uniform_buffer, 0, &uniform_view);

        // Get current texture
        let texture = self.context.get_current_texture().expect("get current texture");
        let view = texture.create_view().expect("create texture view");

        // Create command encoder
        let encoder = self.device.create_command_encoder();

        let entity_workgroups = (ENTITY_COUNT + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let frame_prep_threads = self.block_count.max(self.egrid_cells);
        let frame_prep_workgroups = (frame_prep_threads + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

        // Compute pass - separate init vs update
        if self.needs_init {
            // INIT PHASE: Two separate passes to ensure proper synchronization
            // Pass 1: Clear all grids
            {
                let compute_pass = encoder.begin_compute_pass();
                compute_pass.set_pipeline(&self.clear_pipeline);
                compute_pass.set_bind_group(0, Some(&self.compute_bind_group_b_to_a));
                compute_pass.dispatch_workgroups(512); // 32768 threads for all blocks
                compute_pass.end();
            }
            // Pass 2: Populate blocks and entities (grids are now guaranteed clear)
            {
                let compute_pass = encoder.begin_compute_pass();
                compute_pass.set_pipeline(&self.populate_pipeline);
                compute_pass.set_bind_group(0, Some(&self.compute_bind_group_b_to_a));
                compute_pass.dispatch_workgroups(512); // 32768 threads for all blocks
                compute_pass.end();
            }
            self.needs_init = false;
        } else {
            // UPDATE PHASE: Three passes for optimal parallelism
            // Pass 1: Frame prep - clear entity counts + block destruction
            {
                let compute_pass = encoder.begin_compute_pass();
                compute_pass.set_pipeline(&self.frame_prep_pipeline);
                compute_pass.set_bind_group(0, Some(&self.compute_bind_group_a_to_b));
                compute_pass.dispatch_workgroups(frame_prep_workgroups);
                compute_pass.end();
            }
            // Pass 2: Physics + block collisions + grid build
            {
                let compute_pass = encoder.begin_compute_pass();
                compute_pass.set_pipeline(&self.update_positions_pipeline);
                compute_pass.set_bind_group(0, Some(&self.compute_bind_group_a_to_b));
                compute_pass.dispatch_workgroups(entity_workgroups);
                compute_pass.end();
            }
            // Pass 3: Entity collisions + AI/input
            {
                let compute_pass = encoder.begin_compute_pass();
                compute_pass.set_pipeline(&self.resolve_collisions_pipeline);
                compute_pass.set_bind_group(0, Some(&self.compute_bind_group_b_to_a));
                compute_pass.dispatch_workgroups(entity_workgroups);
                compute_pass.end();
            }
        }

        // Render pass - multi-pass instanced rendering
        {
            let color_attachment = create_color_attachment(&view);
            let render_pass_desc = create_render_pass_descriptor(&color_attachment);
            let render_pass = encoder.begin_render_pass(&render_pass_desc).expect("begin render pass");

            // All passes share the same bind group
            render_pass.set_bind_group(0, Some(&self.render_bind_group));

            // Pass 1: Blocks (6 vertices × active_block_count instances)
            render_pass.set_pipeline(&self.block_pipeline);
            render_pass.draw_with_instance_count(6, self.block_count);

            // Pass 2: Entities (6 vertices × ENTITY_COUNT instances)
            render_pass.set_pipeline(&self.entity_pipeline);
            render_pass.draw_with_instance_count(6, ENTITY_COUNT);

            render_pass.end();
        }

        // Submit commands
        let command_buffer = encoder.finish();
        let commands = js_sys::Array::new();
        commands.push(&command_buffer);
        self.queue.submit(&commands);

        self.frame += 1;
    }
}

// Helper functions to create WebGPU objects

fn gpu_buffer_usage_uniform() -> u32 { 0x0040 }
fn gpu_buffer_usage_storage() -> u32 { 0x0080 }
fn gpu_buffer_usage_copy_dst() -> u32 { 0x0008 }

fn create_buffer(device: &GpuDevice, size: u32, usage: u32) -> GpuBuffer {
    let desc = web_sys::GpuBufferDescriptor::new(size as f64, usage);
    device.create_buffer(&desc).expect("Failed to create buffer")
}

fn create_buffer_with_data(device: &GpuDevice, data: &[u8], usage: u32) -> GpuBuffer {
    let desc = web_sys::GpuBufferDescriptor::new(data.len() as f64, usage | gpu_buffer_usage_copy_dst());
    desc.set_mapped_at_creation(true);
    let buffer = device.create_buffer(&desc).expect("Failed to create buffer");

    let mapped = buffer.get_mapped_range().expect("get mapped range");
    let array = Uint8Array::new(&mapped);
    array.copy_from(data);
    buffer.unmap();

    buffer
}

fn create_compute_bind_group_layout(device: &GpuDevice) -> web_sys::GpuBindGroupLayout {
    let entries = js_sys::Array::new();
    const COMPUTE: u32 = 0x4;

    // Binding 0: Uniforms
    entries.push(&create_bgl_entry(0, COMPUTE, "uniform"));

    // Bindings 1-3: Storage buffers (entities in/out, blocks)
    for i in 1..=3 {
        entries.push(&create_bgl_entry(i, COMPUTE, "storage"));
    }

    // Binding 4: Spatial grid (read-write storage)
    entries.push(&create_bgl_entry(4, COMPUTE, "storage"));

    // Binding 5: Entity grid (read-write storage)
    entries.push(&create_bgl_entry(5, COMPUTE, "storage"));

    // Binding 6: Entity cell counts (read-write storage)
    entries.push(&create_bgl_entry(6, COMPUTE, "storage"));

    let desc = web_sys::GpuBindGroupLayoutDescriptor::new(&entries);
    device.create_bind_group_layout(&desc).expect("Failed to create compute BGL")
}

fn create_render_bind_group_layout(device: &GpuDevice) -> web_sys::GpuBindGroupLayout {
    let entries = js_sys::Array::new();
    const VERTEX: u32 = 0x1;
    const FRAGMENT: u32 = 0x2;

    // Binding 0: Uniforms (vertex needs resolution for NDC conversion)
    entries.push(&create_bgl_entry(0, VERTEX | FRAGMENT, "uniform"));

    // Binding 1: Entities (vertex reads positions)
    entries.push(&create_bgl_entry(1, VERTEX, "read-only-storage"));

    // Binding 2: Blocks (vertex reads positions)
    entries.push(&create_bgl_entry(2, VERTEX, "read-only-storage"));

    // Binding 3: Sprites (fragment samples pixels)
    entries.push(&create_bgl_entry(3, FRAGMENT, "read-only-storage"));

    // Binding 4: Palettes (fragment gets colors)
    entries.push(&create_bgl_entry(4, FRAGMENT, "read-only-storage"));

    let desc = web_sys::GpuBindGroupLayoutDescriptor::new(&entries);
    device.create_bind_group_layout(&desc).expect("Failed to create render BGL")
}

fn create_bgl_entry(binding: u32, visibility: u32, buffer_type: &str) -> JsValue {
    let entry = Object::new();
    Reflect::set(&entry, &"binding".into(), &binding.into()).unwrap();
    Reflect::set(&entry, &"visibility".into(), &visibility.into()).unwrap();

    let buffer = Object::new();
    Reflect::set(&buffer, &"type".into(), &buffer_type.into()).unwrap();
    Reflect::set(&entry, &"buffer".into(), &buffer).unwrap();

    entry.into()
}

fn create_compute_bind_group(
    device: &GpuDevice,
    layout: &web_sys::GpuBindGroupLayout,
    uniform: &GpuBuffer,
    entity_in: &GpuBuffer,
    entity_out: &GpuBuffer,
    block: &GpuBuffer,
    spatial_grid: &GpuBuffer,
    entity_grid: &GpuBuffer,
    entity_counts: &GpuBuffer,
) -> GpuBindGroup {
    let entries = js_sys::Array::new();

    let buffers = [
        uniform,
        entity_in,
        entity_out,
        block,
        spatial_grid,
        entity_grid,
        entity_counts,
    ];
    for (i, buffer) in buffers.iter().enumerate() {
        let entry = Object::new();
        Reflect::set(&entry, &"binding".into(), &(i as u32).into()).unwrap();

        let resource = Object::new();
        Reflect::set(&resource, &"buffer".into(), buffer).unwrap();
        Reflect::set(&entry, &"resource".into(), &resource).unwrap();

        entries.push(&entry);
    }

    let desc = web_sys::GpuBindGroupDescriptor::new(&entries, layout);
    device.create_bind_group(&desc)
}

fn create_render_bind_group(
    device: &GpuDevice,
    layout: &web_sys::GpuBindGroupLayout,
    uniform: &GpuBuffer,
    entity: &GpuBuffer,
    block: &GpuBuffer,
    sprite: &GpuBuffer,
    palette: &GpuBuffer,
) -> GpuBindGroup {
    let entries = js_sys::Array::new();

    // Only 5 bindings for render: uniform, entities, blocks, sprites, palettes
    let buffers = [uniform, entity, block, sprite, palette];
    for (i, buffer) in buffers.iter().enumerate() {
        let entry = Object::new();
        Reflect::set(&entry, &"binding".into(), &(i as u32).into()).unwrap();

        let resource = Object::new();
        Reflect::set(&resource, &"buffer".into(), buffer).unwrap();
        Reflect::set(&entry, &"resource".into(), &resource).unwrap();

        entries.push(&entry);
    }

    let desc = web_sys::GpuBindGroupDescriptor::new(&entries, layout);
    device.create_bind_group(&desc)
}

fn create_pipeline_layout(device: &GpuDevice, bgl: &web_sys::GpuBindGroupLayout) -> web_sys::GpuPipelineLayout {
    let layouts = js_sys::Array::new();
    layouts.push(bgl);
    let desc = web_sys::GpuPipelineLayoutDescriptor::new(&layouts);
    device.create_pipeline_layout(&desc)
}

fn create_compute_pipeline(
    device: &GpuDevice,
    shader: &web_sys::GpuShaderModule,
    layout: &web_sys::GpuPipelineLayout,
    entry_point: &str,
) -> GpuComputePipeline {
    let compute_stage = web_sys::GpuProgrammableStage::new(shader);
    compute_stage.set_entry_point(entry_point);
    let desc = web_sys::GpuComputePipelineDescriptor::new(layout, &compute_stage);
    device.create_compute_pipeline(&desc)
}

fn create_render_pipeline(
    device: &GpuDevice,
    shader: &web_sys::GpuShaderModule,
    layout: &web_sys::GpuPipelineLayout,
    format: web_sys::GpuTextureFormat,
    vs_entry: &str,
    fs_entry: &str,
) -> GpuRenderPipeline {
    let vertex = web_sys::GpuVertexState::new(shader);
    vertex.set_entry_point(vs_entry);

    // Fragment state - use the canvas preferred format
    let target = Object::new();
    let format_val: JsValue = format.into();
    Reflect::set(&target, &"format".into(), &format_val).unwrap();

    let targets = js_sys::Array::new();
    targets.push(&target);

    let fragment = Object::new();
    Reflect::set(&fragment, &"module".into(), shader).unwrap();
    Reflect::set(&fragment, &"entryPoint".into(), &fs_entry.into()).unwrap();
    Reflect::set(&fragment, &"targets".into(), &targets).unwrap();

    let desc = Object::new();
    Reflect::set(&desc, &"layout".into(), layout).unwrap();
    Reflect::set(&desc, &"vertex".into(), &vertex).unwrap();
    Reflect::set(&desc, &"fragment".into(), &fragment).unwrap();

    let desc: web_sys::GpuRenderPipelineDescriptor = desc.unchecked_into();
    device.create_render_pipeline(&desc).expect("create render pipeline")
}

fn create_color_attachment(view: &web_sys::GpuTextureView) -> JsValue {
    let attachment = Object::new();
    Reflect::set(&attachment, &"view".into(), view).unwrap();
    Reflect::set(&attachment, &"loadOp".into(), &"clear".into()).unwrap();
    Reflect::set(&attachment, &"storeOp".into(), &"store".into()).unwrap();

    let clear_color = Object::new();
    Reflect::set(&clear_color, &"r".into(), &0.0.into()).unwrap();
    Reflect::set(&clear_color, &"g".into(), &0.0.into()).unwrap();
    Reflect::set(&clear_color, &"b".into(), &0.05.into()).unwrap();  // Dark blue background
    Reflect::set(&clear_color, &"a".into(), &1.0.into()).unwrap();
    Reflect::set(&attachment, &"clearValue".into(), &clear_color).unwrap();

    attachment.into()
}

fn create_render_pass_descriptor(color_attachment: &JsValue) -> web_sys::GpuRenderPassDescriptor {
    let attachments = js_sys::Array::new();
    attachments.push(color_attachment);
    web_sys::GpuRenderPassDescriptor::new(&attachments)
}

fn start_game_loop(gpu_state: Rc<RefCell<Option<GpuState>>>, set_fps: impl Fn(u32) + 'static) {
    let game_loop: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let game_loop_inner = game_loop.clone();
    let last_fps_update: Rc<RefCell<f64>> = Rc::new(RefCell::new(0.0));
    let frame_count: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));

    let closure = Closure::new(move |timestamp: f64| {
        if let Some(ref mut state) = *gpu_state.borrow_mut() {
            state.update();
        }

        // Update FPS counter every second
        *frame_count.borrow_mut() += 1;
        let elapsed = timestamp - *last_fps_update.borrow();
        if elapsed >= 1000.0 {
            let fps_val = (*frame_count.borrow() as f64 * 1000.0 / elapsed) as u32;
            set_fps(fps_val);
            *frame_count.borrow_mut() = 0;
            *last_fps_update.borrow_mut() = timestamp;
        }

        // Request next frame
        if let Some(window) = web_sys::window() {
            if let Some(ref closure) = *game_loop_inner.borrow() {
                window.request_animation_frame(closure.as_ref().unchecked_ref()).ok();
            }
        }
    });

    // Start the loop
    if let Some(window) = web_sys::window() {
        window.request_animation_frame(closure.as_ref().unchecked_ref()).ok();
    }

    *game_loop.borrow_mut() = Some(closure);
    std::mem::forget(game_loop);
}
