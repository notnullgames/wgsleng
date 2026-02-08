@set_title("Snake")
@set_size(600, 600)

@import("draw2d.wgsl")

// Grid settings
const GRID_SIZE = 20.0;     // 20x20 grid
const CELL_SIZE = 30.0;     // pixels per cell
const MAX_SNAKE_LENGTH = 100u; // max segments (fits in buffer safely)
const MOVE_DELAY = 0.15;    // seconds between moves

// Game state that persists across frames
// Note: Using vec2f for compatibility with engine's size calculation
struct GameState {
    length: f32,              // current snake length
    direction: vec2f,         // current direction (-1,0,1 values)
    next_direction: vec2f,    // buffered input for next move
    food_pos: vec2f,          // food position in grid coords
    move_timer: f32,          // timer for movement
    game_over: f32,           // 0 = playing, 1 = game over
    score: f32,               // player score
    snake: array<vec2f, 100>, // snake body positions (x, y) in grid coords
}

// Simple random number generator (LCG)
fn random_f32(seed: f32) -> f32 {
    let s = u32(seed * 1000.0);
    let r = (s * 1103515245u + 12345u) & 0x7FFFFFFFu;
    return f32(r) / f32(0x7FFFFFFFu);
}

// Generate random grid position that doesn't overlap with snake
fn spawn_food(state: ptr<function, GameState>) -> vec2f {
    // Use score and length as seed for variation
    var seed = (*state).score * 79.19 + (*state).length * 35.71 + (*state).move_timer * 10.0;

    // Try up to 100 times to find a free spot
    for (var attempt = 0; attempt < 100; attempt++) {
        seed = random_f32(seed) * 1000.0;
        let x = floor(random_f32(seed) * GRID_SIZE);
        seed = random_f32(seed + 1.0) * 1000.0;
        let y = floor(random_f32(seed) * GRID_SIZE);
        let pos = vec2f(x, y);

        // Check if this position overlaps with snake
        var overlap = false;
        let len = u32((*state).length);
        for (var i = 0u; i < len; i++) {
            if (all((*state).snake[i] == pos)) {
                overlap = true;
                break;
            }
        }

        if (!overlap) {
            return pos;
        }
    }

    // Fallback: return center position
    return vec2f(GRID_SIZE * 0.5, GRID_SIZE * 0.5);
}

// Check if position collides with snake body (excluding head)
fn check_self_collision(state: ptr<function, GameState>, pos: vec2f) -> bool {
    // Start from index 1 to skip the head
    let len = u32((*state).length);
    for (var i = 1u; i < len; i++) {
        if (all((*state).snake[i] == pos)) {
            return true;
        }
    }
    return false;
}

// Initialize game state
fn init_game(state: ptr<function, GameState>) {
    (*state).length = 1.0;
    (*state).snake[0] = vec2f(floor(GRID_SIZE * 0.5), floor(GRID_SIZE * 0.5)); // center
    (*state).direction = vec2f(1.0, 0.0); // moving right
    (*state).next_direction = vec2f(1.0, 0.0);
    (*state).food_pos = spawn_food(state);
    (*state).move_timer = 0.0;
    (*state).game_over = 0.0;
    (*state).score = 0.0;
}

