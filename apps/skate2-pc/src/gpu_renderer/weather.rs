//! Local adaptation of the installed Skate 3 Remaster rain/snow shaders.
//! World colour/depth and a separate vertical shelter pass leave gameplay untouched.
use super::{GpuMap,RenderResult,CameraRelativeClip};
use crate::weather::Mode;
const SHELTER_SIZE:u32=2048;
const SHELTER_HALF:f32=48.;


#[derive(Clone,Copy)]
pub(super) struct Frame {
    pub clip:[f32;16], pub camera:[f32;3], pub axes:[[f32;4];3],
    pub clock:f32,pub fog:[f32;4],pub mode:Mode,
}
impl Frame {
    pub fn shelter_clip(&self)->[f32;16] {
        let [x,y,z]=self.camera;
        // Anchor texels to the world so camera movement does not crawl the boundary.
        let sx=0.;let sz=0.;
        let step=2.*SHELTER_HALF/SHELTER_SIZE as f32;
        let x=((x+sx*y)/step).round()*step;let z=((z+sz*y)/step).round()*step;
        [1./SHELTER_HALF,0.,0.,0., sx/SHELTER_HALF,sz/SHELTER_HALF,-1./160.,0.,
            0.,1./SHELTER_HALF,0.,0., -x/SHELTER_HALF,-z/SHELTER_HALF,(y+80.)/160.,1.]
    }
    fn bytes(&self,width:u32,height:u32)->RenderResult<Vec<u8>> {
        let clip=CameraRelativeClip::new(&self.clip,self.camera).clip_from_camera;
        let inverse=super::stereoscopy::inverse(clip.map(f64::from))?.map(|v|v as f32);
        let projection_y=(clip[1]*clip[1]+clip[5]*clip[5]+clip[9]*clip[9]).sqrt();
        let [rain,snow]=self.mode.amounts();
        let mut values=Vec::from(clip);values.extend(inverse);values.extend(self.shelter_clip());
        values.extend([self.camera[0],self.camera[1],self.camera[2],1.]);
        values.extend(self.axes[0]);values.extend(self.axes[1]);
        values.extend([rain,snow,self.clock,0.]);values.extend(self.fog);
        values.extend([projection_y,width as f32,height as f32,0.]);
        if std::env::var_os("SKATE2_DIAGNOSTIC_WEATHER_SHADER").is_some() {
            static LOGGED:std::sync::OnceLock<()>=std::sync::OnceLock::new();
            LOGGED.get_or_init(||eprintln!("weather diagnostic camera={:?} fog={:?} axes={:?} clip={:?}",self.camera,self.fog,self.axes,clip));
        }
        Ok(values.into_iter().flat_map(f32::to_ne_bytes).collect())
    }
}

