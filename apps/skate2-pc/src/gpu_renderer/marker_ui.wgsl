struct In { @location(0) clip: vec4<f32>, @location(1) uv: vec2<f32>, @location(2) colour: vec4<f32> };
struct Out { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) colour: vec4<f32> };
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
@vertex fn vs(v:In)->Out { var o:Out; o.clip=v.clip; o.uv=v.uv; o.colour=v.colour; return o; }
@fragment fn fs(v:Out)->@location(0) vec4<f32> { return textureSample(image,image_sampler,v.uv)*v.colour; }
// Render3D8262a550's CSrcSrcAlpha_CDestInvSrcAlpha_Add (AddDefaultBlendStates 0x82e181a0) alpha
// tests GREATEREQUAL 1: an 8-bit alpha of 0 neither colours nor writes depth.
@fragment fn fs_world(v:Out)->@location(0) vec4<f32> {
    let c=textureSample(image,image_sampler,v.uv)*v.colour;
    if (c.a*255.0<0.5) { discard; }
    return c;
}

@group(1) @binding(0) var<uniform> eye_clip:mat4x4<f32>;
@vertex fn vs_world(v:In)->Out { var o:Out; o.clip=v.clip; o.uv=v.uv; o.colour=v.colour; o.clip=eye_clip*o.clip; return o; }
