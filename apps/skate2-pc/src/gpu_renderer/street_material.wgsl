@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler: sampler;
struct StreetIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fog: vec4<f32>,
    @location(2) @interpolate(flat) exposure: f32,
}
fn traffic_vs(position: vec3<f32>, uv: vec2<f32>, lookup: vec2<f32>, alternate: vec2<f32>, exposure: f32, status: vec4<f32>) -> StreetIn {
    var output: StreetIn;
    output.position=world_clip(position);
    let index=i32(floor(lookup.x*4.));
    var value=0.;
    // Native out-of-range lookup stays unlit; do not clamp to a live lane.
    if index == 0 { value=status.x; }
    if index == 1 { value=status.y; }
    if index == 2 { value=status.z; }
    if index == 3 { value=status.w; }
    output.uv=select(uv,alternate,value>1.);
    output.fog=compute_fog_factor(position);
    output.exposure=exposure;
    return output;
}
@vertex fn trafficOneVS(@location(0) position:vec3<f32>, @location(1) uv:vec2<f32>,
    @location(11) params:vec4<f32>, @location(12) alternate:vec4<f32>, @location(14) lookup:vec2<f32>) -> StreetIn {
    return traffic_vs(position,uv,lookup,alternate.xy,params.y,globals.traffic_lights[0]);
}
@vertex fn trafficTwoVS(@location(0) position:vec3<f32>, @location(1) uv:vec2<f32>,
    @location(11) params:vec4<f32>, @location(12) alternate:vec4<f32>, @location(14) lookup:vec2<f32>) -> StreetIn {
    return traffic_vs(position,uv,lookup,alternate.xy,params.y,globals.traffic_lights[1]);
}
@fragment fn trafficPS(input:StreetIn) -> @location(0) vec4<f32> {
    let diffuse=textureSample(i_diffuse,diffuse_sampler,input.uv);
    // Final FPO consumes unsquared diffuse RGB.
    return vec4<f32>(fog_bloom_tone(diffuse*input.exposure,input.fog,globals.envattributes2.x).rgb,diffuse.a);
}
@vertex fn glareVS(@location(0) position:vec3<f32>, @location(1) uv:vec2<f32>,
    @location(8) scale:vec4<f32>) -> StreetIn {
    var output:StreetIn;
    let offset=uv-vec2<f32>(0.5);
    let centre=position-vec3<f32>(offset.x,0.,offset.y);
    let point=centre+scale.x*(offset.x*globals.camera_axes[0].xyz-offset.y*globals.camera_axes[1].xyz);
    let nearer=world_clip(point+scale.x*0.5*globals.camera_axes[2].xyz);
    let clip=world_clip(point);
    output.position=vec4<f32>(clip.xy,min(clip.z,(nearer.z/nearer.w)*clip.w),clip.w);
    output.uv=uv;
    return output;
}
@fragment fn glarePS(input:StreetIn) -> @location(0) vec4<f32> {
    let diffuse=textureSample(i_diffuse,diffuse_sampler,input.uv);
    if diffuse.a < 1./255. { discard; }
    return diffuse;
}

struct SpotlightIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world: vec3<f32>,
    @location(2) normal: vec3<f32>,
}
@vertex fn spotlightVS(@location(0) position:vec3<f32>, @location(1) uv:vec2<f32>,
    @location(2) world0:vec4<f32>, @location(3) world1:vec4<f32>,
    @location(4) world2:vec4<f32>, @location(5) world3:vec4<f32>,
    @location(13) normal:vec3<f32>) -> SpotlightIn {
    let world=mat4x4<f32>(world0,world1,world2,world3);
    var output:SpotlightIn;
    let point=world*vec4<f32>(position,1.);
    output.world=point.xyz;
    output.position=world_clip(point.xyz);
    output.uv=uv;
    output.normal=(world*vec4<f32>(normal,0.)).xyz;
    return output;
}
@fragment fn spotlightPS(input:SpotlightIn) -> @location(0) vec4<f32> {
    let colour=textureSample(i_diffuse,diffuse_sampler,input.uv).rgb;
    // Final program does not normalize the transformed normal or apply fog.
    let view_angle=abs(dot(input.normal,normalize(globals.view_position.xyz-input.world)));
    return vec4<f32>(colour*colour*view_angle,0.);
}
