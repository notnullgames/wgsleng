@set_title("Ray Marched Primitives")
@set_size(800, 600)

@import("draw3d.wgsl")

// Game state
struct GameState {
    time: f32,
}

@compute @workgroup_size(1)
fn update() {
    @engine.state.time = @engine.time;
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

// Scene definition - returns distance to nearest surface
fn scene(p: vec3f) -> vec3f {
    let time = @engine.state.time;

    // Ground plane at Y = 0
    let ground = sdf_plane(p, vec3f(0.0, 1.0, 0.0), 0.0);
    var closest_dist = ground;
    var material = 0.0; // Ground material

    // Sphere on ground
    let sphere_pos = vec3f(-3.0, 0.8, 0.0);
    let sphere = sdf_sphere(p - sphere_pos, 0.8);
    if (sphere < closest_dist) {
        closest_dist = sphere;
        material = 1.0; // Red sphere
    }

    // Cube on ground with animation - ensure it stays above ground
    let cube_pos = vec3f(0.0, 1.0 + abs(sin(time * 0.5)) * 0.3, 0.0);
    let rotated_p = rotate_euler(p - cube_pos, vec3f(time * 0.5, time * 0.7, time * 0.3));
    let cube = sdf_box(rotated_p, vec3f(0.6, 0.6, 0.6));
    if (cube < closest_dist) {
        closest_dist = cube;
        material = 2.0; // Green cube
    }

    // Torus on ground - raised slightly
    let torus_pos = vec3f(3.0, 1.0, 0.0);
    let torus_p = rotate_x(p - torus_pos, time * 0.8);
    let torus = sdf_torus(torus_p, 0.6, 0.25);
    if (torus < closest_dist) {
        closest_dist = torus;
        material = 3.0; // Blue torus
    }

    // Floating capsule
    let capsule_pos = vec3f(-1.5, 1.8 + sin(time) * 0.3, 2.0);
    let capsule_a = capsule_pos + vec3f(0.0, 0.5, 0.0);
    let capsule_b = capsule_pos - vec3f(0.0, 0.5, 0.0);
    let capsule = sdf_capsule(p, capsule_a, capsule_b, 0.3);
    if (capsule < closest_dist) {
        closest_dist = capsule;
        material = 4.0; // Yellow capsule
    }

    // Octahedron on ground (bipyramid shape - two pyramids base-to-base)
    let octahedron_pos = vec3f(1.5, 0.7, 2.0);
    let octahedron = sdf_octahedron(p - octahedron_pos, 0.7);
    if (octahedron < closest_dist) {
        closest_dist = octahedron;
        material = 5.0; // Purple octahedron
    }

    return vec3f(closest_dist, material, 0.0);
}

// Calculate normal at point p
fn get_normal_at(p: vec3f) -> vec3f {
    let h = 0.001;
    return normalize(vec3f(
        scene(p + vec3f(h, 0.0, 0.0)).x - scene(p - vec3f(h, 0.0, 0.0)).x,
        scene(p + vec3f(0.0, h, 0.0)).x - scene(p - vec3f(0.0, h, 0.0)).x,
        scene(p + vec3f(0.0, 0.0, h)).x - scene(p - vec3f(0.0, 0.0, h)).x
    ));
}

// Ray march through the scene
fn march(ray_origin: vec3f, ray_dir: vec3f) -> vec3f {
    var dist = 0.0;
    var material = 0.0;

    for (var i = 0; i < MAX_STEPS; i++) {
        let p = ray_origin + ray_dir * dist;
        let result = scene(p);
        let d = result.x;
        material = result.y;

        dist += d;

        if (d < SURF_DIST || dist > MAX_DIST) {
            break;
        }
    }

    return vec3f(dist, material, 0.0);
}

// Get material color
fn get_material_color(material: f32, normal: vec3f) -> vec3f {
    if (material == 0.0) {
        // Ground - checkered pattern
        return vec3f(0.4, 0.4, 0.4);
    } else if (material == 1.0) {
        // Red sphere - vibrant red
        return vec3f(1.0, 0.1, 0.1);
    } else if (material == 2.0) {
        // Green cube - vibrant green
        return vec3f(0.1, 1.0, 0.1);
    } else if (material == 3.0) {
        // Blue torus - vibrant blue
        return vec3f(0.1, 0.3, 1.0);
    } else if (material == 4.0) {
        // Yellow capsule - vibrant yellow
        return vec3f(1.0, 1.0, 0.0);
    } else if (material == 5.0) {
        // Purple pyramid - vibrant purple
        return vec3f(0.8, 0.1, 1.0);
    }

    return vec3f(1.0, 0.0, 1.0); // Magenta for unknown
}

// Apply lighting
fn apply_lighting(p: vec3f, normal: vec3f, view_dir: vec3f, base_color: vec3f) -> vec3f {
    // Light positions
    let light1_pos = vec3f(5.0, 5.0, -5.0);
    let light2_pos = vec3f(-5.0, 3.0, 5.0);

    // Light 1 - white main light
    let light1_dir = normalize(light1_pos - p);
    let diffuse1 = lighting_diffuse(normal, light1_dir);
    let specular1 = lighting_blinn_phong(normal, light1_dir, view_dir, 32.0);
    let light1_color = vec3f(1.0, 1.0, 1.0) * (diffuse1 * 1.2 + specular1 * 0.8);

    // Light 2 - blue fill light
    let light2_dir = normalize(light2_pos - p);
    let diffuse2 = lighting_diffuse(normal, light2_dir);
    let light2_color = vec3f(0.4, 0.5, 0.8) * diffuse2 * 0.4;

    // Ambient - minimal for good contrast
    let ambient = vec3f(0.15, 0.15, 0.2);

    // Rim light
    let rim = lighting_rim(normal, view_dir, 3.0) * 0.4;

    return base_color * (light1_color + light2_color + ambient + rim);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Normalize coordinates to -1..1
    let uv = (coord.xy / vec2f(@engine.screen_width, @engine.screen_height)) * 2.0 - 1.0;
    let aspect = @engine.screen_width / @engine.screen_height;
    let uv_correct = vec2f(uv.x * aspect, -uv.y);  // Flip Y to correct orientation

    // Camera setup
    let time = @engine.state.time;
    let camera_angle = time * 0.3;
    let camera_distance = 8.0;
    let camera_pos = vec3f(
        sin(camera_angle) * camera_distance,
        3.0,
        cos(camera_angle) * camera_distance
    );
    let look_at_pos = vec3f(0.0, 0.0, 0.0);

    // Generate ray
    let ray_dir = get_ray_direction(uv_correct, camera_pos, look_at_pos, 2.0);

    // Ray march
    let result = march(camera_pos, ray_dir);
    let dist = result.x;
    let material = result.y;

    // Background gradient
    var color = mix(
        vec3f(0.5, 0.7, 1.0),  // Sky blue
        vec3f(0.2, 0.3, 0.5),  // Darker blue
        uv.y * 0.5 + 0.5
    );

    // If we hit something
    if (dist < MAX_DIST) {
        let hit_pos = camera_pos + ray_dir * dist;
        let normal = get_normal_at(hit_pos);
        let view_dir = -ray_dir;

        // Get base color from material
        let base_color = get_material_color(material, normal);

        // Add checkerboard pattern to ground
        if (material == 0.0) {
            let checker = step(0.5, fract(hit_pos.x * 0.5)) + step(0.5, fract(hit_pos.z * 0.5));
            let pattern = select(0.5, 1.0, checker == 1.0);
            color = base_color * pattern;
        } else {
            color = base_color;
        }

        // Apply lighting
        color = apply_lighting(hit_pos, normal, view_dir, color);

        // Fog - very subtle for crisp colors
        let fog_amount = 1.0 - exp(-dist * 0.01);
        let fog_color = vec3f(0.5, 0.7, 1.0);
        color = mix(color, fog_color, fog_amount * 0.5);
    }

    // Gamma correction
    color = gamma_correct(color, 2.2);

    return vec4f(color, 1.0);
}
