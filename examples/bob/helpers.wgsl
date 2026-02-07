// Helper functions that can be imported
// Usage: @import("helpers.wgsl")

// Clamp a 2D position to screen bounds with padding
fn clamp_to_screen(pos: vec2f, padding: f32, screen: vec2f) -> vec2f {
    return vec2f(
        clamp(pos.x, padding, screen.x - padding),
        clamp(pos.y, padding, screen.y - padding)
    );
}

// Check if position is at screen edge with padding
fn is_at_edge(pos: vec2f, padding: f32, screen: vec2f) -> bool {
    return pos.x < padding || pos.x > screen.x - padding ||
           pos.y < padding || pos.y > screen.y - padding;
}
