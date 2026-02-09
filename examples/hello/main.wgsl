@set_title("Hello!")
@set_size(500, 150)

@import("draw2d.wgsl")
@import("font.wgsl")

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
    var color = vec4f(0.1, 0.1, 0.2, 1.0);

    // Render text (scale 2.0)
    color = blend_over(color, font_text(coord.xy, vec2f(50.0, 50.0), @str("Hello, World!"), 4.0, @texture("font8x8.png"), font8));
    color = blend_over(color, font_text(coord.xy, vec2f(50.0, 90.0), @str("@str() works!"), 2.0, @texture("font8x8.png"), font8));

    return color;
}

