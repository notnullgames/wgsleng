# Practical 3D Rendering Example

This example demonstrates traditional triangle-based 3D rendering, which is the standard approach for real games and applications.

## Overview

Unlike the ray marching example (`examples/3d/`), this uses:
- **Vertex buffers** - Explicit triangle data
- **Vertex shader** - Transforms each vertex once
- **Fragment shader** - Shades each visible pixel
- **Model/View/Projection matrices** - Standard 3D transforms

## Current Implementation

This example uses **software rasterization in the fragment shader** as a workaround for the engine's current limitations. It:

1. Uses fullscreen triangle (3 vertices - current engine limitation)
2. Projects cube vertices per-pixel in fragment shader
3. Tests each triangle to find which face is visible
4. Applies lighting to the visible face

This is **not** how real 3D games work, but demonstrates the concepts within current engine constraints.

### What it shows:
- ✅ Model/View/Projection matrix transforms
- ✅ 3D perspective projection
- ✅ Per-face normals and colors
- ✅ Diffuse lighting
- ✅ Rotating 3D object
- ✅ Depth sorting (software Z-buffer)

## Future: `@model` Directive

To make this practical for real games, the engine should support loading external models:

```wgsl
// Future syntax (not implemented yet)
@model("character.obj")

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    // Access loaded model data
    let pos = @model("character.obj").positions[idx];
    let normal = @model("character.obj").normals[idx];
    let uv = @model("character.obj").uvs[idx];

    // Transform and return...
}
```

See `DESIGN.md` for full proposal.

## How to Run

**Web:**
```bash
npm start
# Navigate to http://localhost:8000/#examples/3d_practical/main.wgsl
```

**Native:**
```bash
./native/target/release/wgsleng examples/3d_practical/main.wgsl
```

## Comparison to Ray Marching

| Feature | Ray Marching (`examples/3d/`) | Traditional (`examples/3d_practical/`) |
|---------|-------------------------------|----------------------------------------|
| **Rendering** | Per-pixel ray march | Per-vertex transform + per-pixel shade |
| **Performance** | Expensive (every pixel) | Efficient (only visible surfaces) |
| **Geometry** | Mathematical functions (SDFs) | Triangle meshes |
| **Assets** | Procedural only | Can load 3D models |
| **Complexity** | Hard (pyramid issues) | Easy (just load it) |
| **Use cases** | Demos, effects, art | Games, apps, visualization |
| **Detail** | Limited by step count | Unlimited triangles |
| **Lighting** | Part of ray march | Separate, flexible |

## Advantages of Traditional Rendering

1. **Performance** - Only processes visible triangles
2. **Asset workflow** - Use Blender, Maya, etc. to create models
3. **Scalability** - Can render millions of triangles
4. **Familiarity** - Standard techniques everyone knows
5. **Flexibility** - Easy to add textures, animations, etc.

## What's Next

To make this production-ready:

1. **Implement `@model` directive** - Load OBJ/glTF files
2. **Add instancing** - Render many copies efficiently
3. **Texture support** - Already have `@texture`, integrate it
4. **Advanced lighting** - PBR, shadows, etc.
5. **Camera controls** - Input handling for FPS camera
6. **Scene management** - Multiple objects, transforms
7. **Optimization** - Frustum culling, LOD, etc.

## Current Limitations

This works as a demonstration, but isn't practical for real games because:

1. **Software rasterization** - Every pixel tests every triangle (very slow)
2. **Hardcoded geometry** - Cube defined in shader, can't load models
3. **No hardware vertex pipeline** - Not using GPU's triangle rasterization
4. **No real depth buffer** - Manual Z-sorting per pixel
5. **Limited complexity** - Can't handle many triangles

### What the Engine Needs for Real 3D:

1. **Variable vertex count** - Currently hardcoded to 3 vertices (fullscreen triangle)
2. **@engine uniforms in vertex shader** - Currently only visible in fragment shader
3. **@model directive** - Load OBJ/glTF files and inject vertex buffers
4. **Hardware depth testing** - Let GPU handle Z-buffer

### Other Limitations:
- Single object only (no scene graph)
- Basic lighting (no shadows, PBR, etc.)
- No textures applied yet (but engine supports `@texture`)
- No input/camera control

## Example for Real Game

Once `@model` is implemented, a real game might look like:

```wgsl
@set_title("My 3D Game")
@set_size(1280, 720)

@model("level.glb")
@model("player.glb")
@model("enemy.glb")
@texture("level_diffuse.png")
@texture("player_texture.png")

struct GameState {
    player_pos: vec3f,
    camera_pos: vec3f,
    camera_rotation: vec2f,
    enemies: array<Enemy, 10>,
}

@compute @workgroup_size(1)
fn update() {
    // Game logic
    update_player();
    update_enemies();
    update_camera();
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32,
           @builtin(instance_index) instance: u32) -> VertexOutput {
    // Render different models based on instance
    if (instance == 0u) {
        // Render level
        let pos = @model("level.glb").positions[idx];
        // ...
    } else if (instance == 1u) {
        // Render player
        let pos = @model("player.glb").positions[idx];
        // Apply player transform
        // ...
    }
    // ...
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // PBR lighting, shadows, etc.
}
```

## Resources

- [WebGPU Fundamentals - 3D](https://webgpufundamentals.org/webgpu/lessons/webgpu-3d-orthographic.html)
- [Learn OpenGL](https://learnopengl.com/) - Concepts translate to WebGPU
- [glTF Format](https://www.khronos.org/gltf/) - Industry standard 3D format
- [OBJ Format](http://paulbourke.net/dataformats/obj/) - Simple 3D format

## Contributing

If you want to help implement `@model` support:

1. See `DESIGN.md` for architecture
2. Add model parsing to `native/src/lib.rs`
3. Extend preprocessor for `@model` directive
4. Create vertex buffer management
5. Update examples

Both approaches (ray marching and traditional) are valuable:
- **Ray marching** - Great for demos, effects, and artistic visuals
- **Traditional** - Essential for practical games and applications

Keep both! 🎮
