@set_title("Bob-Bonker")
@set_size(800, 600)

@import("helpers.wgsl")

// Only define things that persist across frames
struct GameState {
    player_pos: vec2f,
    player_vel: vec2f,
    at_edge: u32,
}

@compute @workgroup_size(1)
fn update() {
    var vel = vec2f(0.0);
    if (@engine.buttons[BTN_RIGHT] == 1) { vel.x += 200.0; }
    if (@engine.buttons[BTN_LEFT] == 1) { vel.x -= 200.0; }
    if (@engine.buttons[BTN_DOWN] == 1) { vel.y += 200.0; }
    if (@engine.buttons[BTN_UP] == 1) { vel.y -= 200.0; }

    game_state.player_vel = vel;
    var new_pos = game_state.player_pos + vel * @engine.delta_time;

    let screen_size = vec2f(@engine.screen_width, @engine.screen_height);
    let hit_edge = is_at_edge(new_pos, 32.0, screen_size);

    if (hit_edge && game_state.at_edge == 0u) {
        @sound("bump.ogg").play();
    }

    game_state.at_edge = select(0u, 1u, hit_edge);
    game_state.player_pos = clamp_to_screen(new_pos, 32.0, screen_size);
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
    let dist = coord.xy - game_state.player_pos;
    
    if (all(abs(dist) < vec2f(32.0))) {
        let uv = (dist + 32.0) / 64.0;
        let sprite = textureSampleLevel(@texture("player.png"), @engine.sampler, uv, 0.0);
        if (sprite.a > 0.1) {
            color = sprite;
        }
    }
    
    return color;
}
