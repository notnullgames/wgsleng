# render_to_image - WGSL Shader Testing Tool

This example renders a WGSL game to a single PNG image for testing purposes. It uses the same shader preprocessing logic as the main game engine, ensuring consistent behavior between testing and runtime.

## Purpose

- **Automated Testing**: Generate reference images for visual regression testing
- **Quick Preview**: See what a shader looks like without running the full game loop
- **CI/CD Integration**: Generate screenshots in headless environments
- **Documentation**: Create images for README files and documentation

## Features

- ✅ Uses **shared preprocessing library** (`lib.rs`) - identical behavior to main engine
- ✅ Supports all WGSL extensions: `@import`, `@set_title`, `@set_size`, `@texture`, `@sound`
- ✅ Handles `.wgsl` files, directories, and `.zip` archives
- ✅ Loads and binds textures automatically
- ✅ Proper GameState initialization
- ✅ Configurable output size (from `@set_size` directive)

## Usage

### From project root:

```bash
cargo run --manifest-path native/Cargo.toml --example render_to_image <input> <output>
```

### Using npm scripts:

```bash
# Render specific examples
npm run test:render:logo
npm run test:render:input
npm run test:render:bob

# Custom render
npm run render examples/input/main.wgsl /tmp/my-output.png
```

### From native directory:

```bash
cd native
cargo run --example render_to_image ../examples/logo/main.wgsl /tmp/logo.png
```

## Arguments

1. **input** - Path to WGSL shader file or directory/zip containing `main.wgsl`
2. **output** - Path to output PNG file

## Examples

### Render the logo demo

```bash
cargo run --manifest-path native/Cargo.toml --example render_to_image \
  examples/logo/main.wgsl \
  /tmp/logo.png
```

### Render the input demo (with 2D drawing library)

```bash
cargo run --manifest-path native/Cargo.toml --example render_to_image \
  examples/input/main.wgsl \
  /tmp/input.png
```

### Render with debug output

```bash
DEBUG_SHADER=1 cargo run --manifest-path native/Cargo.toml --example render_to_image \
  examples/input/main.wgsl \
  /tmp/input_debug.png
```

This will print the preprocessed WGSL code showing all macro expansions.

## How It Works

1. **Parse Input**: Determines if input is a `.wgsl` file, directory, or `.zip`
2. **Preprocess**: Uses `PreprocessorState` from `lib.rs` to:
   - Resolve `@import` directives
   - Extract metadata (`@set_title`, `@set_size`, textures, sounds)
   - Calculate GameState struct size
   - Replace all `@` macros with proper WGSL bindings
3. **Load Assets**: Loads any referenced textures
4. **Setup GPU**: Creates WebGPU device, buffers, bind groups, and pipeline
5. **Initialize Engine Buffer**: Sets up button state, time, screen size, and GameState
6. **Render**: Executes one frame of the fragment shader
7. **Save**: Copies render texture to PNG file

## Differences from Main Engine

- **No Game Loop**: Renders a single frame then exits
- **No Audio**: Sound triggers are initialized but not played
- **No Input**: Buttons are all set to unpressed (0)
- **No Compute Shader**: Only runs the fragment shader, not the `update()` function
- **No Window**: Runs headless, no GUI window displayed

## Technical Details

### Buffer Layout

The engine buffer matches the main runtime exactly:

```
Offset 0:   buttons (48 bytes) - array<i32, 12>
Offset 48:  time (4 bytes) - f32 (initialized to 0.0)
Offset 52:  delta_time (4 bytes) - f32 (initialized to 0.0)
Offset 56:  screen_width (4 bytes) - f32
Offset 60:  screen_height (4 bytes) - f32
Offset 64:  GameState (variable size, aligned to 8 bytes)
Offset 64+: audio triggers (variable size)
```

### GameState Initialization

- `player_pos` (if vec2f): Initialized to screen center
- All other fields: Initialized to zero

### Bind Groups

- **Group 0**: Sampler (binding 0) + Textures (bindings 1+)
- **Group 1**: Engine storage buffer (binding 0)

## Troubleshooting

### "No such file or directory"

Make sure you're running from the project root, not the `native/` directory:

```bash
# ✅ Correct
cargo run --manifest-path native/Cargo.toml --example render_to_image examples/logo/main.wgsl /tmp/out.png

# ❌ Wrong (from native/ directory)
cargo run --example render_to_image examples/logo/main.wgsl /tmp/out.png
```

### "Failed to preprocess shader"

Enable debug output to see the preprocessed WGSL:

```bash
DEBUG_SHADER=1 cargo run --manifest-path native/Cargo.toml --example render_to_image <input> <output>
```

### "Failed to load texture"

Ensure texture files are:
- In the same directory as the `.wgsl` file (for directories)
- Included in the `.zip` file (for zip archives)
- Properly referenced in `@texture("filename.png")` directives

## Integration with CI/CD

### Generate reference images in CI

```yaml
- name: Build native tools
  run: cargo build --manifest-path native/Cargo.toml --release --examples

- name: Generate reference images
  run: |
    cargo run --manifest-path native/Cargo.toml --release --example render_to_image \
      examples/logo/main.wgsl reference/logo.png
    cargo run --manifest-path native/Cargo.toml --release --example render_to_image \
      examples/input/main.wgsl reference/input.png

- name: Upload artifacts
  uses: actions/upload-artifact@v3
  with:
    name: reference-images
    path: reference/*.png
```

## Future Enhancements

Potential improvements:

- [ ] Support multiple frames (render N frames, save as sprite sheet or GIF)
- [ ] Simulate button presses via command-line arguments
- [ ] Run compute shader before rendering
- [ ] Support custom engine buffer initialization
- [ ] Parallel rendering of multiple shaders
