@set_title("Tetris")

// 320x400 * DISPLAY_SCALE
@set_size(960, 1200)

@import("draw2d.wgsl")
@import("font.wgsl")

// Display scaling (game renders at 320x400, displayed at 640x800)
const DISPLAY_SCALE = 3.0;

// Grid settings
const GRID_WIDTH = 10.0;
const GRID_HEIGHT = 18.0;
const CELL_SIZE = 20.0;
const BOARD_SIZE = 180u; // 10 * 18

// Piece types (0 = empty, 1-7 = pieces)
const PIECE_NONE = 0.0;
const PIECE_I = 1.0;
const PIECE_O = 2.0;
const PIECE_T = 3.0;
const PIECE_S = 4.0;
const PIECE_Z = 5.0;
const PIECE_J = 6.0;
const PIECE_L = 7.0;

// Game timing
const INITIAL_FALL_SPEED = 1.0; // seconds per cell
const FAST_FALL_SPEED = 0.05;   // when holding down
const LOCK_DELAY = 0.5;          // delay before piece locks

// Game state that persists across frames
// Organized to minimize padding: vec2f first, then scalars, then array
struct GameState {
    current_pos: vec2f,       // current piece position (8 bytes)
    current_piece: f32,       // current piece type
    current_rotation: f32,    // 0-3 rotation state
    next_piece: f32,          // next piece to spawn
    score: f32,               // player score
    lines: f32,               // lines cleared
    level: f32,               // current level
    fall_timer: f32,          // timer for automatic falling
    lock_timer: f32,          // timer for piece lock delay
    game_over: f32,           // game over flag
    move_timer: f32,          // timer for move repeat delay
    board: array<f32, 180>,  // 10x18 grid (720 bytes)
}

// Get piece shape data for a given piece type and rotation
// Returns a 4x4 grid encoded as which cells are filled
fn get_piece_shape(piece_type: f32, rotation: f32) -> array<f32, 16> {
    var shape: array<f32, 16>;
    let rot = i32(rotation) % 4;

    // I piece (cyan)
    if (piece_type == PIECE_I) {
        if (rot == 0 || rot == 2) {
            shape[4] = 1.0; shape[5] = 1.0; shape[6] = 1.0; shape[7] = 1.0;
        } else {
            shape[1] = 1.0; shape[5] = 1.0; shape[9] = 1.0; shape[13] = 1.0;
        }
    }
    // O piece (yellow)
    else if (piece_type == PIECE_O) {
        shape[5] = 1.0; shape[6] = 1.0;
        shape[9] = 1.0; shape[10] = 1.0;
    }
    // T piece (purple)
    else if (piece_type == PIECE_T) {
        if (rot == 0) {
            shape[1] = 1.0; shape[4] = 1.0; shape[5] = 1.0; shape[6] = 1.0;
        } else if (rot == 1) {
            shape[1] = 1.0; shape[5] = 1.0; shape[6] = 1.0; shape[9] = 1.0;
        } else if (rot == 2) {
            shape[4] = 1.0; shape[5] = 1.0; shape[6] = 1.0; shape[9] = 1.0;
        } else {
            shape[1] = 1.0; shape[4] = 1.0; shape[5] = 1.0; shape[9] = 1.0;
        }
    }
    // S piece (green)
    else if (piece_type == PIECE_S) {
        if (rot == 0 || rot == 2) {
            shape[1] = 1.0; shape[2] = 1.0; shape[4] = 1.0; shape[5] = 1.0;
        } else {
            shape[1] = 1.0; shape[5] = 1.0; shape[6] = 1.0; shape[10] = 1.0;
        }
    }
    // Z piece (red)
    else if (piece_type == PIECE_Z) {
        if (rot == 0 || rot == 2) {
            shape[0] = 1.0; shape[1] = 1.0; shape[5] = 1.0; shape[6] = 1.0;
        } else {
            shape[2] = 1.0; shape[5] = 1.0; shape[6] = 1.0; shape[9] = 1.0;
        }
    }
    // J piece (blue)
    else if (piece_type == PIECE_J) {
        if (rot == 0) {
            shape[0] = 1.0; shape[4] = 1.0; shape[5] = 1.0; shape[6] = 1.0;
        } else if (rot == 1) {
            shape[1] = 1.0; shape[2] = 1.0; shape[5] = 1.0; shape[9] = 1.0;
        } else if (rot == 2) {
            shape[4] = 1.0; shape[5] = 1.0; shape[6] = 1.0; shape[10] = 1.0;
        } else {
            shape[1] = 1.0; shape[5] = 1.0; shape[8] = 1.0; shape[9] = 1.0;
        }
    }
    // L piece (orange)
    else if (piece_type == PIECE_L) {
        if (rot == 0) {
            shape[2] = 1.0; shape[4] = 1.0; shape[5] = 1.0; shape[6] = 1.0;
        } else if (rot == 1) {
            shape[1] = 1.0; shape[5] = 1.0; shape[9] = 1.0; shape[10] = 1.0;
        } else if (rot == 2) {
            shape[4] = 1.0; shape[5] = 1.0; shape[6] = 1.0; shape[8] = 1.0;
        } else {
            shape[0] = 1.0; shape[1] = 1.0; shape[5] = 1.0; shape[9] = 1.0;
        }
    }

    return shape;
}

