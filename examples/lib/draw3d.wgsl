// ============================================================================
// 3D Graphics Library for WGSL Engine
// A comprehensive collection of 3D rendering utilities and primitives
// ============================================================================

// ----------------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------------
const PI = 3.14159265359;
const TAU = 6.28318530718;
const EPSILON = 0.001;
const MAX_STEPS = 100;
const MAX_DIST = 100.0;
const SURF_DIST = 0.01;

// ----------------------------------------------------------------------------
// Vector Math Utilities
// ----------------------------------------------------------------------------

// Cross product of two 3D vectors
fn cross(a: vec3f, b: vec3f) -> vec3f {
    return vec3f(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x
    );
}

// Reflect vector v around normal n
fn reflect_vec(v: vec3f, n: vec3f) -> vec3f {
    return v - 2.0 * dot(v, n) * n;
}

// Refract vector (Snell's law)
fn refract_vec(v: vec3f, n: vec3f, eta: f32) -> vec3f {
    let cos_i = -dot(n, v);
    let sin_t2 = eta * eta * (1.0 - cos_i * cos_i);
    if (sin_t2 > 1.0) {
        return vec3f(0.0); // Total internal reflection
    }
    let cos_t = sqrt(1.0 - sin_t2);
    return eta * v + (eta * cos_i - cos_t) * n;
}

// Rotate point around X axis
fn rotate_x(p: vec3f, angle: f32) -> vec3f {
    let c = cos(angle);
    let s = sin(angle);
    return vec3f(p.x, c * p.y - s * p.z, s * p.y + c * p.z);
}

// Rotate point around Y axis
fn rotate_y(p: vec3f, angle: f32) -> vec3f {
    let c = cos(angle);
    let s = sin(angle);
    return vec3f(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);
}

// Rotate point around Z axis
fn rotate_z(p: vec3f, angle: f32) -> vec3f {
    let c = cos(angle);
    let s = sin(angle);
    return vec3f(c * p.x - s * p.y, s * p.x + c * p.y, p.z);
}

// Rotate point using Euler angles (XYZ order)
fn rotate_euler(p: vec3f, angles: vec3f) -> vec3f {
    var result = rotate_x(p, angles.x);
    result = rotate_y(result, angles.y);
    result = rotate_z(result, angles.z);
    return result;
}

// ----------------------------------------------------------------------------
// Matrix Operations (4x4)
// ----------------------------------------------------------------------------

