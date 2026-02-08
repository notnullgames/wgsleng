# Bitmap Font Rendering Quick Reference

(Sort of) easy-to-use bitmap font rendering system for WGSL Engine.

## Overview

The font library provides functions for rendering text using bitmap font textures. It uses a simple grid-based layout and includes helper functions for characters, digits, and multi-digit numbers.

## Compatibility

- **Pure WGSL**: No external dependencies
- **GPU Optimized**: All calculations done on GPU
- **Flexible**: Works with any bitmap font in grid format
- **Simple Integration**: Just 3 core functions to learn

## Font Texture Format

The default font texture format expects:

- **Grid Layout**: 20 characters wide × 3 characters tall (320×48 pixels)
- **Character Size**: 16×16 pixels per character
- **ASCII Range**: Characters 32-91 (space through `[`)
- **Format**: White-on-black bitmap (or any monochrome format)

### Supported Characters

```
 !"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[
```

Including space (32), all digits (48-57), uppercase letters (65-90), and common symbols.

---

## Core Functions (3)

### `get_char_uv(coord: vec2f, char_code: u32, pos: vec2f, size: f32) -> vec4f`

Calculate UV coordinates for a single character.

**Parameters:**

- `coord`: Current pixel position (fragment shader builtin)
- `char_code`: ASCII code of character (e.g., 65 for 'A')
- `pos`: Top-left position to draw character
- `size`: Character size in pixels (typically 8.0 or 16.0)

**Returns:** `vec4f(uv.x, uv.y, in_bounds, unused)`

- `uv.xy`: Texture coordinates (0.0-1.0)
- `z`: 1.0 if pixel is within character bounds, 0.0 otherwise
- `w`: Unused (always 0.0)

**Example:**

```wgsl
let uv_data = get_char_uv(coord.xy, 65u, vec2f(100.0, 50.0), 16.0); // 'A'
if (uv_data.z > 0.5) {
    let glyph = textureSampleLevel(font_texture, sampler, uv_data.xy, 0.0);
    // Use glyph color...
}
```

### `get_digit_uv(coord: vec2f, digit: u32, pos: vec2f, size: f32) -> vec4f`

Helper for rendering a single digit (0-9).

**Parameters:**

- `digit`: Digit to render (0-9)
- Other parameters same as `get_char_uv`

**Returns:** Same format as `get_char_uv`

**Example:**

```wgsl
let uv_data = get_digit_uv(coord.xy, 7u, vec2f(100.0, 50.0), 16.0);
```

This is equivalent to:

```wgsl
let uv_data = get_char_uv(coord.xy, 48u + 7u, vec2f(100.0, 50.0), 16.0);
```

### `get_number_uv(coord: vec2f, number: u32, pos: vec2f, size: f32) -> vec4f`

Render a multi-digit number with automatic layout.

**Parameters:**

- `number`: Unsigned integer to render (e.g., 12345)
- Other parameters same as `get_char_uv`

**Returns:** Same format as `get_char_uv`, but checks all digit positions

**Notes:**

- Digits are laid out horizontally, spaced by `size` pixels
- Numbers are rendered left-to-right
- Handles 0 as a special case (renders "0" not empty)

**Example:**

```wgsl
// Draw score of 12345
let uv_data = get_number_uv(coord.xy, 12345u, vec2f(100.0, 50.0), 8.0);
if (uv_data.z > 0.5) {
    let glyph = textureSampleLevel(font_texture, sampler, uv_data.xy, 0.0);
    // Use glyph color...
}
```

---

## Constants

```wgsl
const FONT_GRID_WIDTH = 20u;   // Characters per row
const FONT_GRID_HEIGHT = 3u;   // Number of rows
```

These constants define the font texture layout. Adjust if using a different font format.

---

## Integration Guide

The font library provides UV coordinate functions. You need to create wrapper functions that sample your font texture.

### Basic Setup

1. **Add font texture to your game:**

```wgsl
@texture("myfont.png")
```

2. **Create wrapper functions:**

