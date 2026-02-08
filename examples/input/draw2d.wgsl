// ============================================================================
// 2D Drawing Library for WGSL Engine
// A comprehensive collection of 2D drawing primitives and utilities
// ============================================================================

// ----------------------------------------------------------------------------
// Color Constants (Raylib-compatible)
// ----------------------------------------------------------------------------
const COLOR_LIGHTGRAY = vec4f(0.784, 0.784, 0.784, 1.0);  // 200, 200, 200
const COLOR_GRAY = vec4f(0.510, 0.510, 0.510, 1.0);       // 130, 130, 130
const COLOR_DARKGRAY = vec4f(0.314, 0.314, 0.314, 1.0);   // 80, 80, 80
const COLOR_YELLOW = vec4f(0.992, 0.976, 0.0, 1.0);       // 253, 249, 0
const COLOR_GOLD = vec4f(1.0, 0.796, 0.0, 1.0);           // 255, 203, 0
const COLOR_ORANGE = vec4f(1.0, 0.631, 0.0, 1.0);         // 255, 161, 0
const COLOR_PINK = vec4f(1.0, 0.427, 0.761, 1.0);         // 255, 109, 194
const COLOR_RED = vec4f(0.902, 0.161, 0.216, 1.0);        // 230, 41, 55
const COLOR_MAROON = vec4f(0.745, 0.129, 0.216, 1.0);     // 190, 33, 55
const COLOR_GREEN = vec4f(0.0, 0.894, 0.188, 1.0);        // 0, 228, 48
const COLOR_LIME = vec4f(0.0, 0.620, 0.184, 1.0);         // 0, 158, 47
const COLOR_DARKGREEN = vec4f(0.0, 0.459, 0.173, 1.0);    // 0, 117, 44
const COLOR_SKYBLUE = vec4f(0.400, 0.749, 1.0, 1.0);      // 102, 191, 255
const COLOR_BLUE = vec4f(0.0, 0.475, 0.945, 1.0);         // 0, 121, 241
const COLOR_DARKBLUE = vec4f(0.0, 0.322, 0.675, 1.0);     // 0, 82, 172
const COLOR_PURPLE = vec4f(0.784, 0.478, 1.0, 1.0);       // 200, 122, 255
const COLOR_VIOLET = vec4f(0.529, 0.235, 0.745, 1.0);     // 135, 60, 190
const COLOR_DARKPURPLE = vec4f(0.439, 0.122, 0.494, 1.0); // 112, 31, 126
const COLOR_BEIGE = vec4f(0.827, 0.690, 0.514, 1.0);      // 211, 176, 131
const COLOR_BROWN = vec4f(0.498, 0.416, 0.310, 1.0);      // 127, 106, 79
const COLOR_DARKBROWN = vec4f(0.298, 0.247, 0.184, 1.0);  // 76, 63, 47
const COLOR_WHITE = vec4f(1.0, 1.0, 1.0, 1.0);            // 255, 255, 255
const COLOR_BLACK = vec4f(0.0, 0.0, 0.0, 1.0);            // 0, 0, 0
const COLOR_BLANK = vec4f(0.0, 0.0, 0.0, 0.0);            // 0, 0, 0, 0
const COLOR_MAGENTA = vec4f(1.0, 0.0, 1.0, 1.0);          // 255, 0, 255
const COLOR_RAYWHITE = vec4f(0.961, 0.961, 0.961, 1.0);   // 245, 245, 245
const COLOR_CYAN = vec4f(0.0, 1.0, 1.0, 1.0);             // Kept for compatibility

// ----------------------------------------------------------------------------
// Utility Functions
// ----------------------------------------------------------------------------

// Linear interpolation between two values
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + (b - a) * t;
}

// Linear interpolation between two colors
fn lerp_color(a: vec4f, b: vec4f, t: f32) -> vec4f {
    return vec4f(
        lerp(a.r, b.r, t),
        lerp(a.g, b.g, t),
        lerp(a.b, b.b, t),
        lerp(a.a, b.a, t)
    );
}

