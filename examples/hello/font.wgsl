// 8x8 font functions - fully parameterized (texture, character map, scale)

fn font_char(coord: vec2f, pos: vec2f, c: u32, s: f32, font_tex: texture_2d<f32>, font_map: array<u32, 128>) -> vec4f {
    var idx = -1;
    for (var i = 0u; i < 128u; i++) {
        if (font_map[i] == 0u) { break; }
        if (font_map[i] == c) { idx = i32(i); break; }
    }
    if (idx < 0) { return vec4f(0.0); }

    let sz = 8.0 * s;
    let off = coord - pos;
    if (off.x < 0.0 || off.x >= sz || off.y < 0.0 || off.y >= sz) { return vec4f(0.0); }

    let local_x = clamp(off.x / s, 0.5, 7.5);
    let local_y = clamp(off.y / s, 0.5, 7.5);

    let uv = vec2f((f32(u32(idx) % 16u) * 8.0 + local_x) / 128.0, (f32(u32(idx) / 16u) * 8.0 + local_y) / 48.0);
    let g = textureSampleLevel(font_tex, @engine.sampler, uv, 0.0);
    return vec4f(1.0, 1.0, 1.0, (g.r + g.g + g.b) / 3.0);
}

fn font_text(coord: vec2f, pos: vec2f, text: array<u32, 128>, s: f32, font_tex: texture_2d<f32>, font_map: array<u32, 128>) -> vec4f {
    for (var i = 0u; i < 128u; i++) {
        if (text[i] == 0u) { break; }
        let c = font_char(coord, vec2f(pos.x + f32(i) * 8.0 * s, pos.y), text[i], s, font_tex, font_map);
        if (c.a > 0.0) { return c; }
    }
    return vec4f(0.0);
}

fn font_number(coord: vec2f, pos: vec2f, num: u32, s: f32, font_tex: texture_2d<f32>, font_map: array<u32, 128>) -> vec4f {
    if (num == 0u) { return font_char(coord, pos, 48u, s, font_tex, font_map); }
    var t = num; var d = 0u;
    while (t > 0u) { d++; t /= 10u; }
    for (var i = 0u; i < d; i++) {
        let dig = (num / u32(pow(10.0, f32(d - i - 1u)))) % 10u;
        let c = font_char(coord, vec2f(pos.x + f32(i) * 8.0 * s, pos.y), 48u + dig, s, font_tex, font_map);
        if (c.a > 0.0) { return c; }
    }
    return vec4f(0.0);
}