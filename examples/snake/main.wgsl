@set_title("Snake")
@set_size(600, 600)

// Grid settings
const grid_size = 20.0;
const line_width = 1.0;

// Only define things that persist across frames
struct GameState {
    dead: u32
}


@compute @workgroup_size(1)
fn update() {}

// Vertex shader for fullscreen rendering
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Calculate grid position
    let cell_x = coord.x % (((@engine.screen_width - line_width) / grid_size));
    let cell_y = coord.y % (((@engine.screen_height - line_width) / grid_size));

    // Draw grid lines
    if (cell_x < line_width || cell_y < line_width) {
        return vec4f(0.9, 0.9, 0.9, 1.0);
    }

    // black background
    return vec4f(0.1, 0.1, 0.1, 1.0);
}