// Clamp a value between min and max
fn clamp_f32(value: f32, min_val: f32, max_val: f32) -> f32 {
    return max(min_val, min(max_val, value));
}

// Smooth step function for anti-aliasing
fn smoothstep_f32(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp_f32((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

// Convert HSV to RGB color
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3f {
    let c = v * s;
    let x = c * (1.0 - abs((h / 60.0) % 2.0 - 1.0));
    let m = v - c;

    var rgb = vec3f(0.0);
    let h_sector = i32(h / 60.0);

    if (h_sector == 0) { rgb = vec3f(c, x, 0.0); }
    else if (h_sector == 1) { rgb = vec3f(x, c, 0.0); }
    else if (h_sector == 2) { rgb = vec3f(0.0, c, x); }
    else if (h_sector == 3) { rgb = vec3f(0.0, x, c); }
    else if (h_sector == 4) { rgb = vec3f(x, 0.0, c); }
    else { rgb = vec3f(c, 0.0, x); }

    return rgb + vec3f(m);
}

// Rotate a 2D point around origin
fn rotate_point(point: vec2f, angle: f32) -> vec2f {
    let cos_a = cos(angle);
    let sin_a = sin(angle);
    return vec2f(
        point.x * cos_a - point.y * sin_a,
        point.x * sin_a + point.y * cos_a
    );
}

// ----------------------------------------------------------------------------
// Basic Shape Drawing Functions
// ----------------------------------------------------------------------------

// Draw a filled circle with anti-aliasing
fn draw_circle(coord: vec2f, center: vec2f, radius: f32, color: vec4f) -> vec4f {
    let dist = length(coord - center);
    let alpha = 1.0 - smoothstep_f32(radius - 1.0, radius + 1.0, dist);
    return vec4f(color.rgb, color.a * alpha);
}

// Draw a circle outline (ring)
fn draw_circle_outline(coord: vec2f, center: vec2f, radius: f32, thickness: f32, color: vec4f) -> vec4f {
    let dist = length(coord - center);
    let inner = radius - thickness * 0.5;
    let outer = radius + thickness * 0.5;

    let alpha_outer = 1.0 - smoothstep_f32(outer - 1.0, outer + 1.0, dist);
    let alpha_inner = smoothstep_f32(inner - 1.0, inner + 1.0, dist);
    let alpha = alpha_outer * alpha_inner;

    return vec4f(color.rgb, color.a * alpha);
}

// Draw a filled rectangle
fn draw_rect(coord: vec2f, pos: vec2f, size: vec2f, color: vec4f) -> vec4f {
    let half_size = size * 0.5;
    let center = pos + half_size;
    let d = abs(coord - center) - half_size;
    let dist = length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0);
    let alpha = 1.0 - smoothstep_f32(-1.0, 1.0, dist);
    return vec4f(color.rgb, color.a * alpha);
}

// Draw a rectangle outline
fn draw_rect_outline(coord: vec2f, pos: vec2f, size: vec2f, thickness: f32, color: vec4f) -> vec4f {
    let half_size = size * 0.5;
    let center = pos + half_size;
    let d = abs(coord - center) - half_size;
    let dist_outer = length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0);

    let d_inner = abs(coord - center) - (half_size - vec2f(thickness));
    let dist_inner = length(max(d_inner, vec2f(0.0))) + min(max(d_inner.x, d_inner.y), 0.0);

    let alpha_outer = 1.0 - smoothstep_f32(-1.0, 1.0, dist_outer);
    let alpha_inner = 1.0 - (1.0 - smoothstep_f32(-1.0, 1.0, dist_inner));
    let alpha = alpha_outer * alpha_inner;

    return vec4f(color.rgb, color.a * alpha);
}

// Draw a rounded rectangle
fn draw_rounded_rect(coord: vec2f, pos: vec2f, size: vec2f, radius: f32, color: vec4f) -> vec4f {
    let half_size = size * 0.5;
    let center = pos + half_size;
    let d = abs(coord - center) - half_size + vec2f(radius);
    let dist = length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0) - radius;
    let alpha = 1.0 - smoothstep_f32(-1.0, 1.0, dist);
    return vec4f(color.rgb, color.a * alpha);
}

