//! `VideoRenderer_RwTexture` (0x8269f0b0) and the movie's background: the frame's planes in
//! three L8 textures (`Init` 0x826b1940: luminance at full size, blue and red at half), drawn
//! with the `vp6` effect as one quad on the front end's screen camera, over a black
//! `AptRenderingIntegration::RenderBackground` while the movie plays.
use skate2_formats::vp6::YuvFrame;
use wgpu::util::DeviceExt;

/// One frame's movie draw on the 1280x720 stage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MovieDraw {
    /// `mbForeground`: after the front end's layers rather than before them.
    pub(crate) foreground: bool,
    /// The black background quad.
    pub(crate) background: bool,
    /// The video quad's left, top, right and bottom on the stage, when a frame is shown.
    pub(crate) video: Option<[f32; 4]>,
}

struct Planes {
    size: (u32, u32),
    textures: [wgpu::Texture; 3],
    binding: wgpu::BindGroup,
}

pub(super) struct MoviePipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vertices: wgpu::Buffer,
    planes: Planes,
    /// A frame has been uploaded into the planes.
    filled: bool,
}

/// Floats per vertex: clip x and y, u and v, and 1 for the video's vertices.
const VERTEX_FLOATS: usize = 5;

impl super::GpuRenderer {
    /// The movie's next frame into its planes.
    pub(crate) fn upload_movie_frame(&self, frame: &YuvFrame) {
        self.movie
            .borrow_mut()
            .upload(&self.device, &self.queue, frame);
    }
}

impl Planes {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        size: (u32, u32),
    ) -> Self {
        let texture = |label, (width, height)| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        };
        let half = (size.0 / 2, size.1 / 2);
        let textures = [
            texture("Movie luminance", size),
            texture("Movie blue", half),
            texture("Movie red", half),
        ];
        let views: Vec<_> = textures
            .iter()
            .map(|t| t.create_view(&Default::default()))
            .collect();
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Movie planes"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        Self {
            size,
            textures,
            binding,
        }
    }
}

impl MoviePipeline {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Movie planes"),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                texture_entry(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Movie planes"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Movie vp6"),
            source: wgpu::ShaderSource::Wgsl(include_str!("movie.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Movie"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("Movie"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: (VERTEX_FLOATS * 4) as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
        });
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Movie quads"),
            contents: &[0u8; 12 * VERTEX_FLOATS * 4],
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let planes = Planes::new(device, &layout, &sampler, (2, 2));
        Self {
            pipeline,
            layout,
            sampler,
            vertices,
            planes,
            filled: false,
        }
    }

    /// `FillTextureData` (0x82690f48) of a decoded frame into the planes, made for its size first.
    pub(super) fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, frame: &YuvFrame) {
        let size = (
            u32::try_from(frame.width).unwrap_or(0),
            u32::try_from(frame.height).unwrap_or(0),
        );
        if self.planes.size != size {
            self.planes = Planes::new(device, &self.layout, &self.sampler, size);
        }
        self.filled = true;
        for (texture, data) in self
            .planes
            .textures
            .iter()
            .zip([&frame.y, &frame.u, &frame.v])
        {
            let extent = texture.size();
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(extent.width),
                    rows_per_image: Some(extent.height),
                },
                extent,
            );
        }
    }

    /// The background, then the video quad, on the 1280x720 stage fitted to the window as the
    /// front end's flat screens are.
    pub(super) fn draw(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        draw: &MovieDraw,
        size: (u32, u32),
    ) {
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        let fit = [((16. / 9.) / aspect).min(1.), (aspect / (16. / 9.)).min(1.)];
        let clip = |x: f32, y: f32| [(x / 640. - 1.) * fit[0], (1. - y / 360.) * fit[1]];
        let mut floats = Vec::with_capacity(12 * VERTEX_FLOATS);
        let mut quad = |[left, top]: [f32; 2], [right, bottom]: [f32; 2], video: f32| {
            for (x, y, u, v) in [
                (left, top, 0., 0.),
                (right, top, 1., 0.),
                (left, bottom, 0., 1.),
                (left, bottom, 0., 1.),
                (right, top, 1., 0.),
                (right, bottom, 1., 1.),
            ] {
                floats.extend_from_slice(&[x, y, u, v, video]);
            }
        };
        let mut count = 0;
        if draw.background {
            quad([-1., 1.], [1., -1.], 0.);
            count += 6;
        }
        if let (Some([left, top, right, bottom]), true) = (draw.video, self.filled) {
            quad(clip(left, top), clip(right, bottom), 1.);
            count += 6;
        }
        if count == 0 {
            return;
        }
        let bytes: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
        queue.write_buffer(&self.vertices, 0, &bytes);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: Some("Movie"),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_bind_group(0, &self.planes.binding, &[]);
        pass.draw(0..count, 0..1);
    }
}
