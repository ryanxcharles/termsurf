// Webview Overlay Shader
// Renders a webview texture as a full-viewport quad

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

// Full-screen triangle technique: 3 vertices cover the entire viewport
// More efficient than a quad (4 vertices + index buffer)
@vertex
fn vs_main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    // Triangle vertices in clip space that cover [-1,1] viewport
    // Vertex 0: (-1, -1) -> bottom-left
    // Vertex 1: (3, -1)  -> extends past right edge
    // Vertex 2: (-1, 3)  -> extends past top edge
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );

    // Texture coordinates: flip Y for correct orientation
    // Maps clip space to [0,1] texture space
    var tex_coords = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0)
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_idx], 0.0, 1.0);
    output.tex_coord = tex_coords[vertex_idx];
    return output;
}

// Webview texture and sampler
@group(0) @binding(0) var webview_texture: texture_2d<f32>;
@group(0) @binding(1) var webview_sampler: sampler;

// HSB transformation uniform (matches WezTerm's inactive_pane_hsb config)
struct HsbUniforms {
    hue: f32,
    saturation: f32,
    brightness: f32,
}
@group(1) @binding(0) var<uniform> hsb_uniforms: HsbUniforms;

// RGB to HSV conversion (from WezTerm's shader.wgsl)
fn rgb2hsv(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4<f32>(c.bg, K.wz), vec4<f32>(c.gb, K.xy), step(c.b, c.g));
    let q = mix(vec4<f32>(p.xyw, c.r), vec4<f32>(c.r, p.yzx), step(p.x, c.r));
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3<f32>(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

// HSV to RGB conversion (from WezTerm's shader.wgsl)
fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3(0.0), vec3(1.0)), c.y);
}

// Apply HSB transformation (from WezTerm's shader.wgsl)
fn apply_hsb(c: vec4<f32>, transform: vec3<f32>) -> vec4<f32> {
    let hsv = rgb2hsv(c.rgb) * transform;
    return vec4<f32>(hsv2rgb(hsv).rgb, c.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the webview texture
    // The viewport is set to pane bounds, so tex_coord [0,1] maps to the pane area
    let color = textureSample(webview_texture, webview_sampler, input.tex_coord);
    // Apply HSB transformation (matches WezTerm's inactive_pane_hsb behavior)
    let hsb_transform = vec3<f32>(
        hsb_uniforms.hue,
        hsb_uniforms.saturation,
        hsb_uniforms.brightness
    );
    return apply_hsb(color, hsb_transform);
}
