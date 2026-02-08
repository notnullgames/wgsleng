# 3D Graphics Library Quick Reference

Complete reference for all 100+ functions in `draw3d.wgsl`.

## Table of Contents

- [Constants](#constants)
- [Vector Math (8 functions)](#vector-math)
- [Matrix Operations (7 functions)](#matrix-operations)
- [Camera Utilities (3 functions)](#camera-utilities)
- [SDF Primitives (12 functions)](#sdf-primitives)
- [SDF Operations (6 functions)](#sdf-operations)
- [SDF Modifiers (6 functions)](#sdf-modifiers)
- [Lighting (6 functions)](#lighting)
- [Ray Marching (2 functions)](#ray-marching)
- [Color Utilities (5 functions)](#color-utilities)
- [Noise (3 functions)](#noise)
- [Utilities (5 functions)](#utilities)

---

## Constants

```wgsl
const PI: f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;
const MAX_STEPS: i32 = 100;       // Maximum ray marching steps
const MAX_DIST: f32 = 100.0;      // Maximum ray distance
const SURF_DIST: f32 = 0.01;      // Surface hit threshold
```

---

## Vector Math

### `cross(a: vec3f, b: vec3f) -> vec3f`

Calculate cross product of two 3D vectors.

**Example:**

```wgsl
let normal = cross(vec3f(1.0, 0.0, 0.0), vec3f(0.0, 1.0, 0.0)); // vec3f(0, 0, 1)
```

### `reflect_vec(v: vec3f, n: vec3f) -> vec3f`

Reflect vector v across normal n.

**Example:**

```wgsl
let reflected = reflect_vec(ray_dir, surface_normal);
```

### `refract_vec(v: vec3f, n: vec3f, eta: f32) -> vec3f`

Refract vector v through surface with normal n and refractive index eta.

**Parameters:**

- `eta`: Ratio of refractive indices (e.g., 1.0/1.33 for air to water)

**Example:**

```wgsl
let refracted = refract_vec(ray_dir, normal, 1.0/1.5); // Glass refraction
```

### `rotate_x(p: vec3f, angle: f32) -> vec3f`

Rotate point p around X axis by angle (radians).

**Example:**

```wgsl
let rotated = rotate_x(point, PI * 0.5); // 90 degree rotation
```

### `rotate_y(p: vec3f, angle: f32) -> vec3f`

Rotate point p around Y axis by angle (radians).

### `rotate_z(p: vec3f, angle: f32) -> vec3f`

Rotate point p around Z axis by angle (radians).

### `rotate_euler(p: vec3f, angles: vec3f) -> vec3f`

Rotate point p using Euler angles (X, Y, Z order).

**Parameters:**

- `angles.x`: Rotation around X axis (pitch)
- `angles.y`: Rotation around Y axis (yaw)
- `angles.z`: Rotation around Z axis (roll)

**Example:**

```wgsl
let rotated = rotate_euler(point, vec3f(time, time * 0.5, 0.0));
```

### `get_normal(scene_fn: fn(vec3f) -> f32, p: vec3f) -> vec3f`

Calculate surface normal at point p using gradient method.

**Note:** Usually you'll create your own version that calls your specific scene function.

---

## Matrix Operations

### `mat4_identity() -> mat4x4f`

Create 4x4 identity matrix.

### `mat4_translate(t: vec3f) -> mat4x4f`

Create translation matrix.

**Example:**

```wgsl
let transform = mat4_translate(vec3f(1.0, 2.0, 3.0));
```

### `mat4_scale(s: vec3f) -> mat4x4f`

Create scale matrix.

### `mat4_rotate_x(angle: f32) -> mat4x4f`

Create rotation matrix around X axis.

### `mat4_rotate_y(angle: f32) -> mat4x4f`

Create rotation matrix around Y axis.

### `mat4_rotate_z(angle: f32) -> mat4x4f`

Create rotation matrix around Z axis.

### `mat4_rotate(axis: vec3f, angle: f32) -> mat4x4f`

Create rotation matrix around arbitrary axis.

**Parameters:**

- `axis`: Rotation axis (should be normalized)
- `angle`: Rotation angle in radians

---

## Camera Utilities

### `look_at(eye: vec3f, center: vec3f, up: vec3f) -> mat4x4f`

Create view matrix looking from eye to center with up vector.

**Example:**

```wgsl
let view_matrix = look_at(
    vec3f(0.0, 5.0, 10.0),  // Camera position
    vec3f(0.0, 0.0, 0.0),   // Look at origin
    vec3f(0.0, 1.0, 0.0)    // Up is +Y
);
```

### `perspective(fov: f32, aspect: f32, near: f32, far: f32) -> mat4x4f`

Create perspective projection matrix.

**Parameters:**

- `fov`: Field of view in radians
- `aspect`: Width / height ratio
- `near`: Near clipping plane
- `far`: Far clipping plane

### `get_ray_direction(uv: vec2f, camera_pos: vec3f, look_at_pos: vec3f, zoom: f32) -> vec3f`

Generate ray direction for given UV coordinates and camera setup.

**Parameters:**

- `uv`: Screen coordinates (-1 to 1, aspect-corrected)
- `zoom`: Field of view control (higher = narrower FOV)

**Example:**

```wgsl
let ray_dir = get_ray_direction(uv, camera_pos, vec3f(0.0), 2.0);
```

---

## SDF Primitives

All SDF functions return the signed distance to the surface:

- **Negative** = inside shape
- **Zero** = on surface
- **Positive** = outside shape

### `sdf_sphere(p: vec3f, radius: f32) -> f32`

Sphere centered at origin.

**Example:**

```wgsl
let dist = sdf_sphere(p - vec3f(1.0, 0.0, 0.0), 0.5); // Sphere at (1,0,0)
```

### `sdf_box(p: vec3f, size: vec3f) -> f32`

Box centered at origin with given half-extents.

**Example:**

```wgsl
let dist = sdf_box(p, vec3f(1.0, 0.5, 1.0)); // 2x1x2 box
```

### `sdf_rounded_box(p: vec3f, size: vec3f, radius: f32) -> f32`

Box with rounded corners.

**Parameters:**

- `radius`: Corner rounding radius

### `sdf_torus(p: vec3f, major_radius: f32, minor_radius: f32) -> f32`

Torus (donut) aligned with Y axis.

**Parameters:**

- `major_radius`: Distance from center to tube center
- `minor_radius`: Tube thickness

**Example:**

```wgsl
let dist = sdf_torus(p, 1.0, 0.3); // Donut with 1.0 hole, 0.3 thick
```

### `sdf_cylinder(p: vec3f, radius: f32) -> f32`

Infinite cylinder along Y axis.

### `sdf_capped_cylinder(p: vec3f, height: f32, radius: f32) -> f32`

Finite cylinder along Y axis.

**Parameters:**

- `height`: Total height (extends height/2 above and below origin)

### `sdf_cone(p: vec3f, angle: f32, height: f32) -> f32`

Cone pointing up along Y axis.

**Parameters:**

- `angle`: Half-angle in radians
- `height`: Cone height

### `sdf_plane(p: vec3f, normal: vec3f, distance: f32) -> f32`

Infinite plane.

**Parameters:**

- `normal`: Plane normal (should be normalized)
- `distance`: Distance from origin along normal

**Example:**

```wgsl
let ground = sdf_plane(p, vec3f(0.0, 1.0, 0.0), 0.0); // XZ plane at Y=0
```

### `sdf_capsule(p: vec3f, a: vec3f, b: vec3f, radius: f32) -> f32`

Capsule between two points.

**Example:**

```wgsl
let dist = sdf_capsule(p, vec3f(0, -1, 0), vec3f(0, 1, 0), 0.3);
```

### `sdf_octahedron(p: vec3f, size: f32) -> f32`

Octahedron (8-sided diamond).

### `sdf_pyramid(p: vec3f, height: f32) -> f32`

Four-sided pyramid pointing up.

---

## SDF Operations

### `sdf_union(d1: f32, d2: f32) -> f32`

Combine two shapes (returns minimum distance).

**Example:**

```wgsl
let sphere = sdf_sphere(p, 1.0);
let box = sdf_box(p - vec3f(1.5, 0, 0), vec3f(0.5));
let combined = sdf_union(sphere, box);
```

### `sdf_subtract(d1: f32, d2: f32) -> f32`

Subtract d2 from d1 (carve out).

**Example:**

```wgsl
let box = sdf_box(p, vec3f(1.0));
let sphere = sdf_sphere(p, 0.8);
let carved = sdf_subtract(box, sphere); // Hollow box
```

### `sdf_intersect(d1: f32, d2: f32) -> f32`

Intersection of two shapes (returns maximum distance).

### `sdf_smooth_union(d1: f32, d2: f32, k: f32) -> f32`

Smooth blend between two shapes.

**Parameters:**

- `k`: Smoothness factor (0.0 = sharp, higher = smoother)

**Example:**

```wgsl
let blended = sdf_smooth_union(sphere, box, 0.3);
```

### `sdf_smooth_subtract(d1: f32, d2: f32, k: f32) -> f32`

Smooth subtraction with blended edges.

### `sdf_smooth_intersect(d1: f32, d2: f32, k: f32) -> f32`

Smooth intersection with blended edges.

---

## SDF Modifiers

### `sdf_elongate(p: vec3f, h: vec3f) -> vec3f`

Elongate space along given axes.

**Example:**

```wgsl
let elongated_p = sdf_elongate(p, vec3f(0.5, 0.0, 0.5));
let dist = sdf_sphere(elongated_p, 0.5); // Stretched sphere
```

### `sdf_round(dist: f32, radius: f32) -> f32`

Add thickness to any shape.

**Example:**

```wgsl
let box = sdf_box(p, vec3f(1.0));
let rounded = sdf_round(box, 0.1); // Slightly larger, rounded
```

### `sdf_onion(dist: f32, thickness: f32) -> f32`

Make shape hollow (only shell remains).

**Example:**

```wgsl
let sphere = sdf_sphere(p, 1.0);
let hollow = sdf_onion(sphere, 0.1); // Thin shell
```

### `sdf_repeat(p: vec3f, spacing: vec3f) -> vec3f`

Infinite repetition of space.

**Example:**

```wgsl
let repeated_p = sdf_repeat(p, vec3f(2.0, 0.0, 2.0));
let dist = sdf_sphere(repeated_p, 0.3); // Infinite grid of spheres
```

### `sdf_repeat_limited(p: vec3f, spacing: vec3f, limit: vec3f) -> vec3f`

Limited repetition (creates grid of N×N×N instances).

**Parameters:**

- `limit`: Number of repetitions per axis

**Example:**

```wgsl
let repeated_p = sdf_repeat_limited(p, vec3f(2.0), vec3f(3.0, 1.0, 3.0));
let dist = sdf_sphere(repeated_p, 0.3); // 3×1×3 grid
```

### `sdf_twist(p: vec3f, k: f32) -> vec3f`

Twist space around Y axis.

**Parameters:**

- `k`: Twist amount (higher = more twisted)

---

## Lighting

### `lighting_diffuse(normal: vec3f, light_dir: vec3f) -> f32`

Lambertian diffuse lighting.

**Returns:** 0.0 to 1.0

**Example:**

```wgsl
let normal = get_normal_at(hit_pos);
let light_dir = normalize(light_pos - hit_pos);
let diffuse = lighting_diffuse(normal, light_dir);
```

### `lighting_specular(normal: vec3f, light_dir: vec3f, view_dir: vec3f, shininess: f32) -> f32`

Phong specular highlights.

**Parameters:**

- `shininess`: Specular power (higher = tighter highlights, typical: 8-128)

### `lighting_blinn_phong(normal: vec3f, light_dir: vec3f, view_dir: vec3f, shininess: f32) -> f32`

Blinn-Phong specular (more physically accurate than Phong).

**Example:**

```wgsl
let specular = lighting_blinn_phong(normal, light_dir, view_dir, 32.0);
let color = base_color * diffuse + vec3f(1.0) * specular * 0.5;
```

### `lighting_rim(normal: vec3f, view_dir: vec3f, power: f32) -> f32`

Rim/edge lighting (Fresnel-like effect).

**Parameters:**

- `power`: Edge sharpness (higher = thinner rim, typical: 2-5)

**Example:**

```wgsl
let rim = lighting_rim(normal, view_dir, 3.0) * 0.3;
color += rim; // Add edge highlight
```

### `lighting_ao(p: vec3f, normal: vec3f, scene_fn: fn(vec3f) -> f32) -> f32`

Ambient occlusion approximation.

**Returns:** 0.0 (occluded) to 1.0 (exposed)

**Note:** Requires your scene function as parameter.

### `lighting_soft_shadow(p: vec3f, light_dir: vec3f, scene_fn: fn(vec3f) -> f32, min_t: f32, max_t: f32, k: f32) -> f32`

Soft shadows with penumbra.

**Parameters:**

- `min_t`: Start distance from surface (avoid self-shadowing)
- `max_t`: Maximum shadow ray distance
- `k`: Shadow softness (higher = softer, typical: 4-32)

**Returns:** 0.0 (full shadow) to 1.0 (no shadow)

---

## Ray Marching

### `ray_march(ray_origin: vec3f, ray_dir: vec3f, scene_fn: fn(vec3f) -> f32) -> f32`

March ray through scene using sphere tracing.

**Returns:** Distance traveled (MAX_DIST if no hit)

**Example:**

```wgsl
let dist = ray_march(camera_pos, ray_dir, my_scene);
if (dist < MAX_DIST) {
    let hit_pos = camera_pos + ray_dir * dist;
    // Shade surface
}
```

### `ray_intersect(ray_origin: vec3f, ray_dir: vec3f, scene_fn: fn(vec3f) -> f32) -> vec3f`

Extended ray march that returns (distance, steps_taken, final_dist_to_surface).

**Useful for:** Debugging, performance analysis, special effects based on march count.

---

## Color Utilities

### `hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3f`

Convert HSV color to RGB.

**Parameters:**

- `h`: Hue (0.0 to 1.0)
- `s`: Saturation (0.0 to 1.0)
- `v`: Value/brightness (0.0 to 1.0)

**Example:**

```wgsl
let rainbow = hsv_to_rgb(time * 0.1, 1.0, 1.0); // Animated rainbow
```

### `rgb_to_hsv(rgb: vec3f) -> vec3f`

Convert RGB color to HSV.

### `gamma_correct(color: vec3f, gamma: f32) -> vec3f`

Apply gamma correction.

**Parameters:**

- `gamma`: Typical value is 2.2 for sRGB

**Example:**

```wgsl
color = gamma_correct(color, 2.2); // Linear to sRGB
```

### `tone_map_reinhard(color: vec3f) -> vec3f`

Reinhard tone mapping (simple HDR to LDR).

### `tone_map_aces(color: vec3f) -> vec3f`

ACES filmic tone mapping (cinematic look).

---

## Noise

### `hash(p: vec3f) -> f32`

3D hash function for pseudo-random values.

**Returns:** 0.0 to 1.0

### `noise_3d(p: vec3f) -> f32`

3D value noise (smooth random).

**Returns:** 0.0 to 1.0

**Example:**

```wgsl
let pattern = noise_3d(hit_pos * 5.0); // Scale for detail
color *= mix(0.5, 1.0, pattern);
```

### `fbm(p: vec3f, octaves: i32) -> f32`

Fractional Brownian Motion (layered noise).

**Parameters:**

- `octaves`: Number of noise layers (more = more detail, typical: 3-6)

**Returns:** ~0.0 to 1.0 (not strictly bounded)

**Example:**

```wgsl
let terrain = fbm(p, 4) * 2.0; // Multi-scale noise
```

---

## Utilities

### `smin(a: f32, b: f32, k: f32) -> f32`

Smooth minimum (for smooth unions).

### `smax(a: f32, b: f32, k: f32) -> f32`

Smooth maximum (for smooth intersections).

### `lerp_vec3(a: vec3f, b: vec3f, t: f32) -> vec3f`

Linear interpolation between 3D vectors.

### `cubic(t: f32) -> f32`

Cubic interpolation curve (smoothstep-like).

### `quintic(t: f32) -> f32`

Quintic interpolation curve (smoother than cubic).

---

## Usage Tips

1. **Performance**: Keep MAX_STEPS low (50-100) for real-time. Increase SURF_DIST if rendering is slow.

2. **Normals**: Always normalize normals before lighting calculations.

3. **Scene Complexity**: Use smooth operations sparingly - they're more expensive than sharp operations.

4. **Material IDs**: Extend scene functions to return `vec2f(distance, material_id)` for multi-material scenes.

5. **Shadows**: Soft shadows are expensive - use sparingly or reduce max_t distance.

6. **Ambient Occlusion**: Very expensive (5 extra ray marches) - use only when needed.

7. **Repetition**: Be careful with infinite repetition near camera - can cause artifacts.

8. **Transformations**: Apply transformations to points before passing to SDFs, not to the distances.

---

## Complete Example

```wgsl
@import("draw3d.wgsl")

fn my_scene(p: vec3f) -> f32 {
    // Two spheres with smooth blend
    let sphere1 = sdf_sphere(p - vec3f(-1.0, 0.0, 0.0), 0.8);
    let sphere2 = sdf_sphere(p - vec3f(1.0, 0.0, 0.0), 0.8);
    let combined = sdf_smooth_union(sphere1, sphere2, 0.3);

    // Ground plane
    let ground = sdf_plane(p, vec3f(0.0, 1.0, 0.0), 1.0);

    return sdf_union(combined, ground);
}

@fragment
fn fs_main(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Setup camera
    let uv = (coord.xy / vec2f(@engine.screen_width, @engine.screen_height)) * 2.0 - 1.0;
    let camera_pos = vec3f(0.0, 2.0, 5.0);
    let ray_dir = get_ray_direction(uv, camera_pos, vec3f(0.0), 2.0);

    // Ray march
    let dist = ray_march(camera_pos, ray_dir, my_scene);

    var color = vec3f(0.5, 0.7, 1.0); // Sky

    if (dist < MAX_DIST) {
        let hit_pos = camera_pos + ray_dir * dist;
        let normal = get_normal(my_scene, hit_pos);
        let view_dir = -ray_dir;

        // Lighting
        let light_dir = normalize(vec3f(1.0, 1.0, -1.0));
        let diffuse = lighting_diffuse(normal, light_dir);
        let specular = lighting_blinn_phong(normal, light_dir, view_dir, 32.0);

        color = vec3f(0.8) * (diffuse + specular * 0.5);
        color = gamma_correct(color, 2.2);
    }

    return vec4f(color, 1.0);
}
```
