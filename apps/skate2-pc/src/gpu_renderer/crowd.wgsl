// render.big `data\render\shaders\final\crowdimposter_defaultVS.vpo` and `_defaultPS.fpo`, as
// their shipped microcode computes. The quad goes through `i_matModelTarget` and
// `i_matModelView` (here one matrix, relative to the camera) and `g_matVP`; its UV through
// `i_matTexture`; the fog factor from its distance (`ComputeFogFactor`). The pixel colorizes the
// atlas texel by its alpha (`(1 - a) * (r * i_colorize_red + b * i_colorize_blue) + a * rgb`),
// fogs and tones it (`PS_Fog_Bloom_Tone`, exposure 1) and takes `i_stencil`'s blue as alpha.

struct Camera {
    clip_from_camera: mat4x4<f32>,
    fog_k1: vec4<f32>,
    fog_colour: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var atlas: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;
@group(1) @binding(2) var stencil: texture_2d<f32>;
@group(1) @binding(3) var stencil_sampler: sampler;
@group(2) @binding(0) var<uniform> eye_clip: mat4x4<f32>;

struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fog: vec4<f32>,
    @location(2) red: vec4<f32>,
    @location(3) blue: vec4<f32>,
};

@vertex
fn vs_imposter(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) right: vec4<f32>,
    @location(3) up: vec4<f32>,
    @location(4) forward: vec4<f32>,
    @location(5) origin: vec4<f32>,
    @location(6) texture_u: vec4<f32>,
    @location(7) texture_v: vec4<f32>,
    @location(8) red: vec4<f32>,
    @location(9) blue: vec4<f32>,
) -> Varyings {
    let relative = position.x * right.xyz + position.y * up.xyz + position.z * forward.xyz + origin.xyz;
    let amount = pow(clamp(length(relative) * camera.fog_k1.x + camera.fog_k1.y, 0.0, 1.0), camera.fog_k1.z);
    var o: Varyings;
    o.position = eye_clip * camera.clip_from_camera * vec4<f32>(relative, 1.0);
    o.uv = vec2<f32>(dot(vec3<f32>(uv, 1.0), texture_u.xyz), dot(vec3<f32>(uv, 1.0), texture_v.xyz));
    o.fog = camera.fog_colour * amount + vec4<f32>(0.0, 0.0, 0.0, 1.0);
    o.red = red;
    o.blue = blue;
    return o;
}

// ALPHATESTENABLE with ALPHAREF 30 and GREATEREQUAL.
@fragment
fn fs_imposter(v: Varyings) -> @location(0) vec4<f32> {
    let alpha = textureSample(stencil, stencil_sampler, v.uv).r;
    let diffuse = textureSample(atlas, atlas_sampler, v.uv);
    if alpha < 30.0 / 255.0 {
        discard;
    }
    let colorized = diffuse.z * v.blue.xyz + diffuse.x * v.red.xyz;
    let colour = (1.0 - diffuse.w) * colorized + diffuse.w * diffuse.xyz;
    let x = colour * v.fog.w + v.fog.xyz;
    let low = clamp(vec3<f32>(1.0) - x, vec3<f32>(0.0), vec3<f32>(1.0));
    let mapped = (max(x * 0.25 + vec3<f32>(0.75), vec3<f32>(1.0)) - low * low) * 0.5;
    return vec4<f32>(sqrt(abs(mapped)), alpha);
}

