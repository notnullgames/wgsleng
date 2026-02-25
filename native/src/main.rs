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
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rosc::{OscPacket, OscType};
use std::collections::HashMap;
use wgsleng::{GameSource, PreprocessorState, OSC_FLOAT_COUNT,
    BTN_UP, BTN_DOWN, BTN_LEFT, BTN_RIGHT, BTN_A, BTN_B, BTN_X, BTN_Y, BTN_L, BTN_R, BTN_START, BTN_SELECT};

enum OscMessage {
    /// /u/name value  or  /u/N value
    SetFloat(String, f32),
    /// /vid/<filename>/position 0.0-1.0
    SetVideoPosition(String, f32),
    /// /shader filename.wgsl
    LoadShader(String),
    /// /reload
    Reload,
}

#[derive(Parser, Debug)]
#[command(name = "wgsl-game")]
#[command(about = "Run WGSL shader games from directory or zip file")]
struct Args {
    /// Path to game.wgsl file or .zip containing main.wgsl
    game_path: String,

    /// Watch for file changes and hot-reload shader/textures (directory sources only)
    #[arg(long, short = 'r')]
    hot_reload: bool,

    /// Listen for OSC messages on this UDP port (e.g. --osc-port 9000)
    #[arg(long)]
    osc_port: Option<u16>,
}

// All preprocessing logic is now in lib.rs

/// Runtime state for a @video() source
enum VideoSourceRuntime {
    Gif {
        frames: Vec<(Vec<u8>, u32)>, // (rgba_bytes, delay_ms)
        width: u32,
        height: u32,
        current_frame: usize,
        frame_elapsed_ms: f32,
    },
    Black(u32, u32),
}

/// Runtime state for a @camera() source
enum CameraSourceRuntime {
    #[cfg(feature = "camera")]
    Live {
        latest_frame: Arc<std::sync::Mutex<Option<Vec<u8>>>>,
        width: u32,
        height: u32,
        stop: Arc<std::sync::atomic::AtomicBool>,
    },
    Black(u32, u32),
}

fn load_gif_source(data: &[u8]) -> Result<(VideoSourceRuntime, u32, u32), Box<dyn std::error::Error>> {
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;
    let decoder = GifDecoder::new(Cursor::new(data))?;
    let mut frames_vec: Vec<(Vec<u8>, u32)> = Vec::new();
    let mut width = 1u32;
    let mut height = 1u32;
    for frame_result in decoder.into_frames() {
        let frame = frame_result?;
        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay_ms = if denom == 0 { 100 } else { (numer / denom).max(10) };
        let img = frame.into_buffer();
        width = img.width();
        height = img.height();
        frames_vec.push((img.into_raw(), delay_ms));
    }
    if frames_vec.is_empty() {
        return Ok((VideoSourceRuntime::Black(1, 1), 1, 1));
    }
    Ok((VideoSourceRuntime::Gif { frames: frames_vec, width, height, current_frame: 0, frame_elapsed_ms: 0.0 }, width, height))
}