pub(super) struct Weather {
    pub output:wgpu::TextureView, pub texture:wgpu::Texture, uniform:wgpu::Buffer,binding:wgpu::BindGroup,
    surface:wgpu::RenderPipeline,particles:wgpu::RenderPipeline,
    shelter_target:wgpu::TextureView,shelter_camera:wgpu::Buffer,shelter_binding:wgpu::BindGroup,
    shelter_pipeline:wgpu::RenderPipeline,width:u32,height:u32,
    rain:super::rain::Rain,
}
impl Weather {
    #[expect(clippy::too_many_arguments,reason="the retained scene and depth resources")]
    pub fn new(device:&wgpu::Device,format:wgpu::TextureFormat,width:u32,height:u32,
        scene:&wgpu::TextureView,depth:&wgpu::TextureView,stencil:&wgpu::TextureView,multisampled:bool)->Self {
        let texture=device.create_texture(&wgpu::TextureDescriptor {
            label:Some("local weather scene"),size:wgpu::Extent3d{width,height,depth_or_array_layers:1},mip_level_count:1,sample_count:1,
            dimension:wgpu::TextureDimension::D2,format,usage:wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::TEXTURE_BINDING|wgpu::TextureUsages::COPY_SRC,view_formats:&[],
        });
        let output=texture.create_view(&Default::default());
        let shelter=device.create_texture(&wgpu::TextureDescriptor {
            label:Some("vertical weather shelter"),size:wgpu::Extent3d{width:SHELTER_SIZE,height:SHELTER_SIZE,depth_or_array_layers:1},mip_level_count:1,sample_count:1,
            dimension:wgpu::TextureDimension::D2,format:wgpu::TextureFormat::Depth24PlusStencil8,
            usage:wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::TEXTURE_BINDING,view_formats:&[],
        });
        let shelter_target=shelter.create_view(&Default::default());
        let shelter_depth=shelter.create_view(&wgpu::TextureViewDescriptor{aspect:wgpu::TextureAspect::DepthOnly,..Default::default()});
        let buffer=|label,size|device.create_buffer(&wgpu::BufferDescriptor{label:Some(label),size,usage:wgpu::BufferUsages::UNIFORM|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        let uniform=buffer("weather camera and clock",288);let shelter_camera=buffer("weather vertical projection",64);
        let uniform_entry=wgpu::BindGroupLayoutEntry{binding:0,visibility:wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty:wgpu::BindingType::Buffer{ty:wgpu::BufferBindingType::Uniform,has_dynamic_offset:false,min_binding_size:None},count:None};
        let texture_entry=|binding,sample_type,multisampled|wgpu::BindGroupLayoutEntry{binding,visibility:wgpu::ShaderStages::FRAGMENT,
            ty:wgpu::BindingType::Texture{sample_type,view_dimension:wgpu::TextureViewDimension::D2,multisampled},count:None};
        let layout=device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{label:Some("weather scene/depth/shelter"),entries:&[
            uniform_entry,texture_entry(1,wgpu::TextureSampleType::Float{filterable:true},false),
            texture_entry(2,wgpu::TextureSampleType::Depth,multisampled),texture_entry(3,wgpu::TextureSampleType::Uint,multisampled),
            wgpu::BindGroupLayoutEntry{binding:4,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),count:None},
            texture_entry(5,wgpu::TextureSampleType::Depth,false),
            wgpu::BindGroupLayoutEntry{binding:6,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),count:None},
        ]});
        let sampler=device.create_sampler(&wgpu::SamplerDescriptor{mag_filter:wgpu::FilterMode::Linear,min_filter:wgpu::FilterMode::Linear,..Default::default()});
        let shelter_compare=device.create_sampler(&wgpu::SamplerDescriptor{label:Some("filtered weather shelter"),
            compare:Some(wgpu::CompareFunction::LessEqual),mag_filter:wgpu::FilterMode::Linear,min_filter:wgpu::FilterMode::Linear,..Default::default()});
        let binding=device.create_bind_group(&wgpu::BindGroupDescriptor{label:Some("weather resources"),layout:&layout,entries:&[
            wgpu::BindGroupEntry{binding:0,resource:uniform.as_entire_binding()},
            wgpu::BindGroupEntry{binding:1,resource:wgpu::BindingResource::TextureView(scene)},
            wgpu::BindGroupEntry{binding:2,resource:wgpu::BindingResource::TextureView(depth)},
            wgpu::BindGroupEntry{binding:3,resource:wgpu::BindingResource::TextureView(stencil)},
            wgpu::BindGroupEntry{binding:4,resource:wgpu::BindingResource::Sampler(&sampler)},
            wgpu::BindGroupEntry{binding:5,resource:wgpu::BindingResource::TextureView(&shelter_depth)},
            wgpu::BindGroupEntry{binding:6,resource:wgpu::BindingResource::Sampler(&shelter_compare)},
        ]});
        let pipeline_layout=device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{label:Some("local weather"),bind_group_layouts:&[Some(&layout)],immediate_size:0});
        let mut source=String::from(include_str!("weather_common.wgsl"));source.push_str(include_str!("weather.wgsl"));
        // Explicit per-process shader inspection; ordinary launches use embedded sources.
        if let Some(path)=std::env::var_os("SKATE2_DIAGNOSTIC_WEATHER_SHADER") {
            source=std::fs::read_to_string(path).expect("diagnostic weather shader");
        }
        if multisampled {source=source.replace("texture_depth_2d;","texture_depth_multisampled_2d;").replace("texture_2d<u32>;","texture_multisampled_2d<u32>;");
            // Only the scene depth is multisampled; the vertical shelter is single sample.
            source=source.replace("var shelter: texture_depth_multisampled_2d;","var shelter: texture_depth_2d;");
            source=source.replace("return f32(textureLoad(stencil,pixel,0).x)/255.;",
                "var total=0.;let count=textureNumSamples(stencil);for(var i=0u;i<count;i+=1u){total+=f32(textureLoad(stencil,pixel,i32(i)).x)/255.;}return total/f32(count);");}
        let module=device.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("ported Skate 3 rain and snow"),source:wgpu::ShaderSource::Wgsl(source.into())});
        let pipeline=|vertex,fragment,blend|device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
            label:Some(fragment),layout:Some(&pipeline_layout),vertex:wgpu::VertexState{module:&module,entry_point:Some(vertex),buffers:&[],compilation_options:Default::default()},
            fragment:Some(wgpu::FragmentState{module:&module,entry_point:Some(fragment),targets:&[Some(wgpu::ColorTargetState{format,blend,write_mask:wgpu::ColorWrites::ALL})],compilation_options:Default::default()}),
            primitive:wgpu::PrimitiveState{cull_mode:None,..Default::default()},depth_stencil:None,multisample:Default::default(),multiview_mask:None,cache:None,
        });
        let surface=pipeline("vs_surface","fs_surface",None);let particles=pipeline("vs_particle","fs_particle",Some(wgpu::BlendState::ALPHA_BLENDING));
        let shelter_layout=device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{label:Some("weather shelter camera"),entries:&[uniform_entry]});
        let shelter_binding=device.create_bind_group(&wgpu::BindGroupDescriptor{label:Some("weather shelter projection"),layout:&shelter_layout,entries:&[wgpu::BindGroupEntry{binding:0,resource:shelter_camera.as_entire_binding()}]});
        let shelter_pipeline=super::map_draw::world_caster_pipeline(device,&shelter_layout,wgpu::DepthStencilState{
            format:wgpu::TextureFormat::Depth24PlusStencil8,depth_write_enabled:Some(true),depth_compare:Some(wgpu::CompareFunction::LessEqual),stencil:Default::default(),bias:Default::default(),
        });
        let rain=super::rain::Rain::new(device,format,width,height,scene,depth,stencil,multisampled);
        Self{output,texture,uniform,binding,surface,particles,shelter_target,shelter_camera,shelter_binding,shelter_pipeline,width,height,rain}
    }
    pub fn record(&self,device:&wgpu::Device,queue:&wgpu::Queue,encoder:&mut wgpu::CommandEncoder,map:&GpuMap,frame:Frame)->RenderResult<()> {
        if matches!(frame.mode,Mode::LightRain|Mode::MediumRain|Mode::HeavyRain|Mode::AfterRain) {
            return self.rain.record(device,queue,encoder,map,frame,&self.output);
        }
        queue.write_buffer(&self.uniform,0,&frame.bytes(self.width,self.height)?);
        let clip=frame.shelter_clip();queue.write_buffer(&self.shelter_camera,0,&clip.into_iter().flat_map(f32::to_ne_bytes).collect::<Vec<_>>());
        let selected=map.weather_shadow_draws(clip);
        let commands=map.prepare_weather_shadow(device,&self.shelter_binding,&self.shelter_pipeline,wgpu::TextureFormat::Depth24PlusStencil8,&selected);
        {
            let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor{
                label:Some("weather vertical shelter"),color_attachments:&[],depth_stencil_attachment:Some(wgpu::RenderPassDepthStencilAttachment{
                    view:&self.shelter_target,depth_ops:Some(wgpu::Operations{load:wgpu::LoadOp::Clear(1.),store:wgpu::StoreOp::Store}),
                    stencil_ops:Some(wgpu::Operations{load:wgpu::LoadOp::Clear(0),store:wgpu::StoreOp::Discard}),
                }),timestamp_writes:None,occlusion_query_set:None,multiview_mask:None,
            });pass.set_pipeline(&self.shelter_pipeline);pass.execute_bundles(std::iter::once(commands.as_ref()));
        }
        let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor{
            label:Some("rain and snow world surfaces"),color_attachments:&[Some(wgpu::RenderPassColorAttachment{view:&self.output,depth_slice:None,resolve_target:None,ops:wgpu::Operations{load:wgpu::LoadOp::Clear(wgpu::Color::BLACK),store:wgpu::StoreOp::Store}})],
            depth_stencil_attachment:None,timestamp_writes:None,occlusion_query_set:None,multiview_mask:None,
        });
        pass.set_bind_group(0,&self.binding,&[]);pass.set_pipeline(&self.surface);pass.draw(0..3,0..1);
        let count=4096;
        pass.set_pipeline(&self.particles);pass.draw(0..6,0..count);
        Ok(())
    }
}
