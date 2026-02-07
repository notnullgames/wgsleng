/** @asset texture player.png */

// Compute shader bindings
@group(0) @binding(0) var<uniform> input_compute: Input;
@group(0) @binding(1) var<storage, read_write> state_compute: GameState;

// Render shader bindings  
@group(0) @binding(0) var player_texture: texture_2d<f32>;
@group(0) @binding(1) var player_sampler: sampler;
@group(1) @binding(0) var<storage, read> state_render: GameState;

const BTN_UP: u32 = 0u;
const BTN_DOWN: u32 = 1u;
const BTN_LEFT: u32 = 2u;
const BTN_RIGHT: u32 = 3u;

struct Input {
    buttons: array<u32, 12>,
    time: f32,
    delta_time: f32,
    screen_width: f32,
    screen_height: f32,
}

struct GameState {
    player_pos: vec2f,
    player_vel: vec2f,
}

@compute @workgroup_size(1)
fn update() {
    var vel = vec2f(0.0);
    
    if (input_compute.buttons[BTN_RIGHT] == 1u) { vel.x += 200.0; }
    if (input_compute.buttons[BTN_LEFT] == 1u) { vel.x -= 200.0; }
    if (input_compute.buttons[BTN_DOWN] == 1u) { vel.y += 200.0; }
    if (input_compute.buttons[BTN_UP] == 1u) { vel.y -= 200.0; }
    
    state_compute.player_vel = vel;
    state_compute.player_pos += vel * input_compute.delta_time;
    
    state_compute.player_pos.x = clamp(state_compute.player_pos.x, 32.0, input_compute.screen_width - 32.0);
    state_compute.player_pos.y = clamp(state_compute.player_pos.y, 32.0, input_compute.screen_height - 32.0);
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
    
    let player_pos = state_render.player_pos;
    let dist = coord.xy - player_pos;
    let sprite_size = 32.0;
    
    if (all(abs(dist) < vec2f(sprite_size))) {
        let uv = (dist + sprite_size) / (sprite_size * 2.0);
        let sprite = textureSampleLevel(player_texture, player_sampler, uv, 0.0);
        if (sprite.a > 0.1) {
            color = sprite;
        }
    }
    
    return color;
}
