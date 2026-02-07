// data helpers for input

struct Input {
    buttons: array<u32, 12>,
    time: f32,
    delta_time: f32,
    screen_width: f32,
    screen_height: f32,
}

const BTN_UP: u32 = 0u;
const BTN_DOWN: u32 = 1u;
const BTN_LEFT: u32 = 2u;
const BTN_RIGHT: u32 = 3u;