/// Decode an arbitrary video file using the system `ffmpeg` CLI.
///
/// Pre-decodes all frames into memory for instant seeking.
/// Works for MP4, WebM, MOV, MKV — anything ffmpeg supports.
fn open_ffmpeg_video(filename: &str, data: Vec<u8>) -> (VideoSourceRuntime, u32, u32) {
    use std::process::{Command, Stdio};
    use std::io::Read;

    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4")
        .to_lowercase();
    let tmp_path = std::env::temp_dir().join(format!("wgsleng_video_{}.{}", std::process::id(), ext));
    if let Err(e) = std::fs::write(&tmp_path, &data) {
        eprintln!("[video] failed to write temp file for {}: {}", filename, e);
        return (VideoSourceRuntime::Black(1, 1), 1, 1);
    }

    // Get dimensions and frame rate via ffprobe
    let probe = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0",
               "-show_entries", "stream=width,height,r_frame_rate", "-of", "csv=p=0",
               tmp_path.to_str().unwrap()])
        .output();

    let (width, height, fps) = match probe {
        Err(e) => {
            eprintln!("[video] ffprobe not found ({}), using black for '{}'", e, filename);
            let _ = std::fs::remove_file(&tmp_path);
            return (VideoSourceRuntime::Black(1, 1), 1, 1);
        }
        Ok(out) => {
            let s = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = s.trim().split(',').collect();
            if parts.len() < 3 {
                eprintln!("[video] ffprobe gave unexpected output for '{}': {:?}", filename, s);
                let _ = std::fs::remove_file(&tmp_path);
                return (VideoSourceRuntime::Black(1, 1), 1, 1);
            }
            let w: u32 = parts[0].trim().parse().unwrap_or(1);
            let h: u32 = parts[1].trim().parse().unwrap_or(1);
            // r_frame_rate is like "30000/1001" or "30/1"
            let fps: f32 = {
                let fr = parts[2].trim();
                if let Some((n, d)) = fr.split_once('/') {
                    let num: f32 = n.parse().unwrap_or(30.0);
                    let den: f32 = d.parse().unwrap_or(1.0);
                    if den == 0.0 { 30.0 } else { num / den }
                } else {
                    fr.parse().unwrap_or(30.0)
                }
            };
            (w, h, fps)
        }
    };

    let delay_ms = ((1000.0 / fps.max(1.0)) as u32).max(1);
    let frame_bytes = (width * height * 4) as usize;

    // Decode all frames as fast as possible (no -re)
    let decode = Command::new("ffmpeg")
        .args([
            "-i", tmp_path.to_str().unwrap(),
            "-f", "rawvideo", "-pix_fmt", "rgba", "-vcodec", "rawvideo",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let mut frames_vec: Vec<(Vec<u8>, u32)> = Vec::new();

    match decode {
        Err(e) => {
            eprintln!("[video] ffmpeg not found ({}), using black for '{}'", e, filename);
            let _ = std::fs::remove_file(&tmp_path);
            return (VideoSourceRuntime::Black(1, 1), 1, 1);
        }
        Ok(mut child) => {
            let mut stdout = child.stdout.take().unwrap();
            let mut buf = vec![0u8; frame_bytes];
            loop {
                let mut total = 0;
                let mut eof = false;
                while total < frame_bytes {
                    match stdout.read(&mut buf[total..]) {
                        Ok(0) => { eof = true; break; }
                        Ok(n) => total += n,
                        Err(_) => { eof = true; break; }
                    }
                }
                if eof || total < frame_bytes { break; }
                frames_vec.push((buf.clone(), delay_ms));
            }
            let _ = child.wait();
        }
    }

    let _ = std::fs::remove_file(&tmp_path);

    if frames_vec.is_empty() {
        eprintln!("[video] no frames decoded for '{}'", filename);
        return (VideoSourceRuntime::Black(width, height), width, height);
    }

    eprintln!("[video] pre-decoded '{}' ({} frames, {}x{}, {:.1}fps)", filename, frames_vec.len(), width, height, fps);
    (VideoSourceRuntime::Gif { frames: frames_vec, width, height, current_frame: 0, frame_elapsed_ms: 0.0 }, width, height)
}

fn load_video_source(filename: &str, data: Vec<u8>) -> (VideoSourceRuntime, u32, u32) {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "gif" {
        match load_gif_source(&data) {
            Ok(result) => return result,
            Err(e) => eprintln!("[video] failed to load GIF {}: {}", filename, e),
        }
    }

    // For anything other than GIF, try the system ffmpeg CLI
    open_ffmpeg_video(filename, data)
}

fn open_camera_source(cam_idx: u32) -> (CameraSourceRuntime, u32, u32) {
    #[cfg(feature = "camera")]
    {
        use nokhwa::{Camera, pixel_format::RgbAFormat, utils::{CameraIndex, RequestedFormat, RequestedFormatType}};
        let index = CameraIndex::Index(cam_idx);
        let requested = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        match Camera::new(index, requested) {
            Ok(mut camera) => {
                let res = camera.resolution();
                let width = res.width_x;
                let height = res.height_y;
                let latest_frame: Arc<std::sync::Mutex<Option<Vec<u8>>>> = Arc::new(std::sync::Mutex::new(None));
                let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let frame_clone = Arc::clone(&latest_frame);
                let stop_clone = Arc::clone(&stop);
                std::thread::spawn(move || {
                    if let Err(e) = camera.open_stream() {
                        eprintln!("[camera] failed to open stream for camera {}: {}", cam_idx, e);
                        return;
                    }
                    while !stop_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        match camera.frame() {
                            Ok(buffer) => {
                                if let Ok(decoded) = buffer.decode_image::<RgbAFormat>() {
                                    *frame_clone.lock().unwrap() = Some(decoded.into_raw());
                                }
                            }
                            Err(e) => {
                                eprintln!("[camera] frame error on camera {}: {}", cam_idx, e);
                                break;
                            }
                        }
                    }
                });
                return (CameraSourceRuntime::Live { latest_frame, width, height, stop }, width, height);
            }
            Err(e) => eprintln!("[camera] failed to open camera {}: {}", cam_idx, e),
        }
    }
    #[cfg(not(feature = "camera"))]
    eprintln!("[camera] camera feature not enabled for camera index {}", cam_idx);
    (CameraSourceRuntime::Black(640, 480), 640, 480)
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
    render_bind_group2: Option<wgpu::BindGroup>,
    engine_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    buffer_offsets: BufferOffsets,
    buttons: [i32; 12],
    last_time: std::time::Instant,
    time: f32,
    model_vertex_count: usize,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sound_buffers: Vec<Vec<u8>>,
    audio_count: usize,
    // For hot-reload state preservation
    engine_buffer_size: usize,
    // OSC name → osc slot index mapping (populated from @osc("name") in shader)
    osc_name_map: HashMap<String, usize>,
    // Dynamic video textures
    video_textures: Vec<wgpu::Texture>,
    video_sources: Vec<VideoSourceRuntime>,
    video_filenames: Vec<String>,
    // Dynamic camera textures
    camera_textures: Vec<wgpu::Texture>,
    camera_sources: Vec<CameraSourceRuntime>,
}

struct BufferOffsets {
    buttons: u64,
    floats: u64,
    state: u64,
    audio: u64,
    osc_floats: u64,
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
            .find(|f| !f.is_srgb())
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

        // Create depth texture for 3D rendering
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: metadata.width,
                height: metadata.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Load audio
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let mut sound_buffers = Vec::new();
        for sound_file in &metadata.sounds {
            let data = preprocessor.game_source.read_file(sound_file)?;
            sound_buffers.push(data);
        }

