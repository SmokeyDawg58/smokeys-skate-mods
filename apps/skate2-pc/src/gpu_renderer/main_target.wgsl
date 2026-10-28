// The scene's central 16:9 view, box-filtered into PostFX's main render target.
struct Crop {
    // The scene UV the target spans (xy) and its origin (zw).
    window: vec4<f32>,
    // A quarter of one target texel in scene UV.
    quarter: vec4<f32>,
}
@group(0) @binding(0) var<uniform> crop: Crop;
@group(0) @binding(1) var scene: texture_2d<f32>;
@group(0) @binding(2) var clamp_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
@vertex fn vs(@builtin(vertex_index) index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var output: VertexOutput;
    output.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    output.uv = uv * crop.window.xy + crop.window.zw;
    return output;
}
@fragment fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
    let q = crop.quarter.xy;
    let sum = textureSampleLevel(scene, clamp_sampler, input.uv - q, 0.0)
        + textureSampleLevel(scene, clamp_sampler, input.uv + vec2<f32>(q.x, -q.y), 0.0)
        + textureSampleLevel(scene, clamp_sampler, input.uv + vec2<f32>(-q.x, q.y), 0.0)
        + textureSampleLevel(scene, clamp_sampler, input.uv + q, 0.0);
    return sum * 0.25;
}