```wgsl
@import("font.wgsl")

fn draw_char(color: vec4f, coord: vec2f, char_code: u32, pos: vec2f, size: f32) -> vec4f {
    let uv_data = get_char_uv(coord, char_code, pos, size);
    if (uv_data.z < 0.5) {
        return color; // Outside character bounds
    }

    let glyph = textureSampleLevel(@texture("myfont.png"), @engine.sampler, uv_data.xy, 0.0);

    // For white-on-black fonts
    let brightness = (glyph.r + glyph.g + glyph.b) / 3.0;
    if (brightness > 0.5) {
        return vec4f(1.0, 1.0, 1.0, 1.0); // White text
    }
    return color;
}

fn draw_number(color: vec4f, coord: vec2f, number: u32, pos: vec2f, size: f32) -> vec4f {
    let uv_data = get_number_uv(coord, number, pos, size);
    if (uv_data.z < 0.5) {
        return color;
    }

    let glyph = textureSampleLevel(@texture("myfont.png"), @engine.sampler, uv_data.xy, 0.0);
    let brightness = (glyph.r + glyph.g + glyph.b) / 3.0;
    if (brightness > 0.5) {
        return vec4f(1.0, 1.0, 1.0, 1.0);
    }
    return color;
}
```

### Colored Text

To render text in different colors:

```wgsl
fn draw_char_colored(color: vec4f, coord: vec2f, char_code: u32, pos: vec2f, size: f32, text_color: vec4f) -> vec4f {
    let uv_data = get_char_uv(coord, char_code, pos, size);
    if (uv_data.z < 0.5) {
        return color;
    }

    let glyph = textureSampleLevel(@texture("myfont.png"), @engine.sampler, uv_data.xy, 0.0);
    let brightness = (glyph.r + glyph.g + glyph.b) / 3.0;
    if (brightness > 0.5) {
        return text_color; // Use custom color
    }
    return color;
}
```

---

## ASCII Code Reference

Quick reference for common characters:

```wgsl
// Digits
48-57  : '0' through '9'

// Uppercase letters
65-90  : 'A' through 'Z'

// Common symbols
32     : ' '  (space)
33     : '!'
44     : ','
46     : '.'
58     : ':'
63     : '?'
```

---

## Usage Examples

### Example 1: Simple Text Label

```wgsl
// Draw "SCORE" label
color = draw_char(color, coord.xy, 83u, vec2f(10.0, 10.0), 8.0); // 'S'
color = draw_char(color, coord.xy, 67u, vec2f(18.0, 10.0), 8.0); // 'C'
color = draw_char(color, coord.xy, 79u, vec2f(26.0, 10.0), 8.0); // 'O'
color = draw_char(color, coord.xy, 82u, vec2f(34.0, 10.0), 8.0); // 'R'
color = draw_char(color, coord.xy, 69u, vec2f(42.0, 10.0), 8.0); // 'E'
```

### Example 2: Score Display

```wgsl
// Draw score value
let score = u32(@engine.state.score);
color = draw_number(color, coord.xy, score, vec2f(10.0, 20.0), 8.0);
```

### Example 3: Lives Counter

```wgsl
// Draw "LIVES: 3"
var x = 10.0;
color = draw_char(color, coord.xy, 76u, vec2f(x, 10.0), 8.0); x += 8.0; // 'L'
color = draw_char(color, coord.xy, 73u, vec2f(x, 10.0), 8.0); x += 8.0; // 'I'
color = draw_char(color, coord.xy, 86u, vec2f(x, 10.0), 8.0); x += 8.0; // 'V'
color = draw_char(color, coord.xy, 69u, vec2f(x, 10.0), 8.0); x += 8.0; // 'E'
color = draw_char(color, coord.xy, 83u, vec2f(x, 10.0), 8.0); x += 8.0; // 'S'
color = draw_char(color, coord.xy, 58u, vec2f(x, 10.0), 8.0); x += 8.0; // ':'
color = draw_char(color, coord.xy, 32u, vec2f(x, 10.0), 8.0); x += 8.0; // ' '

let lives = u32(@engine.state.lives);
color = draw_digit(color, coord.xy, lives, vec2f(x, 10.0), 8.0);
```

