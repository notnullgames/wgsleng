// ============================================================================
// Bitmap Font Rendering for WGSL Engine
// Easy-to-use text rendering with bitmap fonts
// ============================================================================

// Font texture layout: 20 characters wide × 3 characters tall (320×48 pixels)
// Characters are arranged in ASCII order starting from space (32)
const FONT_GRID_WIDTH = 20u;
const FONT_GRID_HEIGHT = 3u;

// Calculate UV coordinates for a character
fn get_char_uv(coord: vec2f, char_code: u32, pos: vec2f, size: f32) -> vec4f {
    // Check if pixel is within character bounds
    let char_offset = coord - pos;
    if (char_offset.x < 0.0 || char_offset.x >= size ||
        char_offset.y < 0.0 || char_offset.y >= size) {
        return vec4f(0.0, 0.0, 0.0, 0.0);
    }

    // Font contains ASCII 32-95 (space through underscore)
    // Calculate grid position (20 wide × 3 tall)
    let font_index = char_code - 32u;
    let grid_x = f32(font_index % FONT_GRID_WIDTH);
    let grid_y = f32(font_index / FONT_GRID_WIDTH);

    // Calculate UV coordinates
    let char_width = 1.0 / f32(FONT_GRID_WIDTH);
    let char_height = 1.0 / f32(FONT_GRID_HEIGHT);

    let uv = vec2f(
        (grid_x + char_offset.x / size) * char_width,
        (grid_y + char_offset.y / size) * char_height
    );

    return vec4f(uv.x, uv.y, 1.0, 0.0);
}

// Helper to get UV for a digit (0-9)
fn get_digit_uv(coord: vec2f, digit: u32, pos: vec2f, size: f32) -> vec4f {
    return get_char_uv(coord, 48u + digit, pos, size);
}

// Helper for multi-digit numbers
fn get_number_uv(coord: vec2f, number: u32, pos: vec2f, size: f32) -> vec4f {
    if (number == 0u) {
        return get_digit_uv(coord, 0u, pos, size);
    }

    // Count digits
    var temp = number;
    var digit_count = 0u;
    while (temp > 0u) {
        digit_count++;
        temp /= 10u;
    }

    // Check each digit position
    for (var i = 0u; i < digit_count; i++) {
        let x_offset = f32(i) * size;
        let divisor = u32(pow(10.0, f32(digit_count - i - 1u)));
        let digit = (number / divisor) % 10u;

        let result = get_digit_uv(coord, digit, vec2f(pos.x + x_offset, pos.y), size);
        if (result.z > 0.5) {
            return result;
        }
    }

    return vec4f(0.0, 0.0, 0.0, 0.0);
}
