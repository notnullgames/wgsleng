use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use regex::Regex;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// Button indices matching WGSL
const BTN_UP: usize = 0;
const BTN_DOWN: usize = 1;
const BTN_LEFT: usize = 2;
const BTN_RIGHT: usize = 3;
const BTN_A: usize = 4;
const BTN_B: usize = 5;
const BTN_X: usize = 6;
const BTN_Y: usize = 7;
const BTN_L: usize = 8;
const BTN_R: usize = 9;
const BTN_START: usize = 10;
const BTN_SELECT: usize = 11;

#[derive(Debug)]
struct Assets {
    textures: Vec<String>,
    sounds: Vec<String>,
}

// Parse /** @asset ... */ directives from shader
fn parse_assets(code: &str) -> Assets {
    let texture_re = Regex::new(r#"/\*\*\s*@asset\s+texture\s+([^\s]+)\s*\*/"#).unwrap();
    let sound_re = Regex::new(r#"/\*\*\s*@asset\s+sound\s+([^\s]+)\s*\*/"#).unwrap();

    let textures = texture_re
        .captures_iter(code)
        .map(|cap| cap[1].to_string())
        .collect();

    let sounds = sound_re
        .captures_iter(code)
        .map(|cap| cap[1].to_string())
        .collect();

    Assets { textures, sounds }
}

// Preprocess shader with #include support
fn preprocess_shader(
    code: &str,
    base_path: &Path,
    visited: &mut HashSet<String>,
) -> Result<String, std::io::Error> {
    let include_re = Regex::new(r#"^\s*#include\s+["<]([^">]+)[">]"#).unwrap();
    let mut result = String::new();

    for line in code.lines() {
        if let Some(cap) = include_re.captures(line) {
            let include_path = &cap[1];
            let full_path = base_path.join(include_path);
            let full_path_str = full_path.to_string_lossy().to_string();

            if visited.contains(&full_path_str) {
                panic!("Circular include detected: {}", full_path_str);
            }

            visited.insert(full_path_str.clone());

            let include_code = fs::read_to_string(&full_path)?;
            let include_base = full_path.parent().unwrap_or(Path::new(""));

            result.push_str(&format!("// --- Begin include: {} ---\n", include_path));
            result.push_str(&preprocess_shader(&include_code, include_base, visited)?);
            result.push_str(&format!("\n// --- End include: {} ---\n", include_path));
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    Ok(result)
}

struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    compute_bind_group: wgpu::BindGroup,
    render_texture_bind_group: wgpu::BindGroup,
    render_state_bind_group: wgpu::BindGroup,
    input_buffer: wgpu::Buffer,
    state_buffer: wgpu::Buffer,
    audio_buffer: wgpu::Buffer,
    audio_read_buffer: wgpu::Buffer,
    buttons: [u32; 12],
    last_time: std::time::Instant,
    // Audio
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sound_buffers: Vec<Vec<u8>>,
    last_audio_trigger: u32,
}

impl State {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // WebGPU setup
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

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Load and preprocess shader
        let shader_code = fs::read_to_string("game.wgsl").expect("Failed to read game.wgsl");
        let mut visited = HashSet::new();
        let processed_code = preprocess_shader(&shader_code, Path::new("."), &mut visited)
            .expect("Failed to preprocess shader");

        let assets = parse_assets(&processed_code);
        println!("Loading assets: {:?}", assets);

        // Load audio
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let mut sound_buffers = Vec::new();
        for sound_file in &assets.sounds {
            let data = fs::read(sound_file).expect(&format!("Failed to load {}", sound_file));
            sound_buffers.push(data);
        }

        // Load texture
        let img = image::open(&assets.textures[0])
            .expect("Failed to load texture")
            .to_rgba8();
        let dimensions = img.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture"),
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

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create buffers
        let input_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Input Buffer"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let state_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("State Buffer"),
            contents: bytemuck::cast_slice(&[
                size.width as f32 / 2.0,
                size.height as f32 / 2.0,
                0.0f32,
                0.0f32,
                0.0f32,
                0.0f32,
            ]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let audio_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Audio Buffer"),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let audio_read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Audio Read Buffer"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(processed_code.into()),
        });

        // Create pipelines
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("update"),
            compilation_options: Default::default(),
            cache: None,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: None,
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

        // Create bind groups
        let compute_bind_group_layout = compute_pipeline.get_bind_group_layout(0);
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: state_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: audio_buffer.as_entire_binding(),
                },
            ],
        });

        let render_texture_bind_group_layout = render_pipeline.get_bind_group_layout(0);
        let render_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Texture Bind Group"),
            layout: &render_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let render_state_bind_group_layout = render_pipeline.get_bind_group_layout(1);
        let render_state_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render State Bind Group"),
            layout: &render_state_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: state_buffer.as_entire_binding(),
            }],
        });

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            compute_pipeline,
            render_pipeline,
            compute_bind_group,
            render_texture_bind_group,
            render_state_bind_group,
            input_buffer,
            state_buffer,
            audio_buffer,
            audio_read_buffer,
            buttons: [0; 12],
            last_time: std::time::Instant::now(),
            _stream,
            stream_handle,
            sound_buffers,
            last_audio_trigger: 0,
        }
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
                    KeyCode::ArrowUp | KeyCode::KeyW => self.buttons[BTN_UP] = value,
                    KeyCode::ArrowDown | KeyCode::KeyS => self.buttons[BTN_DOWN] = value,
                    KeyCode::ArrowLeft | KeyCode::KeyA => self.buttons[BTN_LEFT] = value,
                    KeyCode::ArrowRight | KeyCode::KeyD => self.buttons[BTN_RIGHT] = value,
                    KeyCode::KeyZ => self.buttons[BTN_A] = value,
                    KeyCode::KeyX => self.buttons[BTN_B] = value,
                    KeyCode::Enter => self.buttons[BTN_START] = value,
                    KeyCode::ShiftLeft => self.buttons[BTN_SELECT] = value,
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

        // Update input buffer with proper alignment
        let mut input_data = [0u32; 16];
        
        // Copy buttons (12 u32s in first 12 slots)
        input_data[0..12].copy_from_slice(&self.buttons);
        
        // Time/delta/screen as f32 in last 4 slots
        let time_data: [f32; 4] = [
            now.elapsed().as_secs_f32(),
            dt,
            self.size.width as f32,
            self.size.height as f32,
        ];
        
        let bytes = bytemuck::cast_slice(&input_data[0..12]);
        let time_bytes = bytemuck::cast_slice(&time_data);
        
        let mut buffer = vec![0u8; 64];
        buffer[0..48].copy_from_slice(bytes);
        buffer[48..64].copy_from_slice(time_bytes);
        
        self.queue.write_buffer(&self.input_buffer, 0, &buffer);
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

        // Compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
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
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_texture_bind_group, &[]);
            render_pass.set_bind_group(1, &self.render_state_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        // Copy audio buffer for reading
        encoder.copy_buffer_to_buffer(&self.audio_buffer, 0, &self.audio_read_buffer, 0, 4);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Check audio triggers
        let audio_slice = self.audio_read_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        audio_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);

        if let Ok(Ok(())) = pollster::block_on(receiver) {
            let data = audio_slice.get_mapped_range();
            let trigger = bytemuck::cast_slice::<u8, u32>(&data)[0];

            if trigger > self.last_audio_trigger && !self.sound_buffers.is_empty() {
                let cursor = Cursor::new(self.sound_buffers[0].clone());
                if let Ok(source) = Decoder::new(cursor) {
                    let sink = Sink::try_new(&self.stream_handle).unwrap();
                    sink.append(source);
                    sink.detach();
                }
                self.last_audio_trigger = trigger;
            }

            drop(data);
            self.audio_read_buffer.unmap();
        }

        Ok(())
    }
}

struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        winit::window::Window::default_attributes()
                            .with_title("WGSL Shader Game")
                            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600)),
                    )
                    .unwrap(),
            );
            self.state = Some(pollster::block_on(State::new(window)));
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
    let event_loop = EventLoop::new().unwrap();
    let mut app = App { state: None };
    event_loop.run_app(&mut app).unwrap();
}
