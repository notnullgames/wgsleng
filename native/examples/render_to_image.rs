// Simple tool to render a WGSL shader to a PNG image for testing
use std::fs::File;
use wgpu::util::DeviceExt;
use wgsleng::{GameSource, PreprocessorState};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <shader.wgsl> <output.png>", args[0]);
        std::process::exit(1);
    }

    let shader_path = &args[1];
    let output_path = &args[2];

    // Determine entry file
    let entry_file = if shader_path.ends_with(".wgsl") {
        std::path::Path::new(shader_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    } else {
        "main.wgsl".to_string()
    };

    // Open game source (handles .wgsl files, directories, or .zip files)
    let mut game_source = GameSource::open(shader_path)
        .expect("Failed to open game source");

    // Read and preprocess shader using the same logic as main program
    let shader_code = game_source.read_text(&entry_file)
        .expect("Failed to read shader file");

    let mut preprocessor = PreprocessorState::new(game_source);
    let (processed_code, metadata) = preprocessor.preprocess_shader(&shader_code, true)
        .expect("Failed to preprocess shader");

    // Debug: print processed shader if requested
    if std::env::var("DEBUG_SHADER").is_ok() {
        println!("\n=== PROCESSED SHADER ===");
        println!("{}", processed_code);
        println!("=== END SHADER ===\n");
    }

    println!("Game: {}", metadata.title);
    println!("Size: {}x{}", metadata.width, metadata.height);
    println!("Textures: {:?}", metadata.textures);
    println!("Sounds: {:?}", metadata.sounds);

    let width = metadata.width;
    let height = metadata.height;

    // Create wgpu instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Failed to find adapter");

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .expect("Failed to create device");

    // Create shader module with preprocessed code
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Test Shader"),
        source: wgpu::ShaderSource::Wgsl(processed_code.into()),
    });

    // Load textures if present
    let mut textures = Vec::new();
    for texture_file in &metadata.textures {
        let img_data = preprocessor.game_source.read_file(texture_file)
            .expect(&format!("Failed to load texture: {}", texture_file));
        let img = image::load_from_memory(&img_data)
            .expect("Failed to decode image")
            .to_rgba8();
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

    // Create texture to render to
    let render_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Render Texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let texture_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Calculate buffer layout matching WGSL struct
    let button_size = 12 * 4; // 48 bytes
    let float_data_size = 4 * 4; // 16 bytes
    let state_alignment = 8;
    let aligned_state_size = ((metadata.state_size + state_alignment - 1) / state_alignment) * state_alignment;
    let audio_size = metadata.sounds.len() * 4;

    let total_size_unaligned = button_size + float_data_size + aligned_state_size + audio_size;
    let total_size = ((total_size_unaligned + 15) / 16) * 16;

    // Create engine buffer
    let mut init_data = vec![0u8; total_size];

    // Initialize screen size in floats section (offset 48+8 = 56)
    let width_bytes = (width as f32).to_le_bytes();
    let height_bytes = (height as f32).to_le_bytes();
    init_data[56..60].copy_from_slice(&width_bytes);
    init_data[60..64].copy_from_slice(&height_bytes);

    // Initialize player position to center in state section (offset 64)
    let center_x = ((width / 2) as f32).to_le_bytes();
    let center_y = ((height / 2) as f32).to_le_bytes();
    init_data[64..68].copy_from_slice(&center_x);
    init_data[68..72].copy_from_slice(&center_y);

    let engine_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Engine Buffer"),
        contents: &init_data,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });

    // Create bind group layouts matching main engine
    // Group 0: sampler (always) and textures (if present)
    let mut render_group0_entries = vec![
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }
    ];

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

    let bind_group_layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout 0"),
        entries: &render_group0_entries,
    });

    // Group 1: engine buffer
    let bind_group_layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout 1"),
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

    // Create bind groups
    let texture_views: Vec<_> = textures.iter()
        .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
        .collect();

    let mut group0_entries = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Sampler(&sampler),
        }
    ];

    for (i, view) in texture_views.iter().enumerate() {
        group0_entries.push(wgpu::BindGroupEntry {
            binding: (i + 1) as u32,
            resource: wgpu::BindingResource::TextureView(view),
        });
    }

    let bind_group0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group 0"),
        layout: &bind_group_layout0,
        entries: &group0_entries,
    });

    let bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group 1"),
        layout: &bind_group_layout1,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: engine_buffer.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout0, &bind_group_layout1],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
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
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    // Render
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
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

        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_bind_group(0, &bind_group0, &[]);
        render_pass.set_bind_group(1, &bind_group1, &[]);
        render_pass.draw(0..3, 0..1);
    }

    // Copy texture to buffer
    let bytes_per_row = 4 * width;
    let padded_bytes_per_row = (bytes_per_row + 255) & !255; // Align to 256
    let buffer_size = (padded_bytes_per_row * height) as u64;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &render_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &output_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(Some(encoder.finish()));

    // Read buffer
    let buffer_slice = output_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);

    let data = buffer_slice.get_mapped_range();

    // Remove padding from rows
    let mut unpadded_data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let start = (y * padded_bytes_per_row) as usize;
        let end = start + (width * 4) as usize;
        unpadded_data.extend_from_slice(&data[start..end]);
    }

    // Save as PNG
    let mut encoder = png::Encoder::new(File::create(output_path).unwrap(), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&unpadded_data).unwrap();

    println!("Rendered to {}", output_path);
}
