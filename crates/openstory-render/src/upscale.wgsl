struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

struct UpscaleParameters {
    // Window origin, supersampled-to-window scale, and integer scene scale.
    viewport: vec4<f32>,
}

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> parameters: UpscaleParameters;

const PI: f32 = 3.14159265359;

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return output;
}

fn lanczos2(x: f32) -> f32 {
    if abs(x) < 1e-5 { return 1.0; }
    if abs(x) >= 2.0 { return 0.0; }
    let pi_x = PI * x;
    return (sin(pi_x) / pi_x) * (sin(pi_x * 0.5) / (pi_x * 0.5));
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let scale = parameters.viewport.z;
    let source_position = (input.position.xy - parameters.viewport.xy) / scale - vec2<f32>(0.5);
    // The backing texture holds a 3x scene; at 2x only its top-left area is used.
    let source_size = vec2<i32>(vec2<f32>(textureDimensions(scene_texture)) * (parameters.viewport.w / 3.0));
    let max_coordinate = source_size - vec2<i32>(1);
    // Widen the kernel when reducing so all contributing texels are low-pass filtered.
    // Bound the work for exceptionally tiny windows.
    let filter_scale = clamp(scale, 0.125, 1.0);
    let radius = 2.0 / filter_scale;
    let first = vec2<i32>(ceil(source_position - vec2<f32>(radius)));
    let last = vec2<i32>(floor(source_position + vec2<f32>(radius)));
    var color = vec4<f32>(0.0);
    var total_weight = 0.0;
    for (var y = first.y; y <= last.y; y += 1) {
        let weight_y = lanczos2((source_position.y - f32(y)) * filter_scale);
        for (var x = first.x; x <= last.x; x += 1) {
            let weight = weight_y * lanczos2((source_position.x - f32(x)) * filter_scale);
            let coordinate = clamp(vec2<i32>(x, y), vec2<i32>(0), max_coordinate);
            color += textureLoad(scene_texture, coordinate, 0) * weight;
            total_weight += weight;
        }
    }
    return clamp(color / total_weight, vec4<f32>(0.0), vec4<f32>(1.0));
}