        // Load models
        let mut models = Vec::new();
        let mut model_vertex_counts = Vec::new();
        for model_file in &metadata.models {
            let model_data = preprocessor.game_source.read_file(model_file)?;
            let model_path = std::path::PathBuf::from(model_file);

            // Write to temp file for OBJ loader
            let temp_path = std::env::temp_dir().join(model_path.file_name().unwrap());
            std::fs::write(&temp_path, model_data)?;

            let model = wgsleng::ObjModel::load(&temp_path)?;
            model_vertex_counts.push(model.vertex_count());

            // Create positions buffer
            // IMPORTANT: array<vec3f> in WGSL storage buffers has 16-byte alignment (like vec4)
            let positions_data: Vec<f32> = model.positions.iter()
                .flat_map(|p| [p[0], p[1], p[2], 0.0]) // Add padding
                .collect();

            let positions_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model Positions"),
                contents: bytemuck::cast_slice(&positions_data),
                usage: wgpu::BufferUsages::STORAGE,
            });

            // Create normals buffer
            // Same padding required for normals
            let normals_data: Vec<f32> = model.normals.iter()
                .flat_map(|n| [n[0], n[1], n[2], 0.0]) // Add padding
                .collect();

            let normals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model Normals"),
                contents: bytemuck::cast_slice(&normals_data),
                usage: wgpu::BufferUsages::STORAGE,
            });

            models.push((positions_buffer, normals_buffer));
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
                format: wgpu::TextureFormat::Rgba8Unorm,
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

        // Load video sources
        let mut video_textures = Vec::new();
        let mut video_sources: Vec<VideoSourceRuntime> = Vec::new();
        for video_file in &metadata.videos {
            let data = preprocessor.game_source.read_file(video_file)?;
            let (source, vid_w, vid_h) = load_video_source(video_file, data);
            let (init_data, vid_w, vid_h) = match &source {
                VideoSourceRuntime::Gif { frames, width, height, current_frame, .. } =>
                    (frames[*current_frame].0.clone(), *width, *height),
                VideoSourceRuntime::Black(w, h) =>
                    (vec![0u8; (*w * *h * 4) as usize], *w, *h),
            };
            let tex_size = wgpu::Extent3d { width: vid_w, height: vid_h, depth_or_array_layers: 1 };
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Video Texture"),
                size: tex_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::ImageCopyTexture { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                &init_data,
                wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * vid_w), rows_per_image: Some(vid_h) },
                tex_size,
            );
            video_textures.push(tex);
            video_sources.push(source);
        }

        // Open camera sources
        let mut camera_textures = Vec::new();
        let mut camera_sources: Vec<CameraSourceRuntime> = Vec::new();
        for &cam_idx in &metadata.cameras {
            let (source, cam_w, cam_h) = open_camera_source(cam_idx);
            let black_data = vec![0u8; (cam_w * cam_h * 4) as usize];
            let cam_size = wgpu::Extent3d { width: cam_w, height: cam_h, depth_or_array_layers: 1 };
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Camera Texture"),
                size: cam_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::ImageCopyTexture { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                &black_data,
                wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * cam_w), rows_per_image: Some(cam_h) },
                cam_size,
            );
            camera_textures.push(tex);
            camera_sources.push(source);
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
        let osc_floats_offset = button_size + float_data_size + aligned_state_size + audio_size;
        let total_size_unaligned = osc_floats_offset + OSC_FLOAT_COUNT * 4;
        let total_size = ((total_size_unaligned + 15) / 16) * 16;

        let buffer_offsets = BufferOffsets {
            buttons: 0,
            floats: button_size as u64,
            state: (button_size + float_data_size) as u64,
            audio: (button_size + float_data_size + aligned_state_size) as u64,
            osc_floats: osc_floats_offset as u64,
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

        // Add video texture bindings
        let video_base = metadata.textures.len() + 1;
        for i in 0..metadata.videos.len() {
            render_group0_entries.push(wgpu::BindGroupLayoutEntry {
                binding: (video_base + i) as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }

        // Add camera texture bindings
        let camera_base = metadata.textures.len() + metadata.videos.len() + 1;
        for i in 0..metadata.cameras.len() {
            render_group0_entries.push(wgpu::BindGroupLayoutEntry {
                binding: (camera_base + i) as u32,
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

        // Group 1: engine buffer (read_write for fragment, not accessible in vertex)
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

        // Group 2: model buffers (positions and normals for each model)
        let mut model_group_entries = Vec::new();
        for i in 0..metadata.models.len() {
            let binding_base = 1 + i * 2;
            // Positions buffer
            model_group_entries.push(wgpu::BindGroupLayoutEntry {
                binding: binding_base as u32,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
            // Normals buffer
            model_group_entries.push(wgpu::BindGroupLayoutEntry {
                binding: (binding_base + 1) as u32,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
        }

        let render_bind_group_layout2 = if !model_group_entries.is_empty() {
            Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Bind Group Layout 2"),
                entries: &model_group_entries,
            }))
        } else {
            None
        };

        let mut render_layouts: Vec<&wgpu::BindGroupLayout> = vec![&render_bind_group_layout0, &render_bind_group_layout1];
        if let Some(ref layout2) = render_bind_group_layout2 {
            render_layouts.push(layout2);
        }

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &render_layouts,
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
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
        let video_texture_views: Vec<_> = video_textures.iter()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect();
        let camera_texture_views: Vec<_> = camera_textures.iter()
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

        // Add video texture views
        for (i, view) in video_texture_views.iter().enumerate() {
            group0_entries.push(wgpu::BindGroupEntry {
                binding: (video_base + i) as u32,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }

        // Add camera texture views
        for (i, view) in camera_texture_views.iter().enumerate() {
            group0_entries.push(wgpu::BindGroupEntry {
                binding: (camera_base + i) as u32,
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

        // Create model bind group if models exist
        let render_bind_group2 = if let Some(ref layout2) = render_bind_group_layout2 {
            let mut model_entries = Vec::new();
            for (i, (positions_buf, normals_buf)) in models.iter().enumerate() {
                let binding_base = 1 + i * 2;
                model_entries.push(wgpu::BindGroupEntry {
                    binding: binding_base as u32,
                    resource: positions_buf.as_entire_binding(),
                });
                model_entries.push(wgpu::BindGroupEntry {
                    binding: (binding_base + 1) as u32,
                    resource: normals_buf.as_entire_binding(),
                });
            }
            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Render Bind Group 2"),
                layout: layout2,
                entries: &model_entries,
            }))
        } else {
            None
        };

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
            render_bind_group2,
            engine_buffer,
            staging_buffer,
            buffer_offsets,
            buttons: [0; 12],
            last_time: std::time::Instant::now(),
            time: 0.0,
            model_vertex_count: model_vertex_counts.get(0).copied().unwrap_or(0),
            depth_texture,
            depth_view,
            _stream,
            stream_handle,
            sound_buffers,
            audio_count: metadata.sounds.len(),
            engine_buffer_size: total_size,
            osc_name_map: metadata.osc_params.iter().cloned().zip(0..).collect(),
            video_textures,
            video_sources,
            video_filenames: metadata.videos.clone(),
            camera_textures,
            camera_sources,
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

    fn update_dynamic_textures(&mut self, dt_secs: f32) {
        // Update GIF video frames
        for i in 0..self.video_sources.len() {
            let maybe_write: Option<(Vec<u8>, u32, u32)> = match &mut self.video_sources[i] {
                VideoSourceRuntime::Gif { frames, width, height, current_frame, frame_elapsed_ms } => {
                    *frame_elapsed_ms += dt_secs * 1000.0;
                    let prev = *current_frame;
                    loop {
                        let delay = frames[*current_frame].1 as f32;
                        if *frame_elapsed_ms >= delay {
                            *frame_elapsed_ms -= delay;
                            *current_frame = (*current_frame + 1) % frames.len();
                        } else {
                            break;
                        }
                    }
                    // Only upload when the frame actually changed
                    if *current_frame != prev {
                        Some((frames[*current_frame].0.clone(), *width, *height))
                    } else {
                        None
                    }
                }
                VideoSourceRuntime::Black(_, _) => None,
            };
            if let Some((data, w, h)) = maybe_write {
                self.queue.write_texture(
                    wgpu::ImageCopyTexture { texture: &self.video_textures[i], mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                    &data,
                    wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
                    wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                );
            }
        }

        // Update camera frames
        for i in 0..self.camera_sources.len() {
            let maybe_write: Option<(Vec<u8>, u32, u32)> = match &mut self.camera_sources[i] {
                #[cfg(feature = "camera")]
                CameraSourceRuntime::Live { latest_frame, width, height, .. } => {
                    latest_frame.try_lock().ok()
                        .and_then(|mut g| g.take())
                        .map(|data| (data, *width, *height))
                }
                CameraSourceRuntime::Black(_, _) => None,
            };
            if let Some((data, w, h)) = maybe_write {
                self.queue.write_texture(
                    wgpu::ImageCopyTexture { texture: &self.camera_textures[i], mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                    &data,
                    wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
                    wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                );
            }
        }
    }

    fn update(&mut self) {
        let now = std::time::Instant::now();
        let dt = (now - self.last_time).as_secs_f32();
        let dt = dt.min(0.1);
        self.last_time = now;
        self.time += dt;

        // Update dynamic textures (video frames + camera frames)
        self.update_dynamic_textures(dt);

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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group0, &[]);
            render_pass.set_bind_group(1, &self.render_bind_group1, &[]);
            if let Some(ref bind_group2) = self.render_bind_group2 {
                render_pass.set_bind_group(2, bind_group2, &[]);
            }

            // Draw either model vertices or fullscreen triangle
            let vertex_count = if self.model_vertex_count > 0 {
                self.model_vertex_count as u32
            } else {
                3  // Fullscreen triangle
            };
            render_pass.draw(0..vertex_count, 0..1);
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

    /// Apply an OSC message by writing directly into the engine buffer.
    fn apply_osc_message(&mut self, msg: &OscMessage) {
        match msg {
            OscMessage::SetFloat(name, value) => {
                // Try name lookup first, then parse as numeric index
                let idx = self.osc_name_map.get(name).copied().or_else(|| name.parse::<usize>().ok());
                match idx {
                    Some(i) if i < OSC_FLOAT_COUNT => {
                        let offset = self.buffer_offsets.osc_floats + (i * 4) as u64;
                        self.queue.write_buffer(&self.engine_buffer, offset, &value.to_le_bytes());
                    }
                    Some(i) => log::warn!("[osc] /u/{} index {} out of range (max {})", name, i, OSC_FLOAT_COUNT - 1),
                    None => log::warn!("[osc] /u/{} not declared with @osc(\"{}\") in shader", name, name),
                }
            }
            OscMessage::SetVideoPosition(filename, position) => {
                if let Some(idx) = self.video_filenames.iter().position(|f| f == filename) {
                    // Gather what we need (may clone frame data for GIF) before touching queue
                    let gif_frame: Option<(Vec<u8>, u32, u32)> = match &mut self.video_sources[idx] {
                        VideoSourceRuntime::Gif { frames, current_frame, frame_elapsed_ms, width, height } => {
                            let new_frame = ((*position * frames.len() as f32) as usize)
                                .min(frames.len().saturating_sub(1));
                            *current_frame = new_frame;
                            *frame_elapsed_ms = 0.0;
                            Some((frames[new_frame].0.clone(), *width, *height))
                        }
                        VideoSourceRuntime::Black(_, _) => None,
                    };
                    // Upload the new GIF frame immediately so the seek is visible this frame
                    if let Some((data, w, h)) = gif_frame {
                        self.queue.write_texture(
                            wgpu::ImageCopyTexture { texture: &self.video_textures[idx], mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                            &data,
                            wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
                            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                        );
                    }
                } else {
                    log::warn!("[osc] /vid/{}/position: no video named '{}' loaded", filename, filename);
                }
            }
            // LoadShader and Reload are handled at the App level
            _ => {}
        }
    }

    /// Read the GameState section from the GPU buffer so we can restore it after reload.
    fn read_game_state_bytes(&self) -> Vec<u8> {
        let state_offset = self.buffer_offsets.state;
        let state_size = if self.audio_count > 0 {
            (self.buffer_offsets.audio - state_offset) as usize
        } else {
            self.engine_buffer_size.saturating_sub(state_offset as usize)
        };

        if state_size == 0 {
            return Vec::new();
        }

        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("State Readback"),
            size: state_size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(&self.engine_buffer, state_offset, &readback, 0, state_size as u64);
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::Maintain::Wait);

        if let Ok(Ok(())) = pollster::block_on(rx) {
            let data = slice.get_mapped_range();
            let result = data.to_vec();
            drop(data);
            readback.unmap();
            result
        } else {
            vec![0u8; state_size]
        }
    }

    /// Hot-reload: re-preprocess shader, rebuild pipelines and textures, preserve GameState.
    fn reload(&mut self, game_path: &str, entry_file: &str) -> Result<(), Box<dyn std::error::Error>> {

        // Signal camera threads to stop before rebuilding
        #[cfg(feature = "camera")]
        for source in &self.camera_sources {
            if let CameraSourceRuntime::Live { stop, .. } = source {
                stop.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }

        // Save GameState bytes before rebuilding
        let saved_state = self.read_game_state_bytes();
        let old_state_size = saved_state.len();

        // Re-open game source
        let mut game_source = GameSource::open(game_path)?;

        // Preprocess shader
        let shader_code = game_source.read_text(entry_file)?;
        let mut preprocessor = PreprocessorState::new(game_source);
        let (processed_code, metadata) = preprocessor.preprocess_shader(&shader_code, true)?;

        println!("[hot-reload] shader preprocessed ({}x{}, {} textures)", metadata.width, metadata.height, metadata.textures.len());

        // Load audio
        let mut sound_buffers = Vec::new();
        for sound_file in &metadata.sounds {
            match preprocessor.game_source.read_file(sound_file) {
                Ok(data) => sound_buffers.push(data),
                Err(e) => eprintln!("[hot-reload] warning: failed to load sound {}: {}", sound_file, e),
            }
        }

        // Load models
        let mut models: Vec<(wgpu::Buffer, wgpu::Buffer)> = Vec::new();
        let mut model_vertex_counts: Vec<usize> = Vec::new();
        for model_file in &metadata.models {
            let model_data = preprocessor.game_source.read_file(model_file)?;
            let model_path = std::path::PathBuf::from(model_file);
            let temp_path = std::env::temp_dir().join(model_path.file_name().unwrap());
            std::fs::write(&temp_path, model_data)?;
            let model = wgsleng::ObjModel::load(&temp_path)?;
            model_vertex_counts.push(model.vertex_count());

            let positions_data: Vec<f32> = model.positions.iter()
                .flat_map(|p| [p[0], p[1], p[2], 0.0])
                .collect();
            let positions_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model Positions"),
                contents: bytemuck::cast_slice(&positions_data),
                usage: wgpu::BufferUsages::STORAGE,
            });

            let normals_data: Vec<f32> = model.normals.iter()
                .flat_map(|n| [n[0], n[1], n[2], 0.0])
                .collect();
            let normals_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model Normals"),
                contents: bytemuck::cast_slice(&normals_data),
                usage: wgpu::BufferUsages::STORAGE,
            });
            models.push((positions_buffer, normals_buffer));
        }

        // Load textures
        let mut textures: Vec<wgpu::Texture> = Vec::new();
        for texture_file in &metadata.textures {
            let img_data = preprocessor.game_source.read_file(texture_file)?;
            let img = image::load_from_memory(&img_data)?.to_rgba8();
            let dimensions = img.dimensions();
            let texture_size = wgpu::Extent3d { width: dimensions.0, height: dimensions.1, depth_or_array_layers: 1 };
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Game Texture"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                wgpu::ImageCopyTexture { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                &img,
                wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * dimensions.0), rows_per_image: Some(dimensions.1) },
                texture_size,
            );
            textures.push(texture);
        }

        // Load video sources
        let mut new_video_textures: Vec<wgpu::Texture> = Vec::new();
        let mut new_video_sources: Vec<VideoSourceRuntime> = Vec::new();
        for video_file in &metadata.videos {
            let data = match preprocessor.game_source.read_file(video_file) {
                Ok(d) => d,
                Err(e) => { eprintln!("[hot-reload] warning: failed to load video {}: {}", video_file, e); Vec::new() }
            };
            let (source, vid_w, vid_h) = if data.is_empty() {
                (VideoSourceRuntime::Black(1, 1), 1u32, 1u32)
            } else {
                load_video_source(video_file, data)
            };
            let (init_data, vid_w, vid_h) = match &source {
                VideoSourceRuntime::Gif { frames, width, height, current_frame, .. } =>
                    (frames[*current_frame].0.clone(), *width, *height),
                VideoSourceRuntime::Black(w, h) =>
                    (vec![0u8; (*w * *h * 4) as usize], *w, *h),
            };
            let tex_size = wgpu::Extent3d { width: vid_w, height: vid_h, depth_or_array_layers: 1 };
            let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Video Texture"),
                size: tex_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                wgpu::ImageCopyTexture { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                &init_data,
                wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * vid_w), rows_per_image: Some(vid_h) },
                tex_size,
            );
            new_video_textures.push(tex);
            new_video_sources.push(source);
        }

        // Open camera sources
        let mut new_camera_textures: Vec<wgpu::Texture> = Vec::new();
        let mut new_camera_sources: Vec<CameraSourceRuntime> = Vec::new();
        for &cam_idx in &metadata.cameras {
            let (source, cam_w, cam_h) = open_camera_source(cam_idx);
            let black_data = vec![0u8; (cam_w * cam_h * 4) as usize];
            let cam_size = wgpu::Extent3d { width: cam_w, height: cam_h, depth_or_array_layers: 1 };
            let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Camera Texture"),
                size: cam_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                wgpu::ImageCopyTexture { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                &black_data,
                wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * cam_w), rows_per_image: Some(cam_h) },
                cam_size,
            );
            new_camera_textures.push(tex);
            new_camera_sources.push(source);
        }

        // Compute buffer layout (same logic as State::new)
        let button_size = 12 * 4usize;
        let float_data_size = 4 * 4usize;
        let state_alignment = 8usize;
        let aligned_state_size = ((metadata.state_size + state_alignment - 1) / state_alignment) * state_alignment;
        let audio_size = metadata.sounds.len() * 4;
        let osc_floats_offset = button_size + float_data_size + aligned_state_size + audio_size;
        let total_size_unaligned = osc_floats_offset + OSC_FLOAT_COUNT * 4;
        let total_size = ((total_size_unaligned + 15) / 16) * 16;

        let new_buffer_offsets = BufferOffsets {
            buttons: 0,
            floats: button_size as u64,
            state: (button_size + float_data_size) as u64,
            audio: (button_size + float_data_size + aligned_state_size) as u64,
            osc_floats: osc_floats_offset as u64,
        };

        let new_state_size = if metadata.sounds.len() > 0 {
            (new_buffer_offsets.audio - new_buffer_offsets.state) as usize
        } else {
            osc_floats_offset.saturating_sub(new_buffer_offsets.state as usize)
        };

        // Build new engine buffer, preserving GameState if sizes match
        let mut init_data = vec![0u8; total_size];
        let w_bytes = (self.config.width as f32).to_le_bytes();
        let h_bytes = (self.config.height as f32).to_le_bytes();
        let f = new_buffer_offsets.floats as usize;
        init_data[f + 8..f + 12].copy_from_slice(&w_bytes);
        init_data[f + 12..f + 16].copy_from_slice(&h_bytes);

        if new_state_size == old_state_size && !saved_state.is_empty() {
            let ss = new_buffer_offsets.state as usize;
            let se = ss + new_state_size;
            if se <= init_data.len() {
                init_data[ss..se].copy_from_slice(&saved_state);
                println!("[hot-reload] GameState preserved ({} bytes)", new_state_size);
            }
        } else if new_state_size != old_state_size {
            println!("[hot-reload] GameState size changed ({} -> {}), resetting state", old_state_size, new_state_size);
        }

        let engine_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Engine Buffer"),
            contents: &init_data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        });

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: total_size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Sampler
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Bind group layouts (same as State::new)
        let mut render_group0_entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }];
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
        let reload_video_base = metadata.textures.len() + 1;
        for i in 0..metadata.videos.len() {
            render_group0_entries.push(wgpu::BindGroupLayoutEntry {
                binding: (reload_video_base + i) as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        let reload_camera_base = metadata.textures.len() + metadata.videos.len() + 1;
        for i in 0..metadata.cameras.len() {
            render_group0_entries.push(wgpu::BindGroupLayoutEntry {
                binding: (reload_camera_base + i) as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        let render_bind_group_layout0 = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout 0"),
            entries: &render_group0_entries,
        });
        let render_bind_group_layout1 = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let mut model_group_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
        for i in 0..metadata.models.len() {
            let bb = 1 + i * 2;
            model_group_entries.push(wgpu::BindGroupLayoutEntry {
                binding: bb as u32,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            });
            model_group_entries.push(wgpu::BindGroupLayoutEntry {
                binding: (bb + 1) as u32,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            });
        }
        let render_bind_group_layout2 = if !model_group_entries.is_empty() {
            Some(self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Bind Group Layout 2"),
                entries: &model_group_entries,
            }))
        } else {
            None
        };

        let empty_bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Empty Bind Group Layout"),
            entries: &[],
        });
        let compute_bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });

        // Create pipelines inside an error scope to catch shader errors gracefully
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Game Shader"),
            source: wgpu::ShaderSource::Wgsl(processed_code.into()),
        });

        let mut render_layouts: Vec<&wgpu::BindGroupLayout> = vec![&render_bind_group_layout0, &render_bind_group_layout1];
        if let Some(ref layout2) = render_bind_group_layout2 {
            render_layouts.push(layout2);
        }
        let render_pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &render_layouts,
            push_constant_ranges: &[],
        });
        let compute_pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&empty_bind_group_layout, &compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_render"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let compute_pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: Some("update"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Check for shader/pipeline errors
        let pipeline_error = pollster::block_on(self.device.pop_error_scope());
        if let Some(err) = pipeline_error {
            eprintln!("[hot-reload] shader error, keeping old pipelines:\n  {}", err);
            return Err(format!("shader error: {}", err).into());
        }

        // Build bind groups with new resources
        let texture_views: Vec<_> = textures.iter()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect();
        let new_video_views: Vec<_> = new_video_textures.iter()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect();
        let new_camera_views: Vec<_> = new_camera_textures.iter()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect();

        let mut group0_entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Sampler(&sampler),
        }];
        for (i, view) in texture_views.iter().enumerate() {
            group0_entries.push(wgpu::BindGroupEntry {
                binding: (i + 1) as u32,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        for (i, view) in new_video_views.iter().enumerate() {
            group0_entries.push(wgpu::BindGroupEntry {
                binding: (reload_video_base + i) as u32,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        for (i, view) in new_camera_views.iter().enumerate() {
            group0_entries.push(wgpu::BindGroupEntry {
                binding: (reload_camera_base + i) as u32,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        let render_bind_group0 = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group 0"),
            layout: &render_bind_group_layout0,
            entries: &group0_entries,
        });
        let render_bind_group1 = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group 1"),
            layout: &render_bind_group_layout1,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: engine_buffer.as_entire_binding() }],
        });
        let render_bind_group2 = if let Some(ref layout2) = render_bind_group_layout2 {
            let mut model_entries = Vec::new();
            for (i, (pos_buf, norm_buf)) in models.iter().enumerate() {
                let bb = 1 + i * 2;
                model_entries.push(wgpu::BindGroupEntry { binding: bb as u32, resource: pos_buf.as_entire_binding() });
                model_entries.push(wgpu::BindGroupEntry { binding: (bb + 1) as u32, resource: norm_buf.as_entire_binding() });
            }
            Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Render Bind Group 2"),
                layout: layout2,
                entries: &model_entries,
            }))
        } else {
            None
        };
        let empty_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Empty Bind Group"),
            layout: &empty_bind_group_layout,
            entries: &[],
        });
        let compute_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: engine_buffer.as_entire_binding() }],
        });

        // Recreate depth texture to match current surface size
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d { width: self.config.width, height: self.config.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Atomically replace all pipeline state
        self.compute_pipeline = compute_pipeline;
        self.render_pipeline = render_pipeline;
        self.empty_bind_group = empty_bind_group;
        self.compute_bind_group = compute_bind_group;
        self.render_bind_group0 = render_bind_group0;
        self.render_bind_group1 = render_bind_group1;
        self.render_bind_group2 = render_bind_group2;
        self.engine_buffer = engine_buffer;
        self.staging_buffer = staging_buffer;
        self.buffer_offsets = new_buffer_offsets;
        self.sound_buffers = sound_buffers;
        self.audio_count = metadata.sounds.len();
        self.model_vertex_count = model_vertex_counts.get(0).copied().unwrap_or(0);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
        self.engine_buffer_size = total_size;
        self.osc_name_map = metadata.osc_params.iter().cloned().zip(0..).collect();
        self.video_textures = new_video_textures;
        self.video_sources = new_video_sources;
        self.video_filenames = metadata.videos.clone();
        self.camera_textures = new_camera_textures;
        self.camera_sources = new_camera_sources;

        println!("[hot-reload] done");
        Ok(())
    }
}

