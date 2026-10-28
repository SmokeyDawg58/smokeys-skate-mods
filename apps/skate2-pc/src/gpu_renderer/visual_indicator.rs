//! `VIPresentation::Render` (0x82803a18) through the `spline` effect
//! (render.big `data\render\shaders\final\spline.xml`), called by
//! `RenderMan::RenderModeScene` 0x82857a78 after the particles and before
//! `PostFX::End`: every indicator's control-point layers as triangle strips,
//! first in `darken` with the back texture, then in `default` with the front.
//!
//! Pass states from the manifest over `cFxParser::EndElement("Pass")`'s reset
//! (no CULLMODE: none): depth test, no depth write; `darken` blends
//! SRCALPHA/INVSRCALPHA for colour and alpha, `default` ONE/ONE. `i_diffuse`
//! is ANISOTROPIC min/mag (8) with the reset's POINT mip, which wgpu cannot
//! combine with anisotropy: it samples LINEAR min/mag, nearest mip. A draw
//! binds no texture when its indicator has none (the follow trail): the
//! effect keeps the technique's last one.

use super::{CameraRelativeClip, DEPTH_FORMAT, GpuRenderer, RenderResult, multisample};
use skate2_core::challenge::vi_presentation::{SplineDraw, spline_vertices};
use skate2_core::texture::NativeTexture;
use std::{collections::BTreeMap, sync::Arc};
use wgpu::util::DeviceExt;

/// One layer's `Draw` record (2432 bytes), at a 256-byte dynamic offset.
const RECORD: usize = 2432;
const RECORD_STRIDE: usize = 2560;

pub(crate) struct Pipelines {
    darken: wgpu::RenderPipeline,
    default: wgpu::RenderPipeline,
    draw_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    eye: super::stereoscopy::Correction,
}

/// The `vitextures.rx2` bindings by asset ID.
#[derive(Default)]
pub(crate) struct Textures {
    bindings: BTreeMap<u64, Arc<wgpu::BindGroup>>,
}

impl Textures {
    pub(crate) fn ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.bindings.keys().copied()
    }
}

/// The vertex buffers by `mVtxCount` and parameter step.
#[derive(Default)]
pub(crate) struct Vertices {
    buffers: BTreeMap<(u16, u32), Arc<wgpu::Buffer>>,
}

struct Strip {
    vertices: Arc<wgpu::Buffer>,
    count: u32,
    offset: u32,
    texture: Arc<wgpu::BindGroup>,
}

/// One frame's records and the strips of both techniques.
pub(crate) struct Frame {
    binding: wgpu::BindGroup,
    techniques: [Vec<Strip>; 2],
}

fn blend(src: wgpu::BlendFactor, dst: wgpu::BlendFactor) -> wgpu::BlendState {
    let component = wgpu::BlendComponent {
        src_factor: src,
        dst_factor: dst,
        operation: wgpu::BlendOperation::Add,
    };
    wgpu::BlendState {
        color: component,
        alpha: component,
    }
}

impl Pipelines {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, samples: u32) -> Self {
        let draw_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("spline effect draw"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(RECORD as u64),
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("spline effect i_diffuse"),
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
            label: Some("spline effect i_diffuse"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("spline effect"),
            source: wgpu::ShaderSource::Wgsl(include_str!("visual_indicator.wgsl").into()),
        });
        let eye=super::stereoscopy::Correction::new(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("spline effect"),
            bind_group_layouts: &[Some(&draw_layout), Some(&texture_layout), Some(&eye.layout)],
            immediate_size: 0,
        });
        let create = |entry: &str, blend_state: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some(entry),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 12,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend_state),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
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
            })
        };
        Self {
            darken: create(
                "fs_darken",
                blend(
                    wgpu::BlendFactor::SrcAlpha,
                    wgpu::BlendFactor::OneMinusSrcAlpha,
                ),
            ),
            default: create(
                "fs_default",
                blend(wgpu::BlendFactor::One, wgpu::BlendFactor::One),
            ),
            draw_layout,
            texture_layout,
            sampler,
            eye,
        }
    }

    pub(super) fn set_eye(&self,queue:&wgpu::Queue,eye:Option<&super::stereoscopy::Eye>) { self.eye.update(queue,eye); }

    /// `VIPresentation::Render`: the darken strips, then the default strips.
    pub(super) fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, frame: &'a Frame) {
        for (pipeline, strips) in [&self.darken, &self.default]
            .into_iter()
            .zip(&frame.techniques)
        {
            if strips.is_empty() {
                continue;
            }
            pass.set_pipeline(pipeline);
            for strip in strips {
                pass.set_bind_group(2,&self.eye.binding,&[]);
                pass.set_bind_group(0, &frame.binding, &[strip.offset]);
                pass.set_bind_group(1, strip.texture.as_ref(), &[]);
                pass.set_vertex_buffer(0, strip.vertices.slice(..));
                pass.draw(0..strip.count, 0..1);
            }
        }
    }
}

