@set_title("Hello!")
@set_size(600, 300)

@import("draw2d.wgsl")
@import("font.wgsl")

// Display scaling (renders at 300x150, displayed at 600x300)
const DISPLAY_SCALE = 2.0;

// Define font (texture must be passed separately)
const font8 = Font(
    @str("!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_ `abcdefghijklmnopqrstuvwxyz{|}~"),
    8.0,   // cell_w
    8.0,   // cell_h
    16u,   // chars_per_row
    128.0, // tex_w
    48.0   // tex_h
);

@compute @workgroup_size(1)
fn update() {}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Scale coordinates down to render resolution
    let game_coord = coord.xy / DISPLAY_SCALE;

    var color = vec4f(0.1, 0.1, 0.2, 1.0);

    // Render text (scale 2.0)
    color = blend_over(color, font_text(game_coord, vec2f(50.0, 50.0), @str("Hello, World!"), 2.0, @texture("font8x8.png"), font8));
    color = blend_over(color, font_text(game_coord, vec2f(95.0, 90.0), @str("@str() works!"), 1.0, @texture("font8x8.png"), font8));

    // FPS display (top-left corner)
    // color = blend_over(color, font_fps(game_coord, vec2f(5.0, 5.0), @engine.delta_time, 1.0, @texture("font8x8.png"), font8));

    return color;
}

