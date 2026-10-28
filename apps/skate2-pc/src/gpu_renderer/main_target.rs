//! `PostFX`'s main render target as the world's video screens sample it.
//!
//! `cModelInstance::Resolve` 0x82853de8 gives every effect scope named `videoscreen_default`
//! (the `videoscreen` fx's `default` technique: `Technique::Technique` 0x82e1cfb8 names a
//! scope `<fx>_<technique>`) `m_PostFX.m_mainRt->m_colorTargets[0].m_pTexture` as its
//! `i_diffuse` (0x82853f90..0x82854028). The stadium's two video screens use that effect.
//! `ApplyFastUber` 0x828469b8 renders the graded frame back into the main target and
//! resolves it there (0x82846ca4), before `Blit_ToDeviceRT` and the HUD, so the screens
//! show the frame before, without the HUD. `InitBuffers_Xbox360` 0x8283ea58 makes the
//! target `m_screenWidth` by `m_screenHeight`, which `PostFX::Init` sets to 1152 by 640
//! (0x8284a908..0x8284a944).
//!
//! The port draws its scene at the window's size into the final pass's scene texture.
//! After the frame's `PostFX`, the scene's central 16:9 view (the view the original draws,
//! which other window shapes widen around) is box-filtered into a texture of the
//! original's size.
use super::GpuTexture;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// `m_screenWidth` and `m_screenHeight`: `li r5, 0x480` and `li r3, 0x280`.
const WIDTH: u32 = 1152;
const HEIGHT: u32 = 640;

pub(super) struct MainTarget {
    view: Arc<wgpu::TextureView>,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

/// One scene texture the main target copies, with the crop its size needs.
pub(super) struct MainTargetSource(wgpu::BindGroup);

impl MainTarget {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let view = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("PostFX m_mainRt"),
                size: wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("main target copy"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main target copy"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("main target copy"),
            source: wgpu::ShaderSource::Wgsl(include_str!("main_target.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("main target copy"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("main target copy linear clamp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        Self {
            view: Arc::new(view),
            layout,
            pipeline,
            sampler,
        }
    }

    /// The texture the `videoscreen` materials bind as `i_diffuse`.
    pub(super) fn texture(&self) -> GpuTexture {
        GpuTexture {
            view: Arc::clone(&self.view),
            binding: None,
            cube: false,
        }
    }

    /// The scene texture of a `width` by `height` frame, as the copy reads it.
    pub(super) fn source(
        &self,
        device: &wgpu::Device,
        scene: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) -> MainTargetSource {
        let bytes: Vec<u8> = crop(width, height)
            .into_iter()
            .flatten()
            .flat_map(f32::to_ne_bytes)
            .collect();
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("main target crop"),
            contents: &bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        MainTargetSource(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("main target copy source"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(scene),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }))
    }

    /// After the frame's `PostFX`, where `ApplyFastUber` resolves into the main target.
    pub(super) fn record(&self, encoder: &mut wgpu::CommandEncoder, source: &MainTargetSource) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: Some("PostFX m_mainRt"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                depth_slice: None,
                view: &self.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &source.0, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The scene UV span and origin of the central 16:9 view, then a quarter of one
/// main-target texel in scene UV: the offset of the box filter's four bilinear taps.
#[expect(clippy::cast_possible_truncation, reason = "UV lanes for the GPU")]
fn crop(width: u32, height: u32) -> [[f32; 4]; 2] {
    let view = 16.0 / 9.0;
    let aspect = f64::from(width) / f64::from(height);
    let (x, y) = if aspect > view {
        (view / aspect, 1.0)
    } else {
        (1.0, aspect / view)
    };
    [
        [x as f32, y as f32, ((1.0 - x) / 2.0) as f32, ((1.0 - y) / 2.0) as f32],
        [
            (x / f64::from(WIDTH) / 4.0) as f32,
            (y / f64::from(HEIGHT) / 4.0) as f32,
            0.0,
            0.0,
        ],
    ]
}

#[cfg(test)]
mod tests {
    use super::crop;

    fn near(actual: [f32; 4], expected: [f32; 4]) -> bool {
        actual.iter().zip(expected).all(|(a, e)| (a - e).abs() < 1e-6)
    }

    #[test]
    fn the_copy_keeps_the_central_sixteen_by_nine_view() {
        assert!(near(crop(1920, 1080)[0], [1.0, 1.0, 0.0, 0.0]));
        // A 21:9 window widens the view either side of the original's.
        assert!(near(crop(2560, 1080)[0], [0.75, 1.0, 0.125, 0.0]));
        // A 4:3 window shows more above and below it.
        assert!(near(crop(1440, 1080)[0], [1.0, 0.75, 0.0, 0.125]));
        assert!(near(crop(2304, 1296)[1], [0.25 / 1152.0, 0.25 / 640.0, 0.0, 0.0]));
    }
}