fn push_vector(bytes: &mut Vec<u8>, vector: [f32; 4]) {
    for lane in vector {
        bytes.extend_from_slice(&lane.to_ne_bytes());
    }
}

impl GpuRenderer {
    /// Uploads the ribbon textures and binds each under its asset ID.
    pub(crate) fn visual_indicator_textures(
        &self,
        textures: &[NativeTexture],
    ) -> RenderResult<Textures> {
        let mut bindings = BTreeMap::new();
        for texture in textures {
            let uploaded = self.upload_native_texture(texture)?;
            let binding = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&texture.name),
                layout: &self.visual_indicators.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&uploaded.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.visual_indicators.sampler),
                    },
                ],
            });
            bindings.insert(texture.guid, Arc::new(binding));
        }
        Ok(Textures { bindings })
    }

    /// One frame's strips: each draw's layers as records, both techniques in draw order, with
    /// the textures `last` keeps for the draws that bind none.
    pub(crate) fn visual_indicator_frame(
        &self,
        draws: &[SplineDraw],
        camera: &CameraRelativeClip,
        textures: &Textures,
        vertices: &mut Vertices,
        last: &mut [Option<u64>; 2],
    ) -> Option<Frame> {
        let mut bytes = Vec::new();
        let mut techniques: [Vec<Strip>; 2] = [Vec::new(), Vec::new()];
        for draw in draws {
            let key = (draw.vertices.0, draw.vertices.1.to_bits());
            let buffer = Arc::clone(vertices.buffers.entry(key).or_insert_with(|| {
                let positions = spline_vertices(draw.vertices.0, draw.vertices.1);
                let contents: Vec<u8> = positions
                    .iter()
                    .flatten()
                    .flat_map(|value| value.to_ne_bytes())
                    .collect();
                Arc::new(
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("spline vertices"),
                            contents: &contents,
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                )
            }));
            let count = draw.vertex_count.min(u32::from(draw.vertices.0));
            for (technique, last) in last.iter_mut().enumerate() {
                if let Some(id) = draw.textures.and_then(|textures| textures[technique]) {
                    *last = Some(id);
                }
            }
            let bound: [Option<Arc<wgpu::BindGroup>>; 2] = std::array::from_fn(|technique| {
                last[technique].and_then(|id| textures.bindings.get(&id).cloned())
            });
            for layer in &draw.layers {
                let offset = u32::try_from(bytes.len()).ok()?;
                for column in draw.world {
                    push_vector(&mut bytes, column);
                }
                push_vector(&mut bytes, draw.intensity);
                push_vector(&mut bytes, draw.clip_values);
                push_vector(
                    &mut bytes,
                    [camera.camera[0], camera.camera[1], camera.camera[2], 0.0],
                );
                for value in camera.clip_from_camera {
                    bytes.extend_from_slice(&value.to_ne_bytes());
                }
                for point in layer {
                    push_vector(&mut bytes, *point);
                }
                bytes.resize(bytes.len() + RECORD_STRIDE - RECORD, 0);
                for (technique, texture) in bound.iter().enumerate() {
                    if let Some(texture) = texture {
                        techniques[technique].push(Strip {
                            vertices: Arc::clone(&buffer),
                            count,
                            offset,
                            texture: Arc::clone(texture),
                        });
                    }
                }
            }
        }
        if bytes.is_empty() {
            return None;
        }
        let records = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("spline effect records"),
                contents: &bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let binding = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("spline effect records"),
            layout: &self.visual_indicators.draw_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &records,
                    offset: 0,
                    size: wgpu::BufferSize::new(RECORD as u64),
                }),
            }],
        });
        Some(Frame {
            binding,
            techniques,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shader_parses_and_its_record_matches_the_upload() {
        let module = wgpu::naga::front::wgsl::parse_str(include_str!("visual_indicator.wgsl"))
            .expect("spline effect WGSL");
        let draw = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some("Draw"))
            .expect("Draw record");
        let wgpu::naga::TypeInner::Struct { span, .. } = draw.1.inner else {
            panic!("Draw is a struct")
        };
        assert_eq!(span as usize, RECORD);
        assert_eq!(RECORD_STRIDE % 256, 0);
    }
}
