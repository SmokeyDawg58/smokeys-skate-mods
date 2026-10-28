//! Small presentation-rate overlay; never measures simulation ticks.
use std::time::Instant;

/// Screen pixels per glyph pixel; must match fps.wgsl SCALE.
const SCALE: u32 = 5;
/// Nine 3x5 glyph cells, each four glyph pixels wide, after a 10px margin.
const TEXT_WIDTH: u32 = 10 + 9 * 4 * SCALE;
const TEXT_HEIGHT: u32 = 10 + 5 * SCALE;
pub(super) struct Fps {
    pipeline: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    binding: wgpu::BindGroup,
    start: Option<Instant>,
    frames: u32,
    pub(super) value: u32,
}
impl Fps {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FPS digits"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("FPS"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FPS"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("FPS bitmap text"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fps.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("FPS"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("FPS"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
        });
        Self {
            pipeline,
            buffer,
            binding,
            start: None,
            frames: 0,
            value: 0,
        }
    }
    pub(super) fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: Some("FPS overlay"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                depth_slice: None,
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_scissor_rect(0, 0, width.min(TEXT_WIDTH), height.min(TEXT_HEIGHT));
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.binding, &[]);
        pass.draw(0..3, 0..1);
    }
    pub(super) fn presented(&mut self, queue: &wgpu::Queue) {
        let now = Instant::now();
        let Some(start) = self.start else {
            self.start = Some(now);
            return;
        };
        self.frames += 1;
        let seconds = now.duration_since(start).as_secs_f64();
        if seconds >= 0.5 {
            let fps = (f64::from(self.frames) / seconds).round().min(9999.0) as u32;
            self.value = fps;
            queue.write_buffer(&self.buffer, 0, &fps.to_le_bytes());
            self.start = Some(now);
            self.frames = 0;
        }
    }
}
