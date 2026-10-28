//! A real high-resolution render target, independent of the window buffer.
//! The ordinary world/PostFX/UI passes draw here; readback precedes the UI.
//! A final fitted blit previews the export in the existing window.
use super::*;

pub(super) struct Target {
    pub texture:wgpu::Texture,
    pub window_size:(u32,u32),
    pipeline:wgpu::RenderPipeline,
    binding:wgpu::BindGroup,
    fit:wgpu::Buffer,
}
impl Target {
    pub fn new(device:&wgpu::Device,format:wgpu::TextureFormat,size:(u32,u32),window_size:(u32,u32))->Self {
        let texture=device.create_texture(&wgpu::TextureDescriptor {
            label:Some("1080p media render"),size:wgpu::Extent3d{width:size.0,height:size.1,depth_or_array_layers:1},
            mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format,
            usage:wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::COPY_SRC|wgpu::TextureUsages::TEXTURE_BINDING,view_formats:&[],
        });
        let layout=device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:Some("Media preview"),entries:&[
                wgpu::BindGroupLayoutEntry {binding:0,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Texture {
                    sample_type:wgpu::TextureSampleType::Float{filterable:true},view_dimension:wgpu::TextureViewDimension::D2,multisampled:false},count:None},
                wgpu::BindGroupLayoutEntry {binding:1,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),count:None},
                wgpu::BindGroupLayoutEntry {binding:2,visibility:wgpu::ShaderStages::VERTEX,ty:wgpu::BindingType::Buffer {
                    ty:wgpu::BufferBindingType::Uniform,has_dynamic_offset:false,min_binding_size:None},count:None},
            ],
        });
        let sampler=device.create_sampler(&wgpu::SamplerDescriptor {mag_filter:wgpu::FilterMode::Linear,min_filter:wgpu::FilterMode::Linear,..Default::default()});
        let fit=device.create_buffer(&wgpu::BufferDescriptor {label:Some("Media preview fit"),size:16,usage:wgpu::BufferUsages::UNIFORM|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        let binding=device.create_bind_group(&wgpu::BindGroupDescriptor {label:Some("Media preview"),layout:&layout,entries:&[
            wgpu::BindGroupEntry {binding:0,resource:wgpu::BindingResource::TextureView(&texture.create_view(&Default::default()))},
            wgpu::BindGroupEntry {binding:1,resource:wgpu::BindingResource::Sampler(&sampler)},
            wgpu::BindGroupEntry {binding:2,resource:fit.as_entire_binding()},
        ]});
        let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("Media preview"),source:wgpu::ShaderSource::Wgsl(r#"
@group(0) @binding(0) var source:texture_2d<f32>;
@group(0) @binding(1) var linear:sampler;
@group(0) @binding(2) var<uniform> fit:vec4<f32>;
struct V { @builtin(position) position:vec4<f32>, @location(0) uv:vec2<f32> }
@vertex fn vs(@builtin(vertex_index) i:u32)->V {
    let uv=vec2<f32>(f32((i<<1u)&2u),f32(i&2u));
    var out:V; out.position=vec4<f32>(uv*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.),0.,1.);
    out.uv=(uv-0.5)/fit.xy+0.5; return out;
}
@fragment fn fs(v:V)->@location(0) vec4<f32> {
    let color=textureSample(source,linear,v.uv);
    if any(v.uv<vec2<f32>(0.))||any(v.uv>vec2<f32>(1.)) {return vec4<f32>(0.,0.,0.,1.);}
    return color;
}
"#.into())});
        let pipeline_layout=device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {label:Some("Media preview"),bind_group_layouts:&[Some(&layout)],immediate_size:0});
        let pipeline=device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:Some("Media preview"),layout:Some(&pipeline_layout),cache:None,
            vertex:wgpu::VertexState {module:&shader,entry_point:Some("vs"),compilation_options:Default::default(),buffers:&[]},
            fragment:Some(wgpu::FragmentState {module:&shader,entry_point:Some("fs"),compilation_options:Default::default(),targets:&[Some(wgpu::ColorTargetState {format,blend:None,write_mask:wgpu::ColorWrites::ALL})]}),
            primitive:Default::default(),depth_stencil:None,multisample:Default::default(),multiview_mask:None,
        });
        Self {texture,window_size,pipeline,binding,fit}
    }
    pub fn preview(&self,device:&wgpu::Device,queue:&wgpu::Queue,view:&wgpu::TextureView)->wgpu::CommandBuffer {
        let source=self.texture.width() as f32/self.texture.height() as f32;
        let window=self.window_size.0 as f32/self.window_size.1.max(1) as f32;
        let fit=[(source/window).min(1.),(window/source).min(1.),0.,0.];
        let bytes:Vec<_>=fit.into_iter().flat_map(f32::to_ne_bytes).collect();queue.write_buffer(&self.fit,0,&bytes);
        let mut encoder=device.create_command_encoder(&Default::default());
        {
            let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor {label:Some("Media preview"),color_attachments:&[Some(wgpu::RenderPassColorAttachment {
                view,depth_slice:None,resolve_target:None,ops:wgpu::Operations {load:wgpu::LoadOp::Clear(wgpu::Color::BLACK),store:wgpu::StoreOp::Store},
            })],depth_stencil_attachment:None,timestamp_writes:None,occlusion_query_set:None,multiview_mask:None});
            pass.set_pipeline(&self.pipeline);pass.set_bind_group(0,&self.binding,&[]);pass.draw(0..3,0..1);
        }
        encoder.finish()
    }
}

impl GpuRenderer {
    /// Preserve the editor's composition, including ultrawide, at `lines` lines.
    pub(crate) fn begin_media_capture(&mut self,lines:u32)->RenderResult<(u32,u32)> {
        if self.media_capture.is_some(){return Err("a media render is already active".into());}
        if !self.drawable {return Err("cannot save media while the window is minimized".into());}
        let window=self.dimensions();
        let width=((window.0 as f64/window.1 as f64*f64::from(lines)/2.).round() as u32).max(1)*2;
        let size=(width,lines);
        if width>self.device.limits().max_texture_dimension_2d {return Err("media dimensions exceed this GPU's limit".into());}
        self.media_capture=Some(Target::new(&self.device,self.config.format,size,window));
        self.resize_render_targets(size.0,size.1);
        Ok(size)
    }
    pub(crate) fn end_media_capture(&mut self) {
        if let Some(target)=self.media_capture.take() {
            self.resize_render_targets(target.window_size.0.max(1),target.window_size.1.max(1));
        }
    }
    fn resize_render_targets(&mut self,width:u32,height:u32) {
        self.config.width=width;self.config.height=height;
        self.rebuild_scene_targets();
    }
}