// Get color for piece type
fn get_piece_color(piece_type: f32) -> vec4f {
    if (piece_type == PIECE_I) { return COLOR_CYAN; }
    if (piece_type == PIECE_O) { return COLOR_YELLOW; }
    if (piece_type == PIECE_T) { return COLOR_PURPLE; }
    if (piece_type == PIECE_S) { return COLOR_GREEN; }
    if (piece_type == PIECE_Z) { return COLOR_RED; }
    if (piece_type == PIECE_J) { return COLOR_BLUE; }
    if (piece_type == PIECE_L) { return COLOR_ORANGE; }
    return COLOR_GRAY;
}

// Wrapper functions for font rendering (handles texture sampling)
fn draw_char(color: vec4f, coord: vec2f, char_code: u32, pos: vec2f, size: f32) -> vec4f {
    let uv_data = get_char_uv(coord, char_code, pos, size);
    if (uv_data.z < 0.5) { // Not within bounds
        return color;
    }

    let glyph = textureSampleLevel(@texture("16X16-F6.png"), @engine.sampler, uv_data.xy, 0.0);
    // Font is white-on-black, use brightness to determine if pixel is character or background
    let brightness = (glyph.r + glyph.g + glyph.b) / 3.0;
    if (brightness > 0.5) {
        return vec4f(1.0, 1.0, 1.0, 1.0); // White text
    }
    return color;
}

fn draw_digit(color: vec4f, coord: vec2f, digit: u32, pos: vec2f, size: f32) -> vec4f {
    return draw_char(color, coord, 48u + digit, pos, size);
}

fn draw_number(color: vec4f, coord: vec2f, number: u32, pos: vec2f, size: f32) -> vec4f {
    let uv_data = get_number_uv(coord, number, pos, size);
    if (uv_data.z < 0.5) { // Not within bounds
        return color;
    }

    let glyph = textureSampleLevel(@texture("16X16-F6.png"), @engine.sampler, uv_data.xy, 0.0);
    // Font is white-on-black, so check brightness instead of alpha
    let brightness = (glyph.r + glyph.g + glyph.b) / 3.0;
    if (brightness > 0.5) {
        return vec4f(1.0, 1.0, 1.0, 1.0); // White text
    }
    return color;
}

// Better random number generator with hash mixing
fn hash_u32(x: u32) -> u32 {
    var h = x;
    h = h ^ (h >> 16u);
    h = h * 0x85ebca6bu;
    h = h ^ (h >> 13u);
    h = h * 0xc2b2ae35u;
    h = h ^ (h >> 16u);
    return h;
}

fn random_f32(seed: f32) -> f32 {
    let s = u32(seed * 1000.0);
    let r = hash_u32(s);
    return f32(r) / f32(0xFFFFFFFFu);
}

// Get random piece type (1-7)
fn random_piece(seed: f32) -> f32 {
    return floor(random_f32(seed) * 7.0) + 1.0;
}

