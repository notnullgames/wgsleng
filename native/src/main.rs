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
    /// Path to game.wgsl file or .zip containing main.wgsl
    game_path: String,
}

enum GameSource {
    Directory(std::path::PathBuf),
    Zip(ZipArchive<std::fs::File>),
}

impl GameSource {
    fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Check if it's a .wgsl file
        if path.ends_with(".wgsl") {
            let path_obj = std::path::Path::new(path);
            if let Some(parent) = path_obj.parent() {
                return Ok(GameSource::Directory(parent.to_path_buf()));
            }
            return Ok(GameSource::Directory(std::path::PathBuf::from(".")));
        }

        // Check if it's a zip file
        if path.ends_with(".zip") {
            let file = std::fs::File::open(path)?;
            let archive = ZipArchive::new(file)?;
            return Ok(GameSource::Zip(archive));
        }

        // Otherwise treat as directory
        Ok(GameSource::Directory(std::path::PathBuf::from(path)))
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
                let stripped = file_path.strip_prefix("./").unwrap_or(file_path);
                match archive.by_name(stripped) {
                    Ok(mut file) => {
                        let mut contents = Vec::new();
                        file.read_to_end(&mut contents)?;
                        Ok(contents)
                    }
                    Err(_) => Err(format!("File not found in zip: {}", file_path).into())
                }
            }
        }
    }

    fn read_text(&mut self, file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let bytes = self.read_file(file_path)?;
        Ok(String::from_utf8(bytes)?)
    }
}

#[derive(Debug, Clone)]
struct Metadata {
    title: String,
    width: u32,
    height: u32,
    textures: Vec<String>,
    sounds: Vec<String>,
    state_size: usize,
}

struct PreprocessorState {
    game_source: GameSource,
    imported_files: HashSet<String>,
}

impl PreprocessorState {
    fn new(game_source: GameSource) -> Self {
        Self {
            game_source,
            imported_files: HashSet::new(),
        }
    }

