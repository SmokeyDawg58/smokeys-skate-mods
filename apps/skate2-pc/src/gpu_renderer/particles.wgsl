// render.big `data\render\shaders\final\lionREC_RenderVS.vpo` and `lionREC_RenderPS.fpo`, as
// their shipped microcode computes: the position through `g_matVP`, the colour and UV passed on;
// the texture times the colour. The quads arrive relative to the camera.

struct Camera {
    clip_from_camera: mat4x4<f32>,
};

@group(2) @binding(0) var<uniform> eye_clip: mat4x4<f32>;
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var particle_texture: texture_2d<f32>;
@group(1) @binding(1) var particle_sampler: sampler;

struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>, @location(2) uv: vec2<f32>) -> Varyings {
    var o: Varyings;
    o.position = eye_clip * camera.clip_from_camera * vec4<f32>(position, 1.0);
    o.color = color;
    o.uv = uv;
    return o;
}

// ALPHATESTENABLE with ALPHAREF 1 and GREATEREQUAL: an alpha below 1/255 draws nothing.
@fragment
fn fs(v: Varyings) -> @location(0) vec4<f32> {
    let colour = textureSample(particle_texture, particle_sampler, v.uv) * v.color;
    if colour.a < 1.0 / 255.0 {
        discard;
    }
    return colour;
}