// Check if piece collides with board
fn check_collision(state: ptr<function, GameState>, piece_type: f32, pos: vec2f, rotation: f32) -> bool {
    let shape = get_piece_shape(piece_type, rotation);

    for (var y = 0; y < 4; y++) {
        for (var x = 0; x < 4; x++) {
            let idx = y * 4 + x;
            if (shape[idx] > 0.5) {
                let board_x = i32(pos.x) + x;
                let board_y = i32(pos.y) + y;

                // Check bounds
                if (board_x < 0 || board_x >= i32(GRID_WIDTH) ||
                    board_y < 0 || board_y >= i32(GRID_HEIGHT)) {
                    return true;
                }

                // Check board cell
                let board_idx = u32(board_y * i32(GRID_WIDTH) + board_x);
                if (board_idx < BOARD_SIZE && (*state).board[board_idx] > 0.5) {
                    return true;
                }
            }
        }
    }

    return false;
}

// Lock current piece to board
fn lock_piece(state: ptr<function, GameState>) {
    let shape = get_piece_shape((*state).current_piece, (*state).current_rotation);

    for (var y = 0; y < 4; y++) {
        for (var x = 0; x < 4; x++) {
            let idx = y * 4 + x;
            if (shape[idx] > 0.5) {
                let board_x = i32((*state).current_pos.x) + x;
                let board_y = i32((*state).current_pos.y) + y;

                if (board_x >= 0 && board_x < i32(GRID_WIDTH) &&
                    board_y >= 0 && board_y < i32(GRID_HEIGHT)) {
                    let board_idx = u32(board_y * i32(GRID_WIDTH) + board_x);
                    if (board_idx < BOARD_SIZE) {
                        (*state).board[board_idx] = (*state).current_piece;
                    }
                }
            }
        }
    }
}

// Check and clear full lines
fn clear_lines(state: ptr<function, GameState>) -> f32 {
    var lines_cleared = 0.0;

    for (var y = i32(GRID_HEIGHT) - 1; y >= 0; y--) {
        var full = true;

        // Check if line is full
        for (var x = 0; x < i32(GRID_WIDTH); x++) {
            let idx = u32(y * i32(GRID_WIDTH) + x);
            if ((*state).board[idx] < 0.5) {
                full = false;
                break;
            }
        }

        if (full) {
            lines_cleared += 1.0;

            // Move lines down
            for (var move_y = y; move_y > 0; move_y--) {
                for (var x = 0; x < i32(GRID_WIDTH); x++) {
                    let src_idx = u32((move_y - 1) * i32(GRID_WIDTH) + x);
                    let dst_idx = u32(move_y * i32(GRID_WIDTH) + x);
                    (*state).board[dst_idx] = (*state).board[src_idx];
                }
            }

            // Clear top line
            for (var x = 0; x < i32(GRID_WIDTH); x++) {
                (*state).board[u32(x)] = 0.0;
            }

            y++; // Check same line again
        }
    }

    return lines_cleared;
}

// Spawn new piece
fn spawn_piece(state: ptr<function, GameState>, time: f32) {
    (*state).current_piece = (*state).next_piece;
    (*state).current_pos = vec2f(3.0, 0.0); // Start at top center
    (*state).current_rotation = 0.0;
    (*state).lock_timer = 0.0;

    // Generate next piece with better entropy
    // Mix time, score, lines, and current piece for better randomness
    let seed = time * 1000.0 + (*state).score * 7.919 + (*state).lines * 13.37 + (*state).current_piece * 3.14159;
    (*state).next_piece = random_piece(seed);

    // Check if game over (piece spawns in collision)
    if (check_collision(state, (*state).current_piece, (*state).current_pos, (*state).current_rotation)) {
        (*state).game_over = 1.0;
    }
}

// Initialize game
fn init_game(state: ptr<function, GameState>) {
    // Clear board
    for (var i = 0u; i < BOARD_SIZE; i++) {
        (*state).board[i] = 0.0;
    }

    (*state).score = 0.0;
    (*state).lines = 0.0;
    (*state).level = 1.0;
    (*state).fall_timer = 0.0;
    (*state).lock_timer = 0.0;
    (*state).game_over = 0.0;
    (*state).move_timer = 0.0;

    // Spawn first pieces
    (*state).next_piece = random_piece(1.0);
    spawn_piece(state, 0.0);
}

