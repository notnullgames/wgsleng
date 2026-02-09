@set_title("RPG Example")
@set_size(640, 640)

@import("map/level1.wgsl")

// Camera and player state
struct GameState {
    player_pos: vec2f,
    camera_pos: vec2f,
    player_vel: vec2f,
}

// Movement speed
const MOVE_SPEED = 64.0;

// Zoom factor (makes tiles appear larger on screen)
const ZOOM = 4.0;

@compute @workgroup_size(1)
fn update() {
    // Initialize player position to center of map on first frame
    if (@engine.state.player_pos.x == 0.0 && @engine.state.player_pos.y == 0.0) {
        let map_center = vec2f(
            f32(LEVEL1_WIDTH) * LEVEL1_TILE_WIDTH * 0.5,
            f32(LEVEL1_HEIGHT) * LEVEL1_TILE_HEIGHT * 0.5
        );
        @engine.state.player_pos = map_center;
        @engine.state.camera_pos = map_center;
    }

    var vel = vec2f(0.0);

    // Handle input
    if (@engine.buttons[BTN_RIGHT] == 1) { vel.x += 1.0; }
    if (@engine.buttons[BTN_LEFT] == 1) { vel.x -= 1.0; }
    if (@engine.buttons[BTN_DOWN] == 1) { vel.y += 1.0; }
    if (@engine.buttons[BTN_UP] == 1) { vel.y -= 1.0; }

    // Normalize diagonal movement
    if (length(vel) > 0.0) {
        vel = normalize(vel) * MOVE_SPEED;
    }

    @engine.state.player_vel = vel;
    @engine.state.player_pos = @engine.state.player_pos + vel * @engine.delta_time;

    // TODO: Collision checking removed - textures not available in compute shader on native
    // Need to either:
    // - Add texture bindings to compute pipeline in engine
    // - Store collision data in a storage buffer instead
    // - Handle collision in fragment shader (not ideal)

    // Camera follows player
    @engine.state.camera_pos = @engine.state.player_pos;
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

// Helper to render a map layer at given tile coordinates
fn render_layer(layer_tex: texture_2d<f32>, tile_x: u32, tile_y: u32, tile_offset: vec2f) -> vec4f {
    if (tile_x >= LEVEL1_WIDTH || tile_y >= LEVEL1_HEIGHT) {
        return vec4f(0.0);
    }

    let tile_id = LEVEL1_get_tile_16bit(layer_tex, tile_x, tile_y);
    if (tile_id == 0u) {
        return vec4f(0.0);
    }

    // Determine which tileset to use and sample
    var uv: vec2f;
    if (tile_id >= 641u) {
        uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_HEROES, tile_id, tile_offset);
        return textureSampleLevel(@texture("map/heroes.png"), @engine.sampler, uv, 0.0);
    } else if (tile_id >= 385u) {
        uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_TILEB, tile_id, tile_offset);
        return textureSampleLevel(@texture("map/tileB.png"), @engine.sampler, uv, 0.0);
    } else if (tile_id >= 193u) {
        uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_TILEA2, tile_id, tile_offset);
        return textureSampleLevel(@texture("map/tileA2.png"), @engine.sampler, uv, 0.0);
    } else if (tile_id >= 1u) {
        uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_TILEA1, tile_id, tile_offset);
        return textureSampleLevel(@texture("map/tileA1.png"), @engine.sampler, uv, 0.0);
    }
    return vec4f(0.0);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let screen_size = vec2f(@engine.screen_width, @engine.screen_height);
    let camera_pos = vec2f(
        f32(LEVEL1_WIDTH) * LEVEL1_TILE_WIDTH * 0.5,
        f32(LEVEL1_HEIGHT) * LEVEL1_TILE_HEIGHT * 0.5
    );

    // Calculate world position and tile coordinates
    let world_pos = (coord.xy / ZOOM) - (screen_size / ZOOM) * 0.5 + camera_pos;

    var color = vec4f(0.0, 0.0, 0.0, 1.0); // Black background

    if (world_pos.x >= 0.0 && world_pos.y >= 0.0) {
        let tile_x = u32(world_pos.x / LEVEL1_TILE_WIDTH);
        let tile_y = u32(world_pos.y / LEVEL1_TILE_HEIGHT);
        let tile_offset = fract(world_pos / vec2f(LEVEL1_TILE_WIDTH, LEVEL1_TILE_HEIGHT));

        // Render ground layer (opaque)
        let ground = render_layer(@texture("map/level1_ground.png"), tile_x, tile_y, tile_offset);
        if (ground.a > 0.0) {
            color = vec4f(ground.rgb, 1.0);
        }

        // Render stuff layer (with alpha blending)
        let stuff = render_layer(@texture("map/level1_stuff.png"), tile_x, tile_y, tile_offset);
        if (stuff.a > 0.5) {
            color = stuff;
        }
    }

    // Draw player sprite (centered on screen)
    var player_pos = @engine.state.player_pos;
    if (player_pos.x == 0.0 && player_pos.y == 0.0) {
        player_pos = camera_pos;
    }

    let player_screen = (player_pos - camera_pos + (screen_size / ZOOM) * 0.5) * ZOOM;
    let dist = coord.xy - player_screen;

    if (all(abs(dist) < vec2f(8.0 * ZOOM))) {
        let uv = (dist + (8.0 * ZOOM)) / (16.0 * ZOOM);
        let player_uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_HEROES, 641u, uv);
        let sprite = textureSampleLevel(@texture("map/heroes.png"), @engine.sampler, player_uv, 0.0);
        if (sprite.a > 0.5) {
            color = sprite;
        }
    }

    return color;
}