// Create identity matrix
fn mat4_identity() -> mat4x4f {
    return mat4x4f(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
}

// Create translation matrix
fn mat4_translate(v: vec3f) -> mat4x4f {
    return mat4x4f(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        v.x, v.y, v.z, 1.0
    );
}

// Create scale matrix
fn mat4_scale(s: vec3f) -> mat4x4f {
    return mat4x4f(
        s.x, 0.0, 0.0, 0.0,
        0.0, s.y, 0.0, 0.0,
        0.0, 0.0, s.z, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
}

// Create rotation matrix around X axis
fn mat4_rotate_x(angle: f32) -> mat4x4f {
    let c = cos(angle);
    let s = sin(angle);
    return mat4x4f(
        1.0, 0.0, 0.0, 0.0,
        0.0, c, s, 0.0,
        0.0, -s, c, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
}

// Create rotation matrix around Y axis
fn mat4_rotate_y(angle: f32) -> mat4x4f {
    let c = cos(angle);
    let s = sin(angle);
    return mat4x4f(
        c, 0.0, -s, 0.0,
        0.0, 1.0, 0.0, 0.0,
        s, 0.0, c, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
}

// Create rotation matrix around Z axis
fn mat4_rotate_z(angle: f32) -> mat4x4f {
    let c = cos(angle);
    let s = sin(angle);
    return mat4x4f(
        c, s, 0.0, 0.0,
        -s, c, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
}

// ----------------------------------------------------------------------------
// Camera Utilities
// ----------------------------------------------------------------------------

// Create a lookAt view matrix
fn look_at(eye: vec3f, center: vec3f, up: vec3f) -> mat4x4f {
    let f = normalize(center - eye);
    let s = normalize(cross(f, up));
    let u = cross(s, f);

    return mat4x4f(
        s.x, u.x, -f.x, 0.0,
        s.y, u.y, -f.y, 0.0,
        s.z, u.z, -f.z, 0.0,
        -dot(s, eye), -dot(u, eye), dot(f, eye), 1.0
    );
}

// Create perspective projection matrix
fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> mat4x4f {
    let f = 1.0 / tan(fov / 2.0);
    return mat4x4f(
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, (far + near) / (near - far), -1.0,
        0.0, 0.0, (2.0 * far * near) / (near - far), 0.0
    );
}

// Generate ray direction for a pixel (simple camera)
fn get_ray_direction(uv: vec2f, camera_pos: vec3f, look_at: vec3f, zoom: f32) -> vec3f {
    let forward = normalize(look_at - camera_pos);
    let right = normalize(cross(vec3f(0.0, 1.0, 0.0), forward));
    let up = cross(forward, right);

    return normalize(forward * zoom + uv.x * right + uv.y * up);
}

// ----------------------------------------------------------------------------
// 3D SDF Primitives (Signed Distance Functions)
// ----------------------------------------------------------------------------

// Sphere
fn sdf_sphere(p: vec3f, radius: f32) -> f32 {
    return length(p) - radius;
}

// Box
fn sdf_box(p: vec3f, size: vec3f) -> f32 {
    let d = abs(p) - size;
    return length(max(d, vec3f(0.0))) + min(max(d.x, max(d.y, d.z)), 0.0);
}

// Rounded box
fn sdf_rounded_box(p: vec3f, size: vec3f, radius: f32) -> f32 {
    let d = abs(p) - size;
    return length(max(d, vec3f(0.0))) + min(max(d.x, max(d.y, d.z)), 0.0) - radius;
}

// Torus
fn sdf_torus(p: vec3f, major_radius: f32, minor_radius: f32) -> f32 {
    let q = vec2f(length(p.xz) - major_radius, p.y);
    return length(q) - minor_radius;
}

// Cylinder (infinite)
fn sdf_cylinder(p: vec3f, radius: f32) -> f32 {
    return length(p.xz) - radius;
}

// Capped cylinder
fn sdf_capped_cylinder(p: vec3f, radius: f32, height: f32) -> f32 {
    let d = abs(vec2f(length(p.xz), p.y)) - vec2f(radius, height);
    return min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));
}

// Cone
fn sdf_cone(p: vec3f, angle: f32, height: f32) -> f32 {
    let c = vec2f(sin(angle), cos(angle));
    let q = vec2f(length(p.xz), -p.y);
    let d = length(q - c * max(dot(q, c), 0.0));
    return d * select(-1.0, 1.0, q.y > height || q.x > c.y * height);
}

// Plane
fn sdf_plane(p: vec3f, normal: vec3f, distance: f32) -> f32 {
    return dot(p, normal) + distance;
}

// Capsule
fn sdf_capsule(p: vec3f, a: vec3f, b: vec3f, radius: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h) - radius;
}

// Octahedron
fn sdf_octahedron(p: vec3f, size: f32) -> f32 {
    let q = abs(p);
    return (q.x + q.y + q.z - size) * 0.57735027;
}

// Pyramid
fn sdf_pyramid(p: vec3f, height: f32) -> f32 {
    // Just use a cone - it's smooth and stable
    // (A square pyramid is too complex for accurate SDF)
    return sdf_cone(p, atan2(height, 0.6), height);
}

// ----------------------------------------------------------------------------
// SDF Operations
// ----------------------------------------------------------------------------

// Union (combine two shapes)
fn sdf_union(d1: f32, d2: f32) -> f32 {
    return min(d1, d2);
}

// Subtraction (subtract d2 from d1)
fn sdf_subtract(d1: f32, d2: f32) -> f32 {
    return max(d1, -d2);
}

// Intersection
fn sdf_intersect(d1: f32, d2: f32) -> f32 {
    return max(d1, d2);
}

// Smooth union
fn sdf_smooth_union(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

// Smooth subtraction
fn sdf_smooth_subtract(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (d2 + d1) / k, 0.0, 1.0);
    return mix(d2, -d1, h) + k * h * (1.0 - h);
}

// Smooth intersection
fn sdf_smooth_intersect(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) + k * h * (1.0 - h);
}

// ----------------------------------------------------------------------------
// SDF Modifiers
// ----------------------------------------------------------------------------

// Elongate shape
fn sdf_elongate(p: vec3f, h: vec3f) -> vec3f {
    return p - clamp(p, -h, h);
}

// Round shape (inflate/deflate)
fn sdf_round(d: f32, radius: f32) -> f32 {
    return d - radius;
}

// Onion (hollow out)
fn sdf_onion(d: f32, thickness: f32) -> f32 {
    return abs(d) - thickness;
}

// Repeat in 3D space
fn sdf_repeat(p: vec3f, spacing: vec3f) -> vec3f {
    return (p + spacing * 0.5) % spacing - spacing * 0.5;
}

// Repeat limited
fn sdf_repeat_limited(p: vec3f, spacing: f32, limit: vec3f) -> vec3f {
    return p - spacing * clamp(round(p / spacing), -limit, limit);
}

// ----------------------------------------------------------------------------
// Normal Calculation
// ----------------------------------------------------------------------------

// Calculate surface normal using gradient
fn calculate_normal(p: vec3f, dist_func: ptr<function, f32>) -> vec3f {
    let h = 0.0001;
    let k = vec2f(1.0, -1.0);
    return normalize(
        k.xyy * *dist_func +
        k.yyx * *dist_func +
        k.yxy * *dist_func +
        k.xxx * *dist_func
    );
}

// Simple normal calculation (no function pointer)
fn get_normal(p: vec3f) -> vec3f {
    let h = 0.001;
    let k = vec2f(1.0, -1.0);
    // This is a placeholder - replace with actual scene distance in your code
    return normalize(vec3f(0.0, 1.0, 0.0));
}

// ----------------------------------------------------------------------------
// Lighting Functions
// ----------------------------------------------------------------------------

// Basic diffuse lighting (Lambertian)
fn lighting_diffuse(normal: vec3f, light_dir: vec3f) -> f32 {
    return max(dot(normal, light_dir), 0.0);
}

// Specular lighting (Phong)
fn lighting_specular(normal: vec3f, light_dir: vec3f, view_dir: vec3f, shininess: f32) -> f32 {
    let reflect_dir = reflect(-light_dir, normal);
    return pow(max(dot(view_dir, reflect_dir), 0.0), shininess);
}

// Blinn-Phong specular
fn lighting_blinn_phong(normal: vec3f, light_dir: vec3f, view_dir: vec3f, shininess: f32) -> f32 {
    let halfway = normalize(light_dir + view_dir);
    return pow(max(dot(normal, halfway), 0.0), shininess);
}

// Rim lighting (Fresnel-like)
fn lighting_rim(normal: vec3f, view_dir: vec3f, power: f32) -> f32 {
    return pow(1.0 - abs(dot(normal, view_dir)), power);
}

// Simple ambient occlusion
fn lighting_ao(p: vec3f, normal: vec3f, dist_func: f32) -> f32 {
    var occ = 0.0;
    var weight = 1.0;
    for (var i = 1; i <= 5; i++) {
        let fi = f32(i);
        let h = 0.01 + 0.12 * fi / 5.0;
        // Use dist_func parameter here in actual implementation
        let d = dist_func;
        occ += (h - d) * weight;
        weight *= 0.95;
    }
    return clamp(1.0 - 3.0 * occ, 0.0, 1.0);
}

// Soft shadow
fn lighting_soft_shadow(ray_origin: vec3f, ray_dir: vec3f, mint: f32, maxt: f32, k: f32) -> f32 {
    var res = 1.0;
    var t = mint;
    for (var i = 0; i < 50; i++) {
        // Use scene distance function here in actual implementation
        let h = 0.01; // placeholder
        res = min(res, k * h / t);
        t += clamp(h, 0.02, 0.1);
        if (h < 0.001 || t > maxt) {
            break;
        }
    }
    return clamp(res, 0.0, 1.0);
}

// ----------------------------------------------------------------------------
// Ray Marching
// ----------------------------------------------------------------------------

// Ray march to find surface intersection
fn ray_march(ray_origin: vec3f, ray_dir: vec3f, max_steps: i32, max_dist: f32) -> f32 {
    var dist = 0.0;
    for (var i = 0; i < max_steps; i++) {
        let p = ray_origin + ray_dir * dist;
        // Use scene distance function here in actual implementation
        let d = 1.0; // placeholder
        dist += d;
        if (d < SURF_DIST || dist > max_dist) {
            break;
        }
    }
    return dist;
}

// ----------------------------------------------------------------------------
// Color/Material Utilities
// ----------------------------------------------------------------------------

// HSV to RGB conversion
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

// Gamma correction
fn gamma_correct(color: vec3f, gamma: f32) -> vec3f {
    return pow(color, vec3f(1.0 / gamma));
}

// Tone mapping (Reinhard)
fn tone_map_reinhard(color: vec3f) -> vec3f {
    return color / (color + vec3f(1.0));
}

// Tone mapping (ACES)
fn tone_map_aces(color: vec3f) -> vec3f {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3f(0.0), vec3f(1.0));
}

