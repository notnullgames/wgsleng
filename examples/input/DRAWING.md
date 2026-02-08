# 2D Drawing Functions Quick Reference

Complete list of all drawing functions available in `draw2d.wgsl`.

## Compatibility

- **Raylib Colors**: All 27 colors match raylib's palette exactly
- **Resolution Independent**: Signed Distance Fields (SDF) for smooth, anti-aliased rendering at any scale.
- **No Dependencies**: Pure WGSL, no external libraries needed
- **GPU Optimized**: All calculations done on GPU for maximum performance

## Color Constants (27 - Raylib-compatible)

All colors match the [raylib](https://www.raylib.com/) color palette exactly, making it easy to port raylib code or match its visual style. Values are converted from RGB (0-255) to WGSL vec4f (0.0-1.0).

```wgsl
// Grayscale
COLOR_LIGHTGRAY  // vec4f(0.784, 0.784, 0.784, 1.0) - RGB(200, 200, 200)
COLOR_GRAY       // vec4f(0.510, 0.510, 0.510, 1.0) - RGB(130, 130, 130)
COLOR_DARKGRAY   // vec4f(0.314, 0.314, 0.314, 1.0) - RGB(80, 80, 80)
COLOR_WHITE      // vec4f(1.0, 1.0, 1.0, 1.0) - RGB(255, 255, 255)
COLOR_BLACK      // vec4f(0.0, 0.0, 0.0, 1.0) - RGB(0, 0, 0)
COLOR_BLANK      // vec4f(0.0, 0.0, 0.0, 0.0) - Transparent
COLOR_RAYWHITE   // vec4f(0.961, 0.961, 0.961, 1.0) - RGB(245, 245, 245)

// Yellows
COLOR_YELLOW     // vec4f(0.992, 0.976, 0.0, 1.0) - RGB(253, 249, 0)
COLOR_GOLD       // vec4f(1.0, 0.796, 0.0, 1.0) - RGB(255, 203, 0)
COLOR_ORANGE     // vec4f(1.0, 0.631, 0.0, 1.0) - RGB(255, 161, 0)

// Reds
COLOR_PINK       // vec4f(1.0, 0.427, 0.761, 1.0) - RGB(255, 109, 194)
COLOR_RED        // vec4f(0.902, 0.161, 0.216, 1.0) - RGB(230, 41, 55)
COLOR_MAROON     // vec4f(0.745, 0.129, 0.216, 1.0) - RGB(190, 33, 55)

// Greens
COLOR_GREEN      // vec4f(0.0, 0.894, 0.188, 1.0) - RGB(0, 228, 48)
COLOR_LIME       // vec4f(0.0, 0.620, 0.184, 1.0) - RGB(0, 158, 47)
COLOR_DARKGREEN  // vec4f(0.0, 0.459, 0.173, 1.0) - RGB(0, 117, 44)

// Blues
COLOR_SKYBLUE    // vec4f(0.400, 0.749, 1.0, 1.0) - RGB(102, 191, 255)
COLOR_BLUE       // vec4f(0.0, 0.475, 0.945, 1.0) - RGB(0, 121, 241)
COLOR_DARKBLUE   // vec4f(0.0, 0.322, 0.675, 1.0) - RGB(0, 82, 172)
COLOR_CYAN       // vec4f(0.0, 1.0, 1.0, 1.0) - Standard cyan

// Purples
COLOR_PURPLE     // vec4f(0.784, 0.478, 1.0, 1.0) - RGB(200, 122, 255)
COLOR_VIOLET     // vec4f(0.529, 0.235, 0.745, 1.0) - RGB(135, 60, 190)
COLOR_DARKPURPLE // vec4f(0.439, 0.122, 0.494, 1.0) - RGB(112, 31, 126)
COLOR_MAGENTA    // vec4f(1.0, 0.0, 1.0, 1.0) - Standard magenta

// Browns
COLOR_BEIGE      // vec4f(0.827, 0.690, 0.514, 1.0) - RGB(211, 176, 131)
COLOR_BROWN      // vec4f(0.498, 0.416, 0.310, 1.0) - RGB(127, 106, 79)
COLOR_DARKBROWN  // vec4f(0.298, 0.247, 0.184, 1.0) - RGB(76, 63, 47)
```

## Utility Functions (6)

### `lerp(a: f32, b: f32, t: f32) -> f32`

Linear interpolation between two values.

### `lerp_color(a: vec4f, b: vec4f, t: f32) -> vec4f`

Linear interpolation between two colors.

### `clamp_f32(value: f32, min_val: f32, max_val: f32) -> f32`

Clamp a value between min and max.

### `smoothstep_f32(edge0: f32, edge1: f32, x: f32) -> f32`

Smooth interpolation for anti-aliasing.

### `hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3f`

Convert HSV color (hue 0-360, saturation 0-1, value 0-1) to RGB.

### `rotate_point(point: vec2f, angle: f32) -> vec2f`

Rotate a 2D point around the origin by angle (in radians).

---

## Shape Drawing Functions (27)

### Basic Circles

#### `draw_circle(coord: vec2f, center: vec2f, radius: f32, color: vec4f) -> vec4f`

Draw a filled circle with anti-aliasing.

- **coord**: Current pixel position
- **center**: Center position of circle
- **radius**: Radius in pixels
- **color**: Fill color

#### `draw_circle_outline(coord: vec2f, center: vec2f, radius: f32, thickness: f32, color: vec4f) -> vec4f`

Draw a circle outline (ring).

- **thickness**: Width of the ring

#### `draw_ring(coord: vec2f, center: vec2f, inner_radius: f32, outer_radius: f32, color: vec4f) -> vec4f`

Draw a ring/donut shape between two radii.

#### `draw_ellipse(coord: vec2f, center: vec2f, radii: vec2f, color: vec4f) -> vec4f`

Draw a filled ellipse.

- **radii**: vec2f(radius_x, radius_y)

---

### Rectangles

#### `draw_rect(coord: vec2f, pos: vec2f, size: vec2f, color: vec4f) -> vec4f`

Draw a filled rectangle.

- **pos**: Top-left corner position
- **size**: Width and height

#### `draw_rect_outline(coord: vec2f, pos: vec2f, size: vec2f, thickness: f32, color: vec4f) -> vec4f`

Draw a rectangle outline.

#### `draw_rounded_rect(coord: vec2f, pos: vec2f, size: vec2f, radius: f32, color: vec4f) -> vec4f`

Draw a rounded rectangle.

- **radius**: Corner radius in pixels

#### `draw_rounded_rect_outline(coord: vec2f, pos: vec2f, size: vec2f, radius: f32, thickness: f32, color: vec4f) -> vec4f`

Draw a rounded rectangle outline.

---

### Lines and Curves

#### `draw_line(coord: vec2f, start: vec2f, end: vec2f, thickness: f32, color: vec4f) -> vec4f`

Draw a line segment with anti-aliasing.

#### `draw_bezier_quadratic(coord: vec2f, p0: vec2f, p1: vec2f, p2: vec2f, thickness: f32, color: vec4f) -> vec4f`

Draw a quadratic bezier curve.

- **p0**: Start point
- **p1**: Control point
- **p2**: End point

---

### Polygons

#### `draw_triangle(coord: vec2f, p0: vec2f, p1: vec2f, p2: vec2f, color: vec4f) -> vec4f`

Draw a filled triangle with anti-aliased edges.

#### `draw_polygon(coord: vec2f, center: vec2f, radius: f32, sides: u32, rotation: f32, color: vec4f) -> vec4f`

Draw a regular polygon.

- **sides**: Number of sides (3=triangle, 4=square, 5=pentagon, 6=hexagon, etc.)
- **rotation**: Rotation angle in radians

#### `draw_star(coord: vec2f, center: vec2f, outer_radius: f32, inner_radius: f32, points: u32, rotation: f32, color: vec4f) -> vec4f`

Draw a star shape.

- **points**: Number of star points (typically 5)
- **outer_radius**: Distance to outer points
- **inner_radius**: Distance to inner points

---

### Arcs and Slices

#### `draw_arc(coord: vec2f, center: vec2f, radius: f32, thickness: f32, start_angle: f32, end_angle: f32, color: vec4f) -> vec4f`

Draw an arc (portion of a circle outline).

- **start_angle**: Start angle in radians
- **end_angle**: End angle in radians

#### `draw_pie_slice(coord: vec2f, center: vec2f, radius: f32, start_angle: f32, end_angle: f32, color: vec4f) -> vec4f`

Draw a filled pie slice.

---

### Special Shapes

#### `draw_cross(coord: vec2f, center: vec2f, size: f32, thickness: f32, color: vec4f) -> vec4f`

Draw a plus/cross shape (+).

#### `draw_x(coord: vec2f, center: vec2f, size: f32, thickness: f32, color: vec4f) -> vec4f`

Draw an X shape.

#### `draw_arrow(coord: vec2f, start: vec2f, end: vec2f, shaft_thickness: f32, head_size: f32, color: vec4f) -> vec4f`

Draw an arrow with head.

- **shaft_thickness**: Thickness of arrow shaft
- **head_size**: Size of arrowhead

---

## Patterns and Fills (5)

### `draw_grid(coord: vec2f, cell_size: vec2f, thickness: f32, color: vec4f) -> vec4f`

Draw a grid pattern.

- **cell_size**: Width and height of grid cells

### `draw_checkerboard(coord: vec2f, cell_size: f32, color1: vec4f, color2: vec4f) -> vec4f`

Draw a checkerboard pattern.

### `draw_gradient_linear(coord: vec2f, start_pos: vec2f, end_pos: vec2f, start_color: vec4f, end_color: vec4f) -> vec4f`

Draw a linear gradient between two points.

### `draw_gradient_radial(coord: vec2f, center: vec2f, radius: f32, inner_color: vec4f, outer_color: vec4f) -> vec4f`

Draw a radial gradient from center.

---

## Compositing (1)

### `blend_over(bottom: vec4f, top: vec4f) -> vec4f`

Blend two colors using alpha compositing (Porter-Duff over operation).

**Usage:** Always use this when layering shapes:

```wgsl
var color = background;
color = blend_over(color, draw_circle(...));
color = blend_over(color, draw_rect(...));
```

---

## Usage Tips

1. **Always blend shapes in order:**

   ```wgsl
   var color = background_color;
   color = blend_over(color, shape1);
   color = blend_over(color, shape2);
   return color;
   ```

2. **All functions return transparent (alpha=0) outside the shape**

3. **Angles are in radians:**
   - 0 = right/east
   - π/2 (1.5708) = down/south
   - π (3.14159) = left/west
   - 3π/2 (4.71239) = up/north

4. **Anti-aliasing is automatic** - shapes have smooth edges

5. **Coordinate system:**
   - Origin (0,0) is top-left
   - X increases to the right
   - Y increases downward

---

## Quick Examples

### Button

```wgsl
// Background with rounded corners
color = blend_over(color, draw_rounded_rect(
    coord.xy, vec2f(100.0, 100.0), vec2f(120.0, 40.0), 8.0, COLOR_BLUE
));
// Border
color = blend_over(color, draw_rounded_rect_outline(
    coord.xy, vec2f(100.0, 100.0), vec2f(120.0, 40.0), 8.0, 2.0, COLOR_WHITE
));
```

### Health Bar

```wgsl
// Background
color = blend_over(color, draw_rect(
    coord.xy, vec2f(10.0, 10.0), vec2f(200.0, 20.0), COLOR_DARKGRAY
));
// Fill (based on health percentage)
let health_width = 200.0 * health_percent;
color = blend_over(color, draw_rect(
    coord.xy, vec2f(10.0, 10.0), vec2f(health_width, 20.0), COLOR_RED
));
```

### Loading Spinner

```wgsl
let rotation = @engine.time * 3.0;
color = blend_over(color, draw_arc(
    coord.xy, center, 30.0, 4.0, rotation, rotation + 4.71239, COLOR_CYAN
));
```

### Direction Indicator

```wgsl
let angle = atan2(target.y - pos.y, target.x - pos.x);
color = blend_over(color, draw_arrow(
    coord.xy, pos, target, 3.0, 15.0, COLOR_YELLOW
));
```

### Collectible Star

```wgsl
let pulse = sin(@engine.time * 5.0) * 0.2 + 1.0;
let rotation = @engine.time;
color = blend_over(color, draw_star(
    coord.xy, pos, 20.0 * pulse, 8.0 * pulse, 5u, rotation, COLOR_YELLOW
));
```
