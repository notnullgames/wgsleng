use std::io::Cursor;
use std::sync::Arc;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};
use clap::Parser;
use wgsleng::{GameSource, PreprocessorState, BTN_UP, BTN_DOWN, BTN_LEFT, BTN_RIGHT, BTN_A, BTN_B, BTN_X, BTN_Y, BTN_L, BTN_R, BTN_START, BTN_SELECT};

#[derive(Parser, Debug)]
#[command(name = "wgsl-game")]
#[command(about = "Run WGSL shader games from directory or zip file")]
struct Args {
    /// Path to game.wgsl file or .zip containing main.wgsl
    game_path: String,
}

// All preprocessing logic is now in lib.rs

struct State {
    window: Arc<Window>,
    title: String,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    empty_bind_group: wgpu::BindGroup,
    compute_bind_group: wgpu::BindGroup,
    render_bind_group0: wgpu::BindGroup,
    render_bind_group1: wgpu::BindGroup,
    engine_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    buffer_offsets: BufferOffsets,
    buttons: [i32; 12],
    last_time: std::time::Instant,
    time: f32,
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sound_buffers: Vec<Vec<u8>>,
    audio_count: usize,
}

struct BufferOffsets {
    buttons: u64,
    floats: u64,
    state: u64,
    audio: u64,
}

impl State {
    async fn new(window: Arc<Window>, mut game_source: GameSource, entry_file: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let _size = window.inner_size();

        // Initialize WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // Load and preprocess shader
        let shader_code = game_source.read_text(entry_file)?;
        let mut preprocessor = PreprocessorState::new(game_source);
        let (processed_code, metadata) = preprocessor.preprocess_shader(&shader_code, true)?;

        // Debug: print processed shader
        if std::env::var("DEBUG_SHADER").is_ok() {
            println!("\n=== PROCESSED SHADER ===");
            println!("{}", processed_code);
            println!("=== END SHADER ===\n");
        }

        println!("Game: {}", metadata.title);
        println!("Size: {}x{}", metadata.width, metadata.height);
        println!("Textures: {:?}", metadata.textures);
        println!("Sounds: {:?}", metadata.sounds);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: metadata.width,
            height: metadata.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Load audio
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let mut sound_buffers = Vec::new();
        for sound_file in &metadata.sounds {
            let data = preprocessor.game_source.read_file(sound_file)?;
            sound_buffers.push(data);
        }

        // Load textures
        let mut textures = Vec::new();
        for texture_file in &metadata.textures {
            let img_data = preprocessor.game_source.read_file(texture_file)?;
            let img = image::load_from_memory(&img_data)?.to_rgba8();
            let dimensions = img.dimensions();

            let texture_size = wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Game Texture"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &img,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * dimensions.0),
                    rows_per_image: Some(dimensions.1),
                },
                texture_size,
            );

