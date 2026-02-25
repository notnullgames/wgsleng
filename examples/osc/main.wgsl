@set_title("OSC Visualizer")
@set_size(800, 450)

// OSC uniforms — sent from Pure Data via /u/<name>:
//   /u/amplitude  overall mic loudness (0-1)
//   /u/bass       ~80 Hz band energy   (0-1)
//   /u/mid        ~500 Hz band energy  (0-1)
//   /u/high       ~3 kHz band energy   (0-1)
//   /u/presence   ~10 kHz band energy  (0-1)
//   /u/hue        base color hue       (0-1, manual slider)
//   /u/speed      animation speed      (0-2, manual slider)
//
// Run with:  wgsleng examples/osc --osc-port 9000
//            (add --hot-reload to live-edit this file too)

@compute @workgroup_size(1)
fn update() {}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4f(x, -y, 0.0, 1.0);
}

// HSV → linear RGB
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

// Simple hash for sparkle noise
fn hash21(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let res = vec2f(@engine.screen_width, @engine.screen_height);
    let uv  = coord.xy / res;              // 0..1
    let p   = uv - vec2f(0.5, 0.45);      // centered
    let t   = @engine.time;

    // Read OSC-controlled uniforms
    let amplitude = clamp(@osc("amplitude"), 0.0, 1.0);
    let bass      = clamp(@osc("bass"),      0.0, 1.0);
    let mid       = clamp(@osc("mid"),       0.0, 1.0);
    let high      = clamp(@osc("high"),      0.0, 1.0);
    let presence  = clamp(@osc("presence"),  0.0, 1.0);
    let hue       = @osc("hue");
    let speed     = @osc("speed");

    // Animated base hue — manual control + slow drift
    let h = fract(hue + t * speed * 0.04);

    // ----------------------------------------------------------------
    // Background: dark base, brightens with amplitude
    // ----------------------------------------------------------------
    let vignette = 1.0 - dot(p * 1.4, p * 1.4);
    var color = hsv2rgb(h, 0.7, (0.06 + amplitude * 0.08) * vignette);

    // ----------------------------------------------------------------
    // Bass → pulsing concentric ring
    // ----------------------------------------------------------------
    let r = length(p);
    let ring_r = 0.1 + bass * 0.28;
    let ring   = exp(-pow(abs(r - ring_r) * 18.0, 1.3)) * bass;
    color += hsv2rgb(h + 0.05, 1.0, ring * 0.9);

    // Soft glow bloom at center from amplitude
    let bloom = exp(-r * (5.0 - amplitude * 3.0)) * amplitude * 0.4;
    color += hsv2rgb(h, 0.5, bloom);

    // ----------------------------------------------------------------
    // Mid → rippling horizontal scan waves
    // ----------------------------------------------------------------
    let wave_phase = uv.y * 42.0 + t * (2.0 + speed) + bass * 8.0;
    let wave       = sin(wave_phase) * 0.5 + 0.5;
    let wave_mask  = smoothstep(0.6, 0.9, wave) * mid;
    color += hsv2rgb(h + 0.3, 0.95, wave_mask * 0.25);

    // ----------------------------------------------------------------
    // High → fine noise sparkles that appear with transients
    // ----------------------------------------------------------------
    let grid      = floor(uv * 72.0);
    let spark_val = hash21(grid + floor(t * 12.0));   // flicker with time
    let spark     = step(1.0 - high * 0.45, spark_val) * high;
    color += hsv2rgb(h + 0.55, 0.4, spark * 0.7);

    // ----------------------------------------------------------------
    // Spectrum bars — bottom 22% of screen, 4 bins
    // ----------------------------------------------------------------
    let bar_top = 0.78;

    // Divider line
    let divider = smoothstep(0.004, 0.001, abs(uv.y - bar_top));
    color = mix(color, vec3f(0.9), divider * 0.6);

    if (uv.y >= bar_top) {
        let bin      = min(u32(uv.x * 4.0), 3u);
        let spectrum = array<f32, 4>(bass, mid, high, presence);
        let bin_val  = spectrum[bin];

        // bar_y: 0 = top of bar section, 1 = bottom of screen
        let bar_y  = (uv.y - bar_top) / (1.0 - bar_top);
        let filled = bar_y >= (1.0 - bin_val);

        // Glowing edge at the top of each bar
        let edge_glow = exp(-abs(bar_y - (1.0 - bin_val)) * 28.0) * bin_val;

        let bar_hue = h + f32(bin) * 0.22;
        let bar_col = hsv2rgb(bar_hue, 1.0, select(edge_glow * 0.9, 1.0, filled));
        color = mix(color, bar_col, select(edge_glow * 0.6, 1.0, filled));

        // Bin label brightness stripe at very bottom
        let label_band = smoothstep(0.02, 0.0, 1.0 - bar_y) * 0.15;
        color += hsv2rgb(bar_hue, 0.3, label_band);
    }

    return vec4f(clamp(color, vec3f(0.0), vec3f(1.0)), 1.0);
}
