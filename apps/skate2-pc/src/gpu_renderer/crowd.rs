//! `CrowdPresentation`'s draws. `RenderCrowdTargets` (0x82857b90, after the shadow cascades)
//! draws each imposter target's no-sim pedestrian into its cell of the atlas with the `crowd`
//! technique (`CrowdImposterRenderer::Render` 0x827e9b88, `CrowdImposterTarget::Render`
//! 0x827e9ac0), whose stencil REPLACE 255 marks the pedestrian; `RenderCrowdImposters`
//! (0x82857d04, after Render3D, before the particles) draws each imposter as the
//! `crowdimposter` effect's quad (`CrowdImposter::Render` 0x827db6b8), its colour from the
//! atlas and its alpha from that stencil, which `m_main2Rt` exposes as a texture and a mask
//! resolved from it carries here.
//!
//! Manifest states (`crowdimposter.xml`): SRCALPHA/INVSRCALPHA blending, alpha test
//! GREATEREQUAL 30, depth test and write, colour writes RGB, the pass reset's CULLMODE NONE;
//! `i_diffuse` clamped anisotropic 4, `i_stencil` clamped linear.

use super::{CameraRelativeClip, DEPTH_FORMAT, GpuObject, GpuRenderer, multisample};
use wgpu::util::DeviceExt;

/// `m_main2Rt`'s colour: the scene's 8-bit target.
const ATLAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const MASK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;
/// Right, up, forward and origin rows, the two texture rows, red and blue.
const INSTANCE_STRIDE: u64 = 8 * 16;
/// `InitializeImposterVBuffer` 0x827d20f8's quad (position, UV), as two triangles.
const QUAD: [[f32; 5]; 6] = [
    [-0.5, 0.0, 0.0, 1.0, 1.0],
    [0.5, 0.0, 0.0, 0.0, 1.0],
    [0.5, 1.0, 0.0, 0.0, 0.0],
    [-0.5, 0.0, 0.0, 1.0, 1.0],
    [0.5, 1.0, 0.0, 0.0, 0.0],
    [-0.5, 1.0, 0.0, 1.0, 0.0],
];

/// `m_main2Rt`'s stencil as the texture `i_stencil` reads: 255 where a pedestrian drew.
const RESOLVE: &str = r"
@group(0) @binding(0) var atlas_stencil: texture_2d<u32>;
@vertex
fn vs_resolve(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - vec2<f32>(1.0), 0.0, 1.0);
}
@fragment
fn fs_resolve(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let value = textureLoad(atlas_stencil, vec2<i32>(position.xy), 0).r;
    return vec4<f32>(f32(value) / 255.0, 0.0, 0.0, 0.0);
}
";

pub(crate) struct Pipelines {
    /// The pedestrian programs for the atlas: its format, no multisampling.
    pub(super) characters: super::character::CharacterPipelines,
    atlas: wgpu::TextureView,
    depth: wgpu::TextureView,
    mask: wgpu::TextureView,
    resolve: wgpu::RenderPipeline,
    resolve_binding: wgpu::BindGroup,
    imposter: wgpu::RenderPipeline,
    camera_layout: wgpu::BindGroupLayout,
    textures: wgpu::BindGroup,
    quad: wgpu::Buffer,
    eye: super::stereoscopy::Correction,
}

/// One imposter target: its pedestrian's objects and its cell's scissor.
pub(crate) struct Target {
    pub(crate) objects: Vec<GpuObject>,
    /// Left, top, right, bottom.
    pub(crate) scissor: [u32; 4],
}

struct Imposters {
    camera: wgpu::BindGroup,
    instances: wgpu::Buffer,
    count: u32,
}

/// One frame's crowd: the targets in order, then the imposters.
#[derive(Default)]
pub(crate) struct Frame {
    pub(crate) targets: Vec<Target>,
    imposters: Option<Imposters>,
}

impl Frame {
    pub(crate) fn take_targets(&mut self) -> Vec<Target> {
        self.imposters = None;
        std::mem::take(&mut self.targets)
    }
}

/// One imposter as `crowdimposter_defaultVS` takes it: its world matrix relative to the camera
/// (`i_matModelTarget` times `i_matModelView`), its texture rows (`i_matTexture`) and colours.
pub(crate) struct Imposter {
    pub(crate) rows: [[f32; 4]; 4],
    pub(crate) texture: [[f32; 4]; 2],
    pub(crate) colours: [[f32; 4]; 2],
}

