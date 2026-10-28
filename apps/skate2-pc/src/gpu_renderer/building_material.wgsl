// building_ceiling_worldspace final FPO: window alpha reveals the authored
// ceiling strip projected onto world XZ; floor dimensions are material channels.
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_diffuse2: texture_2d<f32>;
@group(1) @binding(4) var i_environment: texture_cube<f32>;
@group(1) @binding(5) var i_lightmap: texture_2d<f32>;
@group(1) @binding(6) var i_chromaticity: texture_2d<f32>;
@group(1) @binding(7) var wrap_sampler: sampler;
@group(1) @binding(8) var clamp_sampler: sampler;
@group(1) @binding(9) var cube_sampler: sampler;

@fragment fn buildingPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse=textureSample(i_diffuse,wrap_sampler,input.uv.xy);
    let masks=textureSample(i_specular,wrap_sampler,input.uv.xy).rgb;
    let normal=textureSample(i_normal,wrap_sampler,input.uv.xy).rgb*2.0-vec3<f32>(1.);
    let view=normalize(normalize(globals.view_position.xyz-input.world));
    let world_normal=normalize(normal.x*input.tangent+normal.y*input.binormal+normal.z*input.normal);
    let light=globals.light_direction.xyz;
    let csm=csm_shadow_map(vec4<f32>(input.world,1.),vec3<f32>(-0.005,10.,25.));
    let lightmap=shadowed_lightmap(
        textureSample(i_lightmap,clamp_sampler,input.uv.zw),textureSample(i_chromaticity,clamp_sampler,input.uv.zw),
        input.lightmap_channel,vec4<f32>(4.5,4.5,4.5,1.5),vec4<f32>(0.07,0.09,0.1,0.04),50.,
        select(1.,csm,dot(world_normal,light)>0.));
    let tangent_light_direction=normalize(vec3<f32>(dot(input.tangent,light),dot(input.binormal,light),dot(input.normal,light)));
    let kd=tangent_light(normal,vec3<f32>(0.64,0.64,0.420465),tangent_light_direction);
    let horizontal_length=length(view.xz);
    let floor_fraction=fract(input.uv.y*input.data_constants.y);
    let distance=(horizontal_length*floor_fraction*input.data_constants.x)/view.y;
    let strip_uv=(input.world.xz+normalize(view.xz)*distance)*0.3;
    let strip=textureSample(i_diffuse2,wrap_sampler,strip_uv).rgb;
    let colour=diffuse.rgb*diffuse.rgb+strip*strip*(1.0-diffuse.a);
    let specular=specular_term(light,world_normal,view,masks,lightmap.a);
    let tone=fog_bloom_tone(vec4<f32>(lightmap.rgb*kd*colour+specular,1.)*input.material.y,input.fog,globals.envattributes2.x);
    return vec4<f32>(tone.rgb,0.);
}
