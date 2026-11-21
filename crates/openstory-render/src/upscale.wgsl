struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

struct UpscaleParameters {
    // Window-space origin and logical-to-physical scale of the fitted game canvas.
    viewport: vec4<f32>,
}

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> parameters: UpscaleParameters;

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

fn catmull_rom_weights(t: f32) -> vec4<f32> {
    let t2 = t * t;
    let t3 = t2 * t;
    return vec4<f32>(
        -0.5 * t + t2 - 0.5 * t3,
        1.0 - 2.5 * t2 + 1.5 * t3,
        0.5 * t + 2.0 * t2 - 1.5 * t3,
        -0.5 * t2 + 0.5 * t3,
    );
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let canvas_size = vec2<f32>(textureDimensions(scene_texture));
    let logical_position = (input.position.xy - parameters.viewport.xy) / parameters.viewport.z;
    let uv = logical_position / canvas_size;

    // Linear filtering remains the best choice when reducing the logical canvas.
    if parameters.viewport.z < 1.0 {
        return textureSampleLevel(scene_texture, scene_sampler, uv, 0.0);
    }

    // Reconstruct the complete composited frame with a sharper cubic kernel when
    // enlarging it. Pixel-center coordinates keep 1:1 rendering unchanged.
    let texel_position = logical_position - vec2<f32>(0.5);
    let base = vec2<i32>(floor(texel_position));
    let fraction = fract(texel_position);
    let weight_x = catmull_rom_weights(fraction.x);
    let weight_y = catmull_rom_weights(fraction.y);
    let canvas_size_i = vec2<i32>(textureDimensions(scene_texture));
    var color = vec4<f32>(0.0);

    for (var y = 0u; y < 4u; y += 1u) {
        for (var x = 0u; x < 4u; x += 1u) {
            let offset = vec2<i32>(i32(x), i32(y)) - vec2<i32>(1);
            let coordinate = clamp(base + offset, vec2<i32>(0), canvas_size_i - vec2<i32>(1));
            color += textureLoad(scene_texture, coordinate, 0) * weight_x[x] * weight_y[y];
        }
    }

    // Catmull-Rom can overshoot around high-contrast text edges.
    return clamp(color, vec4<f32>(0.0), vec4<f32>(1.0));
}
