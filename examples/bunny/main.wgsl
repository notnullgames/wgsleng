@set_title("Stanford Bunny - Placeholder")
@set_size(800, 600)

// @model("bunny.obj")  // Commented out - hosts don't load model data yet

// Game state
struct GameState {
    time: f32,
}

@compute @workgroup_size(1)
fn update() {
    game_state.time = @engine.time;
}

// FUTURE: Once @model is fully implemented, this would work:
//
// @vertex
// fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
//     // Access loaded model data
//     let pos = _model_0.positions[idx];
//     let normal = _model_0.normals[idx];
//
//     // Transform vertices...
//     let mvp = projection * view * model;
//     let clip_pos = mvp * vec4f(pos, 1.0);
//
//     return VertexOutput(clip_pos, normal, ...);
// }

// For now, use fullscreen triangle like the cube example
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // For now, just render a placeholder
    let uv = coord.xy / vec2f(@engine.screen_width, @engine.screen_height);

    // Show that @model was recognized
    let text_color = vec3f(1.0, 1.0, 1.0);
    let bg_color = vec3f(0.1, 0.1, 0.15);

    return vec4f(mix(bg_color, text_color, uv.y * 0.5), 1.0);
}
