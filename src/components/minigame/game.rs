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
use js_sys::{Object, Reflect, Uint8Array};

use super::gpu::{pack_sprite_atlas, pack_palettes, Uniforms, u32_slice_to_bytes};

const COMPUTE_SHADER: &str = include_str!("shaders/compute.wgsl");
const RENDER_SHADER: &str = include_str!("shaders/render.wgsl");

/// The Mario Mini-Game component (WebGPU)
#[component]
pub fn MarioMinigame() -> impl IntoView {
    let canvas_ref = NodeRef::<html::Canvas>::new();
    let initialized = Rc::new(RefCell::new(false));
    let gpu_state: Rc<RefCell<Option<GpuState>>> = Rc::new(RefCell::new(None));
    let (error_msg, set_error_msg) = signal(Option::<String>::None);

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
                        start_game_loop(gpu_state_inner);
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
        </div>
    }
}

/// WebGPU state using raw web-sys types
struct GpuState {
    device: GpuDevice,
    queue: GpuQueue,
    context: GpuCanvasContext,
    compute_pipeline: GpuComputePipeline,
    render_pipeline: GpuRenderPipeline,
    uniform_buffer: GpuBuffer,
    entity_buffer: GpuBuffer,
    block_buffer: GpuBuffer,
    platform_buffer: GpuBuffer,
    compute_bind_group: GpuBindGroup,
    render_bind_group: GpuBindGroup,
    uniforms: Uniforms,
    frame: u32,
    start_time: f64,
    width: u32,
    height: u32,
    input_left: bool,
    input_right: bool,
    input_jump: bool,
}

impl GpuState {
    async fn new(canvas: &HtmlCanvasElement) -> Result<Self, String> {
        let window = web_sys::window().ok_or("No window")?;
        let navigator = window.navigator();

        // Check for WebGPU support
        let gpu = navigator.gpu();

        // Set canvas size
        let window_width = window.inner_width().ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1200.0);
        let window_height = window.inner_height().ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(800.0);

        let sidebar_width = 240.0;
        let toolbar_height = 52.0;
        let width = ((window_width - sidebar_width).max(400.0)) as u32;
        let height = ((window_height - toolbar_height).max(300.0)) as u32;

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

        // Request device
        let device_promise = adapter.request_device();
        let device = wasm_bindgen_futures::JsFuture::from(device_promise)
            .await
            .map_err(|e| format!("Failed to get device: {:?}", e))?;

        let device: GpuDevice = device.dyn_into()
            .map_err(|_| "Failed to cast device")?;

        let queue = device.queue();

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

        // Create shader modules (separate to avoid binding conflicts)
        let compute_shader_desc = web_sys::GpuShaderModuleDescriptor::new(COMPUTE_SHADER);
        let compute_shader = device.create_shader_module(&compute_shader_desc);

        let render_shader_desc = web_sys::GpuShaderModuleDescriptor::new(RENDER_SHADER);
        let render_shader = device.create_shader_module(&render_shader_desc);

        // Create buffers
        let uniform_buffer = create_buffer(&device, 32, gpu_buffer_usage_uniform() | gpu_buffer_usage_copy_dst());
        let entity_buffer = create_buffer(&device, 128 * 32, gpu_buffer_usage_storage() | gpu_buffer_usage_copy_dst());
        let block_buffer = create_buffer(&device, 512 * 16, gpu_buffer_usage_storage() | gpu_buffer_usage_copy_dst());
        let platform_buffer = create_buffer(&device, 64 * 16, gpu_buffer_usage_storage() | gpu_buffer_usage_copy_dst());

        // Create and upload sprite/palette buffers
        let sprite_data = pack_sprite_atlas();
        let sprite_buffer = create_buffer_with_data(&device, &u32_slice_to_bytes(&sprite_data), gpu_buffer_usage_storage());

        let palette_data = pack_palettes();
        let palette_buffer = create_buffer_with_data(&device, &u32_slice_to_bytes(&palette_data), gpu_buffer_usage_storage());

        // Create bind group layouts - separate for compute (read_write) and render (read-only)
        // With separate shader modules, each can use group 0 with different access modes
        let compute_bgl = create_compute_bind_group_layout(&device);
        let render_bgl = create_render_bind_group_layout(&device);

        // Create bind groups - both point to the same buffers but with different layouts
        let compute_bind_group = create_bind_group(
            &device, &compute_bgl,
            &uniform_buffer, &entity_buffer, &block_buffer, &platform_buffer,
            &sprite_buffer, &palette_buffer,
        );
        let render_bind_group = create_bind_group(
            &device, &render_bgl,
            &uniform_buffer, &entity_buffer, &block_buffer, &platform_buffer,
            &sprite_buffer, &palette_buffer,
        );

        // Create pipeline layouts - each uses its own bind group layout at group 0
        let compute_pipeline_layout = create_pipeline_layout(&device, &compute_bgl);
        let render_pipeline_layout = create_pipeline_layout(&device, &render_bgl);

        // Create pipelines with their respective shaders and layouts
        let compute_pipeline = create_compute_pipeline(&device, &compute_shader, &compute_pipeline_layout);
        let render_pipeline = create_render_pipeline(&device, &render_shader, &render_pipeline_layout, preferred_format);