@compute @workgroup_size(1)
fn update() {
    // Initialize on first frame (when direction is zero)
    if (@engine.state.direction.x == 0.0 && @engine.state.direction.y == 0.0) {
        var state = @engine.state;
        init_game(&state);
        @engine.state = state;
        return;
    }

    // Game over - press any non-dpad button to restart
    if (@engine.state.game_over > 0.5) {
        // Check A, B, X, Y, L, R, START, SELECT buttons
        if (@engine.buttons[BTN_A] == 1 || @engine.buttons[BTN_B] == 1 ||
            @engine.buttons[BTN_X] == 1 || @engine.buttons[BTN_Y] == 1 ||
            @engine.buttons[BTN_L] == 1 || @engine.buttons[BTN_R] == 1 ||
            @engine.buttons[BTN_START] == 1 || @engine.buttons[BTN_SELECT] == 1) {
            var state = @engine.state;
            init_game(&state);
            @engine.state = state;
        }
        return;
    }

    // Handle input - buffer the next direction
    let current_dir = @engine.state.direction;
    var next_dir = @engine.state.next_direction;

    if (@engine.buttons[BTN_UP] == 1 && current_dir.y != 1.0) {
        next_dir = vec2f(0.0, -1.0);
    } else if (@engine.buttons[BTN_DOWN] == 1 && current_dir.y != -1.0) {
        next_dir = vec2f(0.0, 1.0);
    } else if (@engine.buttons[BTN_LEFT] == 1 && current_dir.x != 1.0) {
        next_dir = vec2f(-1.0, 0.0);
    } else if (@engine.buttons[BTN_RIGHT] == 1 && current_dir.x != -1.0) {
        next_dir = vec2f(1.0, 0.0);
    }

    @engine.state.next_direction = next_dir;

    // Update movement timer
    @engine.state.move_timer += @engine.delta_time;

    // Only move when timer expires
    if (@engine.state.move_timer < MOVE_DELAY) {
        return;
    }

    @engine.state.move_timer = 0.0;

    // Apply buffered direction
    @engine.state.direction = @engine.state.next_direction;

    // Calculate new head position
    let head = @engine.state.snake[0];
    let new_head = head + @engine.state.direction;

    // Check wall collision
    if (new_head.x < 0.0 || new_head.x >= GRID_SIZE ||
        new_head.y < 0.0 || new_head.y >= GRID_SIZE) {
        @engine.state.game_over = 1.0;
        return;
    }

    // Check self collision
    var state = @engine.state;
    if (check_self_collision(&state, new_head)) {
        @engine.state.game_over = 1.0;
        return;
    }

    // Check food collision
    let ate_food = all(new_head == @engine.state.food_pos);

    if (ate_food) {
        // Grow snake
        @engine.state.length = min(@engine.state.length + 1.0, f32(MAX_SNAKE_LENGTH));
        @engine.state.score += 10.0;

        // Spawn new food
        state = @engine.state;
        @engine.state.food_pos = spawn_food(&state);
    }

    // Move snake body (shift all segments back)
    let len = u32(@engine.state.length);
    for (var i = len - 1u; i > 0u; i--) {
        @engine.state.snake[i] = @engine.state.snake[i - 1u];
    }

    // Move head to new position
    @engine.state.snake[0] = new_head;
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Background color (red if game over, dark otherwise)
    var color = select(
        vec4f(0.1, 0.1, 0.1, 1.0),
        vec4f(0.3, 0.05, 0.05, 1.0),
        @engine.state.game_over > 0.5
    );

    // Draw grid
    let grid_coord = coord.xy / CELL_SIZE;
    let grid_fract = fract(grid_coord);
    if (grid_fract.x < 0.03 || grid_fract.y < 0.03) {
        color = blend_over(color, vec4f(0.2, 0.2, 0.2, 1.0));
    }

    // Calculate which grid cell this pixel is in
    let cell = floor(coord.xy / CELL_SIZE);

    // Draw snake body
    let len = u32(@engine.state.length);
    for (var i = 1u; i < len; i++) {
        if (all(cell == @engine.state.snake[i])) {
            // Body segments are blue-green
            color = blend_over(color, draw_rect(
                coord.xy,
                @engine.state.snake[i] * CELL_SIZE + vec2f(2.0),
                vec2f(CELL_SIZE - 4.0),
                COLOR_DARKGREEN
            ));
        }
    }

    // Draw snake head (brighter)
    if (all(cell == @engine.state.snake[0])) {
        color = blend_over(color, draw_rect(
            coord.xy,
            @engine.state.snake[0] * CELL_SIZE + vec2f(1.0),
            vec2f(CELL_SIZE - 2.0),
            COLOR_GREEN
        ));
    }

    // Draw food
    if (all(cell == @engine.state.food_pos)) {
        color = blend_over(color, draw_circle(
            coord.xy,
            @engine.state.food_pos * CELL_SIZE + vec2f(CELL_SIZE * 0.5),
            CELL_SIZE * 0.4,
            COLOR_GOLD
        ));
    }

    return color;
}
