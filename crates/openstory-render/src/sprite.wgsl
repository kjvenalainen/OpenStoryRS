struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) tint: vec3<f32>,
    @location(3) @interpolate(flat) uv_bounds: vec4<f32>,
}

const QUAD_CORNERS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(1.0, 0.0),
);

@group(0) @binding(0) var sprite_texture: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) rectangle: vec4<f32>,
    @location(1) uv_rectangle: vec4<f32>,
    @location(2) opacity: f32,
    @location(3) tint: vec3<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    let corner = QUAD_CORNERS[vertex_index];
    output.position = vec4<f32>(mix(rectangle.xy, rectangle.zw, corner), 0.0, 1.0);
    output.uv = vec2<f32>(
        mix(uv_rectangle.x, uv_rectangle.z, corner.x),
        mix(uv_rectangle.y, uv_rectangle.w, corner.y),
    );
    output.opacity = opacity;
    output.tint = tint;
    output.uv_bounds = uv_rectangle;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texture_size = vec2<f32>(textureDimensions(sprite_texture));
    let texel_position = input.uv * texture_size - vec2<f32>(0.5);
    // Keep texel interiors flat and restrict bilinear blending to a narrow
    // transition at each edge. Minification falls back to ordinary bilinear.
    let pixels_per_texel = max(vec2<f32>(1.0) / max(fwidth(texel_position), vec2<f32>(1e-6)), vec2<f32>(1.0));
    let fraction = clamp((fract(texel_position) - vec2<f32>(0.5)) * pixels_per_texel + vec2<f32>(0.5), vec2<f32>(0.0), vec2<f32>(1.0));
    let sharpened = (floor(texel_position) + vec2<f32>(0.5) + fraction) / texture_size;
    // Atlased sprites must not borrow pixels from neighboring images.
    let half_texel = vec2<f32>(0.5) / texture_size;
    let lower = min(input.uv_bounds.xy, input.uv_bounds.zw) + half_texel;
    let upper = max(input.uv_bounds.xy, input.uv_bounds.zw) - half_texel;
    let uv = clamp(sharpened, min(lower, upper), max(lower, upper));
    let color = textureSample(sprite_texture, sprite_sampler, uv);
    return vec4<f32>(color.rgb * input.tint, color.a * input.opacity);
}