impl Pipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        samples: u32,
        object_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let [width, height] = skate2_core::IMPOSTER_ATLAS;
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let target = |label, format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let attachment = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let atlas = target("crowd imposter atlas", ATLAS_FORMAT, attachment).create_view(&Default::default());
        let depth_texture = target("crowd imposter atlas depth/stencil", DEPTH_FORMAT, attachment);
        let depth = depth_texture.create_view(&Default::default());
        let stencil = depth_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("crowd imposter atlas stencil"),
            aspect: wgpu::TextureAspect::StencilOnly,
            ..Default::default()
        });
        let mask = target("crowd imposter stencil mask", MASK_FORMAT, attachment).create_view(&Default::default());
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("crowdimposter"),
            source: wgpu::ShaderSource::Wgsl(include_str!("crowd.wgsl").into()),
        });
        let resolve_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("crowd stencil resolve"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let resolve_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crowd stencil resolve"),
            layout: &resolve_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&stencil) }],
        });
        let resolve_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("crowd stencil resolve"),
            source: wgpu::ShaderSource::Wgsl(RESOLVE.into()),
        });
        let resolve = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("crowd stencil resolve"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("crowd stencil resolve"),
                bind_group_layouts: &[Some(&resolve_layout)],
                immediate_size: 0,
            })),
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &resolve_shader,
                entry_point: Some("vs_resolve"),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &resolve_shader,
                entry_point: Some("fs_resolve"),
                targets: &[Some(wgpu::ColorTargetState { format: MASK_FORMAT, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
        });
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("crowdimposter camera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(96),
                },
                count: None,
            }],
        });
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
        let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let texture_layout_crowd = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("crowdimposter i_diffuse and i_stencil"),
            entries: &[texture_entry(0), sampler_entry(1), texture_entry(2), sampler_entry(3)],
        });
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("crowdimposter i_diffuse"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 4,
            ..Default::default()
        });
        let stencil_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("crowdimposter i_stencil"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let textures = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crowdimposter i_diffuse and i_stencil"),
            layout: &texture_layout_crowd,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&atlas) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&diffuse_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&mask) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&stencil_sampler) },
            ],
        });
        let eye = super::stereoscopy::Correction::new(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("crowdimposter"),
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout_crowd), Some(&eye.layout)],
            immediate_size: 0,
        });
        let blend = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };
        let imposter = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("crowdimposter default"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("vs_imposter"),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 20,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: INSTANCE_STRIDE,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4,
                            6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4
                        ],
                    }),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("fs_imposter"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState { color: blend, alpha: blend }),
                    write_mask: wgpu::ColorWrites::COLOR,
                })],
            }),
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: multisample(samples),
            multiview_mask: None,
        });
        let quad = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("crowdimposter quad"),
            contents: &QUAD.iter().flatten().flat_map(|value| value.to_ne_bytes()).collect::<Vec<_>>(),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            characters: super::character::CharacterPipelines::new(device, ATLAS_FORMAT, 1, object_layout, texture_layout),
            atlas,
            depth,
            mask,
            resolve,
            resolve_binding,
            imposter,
            camera_layout,
            textures,
            quad,
            eye,
        }
    }

    pub(super) fn set_eye(&self, queue: &wgpu::Queue, eye: Option<&super::stereoscopy::Eye>) {
        self.eye.update(queue, eye);
    }

    /// `CrowdImposterRenderer::Render`: the atlas cleared, each target's pedestrian drawn in its
    /// cell (`crowd` technique), then the stencil resolved into the mask the imposters sample.
    pub(super) fn record_targets(&self, encoder: &mut wgpu::CommandEncoder, frame: &Frame) {
        if frame.targets.is_empty() {
            return;
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                multiview_mask: None,
                label: Some("crowd imposter targets"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    depth_slice: None,
                    view: &self.atlas,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0), store: wgpu::StoreOp::Store }),
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            for target in &frame.targets {
                let [left, top, right, bottom] = target.scissor;
                pass.set_scissor_rect(left, top, right - left, bottom - top);
                for object in &target.objects {
                    let Some(character) = object.character.as_ref() else { continue };
                    for pipeline in self.characters.crowd_technique(character) {
                        pass.set_pipeline(pipeline);
                        super::draw_object(&mut pass, object);
                    }
                }
            }
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: Some("crowd imposter stencil mask"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                depth_slice: None,
                view: &self.mask,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.resolve);
        pass.set_bind_group(0, &self.resolve_binding, &[]);
        pass.draw(0..3, 0..1);
    }

    /// `CrowdImposterRenderer::Render(CrowdImposter*)` 0x827db9b0: every imposter's quad.
    pub(super) fn draw_imposters<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, frame: &'a Frame) {
        let Some(imposters) = &frame.imposters else { return };
        pass.set_pipeline(&self.imposter);
        pass.set_bind_group(0, &imposters.camera, &[]);
        pass.set_bind_group(1, &self.textures, &[]);
        pass.set_bind_group(2, &self.eye.binding, &[]);
        pass.set_vertex_buffer(0, self.quad.slice(..));
        pass.set_vertex_buffer(1, imposters.instances.slice(..));
        pass.draw(0..6, 0..imposters.count);
    }
}

impl GpuRenderer {
    /// The frame's crowd: its targets' objects and scissors, and the imposters with the camera
    /// and the frame's fog (`g_FogK1`, `g_FogColour`).
    pub(crate) fn crowd_frame(
        &self,
        targets: Vec<Target>,
        imposters: &[Imposter],
        camera: &CameraRelativeClip,
        fog: [[f32; 4]; 2],
    ) -> Frame {
        let imposters = (!imposters.is_empty()).then(|| {
            let bytes: Vec<u8> = imposters
                .iter()
                .flat_map(|imposter| imposter.rows.iter().chain(&imposter.texture).chain(&imposter.colours))
                .flatten()
                .flat_map(|value| value.to_ne_bytes())
                .collect();
            let instances = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("crowdimposter instances"),
                contents: &bytes,
                usage: wgpu::BufferUsages::VERTEX,
            });
            let uniform: Vec<u8> =
                camera.clip_from_camera.iter().chain(fog.iter().flatten()).flat_map(|value| value.to_ne_bytes()).collect();
            let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("crowdimposter camera"),
                contents: &uniform,
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let camera = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("crowdimposter camera"),
                layout: &self.crowd.camera_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
            });
            #[expect(clippy::cast_possible_truncation, reason = "at most 200 imposters")]
            Imposters { camera, instances, count: imposters.len() as u32 }
        });
        Frame { targets, imposters }
    }
}

#[cfg(test)]
mod tests {
    fn validate(source: &str) {
        let module = wgpu::naga::front::wgsl::parse_str(source).expect("WGSL");
        wgpu::naga::valid::Validator::new(wgpu::naga::valid::ValidationFlags::all(), wgpu::naga::valid::Capabilities::all())
            .validate(&module)
            .expect("WGSL validation");
    }

    #[test]
    fn the_shaders_validate() {
        validate(include_str!("crowd.wgsl"));
        validate(super::RESOLVE);
    }
}