    fn preprocess_shader(&mut self, source: &str, is_top_level: bool) -> Result<(String, Metadata), Box<dyn std::error::Error>> {
        let mut source = source.to_string();

        // Process @import directives first (recursive, like C #include)
        let import_re = Regex::new(r#"@import\("([^"]+)"\)"#)?;
        loop {
            // Collect captures into owned strings to avoid borrow issues
            let captures: Vec<(String, String)> = import_re.captures_iter(&source)
                .map(|cap| (cap.get(0).unwrap().as_str().to_string(), cap[1].to_string()))
                .collect();

            if captures.is_empty() {
                break;
            }

            for (full_match, filename) in captures {
                if self.imported_files.contains(&filename) {
                    source = source.replace(&full_match, &format!("// Already imported: {}", filename));
                    continue;
                }

                self.imported_files.insert(filename.clone());
                let imported_code = self.game_source.read_text(&filename)?;
                let (processed, _) = self.preprocess_shader(&imported_code, false)?;

                source = source.replace(&full_match, &format!("// Imported from {}\n{}\n", filename, processed));
            }
        }

        // Extract metadata
        let mut metadata = Metadata {
            title: "WGSL Game".to_string(),
            width: 800,
            height: 600,
            textures: Vec::new(),
            sounds: Vec::new(),
            state_size: 16,
        };

        // Extract @set_title
        if let Some(cap) = Regex::new(r#"@set_title\("([^"]+)"\)"#)?.captures(&source) {
            metadata.title = cap[1].to_string();
        }

        // Extract @set_size
        if let Some(cap) = Regex::new(r#"@set_size\((\d+),\s*(\d+)\)"#)?.captures(&source) {
            metadata.width = cap[1].parse()?;
            metadata.height = cap[2].parse()?;
        }

        // Find all @sound() references
        let sound_re = Regex::new(r#"@sound\("([^"]+)"\)(?:\.(?:play|stop)\(\))?"#)?;
        for cap in sound_re.captures_iter(&source) {
            let sound_file = cap[1].to_string();
            if !metadata.sounds.contains(&sound_file) {
                metadata.sounds.push(sound_file);
            }
        }

        // Find all @texture() references
        let texture_re = Regex::new(r#"@texture\("([^"]+)"\)"#)?;
        for cap in texture_re.captures_iter(&source) {
            let texture_file = cap[1].to_string();
            if !metadata.textures.contains(&texture_file) {
                metadata.textures.push(texture_file);
            }
        }

        // Remove @set_* directives
        source = Regex::new(r#"@set_title\([^)]+\)[^\n]*"#)?.replace_all(&source, "").to_string();
        source = Regex::new(r#"@set_size\([^)]+\)[^\n]*"#)?.replace_all(&source, "").to_string();

        // Find GameState struct
        let game_state_re = Regex::new(r"struct GameState\s*\{[^}]+\}")?;
        let game_state_struct = game_state_re.find(&source).map(|m| m.as_str().to_string());

        // Calculate GameState size
        if let Some(ref gs) = game_state_struct {
            // Match field types, including arrays with angle brackets
            let field_re = Regex::new(r":\s*(?:array<[^>]+>|[^,;\n]+)")?;
            let array_re = Regex::new(r"array<([^,>]+),\s*(\d+)>")?;
            let mut size = 0;
            let mut alignment = 4; // Track the largest member alignment

            for cap in field_re.captures_iter(gs) {
                let field = cap.get(0).unwrap().as_str();

                // Check if it's an array
                if let Some(array_cap) = array_re.captures(field) {
                    let element_type = &array_cap[1];
                    let count: usize = array_cap[2].parse().unwrap_or(1);

                    let (element_size, element_align) = if element_type.contains("vec4f") {
                        (16, 16)
                    } else if element_type.contains("vec3f") {
                        (16, 16) // vec3 aligns to 16 in arrays
                    } else if element_type.contains("vec2f") {
                        (8, 8)
                    } else {
                        (4, 4) // u32, i32, f32
                    };

                    alignment = alignment.max(element_align);
                    size += element_size * count;
                } else {
                    // Regular field
                    if field.contains("vec4f") {
                        size += 16;
                        alignment = alignment.max(16);
                    } else if field.contains("vec3f") {
                        size += 12;
                        alignment = alignment.max(16);
                    } else if field.contains("vec2f") {
                        size += 8;
                        alignment = alignment.max(8);
                    } else if field.contains("u32") || field.contains("i32") || field.contains("f32") {
                        size += 4;
                        alignment = alignment.max(4);
                    }
                }
            }

            // Round up to struct's alignment (largest member)
            metadata.state_size = ((size + alignment - 1) / alignment) * alignment;
        }

        // Build header (only for top-level)
        let mut header = String::new();
        if is_top_level {
            header.push_str("// Preprocessed WGSL - generated from macros\n\n");

            // Add GameState first
            if let Some(ref gs) = game_state_struct {
                header.push_str(gs);
                header.push_str("\n\n");
            }

            // Add GameEngineHost struct
            header.push_str("// Engine host struct that contains all engine state\n");
            header.push_str("struct GameEngineHost {\n");
            header.push_str("    buttons: array<i32, 12>, // the current state of virtual SNES gamepad (BTN_*)\n");
            header.push_str("    time: f32, // clock time\n");
            header.push_str("    delta_time: f32, // time since last frame\n");
            header.push_str("    screen_width: f32, // current screensize\n");
            header.push_str("    screen_height: f32, // current screensize\n");
            if game_state_struct.is_some() {
                header.push_str("    state: GameState, // user's game state that persists across frames\n");
            }
            if !metadata.sounds.is_empty() {
                header.push_str(&format!("    audio: array<u32, {}>, // audio trigger counters\n", metadata.sounds.len()));
            }
            header.push_str("}\n\n");

            // Add button constants
            header.push_str("// Button constants for input\n");
            header.push_str("const BTN_UP: u32 = 0u;\n");
            header.push_str("const BTN_DOWN: u32 = 1u;\n");
            header.push_str("const BTN_LEFT: u32 = 2u;\n");
            header.push_str("const BTN_RIGHT: u32 = 3u;\n");
            header.push_str("const BTN_A: u32 = 4u;\n");
            header.push_str("const BTN_B: u32 = 5u;\n");
            header.push_str("const BTN_X: u32 = 6u;\n");
            header.push_str("const BTN_Y: u32 = 7u;\n");
            header.push_str("const BTN_L: u32 = 8u;\n");
            header.push_str("const BTN_R: u32 = 9u;\n");
            header.push_str("const BTN_START: u32 = 10u;\n");
            header.push_str("const BTN_SELECT: u32 = 11u;\n\n");

            // Add bindings
            header.push_str("// Bindings: group 0 = textures, group 1 = engine state\n\n");
            header.push_str("@group(0) @binding(0) var _engine_sampler: sampler;\n");

            for (i, tex) in metadata.textures.iter().enumerate() {
                header.push_str(&format!("@group(0) @binding({}) var _texture_{}: texture_2d<f32>; // {}\n", i + 1, i, tex));
            }

            header.push_str("\n@group(1) @binding(0) var<storage, read_write> _engine: GameEngineHost;\n\n");

            // Remove GameState from source
            if game_state_struct.is_some() {
                source = game_state_re.replace(&source, "").to_string();
            }
        }

        // Replace macros
        source = source.replace("@engine.buttons", "_engine.buttons");
        source = source.replace("@engine.time", "_engine.time");
        source = source.replace("@engine.delta_time", "_engine.delta_time");
        source = source.replace("@engine.screen_width", "_engine.screen_width");
        source = source.replace("@engine.screen_height", "_engine.screen_height");
        source = source.replace("@engine.sampler", "_engine_sampler");

        // Replace game_state. with _engine.state.
        let game_state_re = Regex::new(r"\bgame_state\.")?;
        source = game_state_re.replace_all(&source, "_engine.state.").to_string();

        // Replace @sound().play() and @sound().stop()
        for (i, sound) in metadata.sounds.iter().enumerate() {
            let escaped = sound.replace(".", "\\.");
            let play_re = Regex::new(&format!(r#"@sound\("{}"\)\.play\(\)"#, escaped))?;
            source = play_re.replace_all(&source, &format!("_engine.audio[{}]++", i)).to_string();

            let stop_re = Regex::new(&format!(r#"@sound\("{}"\)\.stop\(\)"#, escaped))?;
            source = stop_re.replace_all(&source, &format!("/* stop sound {} - not implemented */", i)).to_string();

            // Legacy @sound() syntax
            let legacy_re = Regex::new(&format!(r#"@sound\("{}"\)"#, escaped))?;
            source = legacy_re.replace_all(&source, &format!("_engine.audio[{}]", i)).to_string();
        }

        // Replace @texture()
        for (i, texture) in metadata.textures.iter().enumerate() {
            let escaped = texture.replace(".", "\\.");
            let texture_re = Regex::new(&format!(r#"@texture\("{}"\)"#, escaped))?;
            source = texture_re.replace_all(&source, &format!("_texture_{}", i)).to_string();
        }

        Ok((header + &source, metadata))
    }
}

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

        // Create pipelines
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
        // Only create group 0 if we have textures (sampler is only needed with textures)
        let mut group0_entries = vec![];

        if !texture_views.is_empty() {
            group0_entries.push(wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&sampler),
            });

            for (i, view) in texture_views.iter().enumerate() {
                group0_entries.push(wgpu::BindGroupEntry {
                    binding: (i + 1) as u32,
                    resource: wgpu::BindingResource::TextureView(view),
                });
            }
        }

        let render_bind_group0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group 0"),
            layout: &render_pipeline.get_bind_group_layout(0),
            entries: &group0_entries,
        });

        let render_bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group 1"),
            layout: &render_pipeline.get_bind_group_layout(1),
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
                    KeyCode::ArrowUp | KeyCode::KeyW => self.buttons[BTN_UP] = value,
                    KeyCode::ArrowDown | KeyCode::KeyS => self.buttons[BTN_DOWN] = value,
                    KeyCode::ArrowLeft | KeyCode::KeyA => self.buttons[BTN_LEFT] = value,
                    KeyCode::ArrowRight | KeyCode::KeyD => self.buttons[BTN_RIGHT] = value,
                    KeyCode::KeyZ | KeyCode::KeyK => self.buttons[BTN_A] = value,
                    KeyCode::KeyX | KeyCode::KeyL => self.buttons[BTN_B] = value,
                    KeyCode::KeyC | KeyCode::KeyI => self.buttons[BTN_X] = value,
                    KeyCode::KeyV | KeyCode::KeyJ => self.buttons[BTN_Y] = value,
                    KeyCode::KeyQ | KeyCode::KeyU => self.buttons[BTN_L] = value,
                    KeyCode::KeyE | KeyCode::KeyO => self.buttons[BTN_R] = value,
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