        let start_time = js_sys::Date::now();

        let uniforms = Uniforms {
            resolution: [width as f32, height as f32],
            ..Default::default()
        };

        Ok(Self {
            device,
            queue,
            context,
            compute_pipeline,
            render_pipeline,
            uniform_buffer,
            entity_buffer,
            block_buffer,
            platform_buffer,
            compute_bind_group,
            render_bind_group,
            uniforms,
            frame: 0,
            start_time,
            width,
            height,
            input_left: false,
            input_right: false,
            input_jump: false,
        })
    }

    fn update(&mut self) {
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
        let uniform_bytes = self.uniforms.to_bytes();
        let uniform_array = Uint8Array::from(&uniform_bytes[..]);
        let _ = self.queue.write_buffer_with_u32_and_buffer_source(&self.uniform_buffer, 0, &uniform_array);

        // Get current texture
        let texture = self.context.get_current_texture().expect("get current texture");
        let view = texture.create_view().expect("create texture view");

        // Create command encoder
        let encoder = self.device.create_command_encoder();

        // Compute pass
        {
            let compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, Some(&self.compute_bind_group));
            compute_pass.dispatch_workgroups(2); // 128 entities / 64 workgroup size
            compute_pass.end();
        }

        // Render pass
        {
            let color_attachment = create_color_attachment(&view);
            let render_pass_desc = create_render_pass_descriptor(&color_attachment);
            let render_pass = encoder.begin_render_pass(&render_pass_desc).expect("begin render pass");
            render_pass.set_pipeline(&self.render_pipeline);
            // Render shader uses group 0 with read-only bindings (separate shader module)
            render_pass.set_bind_group(0, Some(&self.render_bind_group));
            // Instanced rendering: 6 vertices per quad, 704 instances (64 platforms + 512 blocks + 128 entities)
            render_pass.draw_with_instance_count(6, 704);
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

    // Bindings 1-3: Storage buffers (read-write)
    for i in 1..=3 {
        entries.push(&create_bgl_entry(i, COMPUTE, "storage"));
    }

    // Bindings 4-5: Read-only storage (sprites, palettes)
    for i in 4..=5 {
        entries.push(&create_bgl_entry(i, COMPUTE, "read-only-storage"));
    }

    let desc = web_sys::GpuBindGroupLayoutDescriptor::new(&entries);
    device.create_bind_group_layout(&desc).expect("Failed to create compute BGL")
}

fn create_render_bind_group_layout(device: &GpuDevice) -> web_sys::GpuBindGroupLayout {
    let entries = js_sys::Array::new();
    const VERTEX: u32 = 0x1;
    const FRAGMENT: u32 = 0x2;

    // Binding 0: Uniforms (vertex needs resolution for NDC conversion)
    entries.push(&create_bgl_entry(0, VERTEX | FRAGMENT, "uniform"));

    // Bindings 1-3: Storage buffers (vertex shader reads entity/block/platform positions)
    for i in 1..=3 {
        entries.push(&create_bgl_entry(i, VERTEX | FRAGMENT, "read-only-storage"));
    }

    // Bindings 4-5: Read-only storage (sprites, palettes - fragment only)
    for i in 4..=5 {
        entries.push(&create_bgl_entry(i, FRAGMENT, "read-only-storage"));
    }

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

fn create_bind_group(
    device: &GpuDevice,
    layout: &web_sys::GpuBindGroupLayout,
    uniform: &GpuBuffer,
    entity: &GpuBuffer,
    block: &GpuBuffer,
    platform: &GpuBuffer,
    sprite: &GpuBuffer,
    palette: &GpuBuffer,
) -> GpuBindGroup {
    let entries = js_sys::Array::new();

    let buffers = [uniform, entity, block, platform, sprite, palette];
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
) -> GpuComputePipeline {
    let compute_stage = web_sys::GpuProgrammableStage::new(shader);
    compute_stage.set_entry_point("update");
    let desc = web_sys::GpuComputePipelineDescriptor::new(layout, &compute_stage);
    device.create_compute_pipeline(&desc)
}

fn create_render_pipeline(
    device: &GpuDevice,
    shader: &web_sys::GpuShaderModule,
    layout: &web_sys::GpuPipelineLayout,
    format: web_sys::GpuTextureFormat,
) -> GpuRenderPipeline {
    let vertex = web_sys::GpuVertexState::new(shader);
    vertex.set_entry_point("vs_main");

    // Fragment state - use the canvas preferred format
    let target = Object::new();
    let format_val: JsValue = format.into();
    Reflect::set(&target, &"format".into(), &format_val).unwrap();

    let targets = js_sys::Array::new();
    targets.push(&target);

    let fragment = Object::new();
    Reflect::set(&fragment, &"module".into(), shader).unwrap();
    Reflect::set(&fragment, &"entryPoint".into(), &"fs_main".into()).unwrap();
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

fn start_game_loop(gpu_state: Rc<RefCell<Option<GpuState>>>) {
    let game_loop: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let game_loop_inner = game_loop.clone();

    let closure = Closure::new(move |_timestamp: f64| {
        if let Some(ref mut state) = *gpu_state.borrow_mut() {
            state.update();
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
