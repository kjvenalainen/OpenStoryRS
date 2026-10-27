struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) tint: vec3<f32>,
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
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(sprite_texture, sprite_sampler, input.uv);
    return vec4<f32>(color.rgb * input.tint, color.a * input.opacity);
}
