@set_title("Snake Game")
@set_size(640, 640)

// Constants
const GRID_SIZE: u32 = 20u;
const CELL_SIZE: f32 = 32.0;
const MAX_SNAKE_LENGTH: u32 = 400u;
const MOVE_DELAY: f32 = 0.15; // seconds between moves

// Only define things that persist across frames
struct GameState {
    snake_x: array<i32, 400>,
    snake_y: array<i32, 400>,
    snake_length: u32,
    food_x: i32,
    food_y: i32,
    dir_x: i32,
    dir_y: i32,
    next_dir_x: i32,
    next_dir_y: i32,
    move_timer: f32,
    game_over: u32,
    score: u32,
}

@compute @workgroup_size(1)
fn update() {
    // Initialize game on first frame
    if (_engine.time < 0.1 && _engine.state.snake_length == 0u) {
        _engine.state.snake_length = 3u;
        _engine.state.snake_x[0] = 10;
        _engine.state.snake_y[0] = 10;
        _engine.state.snake_x[1] = 9;
        _engine.state.snake_y[1] = 10;
        _engine.state.snake_x[2] = 8;
        _engine.state.snake_y[2] = 10;
        _engine.state.dir_x = 1;
        _engine.state.dir_y = 0;
        _engine.state.next_dir_x = 1;
        _engine.state.next_dir_y = 0;
        _engine.state.food_x = 15;
        _engine.state.food_y = 10;
        _engine.state.move_timer = 0.0;
        _engine.state.game_over = 0u;
        _engine.state.score = 0u;
    }

    if (_engine.state.game_over == 0u) {
        // Handle input (prevent reversing direction)
        if (_engine.buttons[BTN_RIGHT] == 1 && _engine.state.dir_x != -1) {
            _engine.state.next_dir_x = 1;
            _engine.state.next_dir_y = 0;
        }
        if (_engine.buttons[BTN_LEFT] == 1 && _engine.state.dir_x != 1) {
            _engine.state.next_dir_x = -1;
            _engine.state.next_dir_y = 0;
        }
        if (_engine.buttons[BTN_DOWN] == 1 && _engine.state.dir_y != -1) {
            _engine.state.next_dir_x = 0;
            _engine.state.next_dir_y = 1;
        }
        if (_engine.buttons[BTN_UP] == 1 && _engine.state.dir_y != 1) {
            _engine.state.next_dir_x = 0;
            _engine.state.next_dir_y = -1;
        }

        // Update move timer
        _engine.state.move_timer += _engine.delta_time;

        // Move snake
        if (_engine.state.move_timer >= MOVE_DELAY) {
            _engine.state.move_timer = 0.0;

            // Update direction
            _engine.state.dir_x = _engine.state.next_dir_x;
            _engine.state.dir_y = _engine.state.next_dir_y;

            // Calculate new head position
            let new_x = _engine.state.snake_x[0] + _engine.state.dir_x;
            let new_y = _engine.state.snake_y[0] + _engine.state.dir_y;

            // Check wall collision
            if (new_x < 0 || new_x >= i32(GRID_SIZE) || new_y < 0 || new_y >= i32(GRID_SIZE)) {
                _engine.state.game_over = 1u;
                return;
            }

            // Check self collision
            for (var i = 0u; i < _engine.state.snake_length; i++) {
                if (_engine.state.snake_x[i] == new_x && _engine.state.snake_y[i] == new_y) {
                    _engine.state.game_over = 1u;
                    return;
                }
            }

            // Check food collision
            let ate_food = (new_x == _engine.state.food_x && new_y == _engine.state.food_y);

            // Move snake body
            if (!ate_food) {
                // Move tail forward
                for (var i = _engine.state.snake_length - 1u; i > 0u; i--) {
                    _engine.state.snake_x[i] = _engine.state.snake_x[i - 1u];
                    _engine.state.snake_y[i] = _engine.state.snake_y[i - 1u];
                }
            } else {
                // Grow snake
                if (_engine.state.snake_length < MAX_SNAKE_LENGTH) {
                    // Shift body
                    for (var i = _engine.state.snake_length; i > 0u; i--) {
                        _engine.state.snake_x[i] = _engine.state.snake_x[i - 1u];
                        _engine.state.snake_y[i] = _engine.state.snake_y[i - 1u];
                    }
                    _engine.state.snake_length++;
                    _engine.state.score++;

                    // Spawn new food
                    // Simple pseudo-random using time
                    let rand_seed = u32(_engine.time * 1000.0) * 1103515245u + 12345u;
                    _engine.state.food_x = i32((rand_seed / 65536u) % GRID_SIZE);
                    _engine.state.food_y = i32(((rand_seed * 1103515245u + 12345u) / 65536u) % GRID_SIZE);
                }
            }

            // Update head position
            _engine.state.snake_x[0] = new_x;
            _engine.state.snake_y[0] = new_y;
        }
    } else {
        // Reset on button press when game over
        if (_engine.buttons[BTN_A] == 1 || _engine.buttons[BTN_START] == 1) {
            _engine.state.snake_length = 3u;
            _engine.state.snake_x[0] = 10;
            _engine.state.snake_y[0] = 10;
            _engine.state.snake_x[1] = 9;
            _engine.state.snake_y[1] = 10;
            _engine.state.snake_x[2] = 8;
            _engine.state.snake_y[2] = 10;
            _engine.state.dir_x = 1;
            _engine.state.dir_y = 0;
            _engine.state.next_dir_x = 1;
            _engine.state.next_dir_y = 0;
            _engine.state.food_x = 15;
            _engine.state.food_y = 10;
            _engine.state.move_timer = 0.0;
            _engine.state.game_over = 0u;
            _engine.state.score = 0u;
        }
    }
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Background color
    var color = vec4f(0.1, 0.1, 0.15, 1.0);

    // Convert screen coordinates to grid coordinates
    let grid_x = i32(coord.x / CELL_SIZE);
    let grid_y = i32(coord.y / CELL_SIZE);

    // Calculate position within cell for border
    let cell_local_x = coord.x - f32(grid_x) * CELL_SIZE;
    let cell_local_y = coord.y - f32(grid_y) * CELL_SIZE;
    let border = 2.0;

    // Draw food (red)
    if (grid_x == _engine.state.food_x && grid_y == _engine.state.food_y) {
        if (cell_local_x > border && cell_local_x < CELL_SIZE - border &&
            cell_local_y > border && cell_local_y < CELL_SIZE - border) {
            color = vec4f(1.0, 0.2, 0.2, 1.0);
        } else {
            color = vec4f(0.8, 0.1, 0.1, 1.0);
        }
    }

    // Draw snake (green, head is brighter)
    for (var i = 0u; i < _engine.state.snake_length; i++) {
        if (grid_x == _engine.state.snake_x[i] && grid_y == _engine.state.snake_y[i]) {
            if (cell_local_x > border && cell_local_x < CELL_SIZE - border &&
                cell_local_y > border && cell_local_y < CELL_SIZE - border) {
                if (i == 0u) {
                    // Head - bright green
                    color = vec4f(0.3, 1.0, 0.3, 1.0);
                } else {
                    // Body - darker green
                    color = vec4f(0.2, 0.7, 0.2, 1.0);
                }
            } else {
                // Border - even darker
                color = vec4f(0.1, 0.5, 0.1, 1.0);
            }
        }
    }

    // Draw grid lines (subtle)
    if (cell_local_x < 1.0 || cell_local_y < 1.0) {
        color = color * 0.9;
    }

    // Game over overlay
    if (_engine.state.game_over == 1u) {
        color = color * 0.5 + vec4f(0.5, 0.0, 0.0, 0.0);
    }

    return color;
}
