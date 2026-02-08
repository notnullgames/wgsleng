@set_title("Stanford Bunny - 3D Model")
@set_size(800, 600)

@model("bunny.obj")

// Transforms buffer that vertex shader can access
struct Transforms {
    rotation: f32,
    _padding: vec3f, // Align to 16 bytes
}
@group(3) @binding(0) var<uniform> transforms: Transforms;

// Game state stores rotation to write to transforms
struct GameState {
    rotation: f32,
}

@compute @workgroup_size(1)
fn update() {
    var speed = 0.5; // Default auto-rotate

    if (@engine.buttons[BTN_LEFT] == 1) {
        speed = -2.0;
    } else if (@engine.buttons[BTN_RIGHT] == 1) {
        speed = 2.0;
    }

    game_state.rotation = game_state.rotation + speed * @engine.delta_time;
    // Note: We'll copy game_state.rotation to the transforms buffer on CPU side each frame
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) world_pos: vec3f,
    @location(1) normal: vec3f,
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

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    let pos = @model("bunny.obj").positions[idx];
    let normal = @model("bunny.obj").normals[idx];

    // Simple rotation
    let model_matrix = mat4_rotate_y(0.5);
    let world_pos = model_matrix * vec4f(pos, 1.0);

    // Camera
    let camera_pos = vec3f(0.0, 0.15, 0.5);
    let view_matrix = mat4_look_at(camera_pos, vec3f(0.0, 0.1, 0.0), vec3f(0.0, 1.0, 0.0));

    // Projection
    let aspect = 800.0 / 600.0;
    let projection_matrix = mat4_perspective(1.2, aspect, 0.01, 10.0);

    let clip_pos = projection_matrix * view_matrix * world_pos;
    let world_normal = normalize((model_matrix * vec4f(normal, 0.0)).xyz);

    return VertexOutput(clip_pos, world_pos.xyz, world_normal);
}

@fragment
fn fs_render(in: VertexOutput) -> @location(0) vec4f {
    // Simple directional light
    let light_dir = normalize(vec3f(1.0, 1.0, -1.0));
    let diffuse = max(dot(in.normal, light_dir), 0.0);

    // Ambient
    let ambient = 0.3;

    // Base color
    let base_color = vec3f(0.9, 0.9, 0.9);

    let lit_color = base_color * (ambient + diffuse * 0.7);

    return vec4f(lit_color, 1.0);
}