@compute @workgroup_size(1)
fn update() {
    // Initialize on first frame
    if (@engine.state.current_piece < 0.5) {
        var state = @engine.state;
        init_game(&state);
        @engine.state = state;
        return;
    }

    // Game over - press any button to restart
    if (@engine.state.game_over > 0.5) {
        if (@engine.buttons[BTN_A] == 1 || @engine.buttons[BTN_B] == 1 ||
            @engine.buttons[BTN_START] == 1) {
            var state = @engine.state;
            init_game(&state);
            @engine.state = state;
        }
        return;
    }

    var state = @engine.state;

    // Handle rotation (A button/UP)
    if ((@engine.buttons[BTN_A] == 1 || @engine.buttons[BTN_UP] == 1) && state.move_timer <= 0.0) {
        let new_rotation = (state.current_rotation + 1.0) % 4.0;
        if (!check_collision(&state, state.current_piece, state.current_pos, new_rotation)) {
            state.current_rotation = new_rotation;
            state.move_timer = 0.1; // Small delay
        }
    }

    // Handle horizontal movement
    var move_x = 0.0;
    if (@engine.buttons[BTN_LEFT] == 1 && state.move_timer <= 0.0) {
        move_x = -1.0;
        state.move_timer = 0.1;
    } else if (@engine.buttons[BTN_RIGHT] == 1 && state.move_timer <= 0.0) {
        move_x = 1.0;
        state.move_timer = 0.1;
    }

    if (move_x != 0.0) {
        let new_pos = state.current_pos + vec2f(move_x, 0.0);
        if (!check_collision(&state, state.current_piece, new_pos, state.current_rotation)) {
            state.current_pos = new_pos;
            state.lock_timer = 0.0; // Reset lock timer on movement
        }
    }

    // Update move timer
    if (state.move_timer > 0.0) {
        state.move_timer -= @engine.delta_time;
    }

    // Calculate fall speed based on level and down button
    var fall_speed = INITIAL_FALL_SPEED / state.level;
    if (@engine.buttons[BTN_DOWN] == 1) {
        fall_speed = FAST_FALL_SPEED;
    }

    // Update fall timer
    state.fall_timer += @engine.delta_time;

    if (state.fall_timer >= fall_speed) {
        state.fall_timer = 0.0;

        // Try to move down
        let new_pos = state.current_pos + vec2f(0.0, 1.0);
        if (!check_collision(&state, state.current_piece, new_pos, state.current_rotation)) {
            state.current_pos = new_pos;
            state.lock_timer = 0.0;
        } else {
            // Can't move down - start lock timer
            state.lock_timer += fall_speed;

            if (state.lock_timer >= LOCK_DELAY) {
                // Lock piece
                lock_piece(&state);

                // Clear lines
                let cleared = clear_lines(&state);
                if (cleared > 0.5) {
                    state.lines += cleared;
                    // Scoring: 100 * cleared * level
                    state.score += 100.0 * cleared * state.level;
                    // Level up every 10 lines
                    state.level = floor(state.lines / 10.0) + 1.0;
                }

                // Spawn new piece
                spawn_piece(&state, @engine.time);
            }
        }
    }

    @engine.state = state;
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Scale coordinates down to game resolution (320x400)
    let game_coord = vec4f(coord.xy / DISPLAY_SCALE, coord.z, coord.w);

    let offset_x = 20.0; // Left padding
    let offset_y = 20.0; // Top padding

    // Background
    var color = vec4f(0.05, 0.05, 0.1, 1.0);

    // Draw board background
    if (game_coord.x >= offset_x && game_coord.x < offset_x + GRID_WIDTH * CELL_SIZE &&
        game_coord.y >= offset_y && game_coord.y < offset_y + GRID_HEIGHT * CELL_SIZE) {
        color = vec4f(0.1, 0.1, 0.15, 1.0);

        // Grid lines
        let grid_x = (game_coord.x - offset_x) / CELL_SIZE;
        let grid_y = (game_coord.y - offset_y) / CELL_SIZE;
        if (fract(grid_x) < 0.05 || fract(grid_y) < 0.05) {
            color = blend_over(color, vec4f(0.15, 0.15, 0.2, 1.0));
        }
    }

    // Calculate grid cell
    let cell_x = floor((game_coord.x - offset_x) / CELL_SIZE);
    let cell_y = floor((game_coord.y - offset_y) / CELL_SIZE);

    // Draw placed pieces on board
    if (cell_x >= 0.0 && cell_x < GRID_WIDTH && cell_y >= 0.0 && cell_y < GRID_HEIGHT) {
        let board_idx = u32(cell_y * GRID_WIDTH + cell_x);
        if (board_idx < BOARD_SIZE) {
            let piece_type = @engine.state.board[board_idx];
            if (piece_type > 0.5) {
                let piece_color = get_piece_color(piece_type);
                color = blend_over(color, draw_rect(
                    game_coord.xy,
                    vec2f(offset_x + cell_x * CELL_SIZE + 1.0, offset_y + cell_y * CELL_SIZE + 1.0),
                    vec2f(CELL_SIZE - 2.0),
                    piece_color
                ));
            }
        }
    }

    // Draw current piece
    if (@engine.state.game_over < 0.5) {
        let shape = get_piece_shape(@engine.state.current_piece, @engine.state.current_rotation);
        let piece_color = get_piece_color(@engine.state.current_piece);

        for (var y = 0; y < 4; y++) {
            for (var x = 0; x < 4; x++) {
                let idx = y * 4 + x;
                if (shape[idx] > 0.5) {
                    let px = @engine.state.current_pos.x + f32(x);
                    let py = @engine.state.current_pos.y + f32(y);

                    if (all(vec2f(px, py) == vec2f(cell_x, cell_y))) {
                        color = blend_over(color, draw_rect(
                            game_coord.xy,
                            vec2f(offset_x + px * CELL_SIZE + 1.0, offset_y + py * CELL_SIZE + 1.0),
                            vec2f(CELL_SIZE - 2.0),
                            piece_color
                        ));
                    }
                }
            }
        }
    }

    // Draw next piece preview
    let preview_x = offset_x + GRID_WIDTH * CELL_SIZE + 20.0;
    let preview_y = 20.0;
    let preview_shape = get_piece_shape(@engine.state.next_piece, 0.0);
    let preview_color = get_piece_color(@engine.state.next_piece);

    for (var y = 0; y < 4; y++) {
        for (var x = 0; x < 4; x++) {
            let idx = y * 4 + x;
            if (preview_shape[idx] > 0.5) {
                color = blend_over(color, draw_rect(
                    game_coord.xy,
                    vec2f(preview_x + f32(x) * 15.0, preview_y + f32(y) * 15.0),
                    vec2f(13.0),
                    preview_color
                ));
            }
        }
    }

    // Draw score and lines
    let score_x = preview_x;
    let score_y = preview_y + 80.0;

    // Draw "SCORE" label at smaller size (8x8)
    color = draw_char(color, game_coord.xy, 83u, vec2f(score_x, score_y), 8.0); // 'S'
    color = draw_char(color, game_coord.xy, 67u, vec2f(score_x + 8.0, score_y), 8.0); // 'C'
    color = draw_char(color, game_coord.xy, 79u, vec2f(score_x + 16.0, score_y), 8.0); // 'O'
    color = draw_char(color, game_coord.xy, 82u, vec2f(score_x + 24.0, score_y), 8.0); // 'R'
    color = draw_char(color, game_coord.xy, 69u, vec2f(score_x + 32.0, score_y), 8.0); // 'E'

    // Draw score value (with more spacing)
    color = draw_number(color, game_coord.xy, u32(@engine.state.score), vec2f(score_x, score_y + 12.0), 8.0);

    // Draw lines text (with more spacing)
    let lines_y = score_y + 32.0;
    color = draw_char(color, game_coord.xy, 76u, vec2f(score_x, lines_y), 8.0); // 'L'
    color = draw_char(color, game_coord.xy, 73u, vec2f(score_x + 8.0, lines_y), 8.0); // 'I'
    color = draw_char(color, game_coord.xy, 78u, vec2f(score_x + 16.0, lines_y), 8.0); // 'N'
    color = draw_char(color, game_coord.xy, 69u, vec2f(score_x + 24.0, lines_y), 8.0); // 'E'
    color = draw_char(color, game_coord.xy, 83u, vec2f(score_x + 32.0, lines_y), 8.0); // 'S'

    // Draw lines value (with more spacing)
    color = draw_number(color, game_coord.xy, u32(@engine.state.lines), vec2f(score_x, lines_y + 12.0), 8.0);

    // Game over overlay
    if (@engine.state.game_over > 0.5) {
        // Red overlay
        color = blend_over(color, vec4f(0.5, 0.0, 0.0, 0.3));
    }

    return color;
}
