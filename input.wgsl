// data helpers for input

// Input struct with proper alignment for uniforms
struct Input {
    buttons: vec4<u32>,      // buttons 0-3 (up, down, left, right)
    buttons2: vec4<u32>,     // buttons 4-7 (A, B, X, Y)
    buttons3: vec4<u32>,     // buttons 8-11 (L, R, start, select)
    time: f32,
    delta_time: f32,
    screen_width: f32,
    screen_height: f32,
}

const BTN_UP: u32 = 0u;
const BTN_DOWN: u32 = 1u;
const BTN_LEFT: u32 = 2u;
const BTN_RIGHT: u32 = 3u;