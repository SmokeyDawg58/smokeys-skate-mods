// baseterrain/decalterrain final VS/PS. Last texture is specular2 for base,
// decal for decalterrain; that program samples i_specular at both layer UVs.
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_macrooverlay: texture_2d<f32>;
@group(1) @binding(4) var i_lightmap: texture_2d<f32>;
@group(1) @binding(5) var i_chromaticity: texture_2d<f32>;
@group(1) @binding(6) var i_diffuse2: texture_2d<f32>;
@group(1) @binding(7) var i_normal2: texture_2d<f32>;
@group(1) @binding(8) var i_last: texture_2d<f32>;
@group(1) @binding(9) var wrap_sampler: sampler;
@group(1) @binding(10) var clamp_sampler: sampler;

@vertex fn terrainVS(
    @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(7) lightmap_dot: vec4<f32>,
    @location(8) uv_scales: vec4<f32>, @location(9) params1: vec4<f32>,
    @location(10) overlay_opacity: vec4<f32>, @location(11) params0: vec4<f32>,
    @location(12) lm_norm: vec4<f32>, @location(13) tangent: vec3<f32>,
    @location(14) decal_uv: vec2<f32>
) -> PsIn {
    var output=environment_vs(position,uv,lightmap_dot,uv_scales,vec4<f32>(0.),overlay_opacity,params0,lm_norm,tangent);
    output.data_constants.w=uv_scales.y;
    output.material1=params1;
    output.decal=decal_uv;
    return output;
}

fn terrain_tone(input: PsIn, has_decal: bool) -> vec4<f32> {
    let uv2=input.uv.xy*input.data_constants.w;
    let diffuse0=textureSample(i_diffuse,wrap_sampler,input.uv.xy).rgb;
    let diffuse1=textureSample(i_diffuse2,wrap_sampler,uv2).rgb;
    let normal0=textureSample(i_normal,wrap_sampler,input.uv.xy).rgb*2.0-vec3<f32>(1.);
    let normal1=textureSample(i_normal2,wrap_sampler,uv2).rgb*2.0-vec3<f32>(1.);
    let mask0=textureSample(i_specular,wrap_sampler,input.uv.xy).rgb;
    var mask1: vec3<f32>;
    if has_decal { mask1=textureSample(i_specular,wrap_sampler,uv2).rgb; }
    else { mask1=textureSample(i_last,wrap_sampler,uv2).rgb; }
    let overlay_sample=textureSample(i_macrooverlay,wrap_sampler,input.uv.xy*input.data_constants.x).rgb;
    let overlay=clamp((overlay_sample-vec3<f32>(0.5))*input.data_constants.z+vec3<f32>(0.5),vec3<f32>(0.),vec3<f32>(1.));
    let normal_blend=1.0-abs(pow(abs(input.normal.y),input.material1.x)-input.material1.y);
    let normal=mix(normal0,normal1,normal_blend);
    let world_normal=normalize(normal.x*input.tangent+normal.y*input.binormal+normal.z*input.normal);
    let surface_blend=1.0-abs(pow(abs(world_normal.y),input.material1.z)-input.material1.w);
    var diffuse=2.0*mix(diffuse0*diffuse0,diffuse1*diffuse1,surface_blend)*overlay;
    if has_decal {
        let decal=textureSample(i_last,clamp_sampler,input.decal);
        diffuse=mix(diffuse,decal.rgb*decal.rgb,decal.a);
    }
    let masks=mix(mask0,mask1,surface_blend);
    let light=globals.light_direction.xyz;
    let csm=csm_shadow_map(vec4<f32>(input.world,1.),vec3<f32>(-0.005,10.,25.));
    if SHADOW_RECEIVER_VIEW!=0u {return shadow_receiver_view(csm,dot(world_normal,light));}
    let lightmap=shadowed_lightmap(
        textureSample(i_lightmap,clamp_sampler,input.uv.zw),
        textureSample(i_chromaticity,clamp_sampler,input.uv.zw),input.lightmap_channel,
        vec4<f32>(4.5,4.5,4.5,1.5),vec4<f32>(0.07,0.09,0.1,0.04),50.,
        select(1.,csm,dot(world_normal,light)>0.));
    let tangent_space_light=normalize(vec3<f32>(dot(input.tangent,light),dot(input.binormal,light),dot(input.normal,light)));
    let kd=tangent_light(normal,vec3<f32>(0.64,0.64,0.420465),tangent_space_light);
    let view=normalize(globals.view_position.xyz-input.world);
    let specular=calculated_specular_term(light,world_normal,view,masks,lightmap.a);
    let colour=vec4<f32>(lightmap.rgb*kd*diffuse+specular,1.);
    let tone=fog_bloom_tone(colour*input.material.y,input.fog,globals.envattributes2.x);
    return vec4<f32>(tone.rgb,1.);
}
@fragment fn baseTerrainPS(input: PsIn) -> @location(0) vec4<f32> { return terrain_tone(input,false); }
@fragment fn decalTerrainPS(input: PsIn) -> @location(0) vec4<f32> { return terrain_tone(input,true); }