            textures.push(texture);
        }

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Calculate buffer layout matching WGSL struct
        let button_size = 12 * 4; // 48 bytes
        let float_data_size = 4 * 4; // 16 bytes
        // State alignment depends on the largest member - vec2f has 8-byte alignment
        let state_alignment = 8;
        let aligned_state_size = ((metadata.state_size + state_alignment - 1) / state_alignment) * state_alignment;
        let audio_size = metadata.sounds.len() * 4;

        let total_size_unaligned = button_size + float_data_size + aligned_state_size + audio_size;
        let total_size = ((total_size_unaligned + 15) / 16) * 16;

        let buffer_offsets = BufferOffsets {
            buttons: 0,
            floats: button_size as u64,
            state: (button_size + float_data_size) as u64,
            audio: (button_size + float_data_size + aligned_state_size) as u64,
        };


        // Create engine buffer
        let mut init_data = vec![0u8; total_size];

        // Initialize screen size in floats section
        let width_bytes = (metadata.width as f32).to_le_bytes();
        let height_bytes = (metadata.height as f32).to_le_bytes();
        init_data[buffer_offsets.floats as usize + 8..buffer_offsets.floats as usize + 12].copy_from_slice(&width_bytes);
        init_data[buffer_offsets.floats as usize + 12..buffer_offsets.floats as usize + 16].copy_from_slice(&height_bytes);

        // Initialize player position to center in state section
        let center_x = ((metadata.width / 2) as f32).to_le_bytes();
        let center_y = ((metadata.height / 2) as f32).to_le_bytes();
        init_data[buffer_offsets.state as usize..buffer_offsets.state as usize + 4].copy_from_slice(&center_x);
        init_data[buffer_offsets.state as usize + 4..buffer_offsets.state as usize + 8].copy_from_slice(&center_y);

        let engine_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Engine Buffer"),
            contents: &init_data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        });

        // Create staging buffer for reading audio triggers
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: total_size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Game Shader"),
            source: wgpu::ShaderSource::Wgsl(processed_code.into()),
        });

        // Create explicit bind group layouts for render pipeline
        // Group 0: sampler (always) and textures (if present)
        let mut render_group0_entries = vec![
            // Sampler (always present since preprocessor always adds it)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            }
        ];

        // Add texture bindings if present
        for i in 0..metadata.textures.len() {
            render_group0_entries.push(wgpu::BindGroupLayoutEntry {
                binding: (i + 1) as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }

        let render_bind_group_layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout 0"),
            entries: &render_group0_entries,
        });

        // Group 1: engine buffer (writable, but only FRAGMENT visibility to avoid VERTEX_WRITABLE_STORAGE feature)
        let render_bind_group_layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout 1"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout0, &render_bind_group_layout1],
            push_constant_ranges: &[],
        });

        // Create pipelines
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
                entry_point: Some("fs_render"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create explicit bind group layouts for compute shader
        // Compute shader uses @group(1) @binding(0) for the engine buffer
        // We need a placeholder for @group(0) since the shader uses @group(1)
        let empty_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Empty Bind Group Layout"),
            entries: &[],
        });

        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&empty_bind_group_layout, &compute_bind_group_layout],
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

        // Create texture views first (need to store them to avoid temporary borrow)
        let texture_views: Vec<_> = textures.iter()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect();

        // Create bind groups
        // Group 0 always includes sampler (since preprocessor always adds it)
        let mut group0_entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&sampler),
            }
        ];

        // Add texture views if present
        for (i, view) in texture_views.iter().enumerate() {
            group0_entries.push(wgpu::BindGroupEntry {
                binding: (i + 1) as u32,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }

        let render_bind_group0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group 0"),
            layout: &render_bind_group_layout0,
            entries: &group0_entries,
        });

        let render_bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group 1"),
            layout: &render_bind_group_layout1,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: engine_buffer.as_entire_binding(),
            }],
        });

        // Create empty bind group for @group(0) (compute shader doesn't use it but layout requires it)
        let empty_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Empty Bind Group"),
            layout: &empty_bind_group_layout,
            entries: &[],
        });

        // Create bind group for compute pipeline using explicit layout
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: engine_buffer.as_entire_binding(),
            }],
        });

        Ok(Self {
            window,
            title: metadata.title.clone(),
            surface,
            device,
            queue,
            config,
            size: winit::dpi::PhysicalSize::new(metadata.width, metadata.height),
            compute_pipeline,
            render_pipeline,
            empty_bind_group,
            compute_bind_group,
            render_bind_group0,
            render_bind_group1,
            engine_buffer,
            staging_buffer,
            buffer_offsets,
            buttons: [0; 12],
            last_time: std::time::Instant::now(),
            time: 0.0,
            _stream,
            stream_handle,
            sound_buffers,
            audio_count: metadata.sounds.len(),
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = *state == ElementState::Pressed;
                let value = if pressed { 1 } else { 0 };

                match key {
                    KeyCode::ArrowUp => self.buttons[BTN_UP] = value,
                    KeyCode::ArrowDown => self.buttons[BTN_DOWN] = value,
                    KeyCode::ArrowLeft => self.buttons[BTN_LEFT] = value,
                    KeyCode::ArrowRight => self.buttons[BTN_RIGHT] = value,
                    KeyCode::KeyX => self.buttons[BTN_A] = value,
                    KeyCode::KeyZ => self.buttons[BTN_B] = value,
                    KeyCode::KeyS => self.buttons[BTN_X] = value,
                    KeyCode::KeyA => self.buttons[BTN_Y] = value,
                    KeyCode::KeyQ => self.buttons[BTN_L] = value,
                    KeyCode::KeyW => self.buttons[BTN_R] = value,
                    KeyCode::Enter => self.buttons[BTN_START] = value,
                    KeyCode::ShiftLeft | KeyCode::ShiftRight => self.buttons[BTN_SELECT] = value,
                    _ => return false,
                }
                true
            }
            _ => false,
        }
    }

    fn update(&mut self) {
        let now = std::time::Instant::now();
        let dt = (now - self.last_time).as_secs_f32();
        let dt = dt.min(0.1);
        self.last_time = now;
        self.time += dt;

        // Write input data to buffer (buttons + floats)
        let mut input_data = Vec::new();

        // Buttons (48 bytes)
        for &button in &self.buttons {
            input_data.extend_from_slice(&button.to_le_bytes());
        }

        // Time data (16 bytes)
        input_data.extend_from_slice(&self.time.to_le_bytes());
        input_data.extend_from_slice(&dt.to_le_bytes());
        input_data.extend_from_slice(&(self.size.width as f32).to_le_bytes());
        input_data.extend_from_slice(&(self.size.height as f32).to_le_bytes());

        self.queue.write_buffer(&self.engine_buffer, 0, &input_data);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Run compute shader
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.empty_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.compute_bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }

        // Render
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
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group0, &[]);
            render_pass.set_bind_group(1, &self.render_bind_group1, &[]);
            render_pass.draw(0..3, 0..1);
        }

        // Copy audio buffer to staging for readback
        if self.audio_count > 0 {
            encoder.copy_buffer_to_buffer(
                &self.engine_buffer,
                self.buffer_offsets.audio,
                &self.staging_buffer,
                0,
                (self.audio_count * 4) as u64,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Read audio triggers
        if self.audio_count > 0 {
            let slice = self.staging_buffer.slice(0..(self.audio_count * 4) as u64);
            let (sender, receiver) = futures::channel::oneshot::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap();
            });
            self.device.poll(wgpu::Maintain::Wait);

            if let Ok(Ok(())) = pollster::block_on(receiver) {
                let data = slice.get_mapped_range();
                let triggers: &[u32] = bytemuck::cast_slice(&data);

                for (i, &trigger) in triggers.iter().enumerate() {
                    if trigger > 0 && i < self.sound_buffers.len() {
                        let cursor = Cursor::new(self.sound_buffers[i].clone());
                        if let Ok(source) = Decoder::new(cursor) {
                            let sink = Sink::try_new(&self.stream_handle).unwrap();
                            sink.append(source);
                            sink.detach();
                        }
                    }
                }

                drop(data);
                self.staging_buffer.unmap();

                // Reset audio triggers
                let zeros = vec![0u8; self.audio_count * 4];
                self.queue.write_buffer(&self.engine_buffer, self.buffer_offsets.audio, &zeros);
            }
        }

        Ok(())
    }
}

