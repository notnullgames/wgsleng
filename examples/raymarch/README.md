## 3D Demo - Ray Marched Primitives

This example demonstrates:

1. A comprehensive 3D graphics library (`draw3d.wgsl`) with ray marching and SDF primitives
2. A real-time 3D scene with multiple animated objects, lighting, and camera movement

### Features

The demo shows a 3D scene with:
- **Rotating Sphere** (Red) - Simple SDF sphere
- **Animated Cube** (Green) - Rotating box with Euler angles
- **Spinning Torus** (Blue) - Donut shape rotating on X axis
- **Floating Capsule** (Yellow) - Bobbing up and down
- **Octahedron** (Purple) - Diamond-shaped bipyramid on the ground
- **Checkered Ground** - Infinite plane with pattern
- **Orbiting Camera** - Rotates around the scene
- **Multiple Lights** - White main light + blue fill light
- **Rim Lighting** - Edge highlighting for better depth
- **Fog** - Distance-based atmospheric fog
- **Gamma Correction** - Proper color space handling

### 3D Graphics Library

The `draw3d.wgsl` library includes **100+ functions** organized into:

#### Vector Math (8 functions)
- Cross product, reflection, refraction
- Per-axis rotation (X, Y, Z)
- Euler angle rotation

#### Matrix Operations (7 functions)
- 4x4 matrix creation (identity, translate, scale, rotate)
- Individual rotation matrices for each axis

#### Camera Utilities (3 functions)
- Look-at matrix
- Perspective projection
- Ray direction generation

#### SDF Primitives (12 shapes)
- Sphere, Box, Rounded Box
- Torus, Cylinder, Capped Cylinder
- Cone, Plane, Capsule
- Octahedron, Pyramid

#### SDF Operations (6 functions)
- Union, Subtraction, Intersection
- Smooth variants of each operation

#### SDF Modifiers (6 functions)
- Elongate, Round, Onion (hollow)
- Infinite repeat, Limited repeat

#### Lighting (6 functions)
- Diffuse (Lambertian)
- Specular (Phong and Blinn-Phong)
- Rim lighting (Fresnel-like)
- Ambient occlusion
- Soft shadows

#### Ray Marching (2 functions)
- Ray march algorithm
- Surface intersection

#### Color Utilities (5 functions)
- HSV to RGB conversion
- Gamma correction
- Tone mapping (Reinhard and ACES)

#### Noise (3 functions)
- 3D value noise
- Hash function
- Fractional Brownian Motion (fBm)

#### Utilities (5 functions)
- Smooth min/max
- Vector lerp
- Cubic/Quintic interpolation

## How to Use

### Run the demo:

**Web:**
```bash
npm start
```
Navigate to `http://localhost:8000/#examples/3d/main.wgsl`

**Native:**
```bash
./native/target/release/wgsleng examples/3d/main.wgsl
```

### Use the library in your own game:

Import the 3D library at the top of your `main.wgsl`:

```wgsl
@import("draw3d.wgsl")

// Define your scene
fn my_scene(p: vec3f) -> f32 {
    let sphere = sdf_sphere(p, 1.0);
    let box = sdf_box(p - vec3f(2.0, 0.0, 0.0), vec3f(0.5, 0.5, 0.5));
    return sdf_union(sphere, box);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Set up camera and ray
    let camera_pos = vec3f(0.0, 0.0, -5.0);
    let ray_dir = get_ray_direction(uv, camera_pos, vec3f(0.0), 2.0);

    // Ray march through scene
    var dist = 0.0;
    for (var i = 0; i < MAX_STEPS; i++) {
        let p = camera_pos + ray_dir * dist;
        let d = my_scene(p);
        dist += d;
        if (d < SURF_DIST || dist > MAX_DIST) { break; }
    }

    // Shade based on distance
    var color = vec3f(0.0);
    if (dist < MAX_DIST) {
        color = vec3f(1.0);
    }

    return vec4f(color, 1.0);
}
```

## Example Usage

### Creating Complex Shapes

