@set_title("Spinning Cube")
@set_size(800, 600)

// Game state
struct GameState {
    time: f32,
}

@compute @workgroup_size(1)
fn update() {
    @engine.state.time = @engine.time;
}

// Use fullscreen triangle (current engine limitation)
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

// Matrix helpers
fn mat4_perspective(fov: f32, aspect: f32, near: f32, far: f32) -> mat4x4<f32> {
    let f = 1.0 / tan(fov / 2.0);
    let nf = 1.0 / (near - far);
    return mat4x4<f32>(
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, (far + near) * nf, -1.0,
        0.0, 0.0, 2.0 * far * near * nf, 0.0
    );
}

fn mat4_look_at(eye: vec3f, center: vec3f, up: vec3f) -> mat4x4<f32> {
    let f = normalize(center - eye);
    let s = normalize(cross(f, up));
    let u = cross(s, f);
    return mat4x4<f32>(
        s.x, u.x, -f.x, 0.0,
        s.y, u.y, -f.y, 0.0,
        s.z, u.z, -f.z, 0.0,
        -dot(s, eye), -dot(u, eye), dot(f, eye), 1.0
    );
}

fn mat4_rotate_y(angle: f32) -> mat4x4<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return mat4x4<f32>(
        c, 0.0, -s, 0.0,
        0.0, 1.0, 0.0, 0.0,
        s, 0.0, c, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
}

fn mat4_rotate_x(angle: f32) -> mat4x4<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, c, s, 0.0,
        0.0, -s, c, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
}

// Cube face colors
fn get_face_color(face_id: i32) -> vec3f {
    if (face_id == 0) { return vec3f(1.0, 0.0, 0.0); }  // Red - Front
    if (face_id == 1) { return vec3f(0.0, 1.0, 1.0); }  // Cyan - Back
    if (face_id == 2) { return vec3f(0.0, 1.0, 0.0); }  // Green - Top
    if (face_id == 3) { return vec3f(1.0, 0.0, 1.0); }  // Magenta - Bottom
    if (face_id == 4) { return vec3f(0.0, 0.0, 1.0); }  // Blue - Right
    return vec3f(1.0, 1.0, 0.0);  // Yellow - Left
}

fn get_face_normal(face_id: i32) -> vec3f {
    if (face_id == 0) { return vec3f( 0.0,  0.0,  1.0); }  // Front
    if (face_id == 1) { return vec3f( 0.0,  0.0, -1.0); }  // Back
    if (face_id == 2) { return vec3f( 0.0,  1.0,  0.0); }  // Top
    if (face_id == 3) { return vec3f( 0.0, -1.0,  0.0); }  // Bottom
    if (face_id == 4) { return vec3f( 1.0,  0.0,  0.0); }  // Right
    return vec3f(-1.0,  0.0,  0.0);  // Left
}

// Check if point is inside a triangle (2D)
fn point_in_triangle(p: vec2f, v0: vec2f, v1: vec2f, v2: vec2f) -> bool {
    let d1 = sign((p.x - v1.x) * (v0.y - v1.y) - (v0.x - v1.x) * (p.y - v1.y));
    let d2 = sign((p.x - v2.x) * (v1.y - v2.y) - (v1.x - v2.x) * (p.y - v2.y));
    let d3 = sign((p.x - v0.x) * (v2.y - v0.y) - (v2.x - v0.x) * (p.y - v0.y));
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    return !(has_neg && has_pos);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Normalize screen coordinates
    let uv = coord.xy / vec2f(@engine.screen_width, @engine.screen_height);
    let ndc = uv * 2.0 - 1.0;

    // Background color
    var color = vec3f(0.1, 0.1, 0.15);

    // Setup camera and projection
    let time = @engine.state.time;
    let camera_pos = vec3f(0.0, 1.5, 3.0);
    let view_matrix = mat4_look_at(camera_pos, vec3f(0.0, 0.0, 0.0), vec3f(0.0, 1.0, 0.0));
    let aspect = @engine.screen_width / @engine.screen_height;
    let projection_matrix = mat4_perspective(1.0, aspect, 0.1, 100.0);

    // Model transform
    let model_matrix = mat4_rotate_y(time * 0.5) * mat4_rotate_x(time * 0.3);

    // Full transform
    let mvp = projection_matrix * view_matrix * model_matrix;

    // Define cube vertices
    let cube_verts = array<vec3f, 8>(
        vec3f(-0.5, -0.5,  0.5), vec3f( 0.5, -0.5,  0.5),
        vec3f( 0.5,  0.5,  0.5), vec3f(-0.5,  0.5,  0.5),
        vec3f(-0.5, -0.5, -0.5), vec3f( 0.5, -0.5, -0.5),
        vec3f( 0.5,  0.5, -0.5), vec3f(-0.5,  0.5, -0.5),
    );

    // Project all vertices
    var projected = array<vec2f, 8>();
    var depths = array<f32, 8>();
    for (var i = 0; i < 8; i++) {
        let clip = mvp * vec4f(cube_verts[i], 1.0);
        projected[i] = clip.xy / clip.w;
        depths[i] = clip.z / clip.w;
    }

    // Define cube faces (vertex indices)
    let faces = array<vec3i, 12>(
        vec3i(0, 1, 2), vec3i(2, 3, 0),  // Front
        vec3i(5, 4, 7), vec3i(7, 6, 5),  // Back
        vec3i(3, 2, 6), vec3i(6, 7, 3),  // Top
        vec3i(4, 5, 1), vec3i(1, 0, 4),  // Bottom
        vec3i(1, 5, 6), vec3i(6, 2, 1),  // Right
        vec3i(4, 0, 3), vec3i(3, 7, 4),  // Left
    );

    // Find closest face at this pixel
    var closest_z = 1.0;
    var hit_face = -1;

    for (var i = 0; i < 12; i++) {
        let face_id = i / 2;

        // Backface culling - check if face is pointing toward camera
        let face_normal_world = (model_matrix * vec4f(get_face_normal(face_id), 0.0)).xyz;
        let face_center = (cube_verts[faces[i].x] + cube_verts[faces[i].y] + cube_verts[faces[i].z]) / 3.0;
        let face_center_world = (model_matrix * vec4f(face_center, 1.0)).xyz;
        let view_dir = normalize(camera_pos - face_center_world);

        // Skip if facing away from camera
        if (dot(normalize(face_normal_world), view_dir) <= 0.0) {
            continue;
        }

        let v0 = projected[faces[i].x];
        let v1 = projected[faces[i].y];
        let v2 = projected[faces[i].z];

        if (point_in_triangle(ndc, v0, v1, v2)) {
            let z = (depths[faces[i].x] + depths[faces[i].y] + depths[faces[i].z]) / 3.0;
            if (z < closest_z) {
                closest_z = z;
                hit_face = face_id;
            }
        }
    }

    // Shade the hit face
    if (hit_face >= 0) {
        let face_color = get_face_color(hit_face);
        let normal = (model_matrix * vec4f(get_face_normal(hit_face), 0.0)).xyz;

        // Simple lighting
        let light_dir = normalize(vec3f(1.0, 1.0, -1.0));
        let diffuse = max(dot(normalize(normal), light_dir), 0.0);
        let ambient = 0.3;

        color = face_color * (ambient + diffuse * 0.7);
    }

    return vec4f(color, 1.0);
}
