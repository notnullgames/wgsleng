# Stanford Bunny - @model Implementation Example

This example demonstrates the `@model` directive for loading 3D models. Currently **not fully functional** - the preprocessing works but the runtime doesn't load the model data yet.

## Current Status

✅ **Working:**
- `@model("bunny.obj")` directive recognized by preprocessor
- OBJ parser implemented (`obj_loader.rs`)
- Storage buffer structures generated automatically
- Bunny model loaded: 2503 vertices, 4968 triangular faces

❌ **Not Yet Implemented:**
- Hosts don't actually load OBJ files at runtime
- No vertex buffers created
- No bind group 2 (models use `@group(2)`)
- Still draws 3 vertices (fullscreen triangle) instead of model
- No variable vertex count support

## What Needs to Be Done

### 1. Load Model Data in Hosts

**Native (`main.rs`):**
```rust
use wgsleng::ObjModel;

// After preprocessing
for model_file in &metadata.models {
    let model_path = game_source.resolve_path(model_file)?;
    let model = ObjModel::load(&model_path)?;

    // Create vertex buffer
    let positions_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Model Positions"),
        contents: bytemuck::cast_slice(&model.positions),
        usage: BufferUsages::STORAGE,
    });

    // Create normals buffer
    let normals_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Model Normals"),
        contents: bytemuck::cast_slice(&model.normals),
        usage: BufferUsages::STORAGE,
    });

    // Store for binding
    model_buffers.push((positions_buffer, normals_buffer));
}
```

**Web (`wgsleng.js`):**
```javascript
// Add OBJ parser (can use existing JS libraries)
async function loadOBJ(url) {
    const response = await fetch(url);
    const text = await response.text();
    return parseOBJ(text);  // Returns {positions, normals, indices}
}

// Create buffers
for (const modelFile of metadata.models) {
    const model = await loadOBJ(modelFile);

    const positionsBuffer = device.createBuffer({
        size: model.positions.byteLength,
        usage: GPUBufferUsage.STORAGE,
        mappedAtCreation: true,
    });
    new Float32Array(positionsBuffer.getMappedRange()).set(model.positions);
    positionsBuffer.unmap();

    // Same for normals...
}
```

### 2. Create Bind Group 2

```rust
let model_bind_group = device.create_bind_group(&BindGroupDescriptor {
    label: Some("Model Bind Group"),
    layout: &model_bind_group_layout,
    entries: &[
        BindGroupEntry {
            binding: 1,  // positions
            resource: positions_buffer.as_entire_binding(),
        },
        BindGroupEntry {
            binding: 2,  // normals
            resource: normals_buffer.as_entire_binding(),
        },
    ],
});

// Set in render pass
render_pass.set_bind_group(2, &model_bind_group, &[]);
```

### 3. Variable Vertex Count

Currently hardcoded to 3 vertices. Needs to support drawing N vertices:

```rust
// Instead of:
render_pass.draw(0..3, 0..1);

// Do:
let vertex_count = model.indices.len() as u32;  // or model.positions.len()
render_pass.draw(0..vertex_count, 0..1);
```

### 4. Optional: Index Buffers

For efficiency, use index buffers:

```rust
let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
    label: Some("Index Buffer"),
    contents: bytemuck::cast_slice(&model.indices),
    usage: BufferUsages::INDEX,
});

render_pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint32);
render_pass.draw_indexed(0..model.indices.len() as u32, 0, 0..1);
```

## Usage (Once Implemented)

```wgsl
@model("bunny.obj")

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    // Access loaded model data
    let pos = @model("bunny.obj").positions[idx];
    let normal = @model("bunny.obj").normals[idx];

    // Transform with MVP matrices
    let world_pos = model_matrix * vec4f(pos, 1.0);
    let clip_pos = projection_matrix * view_matrix * world_pos;

    return VertexOutput(clip_pos, normal, world_pos);
}

@fragment
fn fs_render(in: VertexOutput) -> @location(0) vec4f {
    // Lighting calculations
    let light_dir = normalize(vec3f(1.0, 1.0, -1.0));
    let diffuse = max(dot(in.normal, light_dir), 0.0);
    return vec4f(vec3f(0.8) * (0.2 + diffuse * 0.8), 1.0);
}
```

## Files

- `bunny.obj` - Stanford Bunny model (2503 vertices, 4968 faces)
- `main.wgsl` - Shader code (currently placeholder)
- `DESIGN.md` - Original @model design document

## Implementation Priority

1. ✅ Preprocessor support (DONE)
2. ✅ OBJ parser (DONE)
3. ⬜ Load OBJ files in hosts
4. ⬜ Create vertex buffers
5. ⬜ Create bind group 2
6. ⬜ Variable vertex count
7. ⬜ Test with bunny
8. ⬜ Add index buffer support
9. ⬜ Optimize for performance

## Resources

- [WebGPU Storage Buffers](https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-buffers.html)
- [OBJ Format Specification](http://paulbourke.net/dataformats/obj/)
- [Stanford Bunny](https://en.wikipedia.org/wiki/Stanford_bunny)

Once the hosts are updated to load model data, this example will render a beautiful rotating 3D bunny! 🐰