struct App {
    state: Option<State>,
    game_source: Option<GameSource>,
    entry_file: String,
    game_path: String,
    hot_reload_rx: Option<std::sync::mpsc::Receiver<()>>,
    _watcher: Option<RecommendedWatcher>,
    osc_rx: Option<std::sync::mpsc::Receiver<OscMessage>>,
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
        // Process OSC messages
        if let Some(ref osc_rx) = self.osc_rx {
            while let Ok(msg) = osc_rx.try_recv() {
                match msg {
                    OscMessage::LoadShader(ref entry) => {
                        println!("[osc] switching shader entry to: {}", entry);
                        self.entry_file = entry.clone();
                        if let Some(state) = &mut self.state {
                            if let Err(e) = state.reload(&self.game_path, &self.entry_file) {
                                eprintln!("[osc] reload error: {}", e);
                            }
                        }
                    }
                    OscMessage::Reload => {
                        println!("[osc] /reload received");
                        if let Some(state) = &mut self.state {
                            if let Err(e) = state.reload(&self.game_path, &self.entry_file) {
                                eprintln!("[osc] reload error: {}", e);
                            }
                        }
                    }
                    ref other => {
                        if let Some(ref mut state) = self.state {
                            state.apply_osc_message(other);
                        }
                    }
                }
            }
        }

