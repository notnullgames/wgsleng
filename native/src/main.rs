use std::collections::HashSet;
use std::fs;
use std::io::{Cursor, Read};
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
use clap::Parser;
use zip::ZipArchive;

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

#[derive(Parser, Debug)]
#[command(name = "wgsl-game")]
#[command(about = "Run WGSL shader games from directory or zip file")]
struct Args {
    /// Path to game directory or .zip file
    #[arg(default_value = ".")]
    game_path: String,
}

enum GameSource {
    Directory(std::path::PathBuf),
    Zip(ZipArchive<std::fs::File>),
}

impl GameSource {
    fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let path_obj = std::path::Path::new(path);
        
        if path_obj.is_dir() {
            Ok(GameSource::Directory(path_obj.to_path_buf()))
        } else if path.ends_with(".zip") {
            let file = std::fs::File::open(path)?;
            let archive = ZipArchive::new(file)?;
            Ok(GameSource::Zip(archive))
        } else {
            Err("Path must be a directory or .zip file".into())
        }
    }
    
    fn read_file(&mut self, file_path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match self {
        GameSource::Directory(base_path) => {
            let requested = std::path::Path::new(file_path);
            if requested.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
                return Err("Directory traversal not allowed".into());
            }
            Ok(fs::read(base_path.join(file_path))?)
        }
        GameSource::Zip(archive) => {
            // Try original path first
            match archive.by_name(file_path) {
                Ok(mut file) => {
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)?;
                    return Ok(contents);
                }
                Err(_) => {
                    // Fall through to try stripped path
                }
            }
            
            // Try with leading ./ stripped
            let stripped = file_path.strip_prefix("./").unwrap_or(file_path);
            match archive.by_name(stripped) {
                Ok(mut file) => {
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)?;
                    Ok(contents)
                }
                Err(_) => Err(format!("File not found: {}", file_path).into())
            }
        }
    }
    }

    
    fn read_text(&mut self, file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let bytes = self.read_file(file_path)?;
        Ok(String::from_utf8(bytes)?)
    }
}

#[derive(Debug)]
struct Metadata {
    title: String,
    textures: Vec<String>,
    sounds: Vec<String>,
}

fn parse_metadata(code: &str) -> Metadata {
    let title_re = Regex::new(r#"/\*\*\s*@title\s+(.+?)\s*\*/"#).unwrap();
    let texture_re = Regex::new(r#"/\*\*\s*@asset\s+texture\s+([^\s]+)\s*\*/"#).unwrap();
    let sound_re = Regex::new(r#"/\*\*\s*@asset\s+sound\s+([^\s]+)\s*\*/"#).unwrap();

    let title = title_re
        .captures(code)
        .map(|cap| cap[1].to_string())
        .unwrap_or_else(|| "WGSL Shader Game".to_string());

    let textures = texture_re
        .captures_iter(code)
        .map(|cap| cap[1].to_string())
        .collect();

    let sounds = sound_re
        .captures_iter(code)
        .map(|cap| cap[1].to_string())
        .collect();

    Metadata { title, textures, sounds }
}

fn preprocess_shader(
    code: &str,
    current_path: &str,
    game_source: &mut GameSource,
    visited: &mut HashSet<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let include_re = Regex::new(r#"/\*\*\s*@include\s+([^\s]+)\s*\*/"#).unwrap();
    let mut result = String::new();
    let mut last_pos = 0;

    for cap in include_re.captures_iter(code) {
        let match_start = cap.get(0).unwrap().start();
        let match_end = cap.get(0).unwrap().end();
        
        result.push_str(&code[last_pos..match_start]);
        
        let include_path = &cap[1];
        
        let full_path = if current_path.is_empty() || current_path == "main.wgsl" {
            include_path.to_string()
        } else {
            let base = std::path::Path::new(current_path)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("");
            if base.is_empty() {
                include_path.to_string()
            } else {
                format!("{}/{}", base, include_path)
            }
        };

        if visited.contains(&full_path) {
            return Err(format!("Circular include detected: {}", full_path).into());
        }

        visited.insert(full_path.clone());
        let include_code = game_source.read_text(&full_path)?;

        result.push_str(&format!("// --- Begin include: {} ---\n", include_path));
        result.push_str(&preprocess_shader(&include_code, &full_path, game_source, visited)?);
        result.push_str(&format!("\n// --- End include: {} ---\n", include_path));
        
        last_pos = match_end;
    }
    
    result.push_str(&code[last_pos..]);
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
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sound_buffers: Vec<Vec<u8>>,
    last_audio_trigger: u32,
}

impl State {
    async fn new(window: Arc<Window>, game_source: &mut GameSource) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();

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

        let shader_code = game_source.read_text("main.wgsl")?;
        let mut visited = HashSet::new();
        let processed_code = preprocess_shader(&shader_code, "main.wgsl", game_source, &mut visited)?;

        let metadata = parse_metadata(&processed_code);

        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let mut sound_buffers = Vec::new();
        for sound_file in &metadata.sounds {
            let data = game_source.read_file(sound_file)?;
            sound_buffers.push(data);
        }

        let img_data = game_source.read_file(&metadata.textures[0])?;
        let img = image::load_from_memory(&img_data)?.to_rgba8();
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(processed_code.into()),
        });

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

        Ok(Self {
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

        let mut input_data = [0u32; 16];
        input_data[0..12].copy_from_slice(&self.buttons);
        
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

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }

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

        encoder.copy_buffer_to_buffer(&self.audio_buffer, 0, &self.audio_read_buffer, 0, 4);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

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
    game_source: Option<GameSource>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let game_source = self.game_source.as_mut().unwrap();
            
            let shader_code = game_source.read_text("main.wgsl")
                .expect("Failed to read main.wgsl");
            let mut visited = HashSet::new();
            let processed_code = preprocess_shader(&shader_code, "main.wgsl", game_source, &mut visited)
                .expect("Failed to preprocess shader");
            let metadata = parse_metadata(&processed_code);
            
            let window = Arc::new(
                event_loop
                    .create_window(
                        winit::window::Window::default_attributes()
                            .with_title(&metadata.title)
                            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600)),
                    )
                    .unwrap(),
            );
            
            self.state = Some(pollster::block_on(State::new(window, game_source)).unwrap());
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
    
    let game_source = GameSource::open(&args.game_path)
        .expect("Failed to open game source");
    
    let event_loop = EventLoop::new().unwrap();
    let mut app = App { 
        state: None,
        game_source: Some(game_source),
    };
    event_loop.run_app(&mut app).unwrap();
}
