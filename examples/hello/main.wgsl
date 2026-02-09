@set_title("Hello!")
@set_size(800, 600)

@import("draw2d.wgsl")
@import("font.wgsl")

// Define font character map
const font8 = @str("!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_ `abcdefghijklmnopqrstuvwxyz{|}~");

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
    color = blend_over(color, font_text(coord.xy, vec2f(50.0, 50.0), @str("Hello, World!"), 2.0, @texture("font8x8.png"), font8));
    color = blend_over(color, font_text(coord.xy, vec2f(50.0, 90.0), @str("@str() works!"), 2.0, @texture("font8x8.png"), font8));
    color = blend_over(color, font_text(coord.xy, vec2f(50.0, 130.0), @str("SCORE:"), 2.0, @texture("font8x8.png"), font8));
    color = blend_over(color, font_number(coord.xy, vec2f(170.0, 130.0), 12345u, 2.0, @texture("font8x8.png"), font8));

    return color;
}