// Draw a rounded rectangle outline
fn draw_rounded_rect_outline(coord: vec2f, pos: vec2f, size: vec2f, radius: f32, thickness: f32, color: vec4f) -> vec4f {
    let half_size = size * 0.5;
    let center = pos + half_size;

    // Outer edge
    let d_outer = abs(coord - center) - half_size + vec2f(radius);
    let dist_outer = length(max(d_outer, vec2f(0.0))) + min(max(d_outer.x, d_outer.y), 0.0) - radius;

    // Inner edge
    let inner_radius = max(0.0, radius - thickness);
    let d_inner = abs(coord - center) - (half_size - vec2f(thickness)) + vec2f(inner_radius);
    let dist_inner = length(max(d_inner, vec2f(0.0))) + min(max(d_inner.x, d_inner.y), 0.0) - inner_radius;

    let alpha_outer = 1.0 - smoothstep_f32(-1.0, 1.0, dist_outer);
    let alpha_inner = 1.0 - (1.0 - smoothstep_f32(-1.0, 1.0, dist_inner));
    let alpha = alpha_outer * alpha_inner;

    return vec4f(color.rgb, color.a * alpha);
}

// Draw a line segment
fn draw_line(coord: vec2f, start: vec2f, end: vec2f, thickness: f32, color: vec4f) -> vec4f {
    let pa = coord - start;
    let ba = end - start;
    let h = clamp_f32(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    let dist = length(pa - ba * h);
    let alpha = 1.0 - smoothstep_f32(thickness * 0.5 - 1.0, thickness * 0.5 + 1.0, dist);
    return vec4f(color.rgb, color.a * alpha);
}

// Draw a triangle (filled)
fn draw_triangle(coord: vec2f, p0: vec2f, p1: vec2f, p2: vec2f, color: vec4f) -> vec4f {
    // Using barycentric coordinates
    let v0 = p1 - p0;
    let v1 = p2 - p0;
    let v2 = coord - p0;

    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);

    let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

    // Check if point is inside triangle
    if (u >= 0.0 && v >= 0.0 && u + v <= 1.0) {
        // Calculate distance to edges for anti-aliasing
        let d0 = abs(dot(coord - p0, normalize(vec2f(-v0.y, v0.x))));
        let d1 = abs(dot(coord - p1, normalize(vec2f(-v1.y, v1.x))));
        let d2 = abs(dot(coord - p2, normalize(vec2f(-(p2.y-p1.y), p2.x-p1.x))));
        let min_dist = min(d0, min(d1, d2));
        let alpha = smoothstep_f32(1.0, 0.0, min_dist);
        return vec4f(color.rgb, color.a * alpha);
    }
    return vec4f(0.0);
}

// Draw a regular polygon (n sides)
fn draw_polygon(coord: vec2f, center: vec2f, radius: f32, sides: u32, rotation: f32, color: vec4f) -> vec4f {
    let p = coord - center;
    let angle = atan2(p.y, p.x) + rotation;
    let segment_angle = 6.28318530718 / f32(sides);
    let segment = floor(angle / segment_angle);
    let local_angle = angle - segment * segment_angle - segment_angle * 0.5;

    let dist = length(p) * cos(local_angle) / cos(segment_angle * 0.5);
    let alpha = 1.0 - smoothstep_f32(radius - 1.0, radius + 1.0, dist);
    return vec4f(color.rgb, color.a * alpha);
}

