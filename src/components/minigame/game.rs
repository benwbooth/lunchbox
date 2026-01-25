//! WebGPU-powered Mario mini-game
//! All game logic runs in WGSL shaders - Rust only bootstraps WebGPU
//!
//! This code is compiled to WASM and runs in Tauri's webview or browser.
//! WebGPU is available in modern webviews (WebKit, WebView2, Chromium).

use leptos::prelude::*;
use leptos::html;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, KeyboardEvent};
use std::cell::RefCell;
use std::rc::Rc;
use wgpu::util::DeviceExt;

use super::gpu::{pack_sprite_atlas, pack_palettes, Uniforms};

/// The Mario Mini-Game component (WebGPU)
#[component]
pub fn MarioMinigame() -> impl IntoView {
    let canvas_ref = NodeRef::<html::Canvas>::new();
    let initialized = Rc::new(RefCell::new(false));
    let gpu_state: Rc<RefCell<Option<GpuState>>> = Rc::new(RefCell::new(None));

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
                <p>"Select a platform to view games."</p>
            </div>
        </div>
    }
}

/// WebGPU state
struct GpuState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    entity_buffer: wgpu::Buffer,
    block_buffer: wgpu::Buffer,
    platform_buffer: wgpu::Buffer,
    sprite_buffer: wgpu::Buffer,
    palette_buffer: wgpu::Buffer,
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,
    uniforms: Uniforms,
    frame: u32,
    start_time: f64,
    input_left: bool,
    input_right: bool,
    input_jump: bool,
}

impl GpuState {
    async fn new(canvas: &HtmlCanvasElement) -> Result<Self, String> {
        let window = web_sys::window().ok_or("No window")?;

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

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        // Create surface from canvas
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| format!("Failed to create surface: {}", e))?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to get adapter")?;

        // Request device
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Mario Minigame Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| format!("Failed to get device: {}", e))?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Load shaders
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Game Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/game.wgsl").into()),
        });

        // Create buffers
        let uniforms = Uniforms {
            resolution: [width as f32, height as f32],
            time: 0.0,
            delta_time: 1.0 / 60.0,
            mouse: [0.0, 0.0],
            mouse_click: 0,
            frame: 0,
        };

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Entity buffer (128 entities * 32 bytes each)
        let entity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Entity Buffer"),
            size: 128 * 32,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Block buffer (512 blocks * 16 bytes each)
        let block_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Block Buffer"),
            size: 512 * 16,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Platform buffer (64 platforms * 16 bytes each)
        let platform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Platform Buffer"),
            size: 64 * 16,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Sprite atlas buffer
        let sprite_data = pack_sprite_atlas();
        let sprite_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Buffer"),
            contents: bytemuck::cast_slice(&sprite_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Palette buffer
        let palette_data = pack_palettes();
        let palette_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Palette Buffer"),
            contents: bytemuck::cast_slice(&palette_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Bind group layout for compute
        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Bind group layout for render
        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create compute bind group
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: entity_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: block_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: platform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: sprite_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: palette_buffer.as_entire_binding(),
                },
            ],
        });

        // Create render bind group
        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: entity_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: block_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: platform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: sprite_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: palette_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: Some("update"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let start_time = js_sys::Date::now();

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            compute_pipeline,
            render_pipeline,
            uniform_buffer,
            entity_buffer,
            block_buffer,
            platform_buffer,
            sprite_buffer,
            palette_buffer,
            compute_bind_group,
            render_bind_group,
            uniforms,
            frame: 0,
            start_time,
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

        // Encode input state in mouse_click for now (player control)
        let mut input_bits = 0u32;
        if self.input_left { input_bits |= 1; }
        if self.input_right { input_bits |= 2; }
        if self.input_jump { input_bits |= 4; }
        self.uniforms.mouse_click = input_bits;

        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));

        // Get output texture
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            // Dispatch enough workgroups for all entities (128 / 64 = 2)
            compute_pass.dispatch_workgroups(2, 1, 1);
        }

        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Full-screen triangle
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.frame += 1;
    }
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
