@set_title("RPG Example")
@set_size(640, 640)

@import("map/level1.wgsl")

// Convert linear color back to sRGB to undo unwanted sRGB->linear conversion by engine
fn linear_to_srgb(value: f32) -> f32 {
    if (value <= 0.0031308) {
        return value * 12.92;
    } else {
        return 1.055 * pow(value, 1.0 / 2.4) - 0.055;
    }
}

// Read tile ID with sRGB correction for native engine
fn get_tile_corrected(map_tex: texture_2d<f32>, x: u32, y: u32) -> u32 {
    if (x >= LEVEL1_WIDTH || y >= LEVEL1_HEIGHT) { return 0u; }
    let pixel = textureLoad(map_tex, vec2u(x, y), 0);
    // Undo sRGB->linear conversion to get original pixel values
    let r_corrected = linear_to_srgb(pixel.r);
    let g_corrected = linear_to_srgb(pixel.g);
    let low = u32(r_corrected * 255.0 + 0.5);
    let high = u32(g_corrected * 255.0 + 0.5);
    return low | (high << 8u);
}

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

// Helper to render a tile at a screen position
fn render_tile_layer(
    screen_pos: vec2f,
    layer_tex: texture_2d<f32>,
    camera_pos: vec2f,
    screen_size: vec2f
) -> vec4f {
    // Convert screen position to world position
    // world_pos = screen_pos (in world scale) - screen_center (in world scale) + camera_pos
    let world_pos = (screen_pos / ZOOM) - (screen_size / ZOOM) * 0.5 + camera_pos;

    // Check if world position is negative (outside map)
    if (world_pos.x < 0.0 || world_pos.y < 0.0) {
        return vec4f(0.0);
    }

    // Get tile coordinates (which tile we're in)
    let tile_x = u32(world_pos.x / LEVEL1_TILE_WIDTH);
    let tile_y = u32(world_pos.y / LEVEL1_TILE_HEIGHT);

    if (tile_x >= LEVEL1_WIDTH || tile_y >= LEVEL1_HEIGHT) {
        return vec4f(0.0);
    }

    // Use 16-bit version since this map has tile IDs > 255
    let tile_id = LEVEL1_get_tile_16bit(layer_tex, tile_x, tile_y);

    // Handle empty tiles
    if (tile_id == 0u) {
        return vec4f(0.0);
    }

    // Calculate offset within the tile (0.0 to 1.0)
    // Use screen_pos divided by zoom to avoid precision issues with camera offset
    let scaled_pos = screen_pos / ZOOM;
    let tile_offset_x = fract(scaled_pos.x / LEVEL1_TILE_WIDTH);
    let tile_offset_y = fract(scaled_pos.y / LEVEL1_TILE_HEIGHT);
    let tile_offset = vec2f(tile_offset_x, tile_offset_y);

    // Determine which tileset to use based on tile_id
    var texture_sample: vec4f;

    if (tile_id >= 641u) {
        let uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_HEROES, tile_id, tile_offset);
        texture_sample = textureSampleLevel(@texture("map/heroes.png"), @engine.sampler, uv, 0.0);
    } else if (tile_id >= 385u) {
        let uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_TILEB, tile_id, tile_offset);
        texture_sample = textureSampleLevel(@texture("map/tileB.png"), @engine.sampler, uv, 0.0);
    } else if (tile_id >= 193u) {
        let uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_TILEA2, tile_id, tile_offset);
        texture_sample = textureSampleLevel(@texture("map/tileA2.png"), @engine.sampler, uv, 0.0);
    } else if (tile_id >= 1u) {
        let uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_TILEA1, tile_id, tile_offset);
        texture_sample = textureSampleLevel(@texture("map/tileA1.png"), @engine.sampler, uv, 0.0);
    } else {
        // tile_id == 0, should have been caught earlier
        texture_sample = vec4f(0.0);
    }

    return texture_sample;
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let screen_size = vec2f(@engine.screen_width, @engine.screen_height);

    // For now, always use map center as camera position
    let camera_pos = vec2f(
        f32(LEVEL1_WIDTH) * LEVEL1_TILE_WIDTH * 0.5,
        f32(LEVEL1_HEIGHT) * LEVEL1_TILE_HEIGHT * 0.5
    );

    var color = vec4f(0.0, 0.0, 0.0, 1.0); // Black background

    // Render ground layer
    let world_pos = (coord.xy / ZOOM) - (screen_size / ZOOM) * 0.5 + camera_pos;

    if (world_pos.x >= 0.0 && world_pos.y >= 0.0) {
        let tile_x = u32(world_pos.x / LEVEL1_TILE_WIDTH);
        let tile_y = u32(world_pos.y / LEVEL1_TILE_HEIGHT);

        if (tile_x < LEVEL1_WIDTH && tile_y < LEVEL1_HEIGHT) {
            // Use sRGB-corrected tile reading for native compatibility
            let tile_id = get_tile_corrected(@texture("map/level1_ground.png"), tile_x, tile_y);

            if (tile_id > 0u) {
                let tile_offset = fract(world_pos / vec2f(LEVEL1_TILE_WIDTH, LEVEL1_TILE_HEIGHT));

                // Ground layer should only have TILEA1 (1-192) or TILEA2 (193-384) tiles
                if (tile_id >= 193u && tile_id < 385u) {
                    let uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_TILEA2, tile_id, tile_offset);
                    color = textureSampleLevel(@texture("map/tileA2.png"), @engine.sampler, uv, 0.0);
                    color = vec4f(color.rgb, 1.0); // Force opaque
                } else if (tile_id >= 1u && tile_id < 193u) {
                    let uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_TILEA1, tile_id, tile_offset);
                    color = textureSampleLevel(@texture("map/tileA1.png"), @engine.sampler, uv, 0.0);
                    color = vec4f(color.rgb, 1.0); // Force opaque
                }
                // else: tile_id is out of expected range, leave color as black background
            }
        }
    }

    // TODO: Render stuff layer

    // Draw player sprite (centered on screen)
    var player_pos = @engine.state.player_pos;
    if (player_pos.x == 0.0 && player_pos.y == 0.0) {
        player_pos = camera_pos;
    }

    // Convert player world position to screen position
    let player_screen = (player_pos - camera_pos + (screen_size / ZOOM) * 0.5) * ZOOM;
    let dist = coord.xy - player_screen;

    if (all(abs(dist) < vec2f(8.0 * ZOOM))) {
        let uv = (dist + (8.0 * ZOOM)) / (16.0 * ZOOM);
        // Using first hero sprite (facing down)
        let player_uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_HEROES, 641u, uv);
        let sprite = textureSampleLevel(@texture("map/heroes.png"), @engine.sampler, player_uv, 0.0);
        if (sprite.a > 0.5) {
            color = sprite;
        }
    }

    return color;
}