// Draw a star shape
fn draw_star(coord: vec2f, center: vec2f, outer_radius: f32, inner_radius: f32, points: u32, rotation: f32, color: vec4f) -> vec4f {
    let p = coord - center;
    let angle = atan2(p.y, p.x) + rotation;
    let dist = length(p);

    let segment_angle = 3.14159265359 / f32(points);
    let a = angle % (segment_angle * 2.0);
    let radius = select(inner_radius, outer_radius, a < segment_angle);

    let alpha = 1.0 - smoothstep_f32(radius - 1.0, radius + 1.0, dist);
    return vec4f(color.rgb, color.a * alpha);
}

// Draw an arc (portion of a circle)
fn draw_arc(coord: vec2f, center: vec2f, radius: f32, thickness: f32, start_angle: f32, end_angle: f32, color: vec4f) -> vec4f {
    let p = coord - center;
    let dist = length(p);
    let angle = atan2(p.y, p.x);

    // Normalize angles
    var normalized_angle = angle;
    var normalized_start = start_angle;
    var normalized_end = end_angle;

    // Check if angle is within arc range
    let in_arc = normalized_angle >= normalized_start && normalized_angle <= normalized_end;

    if (in_arc) {
        let inner = radius - thickness * 0.5;
        let outer = radius + thickness * 0.5;

        let alpha_outer = 1.0 - smoothstep_f32(outer - 1.0, outer + 1.0, dist);
        let alpha_inner = smoothstep_f32(inner - 1.0, inner + 1.0, dist);
        let alpha = alpha_outer * alpha_inner;

        return vec4f(color.rgb, color.a * alpha);
    }

    return vec4f(0.0);
}

// Draw an ellipse
fn draw_ellipse(coord: vec2f, center: vec2f, radii: vec2f, color: vec4f) -> vec4f {
    let p = coord - center;
    let normalized = p / radii;
    let dist = length(normalized);
    let alpha = 1.0 - smoothstep_f32(0.98, 1.02, dist);
    return vec4f(color.rgb, color.a * alpha);
}

// Draw a ring (donut shape)
fn draw_ring(coord: vec2f, center: vec2f, inner_radius: f32, outer_radius: f32, color: vec4f) -> vec4f {
    let dist = length(coord - center);
    let alpha_outer = 1.0 - smoothstep_f32(outer_radius - 1.0, outer_radius + 1.0, dist);
    let alpha_inner = smoothstep_f32(inner_radius - 1.0, inner_radius + 1.0, dist);
    let alpha = alpha_outer * alpha_inner;
    return vec4f(color.rgb, color.a * alpha);
}

// Draw a pie slice
fn draw_pie_slice(coord: vec2f, center: vec2f, radius: f32, start_angle: f32, end_angle: f32, color: vec4f) -> vec4f {
    let p = coord - center;
    let dist = length(p);
    let angle = atan2(p.y, p.x);

    // Check if within radius
    if (dist > radius) {
        return vec4f(0.0);
    }

    // Check if within angle range
    var in_slice = false;
    if (start_angle <= end_angle) {
        in_slice = angle >= start_angle && angle <= end_angle;
    } else {
        in_slice = angle >= start_angle || angle <= end_angle;
    }

    if (in_slice) {
        let alpha = 1.0 - smoothstep_f32(radius - 1.0, radius + 1.0, dist);
        return vec4f(color.rgb, color.a * alpha);
    }

    return vec4f(0.0);
}

// Draw a bezier curve (quadratic)
fn draw_bezier_quadratic(coord: vec2f, p0: vec2f, p1: vec2f, p2: vec2f, thickness: f32, color: vec4f) -> vec4f {
    // Sample the curve and find closest point
    var min_dist = 10000.0;

    for (var t = 0.0; t <= 1.0; t += 0.02) {
        let point = (1.0 - t) * (1.0 - t) * p0 + 2.0 * (1.0 - t) * t * p1 + t * t * p2;
        let dist = length(coord - point);
        min_dist = min(min_dist, dist);
    }

    let alpha = 1.0 - smoothstep_f32(thickness * 0.5 - 1.0, thickness * 0.5 + 1.0, min_dist);
    return vec4f(color.rgb, color.a * alpha);
}

