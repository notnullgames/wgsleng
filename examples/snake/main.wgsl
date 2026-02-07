@set_title("Snake")
@set_size(640, 640)

// Constants
const GRID_SIZE: u32 = 20u;
const CELL_SIZE: f32 = 32.0;
const MAX_SNAKE_LENGTH: u32 = 400u;
const MOVE_DELAY: f32 = 0.15; // seconds between moves
const FOOD_COUNT: u32 = 10u;

// Only define things that persist across frames
struct GameState {
    snake_x: array<i32, 400>,
    snake_y: array<i32, 400>,
    snake_length: u32,
    food_x: array<i32, 10>,
    food_y: array<i32, 10>,
    dir_x: i32,
    dir_y: i32,
    next_dir_x: i32,
    next_dir_y: i32,
    move_timer: f32,
    game_over: u32,
    score: u32,
    paused: u32,
    pause_button_prev: u32,
    reset_button_prev: u32,
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

        // Spawn initial food items randomly
        var seed = 12345u;
        for (var i = 0u; i < FOOD_COUNT; i++) {
            seed = (seed + i * 7919u) * 1103515245u + 12345u;
            _engine.state.food_x[i] = i32((seed / 65536u) % GRID_SIZE);
            seed = seed * 1103515245u + 12345u;
            _engine.state.food_y[i] = i32((seed / 65536u) % GRID_SIZE);
        }

        _engine.state.move_timer = 0.0;
        _engine.state.game_over = 0u;
        _engine.state.score = 0u;
        _engine.state.paused = 0u;
        _engine.state.pause_button_prev = 0u;
        _engine.state.reset_button_prev = 0u;
    }

    // Toggle pause with SELECT button (detect button press, not hold)
    let pause_button = _engine.buttons[BTN_SELECT];
    if (pause_button == 1 && _engine.state.pause_button_prev == 0) {
        _engine.state.paused = 1u - _engine.state.paused;
    }
    _engine.state.pause_button_prev = u32(pause_button);

    // Skip game logic if paused
    if (_engine.state.paused == 1u) {
        return;
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

            // Check self collision (skip head at index 0, only check body)
            for (var i = 1u; i < _engine.state.snake_length; i++) {
                if (_engine.state.snake_x[i] == new_x && _engine.state.snake_y[i] == new_y) {
                    _engine.state.game_over = 1u;
                    return;
                }
            }

            // Check food collision with any food item
            var ate_food = false;
            var eaten_food_idx = 0u;
            for (var i = 0u; i < FOOD_COUNT; i++) {
                if (new_x == _engine.state.food_x[i] && new_y == _engine.state.food_y[i]) {
                    ate_food = true;
                    eaten_food_idx = i;
                    break;
                }
            }

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

                    // Respawn the eaten food at a new random location
                    var seed = u32(_engine.time * 1000.0) * 1103515245u + eaten_food_idx;
                    seed = seed * 1103515245u + 12345u;
                    _engine.state.food_x[eaten_food_idx] = i32((seed / 65536u) % GRID_SIZE);
                    seed = seed * 1103515245u + 12345u;
                    _engine.state.food_y[eaten_food_idx] = i32((seed / 65536u) % GRID_SIZE);
                }
            }

            // Update head position
            _engine.state.snake_x[0] = new_x;
            _engine.state.snake_y[0] = new_y;
        }
    } else {
        // Reset on button press when game over (detect press, not hold)
        let reset_button = _engine.buttons[BTN_A] | _engine.buttons[BTN_START];
        if (reset_button == 1 && _engine.state.reset_button_prev == 0) {
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

            // Respawn all food items randomly
            var seed = u32(_engine.time * 1000.0) + 12345u;
            for (var i = 0u; i < FOOD_COUNT; i++) {
                seed = (seed + i * 7919u) * 1103515245u + 12345u;
                _engine.state.food_x[i] = i32((seed / 65536u) % GRID_SIZE);
                seed = seed * 1103515245u + 12345u;
                _engine.state.food_y[i] = i32((seed / 65536u) % GRID_SIZE);
            }

            _engine.state.move_timer = 0.0;
            _engine.state.game_over = 0u;
            _engine.state.score = 0u;
        }
        _engine.state.reset_button_prev = u32(reset_button);
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

    // Draw food (red) - check all food items
    for (var i = 0u; i < FOOD_COUNT; i++) {
        if (grid_x == _engine.state.food_x[i] && grid_y == _engine.state.food_y[i]) {
            if (cell_local_x > border && cell_local_x < CELL_SIZE - border &&
                cell_local_y > border && cell_local_y < CELL_SIZE - border) {
                color = vec4f(1.0, 0.2, 0.2, 1.0);
            } else {
                color = vec4f(0.8, 0.1, 0.1, 1.0);
            }
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

    // Paused overlay
    if (_engine.state.paused == 1u) {
        color = color * 0.7 + vec4f(0.0, 0.0, 0.3, 0.0);
    }

    return color;
}
