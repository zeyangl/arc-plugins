struct Frame { size: vec2f, seconds: f32, padding: f32 };
@group(0) @binding(0) var<uniform> frame: Frame;
struct Vertex { @builtin(position) position: vec4f, @location(0) uv: vec2f };

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Vertex {
    let positions = array<vec2f, 3>(vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0));
    let p = positions[index];
    return Vertex(vec4f(p, 0.0, 1.0), vec2f(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5));
}

@fragment
fn fragment(input: Vertex) -> @location(0) vec4f {
    let wave = 0.5 + 0.5 * sin(input.uv.x * 5.0 + input.uv.y * 3.0 + frame.seconds * 0.3);
    return vec4f(mix(vec3f(0.035, 0.04, 0.06), vec3f(0.07, 0.11, 0.15), wave), 1.0);
}
