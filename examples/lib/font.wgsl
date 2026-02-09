// Font struct (texture must be passed separately - WGSL doesn't allow textures in structs)
struct Font {
    map: array<u32, 128>,
    cell_w: f32,
    cell_h: f32,
    chars_per_row: u32,
    tex_w: f32,
    tex_h: f32
}

// Fast character index lookup - optimized for ASCII fonts
fn get_char_index(c: u32, font_map: array<u32, 128>) -> i32 {
    // Early exit for common case: sequential ASCII starting at 33
    // This handles !"#$%&'()*+,-./0-9:;<=>?@A-Z[\]^_
    if (c >= 33u && c < 96u) {
        if (font_map[c - 33u] == c) { return i32(c - 33u); }
    }
    // Handle lowercase: `a-z{|}~
    else if (c >= 96u && c <= 126u) {
        let idx = c - 33u;
        if (idx < 128u && font_map[idx] == c) { return i32(idx); }
    }
    // Space character is often at position 63
    else if (c == 32u && font_map[63] == 32u) {
        return 63;
    }

    // Fallback: linear search (only for non-standard fonts)
    for (var i = 0u; i < 96u; i++) { // Only search first 96 (standard ASCII printable)
        if (font_map[i] == c) { return i32(i); }
    }
    return -1;
}

fn font_char(coord: vec2f, pos: vec2f, c: u32, s: f32, font_tex: texture_2d<f32>, font: Font) -> vec4f {
    // Early bounds check BEFORE expensive lookup
    let sz_w = font.cell_w * s;
    let sz_h = font.cell_h * s;
    let off = coord - pos;
    if (off.x < 0.0 || off.x >= sz_w || off.y < 0.0 || off.y >= sz_h) { return vec4f(0.0); }

    // Now do the character lookup
    let idx = get_char_index(c, font.map);
    if (idx < 0) { return vec4f(0.0); }

    let local_x = clamp(off.x / s, 0.5, font.cell_w - 0.5);
    let local_y = clamp(off.y / s, 0.5, font.cell_h - 0.5);

    let grid_x = u32(idx) % font.chars_per_row;
    let grid_y = u32(idx) / font.chars_per_row;
    let uv = vec2f(
        (f32(grid_x) * font.cell_w + local_x) / font.tex_w,
        (f32(grid_y) * font.cell_h + local_y) / font.tex_h
    );
    let g = textureSampleLevel(font_tex, @engine.sampler, uv, 0.0);
    return vec4f(1.0, 1.0, 1.0, (g.r + g.g + g.b) / 3.0);
}

fn font_text(coord: vec2f, pos: vec2f, text: array<u32, 128>, s: f32, font_tex: texture_2d<f32>, font: Font) -> vec4f {
    // Early bounds check - reject if pixel is nowhere near the text area
    let char_h = font.cell_h * s;
    if (coord.y < pos.y || coord.y >= pos.y + char_h) { return vec4f(0.0); }

    // Calculate which character this pixel might be in
    let char_w = font.cell_w * s;
    let rel_x = coord.x - pos.x;
    if (rel_x < 0.0) { return vec4f(0.0); }

    let char_idx = u32(rel_x / char_w);
    if (char_idx >= 128u || text[char_idx] == 0u) { return vec4f(0.0); }

    // Only check the one character at this position
    return font_char(coord, vec2f(pos.x + f32(char_idx) * char_w, pos.y), text[char_idx], s, font_tex, font);
}

fn font_number(coord: vec2f, pos: vec2f, num: u32, s: f32, font_tex: texture_2d<f32>, font: Font) -> vec4f {
    // Early bounds check
    let char_h = font.cell_h * s;
    if (coord.y < pos.y || coord.y >= pos.y + char_h) { return vec4f(0.0); }

    let char_w = font.cell_w * s;
    let rel_x = coord.x - pos.x;
    if (rel_x < 0.0) { return vec4f(0.0); }

    // Count digits
    var digit_count = 1u;
    if (num > 0u) {
        var t = num;
        digit_count = 0u;
        while (t > 0u) { digit_count++; t /= 10u; }
    }

    // Calculate which digit position this pixel is in
    let digit_idx = u32(rel_x / char_w);
    if (digit_idx >= digit_count) { return vec4f(0.0); }

    // Calculate the digit at this position
    let digit = select((num / u32(pow(10.0, f32(digit_count - digit_idx - 1u)))) % 10u, 0u, num == 0u);

    return font_char(coord, vec2f(pos.x + f32(digit_idx) * char_w, pos.y), 48u + digit, s, font_tex, font);
}

// Display FPS (frames per second) - convenience function
fn font_fps(coord: vec2f, pos: vec2f, delta_time: f32, s: f32, font_tex: texture_2d<f32>, font: Font) -> vec4f {
    let fps = u32(1.0 / max(delta_time, 0.001)); // Prevent division by zero
    return font_number(coord, pos, fps, s, font_tex, font);
}