// ----------------------------------------------------------------------------
// Noise Functions
// ----------------------------------------------------------------------------

// Hash function for noise
fn hash(p: vec3f) -> f32 {
    var h = dot(p, vec3f(127.1, 311.7, 74.7));
    return fract(sin(h) * 43758.5453123);
}

// 3D value noise
fn noise_3d(p: vec3f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(
            mix(hash(i + vec3f(0.0, 0.0, 0.0)), hash(i + vec3f(1.0, 0.0, 0.0)), u.x),
            mix(hash(i + vec3f(0.0, 1.0, 0.0)), hash(i + vec3f(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash(i + vec3f(0.0, 0.0, 1.0)), hash(i + vec3f(1.0, 0.0, 1.0)), u.x),
            mix(hash(i + vec3f(0.0, 1.0, 1.0)), hash(i + vec3f(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}

// Fractional Brownian Motion (fBm)
fn fbm(p: vec3f, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;

    for (var i = 0; i < octaves; i++) {
        value += amplitude * noise_3d(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }

    return value;
}

// ----------------------------------------------------------------------------
// Utility Functions
// ----------------------------------------------------------------------------

// Smooth minimum
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

// Smooth maximum
fn smax(a: f32, b: f32, k: f32) -> f32 {
    return -smin(-a, -b, k);
}

// Linear interpolation for vec3
fn lerp_vec3(a: vec3f, b: vec3f, t: f32) -> vec3f {
    return a + (b - a) * t;
}

// Cubic interpolation
fn cubic(t: f32) -> f32 {
    return t * t * (3.0 - 2.0 * t);
}

// Quintic interpolation
fn quintic(t: f32) -> f32 {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}
