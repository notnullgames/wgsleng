@set_title("Chromakey + OSC")
@set_size(1280, 720)

// Composite a chromakey video over your camera, with OSC-driven effects.
//
// OSC uniforms (from patch.pd in Pure Data):
//   /u/bass       low-freq energy  (0-1) → tinted glow ring on background
//   /u/mid        mid-freq energy  (0-1) → video saturation boost
//   /u/high       high-freq energy (0-1) → sparkle noise
//   /u/beat       beat pulse       (0-1, decays to 0) → edge halo + flash
//   /u/hue        base tint hue    (0-1, manual slider)
//   /u/note       note value       (0-1) → rotates tint hue each step
//
// Run (native, camera + OSC):
//   cargo run --features camera -- examples/video --osc-port 9000
//
// Run (native, no camera — background shows black):
//   cargo run -- examples/video --osc-port 9000
//
// Then open examples/video/patch.pd in Pure Data / plugdata.

@compute @workgroup_size(1)
fn update() {}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

// Returns 1.0 where the pixel should be kept, 0.0 where it is green-screen.
fn chroma_alpha(c: vec3f) -> f32 {
    let greenness = c.g - max(c.r, c.b);
    return 1.0 - smoothstep(0.20, 0.45, greenness);
}

// Remove green colour spill from semi-transparent edge pixels.
fn despill(c: vec3f) -> vec3f {
    var out = c;
    if out.g > max(out.r, out.b) {
        out.g = max(out.r, out.b);
    }
    return out;
}

fn hsv2rgb(h: f32, s: f32, v: f32) -> vec3f {
    let h6 = fract(h) * 6.0;
    let c  = v * s;
    let x  = c * (1.0 - abs(h6 % 2.0 - 1.0));
    let m  = v - c;
    let sector = u32(h6) % 6u;
    var rgb = vec3f(m);
    if      (sector == 0u) { rgb = vec3f(c + m, x + m,     m); }
    else if (sector == 1u) { rgb = vec3f(x + m, c + m,     m); }
    else if (sector == 2u) { rgb = vec3f(    m, c + m, x + m); }
    else if (sector == 3u) { rgb = vec3f(    m, x + m, c + m); }
    else if (sector == 4u) { rgb = vec3f(x + m,     m, c + m); }
    else                   { rgb = vec3f(c + m,     m, x + m); }
    return rgb;
}

fn hash21(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let res = vec2f(@engine.screen_width, @engine.screen_height);
    let uv  = coord.xy / res;
    let t   = @engine.time;

    let bass  = clamp(@osc("bass"),  0.0, 1.0);
    let mid   = clamp(@osc("mid"),   0.0, 1.0);
    let high  = clamp(@osc("high"),  0.0, 1.0);
    let beat  = clamp(@osc("beat"),  0.0, 1.0);
    let hue   = @osc("hue");
    let note  = @osc("note");

    // Each sequencer step shifts tint hue by note value (0, 0.25, 0.5, 0.75)
    let tint_hue = fract(hue + note);

    // Camera background
    let cam = textureSample(@camera(0), @engine.sampler, uv);

    // Video foreground
    let vid = textureSample(@video("britney.mp4"), @engine.sampler, uv);
    let a   = chroma_alpha(vid.rgb);

    // Background: camera + bass-driven glow ring in tint color
    var bg    = cam.rgb;
    let p     = uv - vec2f(0.5, 0.5);
    let r     = length(p);
    let ring  = exp(-pow(abs(r - (0.15 + bass * 0.35)) * 14.0, 1.5)) * bass;
    bg += hsv2rgb(tint_hue, 1.0, ring * 0.8);

    // Video: boost saturation with mid energy
    var fg_rgb = despill(vid.rgb);
    let luma   = dot(fg_rgb, vec3f(0.299, 0.587, 0.114));
    fg_rgb     = mix(vec3f(luma), fg_rgb, 1.0 + mid * 0.7);

    // Composite video over background
    var color = mix(bg, fg_rgb, a);

    // Beat: colored halo glows at the subject edges on each hit
    let edge = a * (1.0 - a) * 4.0;
    color += hsv2rgb(tint_hue, 1.0, edge * beat * 1.2);

    // Beat: brief screen-wide brightness flash
    color *= 1.0 + beat * 0.35;

    // High: sparkle noise in complementary color
    let grid  = floor(uv * 80.0);
    let spark = step(1.0 - high * 0.5, hash21(grid + floor(t * 15.0))) * high;
    color += hsv2rgb(tint_hue + 0.5, 0.5, spark * 0.6);

    return vec4f(clamp(color, vec3f(0.0), vec3f(1.0)), 1.0);
}
