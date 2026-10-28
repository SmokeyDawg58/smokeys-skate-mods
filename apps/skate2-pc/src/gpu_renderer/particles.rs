//! `cParticleRender::Render` (0x82de5f08) through the `lionREC` effect (render.big
//! `data\render\shaders\final\lionREC.xml`), which `ParticlePresentation::Draw` runs from
//! `RenderMan::RenderModeScene` (0x82857d14) after Render3D and the crowd imposters, before the
//! visual indicators: each effect type's quad list with its descriptor's texture.
//!
//! Pass states from the manifest over `cFxParser::EndElement("Pass")`'s reset: SRCALPHA and
//! INVSRCALPHA blending, depth test without depth write, alpha test GREATEREQUAL 1, no culling.
//! `i_textureSampler` has no sampler block, so it takes the reset's WRAP, LINEAR min/mag and
//! POINT mip.

use super::{CameraRelativeClip, DEPTH_FORMAT, GpuRenderer, RenderResult, multisample};
use skate2_core::texture::NativeTexture;
use skate2_core::world_particles::{EFFECT_TYPES, ParticleVertex};
use std::{collections::BTreeMap, sync::Arc};
use wgpu::util::DeviceExt;

/// Position, colour and UV as floats.
const VERTEX_STRIDE: u64 = 36;

pub(crate) struct Pipelines {
    pipeline: wgpu::RenderPipeline,
    camera_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    eye: super::stereoscopy::Correction,
}

/// `particletextures.rx2`'s bindings by asset ID.
#[derive(Default)]
pub(crate) struct Textures {
    bindings: BTreeMap<u64, Arc<wgpu::BindGroup>>,
}

struct List {
    vertices: wgpu::Buffer,
    count: u32,
    texture: Arc<wgpu::BindGroup>,
}

/// One frame's quad lists, in effect type order.
pub(crate) struct Frame {
    camera: wgpu::BindGroup,
    lists: Vec<List>,
}

impl Pipelines {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, samples: u32) -> Self {
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lionREC camera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lionREC i_textureSampler"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lionREC i_textureSampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lionREC"),
            source: wgpu::ShaderSource::Wgsl(include_str!("particles.wgsl").into()),
        });
        let eye = super::stereoscopy::Correction::new(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lionREC"),
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout), Some(&eye.layout)],
            immediate_size: 0,
        });
        let component = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("lionREC default"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: VERTEX_STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4, 2 => Float32x2],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState { color: component, alpha: component }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: multisample(samples),
            multiview_mask: None,
        });
        Self { pipeline, camera_layout, texture_layout, sampler, eye }
    }

    pub(super) fn set_eye(&self, queue: &wgpu::Queue, eye: Option<&super::stereoscopy::Eye>) {
        self.eye.update(queue, eye);
    }

    pub(super) fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, frame: &'a Frame) {
        if frame.lists.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &frame.camera, &[]);
        pass.set_bind_group(2, &self.eye.binding, &[]);
        for list in &frame.lists {
            pass.set_bind_group(1, list.texture.as_ref(), &[]);
            pass.set_vertex_buffer(0, list.vertices.slice(..));
            pass.draw(0..list.count, 0..1);
        }
    }
}

impl GpuRenderer {
    /// Uploads the particle textures and binds each under its asset ID.
    pub(crate) fn particle_textures(&self, textures: &[NativeTexture]) -> RenderResult<Textures> {
        let mut bindings = BTreeMap::new();
        for texture in textures {
            let uploaded = self.upload_native_texture(texture)?;
            let binding = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&texture.name),
                layout: &self.particles.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&uploaded.view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.particles.sampler) },
                ],
            });
            bindings.insert(texture.guid, Arc::new(binding));
        }
        Ok(Textures { bindings })
    }

    /// One frame's quad lists: each type's quads (triangle pairs of `D3DPT_QUADLIST`'s four
    /// corners) relative to the camera, with its texture; a type without one draws nothing.
    pub(crate) fn particle_frame(
        &self,
        quads: &[Vec<ParticleVertex>; EFFECT_TYPES],
        textures: &[Option<u64>; EFFECT_TYPES],
        bound: &Textures,
        camera: &CameraRelativeClip,
    ) -> Option<Frame> {
        let mut lists = Vec::new();
        for (vertices, texture) in quads.iter().zip(textures) {
            let Some(texture) = texture.and_then(|id| bound.bindings.get(&id)) else { continue };
            if vertices.is_empty() {
                continue;
            }
            let mut bytes = Vec::with_capacity(vertices.len() / 4 * 6 * VERTEX_STRIDE as usize);
            for quad in vertices.chunks_exact(4) {
                for corner in [0, 1, 2, 0, 2, 3] {
                    let vertex = quad[corner];
                    let relative = [
                        vertex.position[0] - camera.camera[0],
                        vertex.position[1] - camera.camera[1],
                        vertex.position[2] - camera.camera[2],
                    ];
                    for value in relative.iter().chain(&vertex.color).chain(&vertex.uv) {
                        bytes.extend_from_slice(&value.to_ne_bytes());
                    }
                }
            }
            let count = u32::try_from(bytes.len() as u64 / VERTEX_STRIDE).ok()?;
            let vertices = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("lionREC quads"),
                contents: &bytes,
                usage: wgpu::BufferUsages::VERTEX,
            });
            lists.push(List { vertices, count, texture: Arc::clone(texture) });
        }
        if lists.is_empty() {
            return None;
        }
        let uniform: Vec<u8> = camera.clip_from_camera.iter().flat_map(|value| value.to_ne_bytes()).collect();
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lionREC camera"),
            contents: &uniform,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let camera = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lionREC camera"),
            layout: &self.particles.camera_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });
        Some(Frame { camera, lists })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_shader_parses() {
        wgpu::naga::front::wgsl::parse_str(include_str!("particles.wgsl")).expect("lionREC WGSL");
    }
}
