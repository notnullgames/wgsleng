@set_title("Input Demo - Virtual Controller")
@set_size(640, 420)

@import("draw2d.wgsl")

// Game state - minimal for this demo
struct GameState {
    dummy: u32
}

@compute @workgroup_size(1)
fn update() {
    // No game logic needed for static display
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Gradient background (blue to green, top to bottom)
    var color = draw_gradient_linear(
        coord.xy,
        vec2f(0.0, 0.0),
        vec2f(0.0, @engine.screen_height),
        COLOR_BLUE,
        COLOR_GREEN
    );

    // ========================================================================
    // L/R Shoulder Buttons
    // ========================================================================

    // Left shoulder button
    let l_pressed = @engine.buttons[BTN_L] == 1;
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(140.0, 214.0), 130.0, 10.0,
        select(COLOR_BLACK, COLOR_DARKGRAY, l_pressed)
    ));

    // Right shoulder button
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

    // Outer ring
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(140.0, 220.0), 130.0, COLOR_LIGHTGRAY
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(140.0, 220.0), 130.0, 5.0, COLOR_BLACK
    ));

    // Inner circle for D-pad
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(140.0, 220.0), 75.0, 5.0, COLOR_BLACK
    ));

    // D-pad buttons
    // Center
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(120.0, 200.0), vec2f(40.0, 40.0), COLOR_BLACK
    ));

    // Up (North)
    let up_pressed = @engine.buttons[BTN_UP] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(120.0, 160.0), vec2f(40.0, 40.0),
        select(COLOR_BLACK, COLOR_DARKGRAY, up_pressed)
    ));

    // Right (East)
    let right_pressed = @engine.buttons[BTN_RIGHT] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(160.0, 200.0), vec2f(40.0, 40.0),
        select(COLOR_BLACK, COLOR_DARKGRAY, right_pressed)
    ));

    // Down (South)
    let down_pressed = @engine.buttons[BTN_DOWN] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(120.0, 240.0), vec2f(40.0, 40.0),
        select(COLOR_BLACK, COLOR_DARKGRAY, down_pressed)
    ));

    // Left (West)
    let left_pressed = @engine.buttons[BTN_LEFT] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(80.0, 200.0), vec2f(40.0, 40.0),
        select(COLOR_BLACK, COLOR_DARKGRAY, left_pressed)
    ));

    // ========================================================================
    // Right Controller (Action Buttons)
    // ========================================================================

    // Outer ring
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(500.0, 220.0), 130.0, COLOR_LIGHTGRAY
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 220.0), 130.0, 5.0, COLOR_BLACK
    ));

    // Inner pad
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(500.0, 220.0), 105.0, COLOR_DARKGRAY
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 220.0), 105.0, 5.0, COLOR_BLACK
    ));

    // Background for action buttons
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(500.0, 220.0), 70.0, COLOR_RAYWHITE
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 220.0), 70.0, 4.0, COLOR_BLACK
    ));

    // Y Button (Left) - Green
    let y_pressed = @engine.buttons[BTN_Y] == 1;
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(460.0, 220.0), 23.0, COLOR_GREEN
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(460.0, 220.0), 23.0, 5.0,
        select(COLOR_BLACK, COLOR_LIME, y_pressed)
    ));

    // X Button (Top) - Blue
    let x_pressed = @engine.buttons[BTN_X] == 1;
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(500.0, 185.0), 23.0, COLOR_BLUE
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 185.0), 23.0, 5.0,
        select(COLOR_BLACK, COLOR_DARKBLUE, x_pressed)
    ));

    // B Button (Bottom) - Yellow
    let b_pressed = @engine.buttons[BTN_B] == 1;
    color = blend_over(color, draw_circle(
        coord.xy, vec2f(500.0, 255.0), 23.0, COLOR_YELLOW
    ));
    color = blend_over(color, draw_circle_outline(
        coord.xy, vec2f(500.0, 255.0), 23.0, 5.0,
        select(COLOR_BLACK, COLOR_ORANGE, b_pressed)
    ));

    // A Button (Right) - Red
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

    // Select button
    let select_pressed = @engine.buttons[BTN_SELECT] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(260.0, 290.0), vec2f(45.0, 17.0), COLOR_BLACK
    ));
    color = blend_over(color, draw_rect_outline(
        coord.xy, vec2f(260.0, 290.0), vec2f(45.0, 17.0), 4.0,
        select(COLOR_BLACK, COLOR_DARKGRAY, select_pressed)
    ));

    // Start button
    let start_pressed = @engine.buttons[BTN_START] == 1;
    color = blend_over(color, draw_rect(
        coord.xy, vec2f(330.0, 290.0), vec2f(45.0, 17.0), COLOR_BLACK
    ));
    color = blend_over(color, draw_rect_outline(
        coord.xy, vec2f(330.0, 290.0), vec2f(45.0, 17.0), 4.0,
        select(COLOR_BLACK, COLOR_DARKGRAY, start_pressed)
    ));

    return color;
}