### Example 4: Timer Display

```wgsl
// Display time in seconds
let seconds = u32(@engine.time);
color = draw_number(color, coord.xy, seconds, vec2f(10.0, 30.0), 8.0);
```

### Example 5: FPS Counter

```wgsl
// Calculate FPS
let fps = u32(1.0 / @engine.delta_time);

// Draw "FPS: "
var x = 10.0;
color = draw_char(color, coord.xy, 70u, vec2f(x, 10.0), 8.0); x += 8.0; // 'F'
color = draw_char(color, coord.xy, 80u, vec2f(x, 10.0), 8.0); x += 8.0; // 'P'
color = draw_char(color, coord.xy, 83u, vec2f(x, 10.0), 8.0); x += 8.0; // 'S'
color = draw_char(color, coord.xy, 58u, vec2f(x, 10.0), 8.0); x += 8.0; // ':'
color = draw_char(color, coord.xy, 32u, vec2f(x, 10.0), 8.0); x += 8.0; // ' '

color = draw_number(color, coord.xy, fps, vec2f(x, 10.0), 8.0);
```

---

## Usage Tips

1. **Size scaling**: Use smaller sizes (8.0) for compact UIs, larger sizes (16.0+) for readability

2. **Spacing**: Characters don't overlap - you control spacing by positioning each character

3. **Alignment**: For right-aligned text, calculate total width first:

   ```wgsl
   let digits = count_digits(number); // You'll need to implement this
   let total_width = f32(digits) * size;
   let right_aligned_x = container_right - total_width;
   ```

4. **Performance**: Text rendering is fast but avoid rendering long strings unnecessarily

5. **Custom fonts**: Adjust `FONT_GRID_WIDTH` and `FONT_GRID_HEIGHT` constants for different layouts

6. **Colored backgrounds**: Render a filled rectangle behind text for better readability:

   ```wgsl
   color = blend_over(color, draw_rect(coord.xy, vec2f(8.0, 8.0), vec2f(60.0, 12.0), COLOR_BLACK));
   color = draw_char(color, coord.xy, 65u, vec2f(10.0, 10.0), 8.0);
   ```

7. **Monospace layout**: All characters have the same width, making alignment easy

---

## Creating Custom Fonts

To use your own font:

1. **Create texture**:
   - Use a monospace bitmap font
   - Arrange characters in a grid (20×3 or custom dimensions)
   - White characters on transparent/black background works best

2. **Update constants** (if using different layout):

   ```wgsl
   const FONT_GRID_WIDTH = 16u;  // Your layout width
   const FONT_GRID_HEIGHT = 4u;  // Your layout height
   ```

3. **Adjust character mapping**:
   - Current implementation assumes ASCII 32-91
   - Modify `get_char_uv` if using different character set

---

## Complete Example

See `examples/tetris/main.wgsl` for a full working example that includes:

- Score display with labels
- Multi-digit number rendering
- Compact text layout
- Integration with game state

```wgsl
@import("font.wgsl")

// Your wrapper functions here...

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    var color = vec4f(0.0, 0.0, 0.0, 1.0);

    // Draw "SCORE"
    color = draw_char(color, coord.xy, 83u, vec2f(10.0, 10.0), 8.0);
    color = draw_char(color, coord.xy, 67u, vec2f(18.0, 10.0), 8.0);
    color = draw_char(color, coord.xy, 79u, vec2f(26.0, 10.0), 8.0);
    color = draw_char(color, coord.xy, 82u, vec2f(34.0, 10.0), 8.0);
    color = draw_char(color, coord.xy, 69u, vec2f(42.0, 10.0), 8.0);

    // Draw score value
    color = draw_number(color, coord.xy, u32(@engine.state.score), vec2f(10.0, 22.0), 8.0);

    return color;
}
```