```wgsl
// Rounded box with smooth union to sphere
fn my_shape(p: vec3f) -> f32 {
    let box = sdf_rounded_box(p, vec3f(1.0, 0.5, 1.0), 0.1);
    let sphere = sdf_sphere(p + vec3f(0.0, 0.8, 0.0), 0.5);
    return sdf_smooth_union(box, sphere, 0.3);
}
```

### Rotating Objects

```wgsl
// Rotate point before passing to SDF
let rotated_p = rotate_euler(p, vec3f(time, time * 0.5, 0.0));
let dist = sdf_box(rotated_p, vec3f(1.0));
```

### Adding Lighting

```wgsl
let normal = get_normal_at(hit_pos);
let light_dir = normalize(vec3f(1.0, 1.0, -1.0));
let diffuse = lighting_diffuse(normal, light_dir);
let specular = lighting_blinn_phong(normal, light_dir, view_dir, 32.0);
let color = base_color * (diffuse + specular * 0.5);
```

### Repeating Objects

```wgsl
// Infinite grid of spheres
let repeated_p = sdf_repeat(p, vec3f(2.0, 2.0, 2.0));
return sdf_sphere(repeated_p, 0.5);
```

### Combining Operations

```wgsl
// Subtract sphere from box (carve out)
let box = sdf_box(p, vec3f(1.0));
let sphere = sdf_sphere(p, 0.8);
return sdf_subtract(box, sphere);
```

## Technical Details

### Ray Marching Algorithm

The demo uses **sphere tracing** (ray marching with SDFs):
1. Start at camera position
2. Query scene distance at current point
3. March forward by that distance (safe step)
4. Repeat until hitting surface (dist < threshold) or max distance

Benefits:
- Can render complex shapes with simple math
- Smooth blending and CSG operations
- Perfect for procedural geometry
- No polygons or vertices needed

### Signed Distance Functions (SDF)

SDFs return the distance to the nearest surface:
- **Negative** inside the shape
- **Zero** on the surface
- **Positive** outside the shape

This property enables:
- Efficient ray marching
- Smooth blending (smooth union/subtract)
- Easy shape combinations (CSG)
- Accurate normal calculation via gradient

### Performance Considerations

- **Max steps**: 100 (adjustable via MAX_STEPS)
- **Surface threshold**: 0.01 (SURF_DIST)
- **Max distance**: 100.0 (MAX_DIST)
- Runs entirely on GPU
- Single fragment shader per pixel

### Lighting Model

Uses a combination of:
- **Diffuse**: Lambertian shading
- **Specular**: Blinn-Phong highlights
- **Rim**: Edge lighting for depth
- **Ambient**: Minimum base lighting
- **Fog**: Exponential distance fog

## Advanced Techniques

### Soft Shadows

```wgsl
let shadow = lighting_soft_shadow(hit_pos, light_dir, 0.1, 10.0, 8.0);
color *= shadow;
```

### Ambient Occlusion

```wgsl
let ao = lighting_ao(hit_pos, normal, scene_dist);
color *= ao;
```

### Procedural Textures

```wgsl
let noise = noise_3d(hit_pos * 5.0);
let pattern = fbm(hit_pos, 4);
color *= mix(vec3f(0.5), vec3f(1.0), pattern);
```

### Camera Animation

```wgsl
let time = @engine.time;
let angle = time * 0.5;
let camera_pos = vec3f(
    sin(angle) * 5.0,
    2.0 + sin(time) * 0.5,
    cos(angle) * 5.0
);
```

## Controls

This demo is non-interactive (automated camera). Future versions could add:
- Arrow keys to move camera
- Mouse look (if supported)
- Time control to pause/speed up

## Resources

- [Inigo Quilez - SDF Functions](https://iquilezles.org/articles/distfunctions/)
- [Ray Marching Explained](https://www.michaelwalczyk.com/blog-ray-marching.html)
- [Shadertoy Examples](https://www.shadertoy.com/)

## Credits

Inspired by the demoscene and shader art community, particularly the work of Inigo Quilez on signed distance functions and ray marching techniques.
