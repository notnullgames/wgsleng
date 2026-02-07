@set_title("Notnull Games")
@set_size(400, 300)

// Minimal game state (required, but can be empty)
struct GameState {
    dummy: u32
}

// Update function (required, but can be empty)
@compute @workgroup_size(1)
fn update() {
    // Game logic goes here
}

// Vertex shader for fullscreen rendering
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

// Fragment shader - renders each pixel
@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Center of the logo
    let center = vec2f(_engine.screen_width / 2.0, _engine.screen_height / 2.0);
    let pos = coord.xy - center;
    let dist = length(pos);

    // Outer prohibition symbol parameters
    let outer_radius = 140.0;
    let outer_ring_thickness = 18.0;
    let outer_slash_width = 10.0;  // Thinner for visual balance

    // Inner prohibition symbol parameters (recursive)
    let inner_scale = 0.55; // Scale of inner symbol relative to outer
    let inner_radius = outer_radius * inner_scale;
    let inner_ring_thickness = 18.0;
    let inner_slash_width = 10.0;  // Thinner for visual balance

    // Default: dark background
    var color = vec3f(0.2, 0.2, 0.2);

    // Fill inside outer circle with white first
    if (dist <= outer_radius) {
        color = vec3f(1.0, 1.0, 1.0);
    }

    // INNER PROHIBITION SYMBOL (white with black outline as unified shape)
    let outline_thickness = 6.0;
    let extended_radius = inner_radius + 25.0;
    let inner_slash_dist = abs(pos.x);

    // Define if we're in the inner circle ring OR the slash (union of both shapes)
    let in_circle_ring = dist <= inner_radius && dist > inner_radius - inner_ring_thickness;
    let in_slash = inner_slash_dist < inner_slash_width && abs(pos.y) < extended_radius;

    // Union of circle and slash (the complete white shape - NO caps, those are outline only)
    let in_white_shape = in_circle_ring || in_slash;

    // Black outline: extended versions of the shapes
    let in_circle_outline = dist <= inner_radius + outline_thickness &&
                           dist > inner_radius - inner_ring_thickness - outline_thickness;
    let in_slash_outline = inner_slash_dist < inner_slash_width + outline_thickness &&
                          abs(pos.y) < extended_radius + outline_thickness;

    // Horizontal caps at top and bottom to seal the outline
    let in_top_cap = inner_slash_dist < inner_slash_width + outline_thickness &&
                     abs(pos.y) >= extended_radius &&
                     abs(pos.y) <= extended_radius + outline_thickness;

    let in_outline = in_circle_outline || in_slash_outline || in_top_cap;

    // Draw black outline first
    if (in_outline) {
        color = vec3f(0.1, 0.1, 0.1);
    }

    // Draw white shape on top (covers interior of outline)
    if (in_white_shape) {
        color = vec3f(1.0, 1.0, 1.0);
    }

    // OUTER PROHIBITION SYMBOL (red) - drawn on top
    if (dist <= outer_radius && dist > outer_radius - outer_ring_thickness) {
        // Outer red ring
        color = vec3f(0.9, 0.1, 0.1);
    }

    // Outer diagonal slash (red) - cuts through everything
    let slash_dist = abs(pos.x + pos.y) / sqrt(2.0);
    if (slash_dist < outer_slash_width && dist < outer_radius) {
        color = vec3f(0.9, 0.1, 0.1);
    }

    return vec4f(color, 1.0);
}
