struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

struct UpscaleParameters {
    // Window-space origin, logical-to-physical scale, and unclamped Lanczos blend.
    viewport: vec4<f32>,
}

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> parameters: UpscaleParameters;

const PI: f32 = 3.14159265359;

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return output;
}

fn lanczos2(x: f32) -> f32 {
    if abs(x) < 1e-5 {
        return 1.0;
    }
    if abs(x) >= 2.0 {
        return 0.0;
    }
    let pi_x = PI * x;
    return (sin(pi_x) / pi_x) * (sin(pi_x * 0.5) / (pi_x * 0.5));
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let canvas_size = vec2<f32>(textureDimensions(scene_texture));
    let logical_position = (input.position.xy - parameters.viewport.xy) / parameters.viewport.z;
    let uv = logical_position / canvas_size;

    // Linear filtering remains preferable when reducing the logical canvas.
    if parameters.viewport.z < 1.0 {
        return textureSampleLevel(scene_texture, scene_sampler, uv, 0.0);
    }

    // Map each destination pixel center into source texel coordinates.
    let source_position = logical_position - vec2<f32>(0.5);
    let base = vec2<i32>(floor(source_position));
    let fraction = fract(source_position);
    let canvas_size_i = vec2<i32>(textureDimensions(scene_texture));
    let max_coordinate = canvas_size_i - vec2<i32>(1);

    var color = vec4<f32>(0.0);
    var total_weight = 0.0;
    var min_color = vec4<f32>(1.0);
    var max_color = vec4<f32>(0.0);

    // Four taps per axis cover the Lanczos-2 support around this pixel.
    for (var y: i32 = -1; y <= 2; y += 1) {
        let weight_y = lanczos2(fraction.y - f32(y));
        for (var x: i32 = -1; x <= 2; x += 1) {
            let weight_x = lanczos2(fraction.x - f32(x));
            let weight = weight_x * weight_y;
            let coordinate = clamp(base + vec2<i32>(x, y), vec2<i32>(0), max_coordinate);
            let texel = textureLoad(scene_texture, coordinate, 0);

            // The nearest 2x2 source pixels bound overshoot around text edges.
            if x >= 0 && x <= 1 && y >= 0 && y <= 1 {
                min_color = min(min_color, texel);
                max_color = max(max_color, texel);
            }

            color += texel * weight;
            total_weight += weight;
        }
    }

    color /= total_weight;
    let clamped_color = clamp(color, min_color, max_color);
    let sharpness = clamp(parameters.viewport.w, 0.0, 1.0);

    // A small unconstrained contribution keeps the Lanczos result crisp while
    // the local clamp suppresses bright and dark ringing around text.
    return clamp(mix(clamped_color, color, sharpness), vec4<f32>(0.0), vec4<f32>(1.0));
}
