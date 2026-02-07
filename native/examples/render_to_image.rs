// Simple tool to render a WGSL shader to a PNG image for testing
use std::fs::File;
use wgpu::util::DeviceExt;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <shader.wgsl> <output.png>", args[0]);
        std::process::exit(1);
    }

    let shader_path = &args[1];
    let output_path = &args[2];
    let width = 400u32;
    let height = 300u32;

    // Read shader
    let mut shader_source = std::fs::read_to_string(shader_path)
        .expect("Failed to read shader file");

    // Remove @set_* directives
    shader_source = shader_source
        .lines()
        .filter(|line| !line.trim().starts_with("@set_"))
        .collect::<Vec<_>>()
        .join("\n");

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

    // Create a simple shader with engine uniforms (matching main engine binding scheme)
    let full_shader = format!(
        r#"
struct GameEngineHost {{
    buttons: array<i32, 12>,
    time: f32,
    delta_time: f32,
    screen_width: f32,
    screen_height: f32,
}}

@group(0) @binding(0) var _engine_sampler: sampler;
@group(1) @binding(0) var<storage, read_write> _engine: GameEngineHost;

{}
"#,
        shader_source
    );

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Test Shader"),
        source: wgpu::ShaderSource::Wgsl(full_shader.into()),
    });

    // Create texture to render to
    let texture = device.create_texture(&wgpu::TextureDescriptor {
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

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Create engine buffer
    let engine_data: Vec<u8> = vec![0; 64]; // buttons(48) + time(4) + delta(4) + width(4) + height(4)
    let mut data = engine_data.clone();

    // Set screen dimensions (at offsets 56 and 60)
    data[56..60].copy_from_slice(&(width as f32).to_le_bytes());
    data[60..64].copy_from_slice(&(height as f32).to_le_bytes());

    let engine_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Engine Buffer"),
        contents: &data,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });

    // Create bind group layouts to match main engine
    // Group 0: sampler (required by preprocessor, even without textures)
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Engine Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let bind_group_layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout 0"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }],
    });

    // Group 1: engine buffer (writable, but only FRAGMENT visibility to avoid VERTEX_WRITABLE_STORAGE feature)
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

    let bind_group0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group 0"),
        layout: &bind_group_layout0,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Sampler(&sampler),
        }],
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
            texture: &texture,
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
