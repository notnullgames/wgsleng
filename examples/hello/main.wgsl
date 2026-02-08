@set_title("Hello!")
@set_size(800, 600)

// for blend_over
@import("draw2d.wgsl")

@compute @workgroup_size(1)
fn update() {
    // game logic updates would go here.
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

@fragment  
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    var color = vec4f(0.1, 0.1, 0.2, 1.0);
    
    // I want to be able to do something like this:
    //                        TEXTURE             CHAR_W, CHAR_H, CHARACTERS
    // font = font_load(@texture("20x20big.png"), 20.0, 20.0, @str(""))
    //           X, Y, STRING
    // color = blend_over(color, draw_font_str(300.0, 200.0, font, @str("SCORE:")));
    // color = blend_over(color, draw_font_int(340.0, 200.0, font, 10));
    
    return color;
}


////////// new font lib below here