        // Check for hot-reload signal
        if let Some(ref rx) = self.hot_reload_rx {
            if rx.try_recv().is_ok() {
                // Drain any additional queued events (debounce)
                while rx.try_recv().is_ok() {}
                if let Some(state) = &mut self.state {
                    println!("[hot-reload] file change detected, reloading...");
                    if let Err(e) = state.reload(&self.game_path, &self.entry_file) {
                        eprintln!("[hot-reload] error: {}", e);
                    }
                }
            }
        }

        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

fn dispatch_osc(tx: &std::sync::mpsc::Sender<OscMessage>, msg: rosc::OscMessage) {
    let addr = msg.addr.as_str();
    log::debug!("[osc] {} {:?}", addr, msg.args);

    // /u/name value  or  /u/N value
    if let Some(name) = addr.strip_prefix("/u/") {
        let value = msg.args.first().and_then(|a| match a {
            OscType::Float(v) => Some(*v),
            OscType::Int(v)   => Some(*v as f32),
            OscType::Double(v) => Some(*v as f32),
            _ => None,
        });
        if let Some(v) = value {
            let _ = tx.send(OscMessage::SetFloat(name.to_string(), v));
        }
        return;
    }

    // /vid/<filename>/position 0.0-1.0
    if let Some(rest) = addr.strip_prefix("/vid/") {
        if let Some(filename) = rest.strip_suffix("/position") {
            let value = msg.args.first().and_then(|a| match a {
                OscType::Float(v)  => Some(*v),
                OscType::Int(v)    => Some(*v as f32),
                OscType::Double(v) => Some(*v as f32),
                _ => None,
            });
            if let Some(v) = value {
                let _ = tx.send(OscMessage::SetVideoPosition(filename.to_string(), v.clamp(0.0, 1.0)));
            }
        }
        return;
    }

    // /shader filename.wgsl
    if addr == "/shader" {
        if let Some(OscType::String(s)) = msg.args.first() {
            let _ = tx.send(OscMessage::LoadShader(s.clone()));
        }
        return;
    }

    // /reload
    if addr == "/reload" {
        let _ = tx.send(OscMessage::Reload);
        return;
    }

    // Unknown path — warn once per unique address so high-rate senders don't spam
    thread_local! {
        static WARNED: std::cell::RefCell<std::collections::HashSet<String>> =
            std::cell::RefCell::new(std::collections::HashSet::new());
    }
    WARNED.with(|w| {
        if w.borrow_mut().insert(addr.to_string()) {
            log::warn!("[osc] unknown path '{}' — expected /u/<name>, /shader, or /reload", addr);
            log::warn!("[osc] (set RUST_LOG=debug to see all received messages)");
        }
    });
}

fn start_osc_listener(port: u16) -> Option<std::sync::mpsc::Receiver<OscMessage>> {
    use std::net::UdpSocket;

    let socket = match UdpSocket::bind(format!("0.0.0.0:{}", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[osc] failed to bind port {}: {}", port, e);
            return None;
        }
    };

    println!("[osc] listening on 0.0.0.0:{}", port);

    let (tx, rx) = std::sync::mpsc::channel::<OscMessage>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 65536];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((size, _addr)) => {
                    match rosc::decoder::decode_udp(&buf[..size]) {
                        Ok((_rem, OscPacket::Message(msg))) => {
                            dispatch_osc(&tx, msg);
                        }
                        Ok((_rem, OscPacket::Bundle(bundle))) => {
                            for content in bundle.content {
                                if let OscPacket::Message(msg) = content {
                                    dispatch_osc(&tx, msg);
                                }
                            }
                        }
                        Err(e) => eprintln!("[osc] decode error: {:?}", e),
                    }
                }
                Err(e) => {
                    eprintln!("[osc] recv error: {}", e);
                    break;
                }
            }
        }
    });

    Some(rx)
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
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

    // Set up hot-reload file watcher if requested
    let (hot_reload_rx, _watcher) = if args.hot_reload {
        if args.game_path.ends_with(".zip") {
            eprintln!("[hot-reload] not available for zip sources");
            (None, None)
        } else {
            let watch_dir = if args.game_path.ends_with(".wgsl") {
                std::path::Path::new(&args.game_path)
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .to_path_buf()
            } else {
                std::path::PathBuf::from(&args.game_path)
            };

            let (tx, rx) = std::sync::mpsc::channel::<()>();
            let mut last_sent = std::time::Instant::now()
                .checked_sub(std::time::Duration::from_millis(500))
                .unwrap_or(std::time::Instant::now());
            let debounce = std::time::Duration::from_millis(200);

            match RecommendedWatcher::new(
                move |res: notify::Result<notify::Event>| {
                    if res.is_ok() {
                        let now = std::time::Instant::now();
                        if now.duration_since(last_sent) >= debounce {
                            last_sent = now;
                            let _ = tx.send(());
                        }
                    }
                },
                notify::Config::default(),
            ) {
                Ok(mut watcher) => {
                    if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::Recursive) {
                        eprintln!("[hot-reload] failed to watch {:?}: {}", watch_dir, e);
                        (None, None)
                    } else {
                        println!("[hot-reload] watching {:?}", watch_dir);
                        (Some(rx), Some(watcher))
                    }
                }
                Err(e) => {
                    eprintln!("[hot-reload] failed to create watcher: {}", e);
                    (None, None)
                }
            }
        }
    } else {
        (None, None)
    };

    let osc_rx = args.osc_port.and_then(start_osc_listener);

    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        state: None,
        game_source: Some(game_source),
        entry_file,
        game_path: args.game_path,
        hot_reload_rx,
        _watcher,
        osc_rx,
    };
    event_loop.run_app(&mut app).unwrap();
}
