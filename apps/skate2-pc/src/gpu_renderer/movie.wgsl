// vp6.fx's defaultPS as compiled (vp6_defaultPS.fpo): Y from the luminance texture, Cb and Cr
// from the half-size blue and red textures, then c254 = (1.16438305, -0.391761988, -0.0625,
// -0.5) and c255 = (1.59602702, -0.812968016, 2.01723194). The background quad is black.

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) video: f32,
};

@group(0) @binding(0) var luminance: texture_2d<f32>;
@group(0) @binding(1) var blue: texture_2d<f32>;
@group(0) @binding(2) var red: texture_2d<f32>;
@group(0) @binding(3) var planes: sampler;

@vertex fn vs(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) video: f32) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = uv;
    out.video = video;
    return out;
}

@fragment fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    if in.video == 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let y = textureSample(luminance, planes, in.uv).r - 0.0625;
    let cb = textureSample(blue, planes, in.uv).r - 0.5;
    let cr = textureSample(red, planes, in.uv).r - 0.5;
    let base = y * 1.16438305;
    let green = base + cb * -0.391761988;
    return vec4<f32>(cr * 1.59602702 + base, cr * -0.812968016 + green, cb * 2.01723194 + base, 1.0);
}