struct App {
    state: Option<State>,
    game_source: Option<GameSource>,
    entry_file: String,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let game_source = self.game_source.take().unwrap();

            // Create window with metadata
            let window = Arc::new(
                event_loop
                    .create_window(
                        winit::window::Window::default_attributes()
                            .with_title("WGSL Game")
                            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600)),
                    )
                    .unwrap(),
            );

            let state = pollster::block_on(State::new(window, game_source, &self.entry_file)).unwrap();

            // Set window title and size from game metadata
            state.window.set_title(&state.title);
            let _ = state.window.request_inner_size(state.size);

            self.state = Some(state);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(state) = &mut self.state {
            if !state.input(&event) {
                match event {
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::Resized(physical_size) => state.resize(physical_size),
                    WindowEvent::RedrawRequested => {
                        state.update();
                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                            Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                            Err(e) => eprintln!("{:?}", e),
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    // Determine entry file
    let entry_file = if args.game_path.ends_with(".wgsl") {
        std::path::Path::new(&args.game_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    } else {
        "main.wgsl".to_string()
    };

    let game_source = GameSource::open(&args.game_path)
        .expect("Failed to open game source");

    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        state: None,
        game_source: Some(game_source),
        entry_file,
    };
    event_loop.run_app(&mut app).unwrap();
}
