@set_title("Input Demo - Keyboard, Controller & Mouse")
@set_size(640, 420)

@import("draw2d.wgsl")
@import("font.wgsl")

struct GameState {
    dummy: u32
}

const font8 = Font(
    @str("!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_ `abcdefghijklmnopqrstuvwxyz{|}~"),
    8.0, 8.0, 16u, 128.0, 48.0
);

@compute @workgroup_size(1)
fn update() {
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

fn key_with_bg(coord: vec2f, pos: vec2f, size: vec2f, label: u32, pressed: bool, font_tex: texture_2d<f32>) -> vec4f {
    var color = vec4f(0.0);
    let bg_color = select(vec4f(0.25, 0.25, 0.25, 1.0), vec4f(0.6, 0.6, 0.6, 1.0), pressed);
    color = blend_over(color, draw_rounded_rect(coord, pos, size, 3.0, bg_color));
    let border_color = select(vec4f(0.4, 0.4, 0.4, 1.0), vec4f(0.9, 0.9, 0.9, 1.0), pressed);
    color = blend_over(color, draw_rounded_rect_outline(coord, pos, size, 3.0, 1.5, border_color));
    let text_col = select(vec4f(0.7, 0.7, 0.7, 1.0), vec4f(1.0, 1.0, 1.0, 1.0), pressed);
    let raw = font_char(coord, pos + (size - vec2f(8.0, 8.0)) * 0.5, label, 1.0, font_tex, font8);
    color = blend_over(color, vec4f(raw.rgb * text_col.rgb, raw.a * text_col.a));
    return color;
}

fn key_wide(coord: vec2f, pos: vec2f, size: vec2f, label: array<u32, 128>, pressed: bool, font_tex: texture_2d<f32>) -> vec4f {
    var color = vec4f(0.0);
    let bg_color = select(vec4f(0.25, 0.25, 0.25, 1.0), vec4f(0.6, 0.6, 0.6, 1.0), pressed);
    color = blend_over(color, draw_rounded_rect(coord, pos, size, 3.0, bg_color));
    let border_color = select(vec4f(0.4, 0.4, 0.4, 1.0), vec4f(0.9, 0.9, 0.9, 1.0), pressed);
    color = blend_over(color, draw_rounded_rect_outline(coord, pos, size, 3.0, 1.5, border_color));
    let text_col = select(vec4f(0.7, 0.7, 0.7, 1.0), vec4f(1.0, 1.0, 1.0, 1.0), pressed);
    var char_idx = 0u;
    loop {
        if (label[char_idx] == 0u) { break; }
        let text_pos = vec2f(pos.x + 8.0 * f32(char_idx) + 4.0, pos.y + (size.y - 8.0) * 0.5);
        let raw = font_char(coord, text_pos, label[char_idx], 1.0, font_tex, font8);
        color = blend_over(color, vec4f(raw.rgb * text_col.rgb, raw.a * text_col.a));
        char_idx++;
        if (char_idx >= 16u) { break; }
    }
    return color;
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    var color = draw_gradient_linear(
        coord.xy,
        vec2f(0.0, 0.0),
        vec2f(0.0, @engine.screen_height),
        COLOR_BLUE,
        COLOR_GREEN
    );

    // ========================================================================
    // Keyboard (top)
    // ========================================================================
    let kb_width = 240.0;
    let kb_x = (640.0 - kb_width) * 0.5;
    color = blend_over(color, draw_rect(coord.xy, vec2f(kb_x, 4.0), vec2f(kb_width, 64.0), vec4f(0.0, 0.0, 0.0, 0.55)));

    let key_w = 14.0;
    let key_h = 10.0;
    let key_gap = 1.5;
    let start_x = kb_x + 5.0;
    let start_y = 6.0;

    // Row 0: 1-0 - = BKSP
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 0.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 49u, @engine.keys[KEY_1] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 1.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 50u, @engine.keys[KEY_2] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 2.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 51u, @engine.keys[KEY_3] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 3.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 52u, @engine.keys[KEY_4] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 4.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 53u, @engine.keys[KEY_5] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 5.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 54u, @engine.keys[KEY_6] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 6.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 55u, @engine.keys[KEY_7] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 7.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 56u, @engine.keys[KEY_8] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 8.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 57u, @engine.keys[KEY_9] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 9.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 48u, @engine.keys[KEY_0] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 10.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 45u, @engine.keys[KEY_MINUS] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 11.0 * (key_w + key_gap), start_y), vec2f(key_w, key_h), 61u, @engine.keys[KEY_EQUAL] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_wide(coord.xy, vec2f(start_x + 12.0 * (key_w + key_gap), start_y), vec2f(key_w * 2.0 + key_gap, key_h), @str("BK"), @engine.keys[KEY_BACKSPACE] == 1u, @texture("font8x8.png")));

    // Row 1: TAB Q-P [ ]
    let r1 = start_y + key_h + key_gap;
    color = blend_over(color, key_wide(coord.xy, vec2f(start_x + 0.0 * (key_w + key_gap), r1), vec2f(key_w * 2.0, key_h), @str("TAB"), @engine.keys[KEY_TAB] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 2.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 81u, @engine.keys[KEY_Q] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 3.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 87u, @engine.keys[KEY_W] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 4.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 69u, @engine.keys[KEY_E] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 5.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 82u, @engine.keys[KEY_R] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 6.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 84u, @engine.keys[KEY_T] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 7.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 89u, @engine.keys[KEY_Y] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 8.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 85u, @engine.keys[KEY_U] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 9.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 73u, @engine.keys[KEY_I] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 10.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 79u, @engine.keys[KEY_O] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 11.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 80u, @engine.keys[KEY_P] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 12.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 91u, @engine.keys[KEY_BRACKET_LEFT] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 13.0 * (key_w + key_gap), r1), vec2f(key_w, key_h), 93u, @engine.keys[KEY_BRACKET_RIGHT] == 1u, @texture("font8x8.png")));

    // Row 2: CAPS A-L ; ' ENT
    let r2 = start_y + 2.0 * (key_h + key_gap);
    color = blend_over(color, key_wide(coord.xy, vec2f(start_x + 0.0 * (key_w + key_gap), r2), vec2f(key_w * 2.0, key_h), @str("CAP"), @engine.keys[KEY_CAPS_LOCK] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 2.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 65u, @engine.keys[KEY_A] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 3.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 83u, @engine.keys[KEY_S] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 4.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 68u, @engine.keys[KEY_D] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 5.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 70u, @engine.keys[KEY_F] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 6.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 71u, @engine.keys[KEY_G] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 7.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 72u, @engine.keys[KEY_H] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 8.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 74u, @engine.keys[KEY_J] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 9.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 75u, @engine.keys[KEY_K] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 10.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 76u, @engine.keys[KEY_L] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 11.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 59u, @engine.keys[KEY_SEMICOLON] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 12.0 * (key_w + key_gap), r2), vec2f(key_w, key_h), 39u, @engine.keys[KEY_QUOTE] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_wide(coord.xy, vec2f(start_x + 13.0 * (key_w + key_gap), r2), vec2f(key_w * 2.0 + key_gap, key_h), @str("ENT"), @engine.keys[KEY_ENTER] == 1u, @texture("font8x8.png")));

    // Row 3: SHIFT Z-M , . / SHIFT
    let r3 = start_y + 3.0 * (key_h + key_gap);
    color = blend_over(color, key_wide(coord.xy, vec2f(start_x + 0.0 * (key_w + key_gap), r3), vec2f(key_w * 2.0, key_h), @str("SH"), @engine.keys[KEY_SHIFT_LEFT] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 2.0 * (key_w + key_gap), r3), vec2f(key_w, key_h), 90u, @engine.keys[KEY_Z] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 3.0 * (key_w + key_gap), r3), vec2f(key_w, key_h), 88u, @engine.keys[KEY_X] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 4.0 * (key_w + key_gap), r3), vec2f(key_w, key_h), 67u, @engine.keys[KEY_C] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 5.0 * (key_w + key_gap), r3), vec2f(key_w, key_h), 86u, @engine.keys[KEY_V] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 6.0 * (key_w + key_gap), r3), vec2f(key_w, key_h), 66u, @engine.keys[KEY_B] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 7.0 * (key_w + key_gap), r3), vec2f(key_w, key_h), 78u, @engine.keys[KEY_N] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 8.0 * (key_w + key_gap), r3), vec2f(key_w, key_h), 77u, @engine.keys[KEY_M] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 9.0 * (key_w + key_gap), r3), vec2f(key_w, key_h), 44u, @engine.keys[KEY_COMMA] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 10.0 * (key_w + key_gap), r3), vec2f(key_w, key_h), 46u, @engine.keys[KEY_PERIOD] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_with_bg(coord.xy, vec2f(start_x + 11.0 * (key_w + key_gap), r3), vec2f(key_w, key_h), 47u, @engine.keys[KEY_SLASH] == 1u, @texture("font8x8.png")));
    color = blend_over(color, key_wide(coord.xy, vec2f(start_x + 12.0 * (key_w + key_gap), r3), vec2f(key_w * 2.0 + key_gap, key_h), @str("SH"), @engine.keys[KEY_SHIFT_RIGHT] == 1u, @texture("font8x8.png")));

    // Row 4: SPACE
    let r4 = start_y + 4.0 * (key_h + key_gap);
    color = blend_over(color, key_wide(coord.xy, vec2f(start_x +5.0 * (key_w + key_gap), r4), vec2f(key_w * 5.0, key_h), @str("SPACE"), @engine.keys[KEY_SPACE] == 1u, @texture("font8x8.png")));

    // ========================================================================
    // L/R Shoulder Buttons
    // ========================================================================
    let l_pressed = @engine.buttons[BTN_L] == 1;
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(140.0, 214.0), 130.0, 10.0,
        select(COLOR_BLACK, COLOR_DARKGRAY, l_pressed)
    ));

    let r_pressed = @engine.buttons[BTN_R] == 1;
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 214.0), 130.0, 10.0,
        select(COLOR_BLACK, COLOR_DARKGRAY, r_pressed)
    ));

    // ========================================================================
    // Center Connecting Bar
    // ========================================================================
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(200.0, 110.0), vec2f(240.0, 220.0), COLOR_LIGHTGRAY
    ));
    color = blend_over(color, draw_rect_outline(
        coord.xy, vec2f(200.0, 110.0), vec2f(240.0, 220.0), 2.0, COLOR_BLACK
    ));

    // ========================================================================
    // Left Controller (D-Pad)
    // ========================================================================
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(140.0, 220.0), 130.0, COLOR_LIGHTGRAY
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(140.0, 220.0), 130.0, 5.0, COLOR_BLACK
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(140.0, 220.0), 75.0, 5.0, COLOR_BLACK
    ));

    color = blend_over(color, draw_rect(
        coord.xy, vec2f(120.0, 200.0), vec2f(40.0, 40.0), COLOR_BLACK
    ));

    let up_pressed = @engine.buttons[BTN_UP] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(120.0, 160.0), vec2f(40.0, 40.0),
        select(COLOR_BLACK, COLOR_DARKGRAY, up_pressed)
    ));

    let right_pressed = @engine.buttons[BTN_RIGHT] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(160.0, 200.0), vec2f(40.0, 40.0),
        select(COLOR_BLACK, COLOR_DARKGRAY, right_pressed)
    ));

    let down_pressed = @engine.buttons[BTN_DOWN] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(120.0, 240.0), vec2f(40.0, 40.0),
        select(COLOR_BLACK, COLOR_DARKGRAY, down_pressed)
    ));

    let left_pressed = @engine.buttons[BTN_LEFT] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(80.0, 200.0), vec2f(40.0, 40.0),
        select(COLOR_BLACK, COLOR_DARKGRAY, left_pressed)
    ));

    // ========================================================================
    // Right Controller (Action Buttons)
    // ========================================================================
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(500.0, 220.0), 130.0, COLOR_LIGHTGRAY
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 220.0), 130.0, 5.0, COLOR_BLACK
    ));
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(500.0, 220.0), 105.0, COLOR_DARKGRAY
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 220.0), 105.0, 5.0, COLOR_BLACK
    ));
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(500.0, 220.0), 70.0, COLOR_RAYWHITE
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 220.0), 70.0, 4.0, COLOR_BLACK
    ));

    let y_pressed = @engine.buttons[BTN_Y] == 1;
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(460.0, 220.0), 23.0, COLOR_GREEN
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(460.0, 220.0), 23.0, 5.0,
        select(COLOR_BLACK, COLOR_LIME, y_pressed)
    ));

    let x_pressed = @engine.buttons[BTN_X] == 1;
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(500.0, 185.0), 23.0, COLOR_BLUE
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 185.0), 23.0, 5.0,
        select(COLOR_BLACK, COLOR_DARKBLUE, x_pressed)
    ));

    let b_pressed = @engine.buttons[BTN_B] == 1;
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(500.0, 255.0), 23.0, COLOR_YELLOW
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 255.0), 23.0, 5.0,
        select(COLOR_BLACK, COLOR_ORANGE, b_pressed)
    ));

    let a_pressed = @engine.buttons[BTN_A] == 1;
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(540.0, 220.0), 23.0, COLOR_RED
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(540.0, 220.0), 23.0, 5.0,
        select(COLOR_BLACK, COLOR_MAROON, a_pressed)
    ));

    // ========================================================================
    // Select/Start Buttons
    // ========================================================================
    let select_pressed = @engine.buttons[BTN_SELECT] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(260.0, 290.0), vec2f(45.0, 17.0), COLOR_BLACK
    ));
    color = blend_over(color, draw_rect_outline(
        coord.xy, vec2f(260.0, 290.0), vec2f(45.0, 17.0), 4.0,
        select(COLOR_BLACK, COLOR_DARKGRAY, select_pressed)
    ));

    let start_pressed = @engine.buttons[BTN_START] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(330.0, 290.0), vec2f(45.0, 17.0), COLOR_BLACK
    ));
    color = blend_over(color, draw_rect_outline(
        coord.xy, vec2f(330.0, 290.0), vec2f(45.0, 17.0), 4.0,
        select(COLOR_BLACK, COLOR_DARKGRAY, start_pressed)
    ));

    // ========================================================================
    // Mouse Cursor
    // ========================================================================
    let mp = @engine.mouse.xy;
    if (mp.x > 0.0 || mp.y > 0.0) {
        color = blend_over(color, draw_cross(coord.xy, mp, 20.0, 2.0, COLOR_WHITE));
        color = blend_over(color, draw_circle_outline(coord.xy, mp, 5.0, 1.5, COLOR_WHITE));
    }

    if (@engine.mouse.z > 0.0) {
        let cp = @engine.mouse.zw;
        color = blend_over(color, draw_circle(coord.xy, cp, 6.0, COLOR_RED));
        color = blend_over(color, draw_circle_outline(coord.xy, cp, 6.0, 2.0, COLOR_MAROON));
    }

    return color;
}