// Draw a grid
fn draw_grid(coord: vec2f, cell_size: vec2f, thickness: f32, color: vec4f) -> vec4f {
    let grid_pos = coord % cell_size;
    let dist_x = min(grid_pos.x, cell_size.x - grid_pos.x);
    let dist_y = min(grid_pos.y, cell_size.y - grid_pos.y);
    let dist = min(dist_x, dist_y);
    let alpha = 1.0 - smoothstep_f32(thickness * 0.5 - 0.5, thickness * 0.5 + 0.5, dist);
    return vec4f(color.rgb, color.a * alpha);
}

// Draw a checkerboard pattern
fn draw_checkerboard(coord: vec2f, cell_size: f32, color1: vec4f, color2: vec4f) -> vec4f {
    let cell = floor(coord / cell_size);
    let checker = (i32(cell.x) + i32(cell.y)) % 2;
    return select(color1, color2, checker == 0);
}

// Draw a gradient (linear)
fn draw_gradient_linear(coord: vec2f, start_pos: vec2f, end_pos: vec2f, start_color: vec4f, end_color: vec4f) -> vec4f {
    let dir = end_pos - start_pos;
    let t = clamp_f32(dot(coord - start_pos, dir) / dot(dir, dir), 0.0, 1.0);
    return lerp_color(start_color, end_color, t);
}

// Draw a radial gradient
fn draw_gradient_radial(coord: vec2f, center: vec2f, radius: f32, inner_color: vec4f, outer_color: vec4f) -> vec4f {
    let dist = length(coord - center);
    let t = clamp_f32(dist / radius, 0.0, 1.0);
    return lerp_color(inner_color, outer_color, t);
}

// Draw a cross/plus shape
fn draw_cross(coord: vec2f, center: vec2f, size: f32, thickness: f32, color: vec4f) -> vec4f {
    let p = abs(coord - center);
    let horizontal = p.x < size && p.y < thickness * 0.5;
    let vertical = p.y < size && p.x < thickness * 0.5;

    if (horizontal || vertical) {
        return color;
    }
    return vec4f(0.0);
}

// Draw an X shape
fn draw_x(coord: vec2f, center: vec2f, size: f32, thickness: f32, color: vec4f) -> vec4f {
    let p = coord - center;
    let d1 = abs(p.x + p.y);
    let d2 = abs(p.x - p.y);
    let in_bounds = abs(p.x) < size && abs(p.y) < size;

    if (in_bounds && (d1 < thickness || d2 < thickness)) {
        return color;
    }
    return vec4f(0.0);
}

// Draw an arrow
fn draw_arrow(coord: vec2f, start: vec2f, end: vec2f, shaft_thickness: f32, head_size: f32, color: vec4f) -> vec4f {
    // Draw shaft
    let shaft = draw_line(coord, start, end, shaft_thickness, color);

    // Draw arrowhead
    let dir = normalize(end - start);
    let perp = vec2f(-dir.y, dir.x);

    let head_p1 = end - dir * head_size + perp * head_size * 0.5;
    let head_p2 = end - dir * head_size - perp * head_size * 0.5;
    let head = draw_triangle(coord, end, head_p1, head_p2, color);

    // Blend shaft and head
    return vec4f(
        max(shaft.rgb, head.rgb),
        max(shaft.a, head.a)
    );
}

// ----------------------------------------------------------------------------
// Alpha Blending Helper
// ----------------------------------------------------------------------------

// Blend two colors using alpha compositing
fn blend_over(bottom: vec4f, top: vec4f) -> vec4f {
    let alpha = top.a + bottom.a * (1.0 - top.a);
    if (alpha == 0.0) {
        return vec4f(0.0);
    }
    let rgb = (top.rgb * top.a + bottom.rgb * bottom.a * (1.0 - top.a)) / alpha;
    return vec4f(rgb, alpha);
}
