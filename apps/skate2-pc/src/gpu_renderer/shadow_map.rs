//! The original dynamic-shadow atlas (`PostFX::Shadows_CSM_Init` 0x8283ec80,
//! `PostFX::RenderShadows_CSM` 0x82855888, debug `sk82_na_m.xex`).
//!
//! Each cascade, every frame:
//! 1. `m_pShadowMap_Z` (512x512 D24S8) is cleared to depth 1 and stencil 0
//!    (`GfxMan->mClearDepthStencilParams`), then `ShadowView::Render`
//!    0x82854d28 draws every listed entity with its `shadow` technique. For
//!    defaultcharacter(_ropa) and defaulthair that is `simpleVS` (the default
//!    skinning into `g_matVP`), COLORWRITE 0, CULLMODE BACK, STENCILENABLE,
//!    STENCILREF 255 and (CCW_)STENCILPASS REPLACE: the stencil marks coverage.
//! 2. `Variance_HBlur_PS` (cascade 0) or `Variance_HBlur_3_PS` (cascade 1)
//!    read the resolved depth and stencil into `m_pShadowMap_HBLUR` (512x512
//!    G16R16): x the depth, y the horizontally blurred coverage.
//! 3. `Variance_VBlur_PS` / `Variance_VBlur_3_PS` draw through
//!    `quad_transform_VS` (posScale (0.5, 1), posTrans (c - 0.5, 0)) into
//!    cascade `c`'s half of `m_pShadowMap_ATLAS` (1024x512 G16R16), resolved to
//!    the texture `RenderMan` binds as `shadowTexture_CSM0`.
//!
//! The four filters are compiled Xenos microcode in the executable (shader
//! containers from 0x8212f4e0, literal constants c252-c255 before the code);
//! the WGSL follows their ALU order and literals. Deviations: the D24S8 target
//! is read by aspect instead of decoding the same 24-bit depth and stencil
//! bytes from an 8888 view. Native G16R16 uses Rg16Unorm for both filter
//! passes and the world map. Adapters lacking normalized render-target support
//! retain the older Rg32Float/Rg16Float approximation (not precision parity).

use super::{
    CameraRelativeClip, GpuMap, GpuObject, RenderResult,
    character::{CharacterShaderFamily, SKINNING},
    dynamic_object::{self, DynamicObjectDraw},
    raster_state::{CullMode, RasterState},
};
use std::{
    cell::RefCell,
    sync::{Arc, Weak},
};

struct WorldImageKey {
    commands: Weak<wgpu::RenderBundle>,
    rows: [[u32; 4]; 3],
}
impl WorldImageKey {
    fn matches(&self, commands: &Arc<wgpu::RenderBundle>, rows: [[u32; 4]; 3]) -> bool {
        self.commands.as_ptr() == Arc::as_ptr(commands) && self.rows == rows
    }
}

/// The spawned DMOs (`cLivingWorldPresEntityManager` adds every LivingWorld
/// entity to `eShadowView` through `AddEntityToRenderViews` 0x827f2998) and
/// which cascades' `ShadowView::Cull` keeps each of them for.
pub(super) struct DynamicCasters<'a> {
    pub(super) draws: &'a [DynamicObjectDraw],
    pub(super) casts: &'a [[bool; 2]],
}

/// Width and height of one cascade (`m_pShadowMap_Z`, `m_pShadowMap_HBLUR`).
const CASCADE_SIZE: u32 = 512;
/// The same size in viewport units.
const CASCADE_PIXELS: f32 = 512.0;
const CASTER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
/// The `shadow` technique's STENCILREF.
const COVERAGE_STENCIL: u32 = 255;
/// STENCILFUNC keeps its ALWAYS default; only a depth-passing fragment
/// replaces the stencil.
const COVERAGE_FACE: wgpu::StencilFaceState = wgpu::StencilFaceState {
    compare: wgpu::CompareFunction::Always,
    fail_op: wgpu::StencilOperation::Keep,
    depth_fail_op: wgpu::StencilOperation::Keep,
    pass_op: wgpu::StencilOperation::Replace,
};
/// Position, blend weights and indices of the character part stream
/// (`character::vertex_bytes`); defaulthair appends TEXCOORD1.
const CASTER_ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Uint8x4,
        offset: 20,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Uint8x4,
        offset: 24,
        shader_location: 3,
    },
];

/// This frame's cascades as `RenderShadows_CSM` leaves them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ShadowCascades {
    /// `CSM_Mat_Row0/1/2[c]`: each cascade's light x, y and depth rows.
    pub(crate) rows: [[[f32; 4]; 3]; 2],
    /// `CSM_Mat_Row0[1]` as the receivers read it: cascade 1's light xy
    /// from cascade 0's, scale in xy and offset in zw.
    pub(crate) second_from_first: [f32; 4],
    /// Whether the local skater is on each cascade's `ShadowView` list.
    pub(crate) casts: [bool; 2],
}

impl ShadowCascades {
    /// `CSM_Mat_Row0[0..2]`, `CSM_Mat_Row1[0]`, `CSM_Mat_Row2[0]`: the
    /// elements the world effects read, in `Globals` order.
    pub(crate) fn globals(&self) -> [[f32; 4]; 4] {
        [
            self.rows[0][0],
            self.second_from_first,
            self.rows[0][1],
            self.rows[0][2],
        ]
    }

    /// Column-major clip-from-world of one cascade's caster pass: its three
    /// rows, then w = 1.
    fn clip_from_world(&self, cascade: usize) -> [f32; 16] {
        let [x, y, z] = self.rows[cascade];
        std::array::from_fn(|index| {
            let column = index / 4;
            match index % 4 {
                0 => x[column],
                1 => y[column],
                2 => z[column],
                _ if column == 3 => 1.0,
                _ => 0.0,
            }
        })
    }
}

pub(super) struct ShadowMaps {
    caster_target: wgpu::TextureView,
    blur_target: wgpu::TextureView,
    atlas: Arc<wgpu::TextureView>,
    cameras: [wgpu::Buffer; 2],
    camera_bindings: [wgpu::BindGroup; 2],
    /// The `shadow` technique for plain28, hair36, lit character/ROPA52,
    /// and full hair/CAS60-byte streams.
    casters: [wgpu::RenderPipeline; 4],
    /// The `dynamicobject` effects' `shadow` technique.
    dynamic_caster: wgpu::RenderPipeline,
    /// `Variance_HBlur_PS`, `Variance_HBlur_3_PS`.
    horizontal: [wgpu::RenderPipeline; 2],
    /// `Variance_VBlur_PS`, `Variance_VBlur_3_PS`.
    vertical: [wgpu::RenderPipeline; 2],
    caster_binding: wgpu::BindGroup,
    blur_binding: wgpu::BindGroup,
    world_map: wgpu::TextureView,
    world_rows: wgpu::Buffer,
    world_camera: wgpu::Buffer,
    world_camera_binding: wgpu::BindGroup,
    world_caster: wgpu::RenderPipeline,
    world_resolve: wgpu::RenderPipeline,
    world_image: RefCell<Option<WorldImageKey>>,
    pending_world_image: RefCell<Option<WorldImageKey>>,
    reuse_world_image: bool,
}

impl ShadowMaps {
    pub(super) fn new(
        device: &wgpu::Device,
        row_layout: &wgpu::BindGroupLayout,
        dynamic_object_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let normalized = device.features().contains(wgpu::Features::TEXTURE_FORMAT_16BIT_NORM
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
        let blur_format = if normalized { wgpu::TextureFormat::Rg16Unorm }
            else { wgpu::TextureFormat::Rg32Float };
        let atlas_format = if normalized { wgpu::TextureFormat::Rg16Unorm }
            else { wgpu::TextureFormat::Rg16Float };
        eprintln!("shadow resolve formats: blur={blur_format:?}, atlas/world={atlas_format:?}");
        let texture = |label, width, format| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height: CASCADE_SIZE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let caster_texture = texture("m_pShadowMap_Z", CASCADE_SIZE, CASTER_FORMAT);
        let aspect_view = |aspect| {
            caster_texture.create_view(&wgpu::TextureViewDescriptor {
                aspect,
                ..wgpu::TextureViewDescriptor::default()
            })
        };
        let caster_depth = aspect_view(wgpu::TextureAspect::DepthOnly);
        let caster_stencil = aspect_view(wgpu::TextureAspect::StencilOnly);
        let blur_target = texture("m_pShadowMap_HBLUR", CASCADE_SIZE, blur_format)
            .create_view(&wgpu::TextureViewDescriptor::default());
        let atlas = Arc::new(
            texture("m_pShadowMap_ATLAS", 2 * CASCADE_SIZE, atlas_format)
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );

        let fragment_texture = |binding, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("CSM cascade g_matVP"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let caster_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("m_pShadowMap_Z depth and stencil"),
            entries: &[
                fragment_texture(0, wgpu::TextureSampleType::Depth),
                fragment_texture(1, wgpu::TextureSampleType::Uint),
            ],
        });
        let blur_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("m_pShadowMap_HBLUR"),
            entries: &[fragment_texture(
                0,
                wgpu::TextureSampleType::Float { filterable: false },
            )],
        });
        let cameras = [0, 1].map(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("CSM cascade g_matVP"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let camera_bindings = [0, 1].map(|cascade| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("CSM cascade g_matVP"),
                layout: &camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cameras[cascade].as_entire_binding(),
                }],
            })
        });
        let caster_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("m_pShadowMap_Z depth and stencil"),
            layout: &caster_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&caster_depth),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&caster_stencil),
                },
            ],
        });
        let blur_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("m_pShadowMap_HBLUR"),
            layout: &blur_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&blur_target),
            }],
        });

        let casters_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("character shadow technique (simpleVS)"),
            source: wgpu::ShaderSource::Wgsl(format!("{SKINNING}{CASTER}").into()),
        });
        let casters_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("character shadow technique"),
            bind_group_layouts: &[Some(&camera_layout), Some(row_layout)],
            immediate_size: 0,
        });
        let caster_depth = wgpu::DepthStencilState {
            format: CASTER_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState {
                front: COVERAGE_FACE,
                back: COVERAGE_FACE,
                read_mask: 0xff,
                write_mask: 0xff,
            },
            bias: wgpu::DepthBiasState::default(),
        };
        let caster = |stride| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("character shadow technique"),
                layout: Some(&casters_layout),
                vertex: wgpu::VertexState {
                    compilation_options: Default::default(),
                    module: &casters_module,
                    entry_point: Some("vs_caster"),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: stride,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &CASTER_ATTRIBUTES,
                    })],
                },
                fragment: None,
                primitive: RasterState::culled(CullMode::Back).primitive(),
                depth_stencil: Some(caster_depth.clone()),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
            })
        };
        let dynamic_caster = dynamic_object::caster_pipeline(
            device,
            &camera_layout,
            dynamic_object_layout,
            caster_depth.clone(),
        );
        let world_caster =
            super::map_draw::world_caster_pipeline(device, &camera_layout, caster_depth.clone());
        let world_camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("WorldShadow g_matVP"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let world_rows = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("WorldShadow_MatRow"),
            size: 48,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let world_camera_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("WorldShadow g_matVP"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: world_camera.as_entire_binding(),
            }],
        });
        let world_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("depthsampler_PS82130890"),
            source: wgpu::ShaderSource::Wgsl(format!("{QUAD}{WORLD_RESOLVE}").into()),
        });
        let world_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("depthsampler_PS82130890"),
            bind_group_layouts: &[Some(&caster_layout)],
            immediate_size: 0,
        });
        let world_resolve = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("depthsampler_PS82130890"),
            layout: Some(&world_layout),
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &world_module,
                entry_point: Some("vs_quad"),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &world_module,
                entry_point: Some("fs_world_depth"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: atlas_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
        });
        let filter = |label, source: String, layout: &wgpu::BindGroupLayout, format| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            ["fs_five_taps", "fs_three_taps"].map(|entry_point| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    cache: None,
                    label: Some(label),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        compilation_options: Default::default(),
                        module: &module,
                        entry_point: Some("vs_quad"),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        compilation_options: Default::default(),
                        module: &module,
                        entry_point: Some(entry_point),
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
                })
            })
        };
        Self {
            world_image: RefCell::new(None),
            pending_world_image: RefCell::new(None),
            reuse_world_image: std::env::var_os("SKATE2_REDRAW_WORLD_SHADOWS").is_none(),
            world_map: texture("m_pWorldShadowMap", CASCADE_SIZE, atlas_format)
                .create_view(&wgpu::TextureViewDescriptor::default()),
            world_rows,
            world_camera,
            world_camera_binding,
            world_caster,
            world_resolve,
            casters: [caster(28), caster(36), caster(52), caster(60)],
            dynamic_caster,
            horizontal: filter(
                "Variance_HBlur",
                format!("{QUAD}{HORIZONTAL}"),
                &caster_layout,
                blur_format,
            ),
            vertical: filter(
                "Variance_VBlur",
                format!("{QUAD}{VERTICAL}"),
                &blur_layout,
                atlas_format,
            ),
            caster_target: caster_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            blur_target,
            atlas,
            cameras,
            camera_bindings,
            caster_binding,
            blur_binding,
        }
    }

    /// The atlas the world effects sample as `shadowTexture_CSM0`.
    pub(super) fn atlas(&self) -> &Arc<wgpu::TextureView> {
        &self.atlas
    }

    pub(super) fn world_map(&self) -> &wgpu::TextureView {
        &self.world_map
    }
    pub(super) fn world_rows(&self) -> &wgpu::Buffer {
        &self.world_rows
    }

    pub(super) fn begin_frame(&self) {
        self.pending_world_image.borrow_mut().take();
    }
    /// Confirm only work actually submitted to the same ordered GPU queue.
    pub(super) fn submitted(&self) {
        if let Some(key) = self.pending_world_image.borrow_mut().take() {
            *self.world_image.borrow_mut() = Some(key);
        }
    }

    /// The third pass in RenderShadows_CSM: reuse m_pShadowMap_Z AFTER the
    /// dynamic cascades, resolve depth/uncovered coverage, publish its rows.
    pub(super) fn record_world(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        map: &GpuMap,
    ) -> RenderResult<usize> {
        let selected = map.world_shadow_draws()?;
        let commands = map.prepare_world_shadow(
            device,
            &self.world_camera_binding,
            &self.world_caster,
            CASTER_FORMAT,
            selected,
        );
        let rows = selected.rows.map(|row| row.map(f32::to_bits));
        if self.reuse_world_image
            && self
                .world_image
                .borrow()
                .as_ref()
                .is_some_and(|key| key.matches(&commands, rows))
        {
            return Ok(selected.draw_count());
        }
        // Only the resolved world_map is retained. Dynamic cascades may freely
        // overwrite the shared caster_target; they never write world_map/rows.
        // Invalidate before queued uniform writes, even if this frame later fails.
        self.world_image.borrow_mut().take();
        let matrix = ShadowCascades {
            rows: [selected.rows; 2],
            ..Default::default()
        }
        .clip_from_world(0);
        let bytes: Vec<u8> = matrix
            .iter()
            .flat_map(|value| value.to_ne_bytes())
            .collect();
        queue.write_buffer(&self.world_camera, 0, &bytes);
        let bytes: Vec<u8> = selected
            .rows
            .iter()
            .flatten()
            .flat_map(|value| value.to_ne_bytes())
            .collect();
        queue.write_buffer(&self.world_rows, 0, &bytes);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                multiview_mask: None,
                label: Some("WorldShadowView::Render"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.caster_target,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            // wgpu sends the stencil reference to the driver only while a pipeline that reads it
            // is bound, and a bundle binds its pipelines without it: bind the casters' first, or
            // every bundled draw runs with an unset reference (undefined on Vulkan and DX12).
            pass.set_pipeline(&self.world_caster);
            pass.set_stencil_reference(COVERAGE_STENCIL);
            pass.execute_bundles(std::iter::once(commands.as_ref()));
        }
        filter_pass(
            encoder,
            &self.world_map,
            &self.world_resolve,
            &self.caster_binding,
            None,
        );
        *self.pending_world_image.borrow_mut() = Some(WorldImageKey {
            commands: Arc::downgrade(&commands),
            rows,
        });
        Ok(selected.draw_count())
    }

    /// Both cascades, in the original order: casters, horizontal filter,
    /// vertical filter into the cascade's atlas half. Each LivingWorld character
    /// and DMO uses its own ShadowView admission. The selected skater retains
    /// its existing cascade membership. Skinned and Part positions
    /// are relative to `camera` (the frame's `g_vViewPos`), so each cascade
    /// matrix is rebased there. The caster order does not change the depth
    /// and coverage the pass leaves.
    pub(super) fn record(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        cascades: &ShadowCascades,
        objects: &[GpuObject],
        dynamic: DynamicCasters<'_>,
        camera: [f32; 3],
    ) {
        for (cascade, buffer) in self.cameras.iter().enumerate() {
            let clip = CameraRelativeClip::new(&cascades.clip_from_world(cascade), camera);
            let mut bytes = [0; 64];
            for (destination, value) in bytes.chunks_exact_mut(4).zip(clip.clip_from_camera) {
                destination.copy_from_slice(&value.to_ne_bytes());
            }
            queue.write_buffer(buffer, 0, &bytes);
        }
        for cascade in 0..2 {
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    multiview_mask: None,
                    label: Some("ShadowView::Render"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.caster_target,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(0),
                            store: wgpu::StoreOp::Store,
                        }),
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_stencil_reference(COVERAGE_STENCIL);
                pass.set_bind_group(0, &self.camera_bindings[cascade], &[]);
                pass.set_pipeline(&self.dynamic_caster);
                for (draw, casts) in dynamic.draws.iter().zip(dynamic.casts) {
                    if casts[cascade] {
                        dynamic_object::draw_caster(&mut pass, draw);
                    }
                }
                    for object in objects.iter().filter(|object| !object.hidden) {
                        let Some(character) = &object.character else {
                            continue;
                        };
                        let casts=character.shadow_bounds.map_or(cascades.casts,|(position,bounds)|
                            crate::render_shadows::caster_cascades(&cascades.rows,position,bounds,camera));
                        if !casts[cascade]{continue;}
                        let hair = if character.stamp.as_ref().is_some_and(|s|s.cas){3}else{match character.family {
                            CharacterShaderFamily::Hair | CharacterShaderFamily::HairRopa if character.stamp.is_some()=>3,
                            CharacterShaderFamily::Character | CharacterShaderFamily::Ropa
                                if character.stamp.is_some()=>2,
                            CharacterShaderFamily::Hair | CharacterShaderFamily::HairRopa => 1,
                            CharacterShaderFamily::LivingWorldStamp => 2,
                            _ => 0,
                        }};
                        pass.set_pipeline(&self.casters[hair]);
                        pass.set_bind_group(1, &character.binding, &[]);
                        pass.set_vertex_buffer(0, object.mesh.vertices.slice(..));
                        pass.set_index_buffer(
                            object.mesh.indices.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        for (range, base) in object.mesh.draws.iter() {
                            pass.draw_indexed(range.clone(), *base, 0..1);
                        }
                    }
            }
            filter_pass(
                encoder,
                &self.blur_target,
                &self.horizontal[cascade],
                &self.caster_binding,
                None,
            );
            filter_pass(
                encoder,
                &self.atlas,
                &self.vertical[cascade],
                &self.blur_binding,
                Some(cascade),
            );
        }
    }
}

/// One full-target filter draw; `half` limits it to that cascade's half of
/// the atlas, as `quad_transform_VS` does, and keeps the other half.
fn filter_pass(
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    binding: &wgpu::BindGroup,
    half: Option<usize>,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        multiview_mask: None,
        label: Some("CSM filter"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            depth_slice: None,
            view: target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: if half.is_some() {
                    wgpu::LoadOp::Load
                } else {
                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                },
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });
    if let Some(cascade) = half {
        let (left, x) = if cascade == 0 {
            (0, 0.0)
        } else {
            (CASCADE_SIZE, CASCADE_PIXELS)
        };
        pass.set_viewport(x, 0.0, CASCADE_PIXELS, CASCADE_PIXELS, 0.0, 1.0);
        pass.set_scissor_rect(left, 0, CASCADE_SIZE, CASCADE_SIZE);
    }
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, binding, &[]);
    pass.draw(0..3, 0..1);
}

/// `simpleVS` of defaultcharacter, defaultcharacter_ropa (its GPU branch)
/// and defaulthair: the default skinning into the cascade's `g_matVP`. Their
/// `shadowPS` output is discarded by COLORWRITE 0, so no fragment stage.
const CASTER: &str = r"
struct ShadowCamera { clip_from_camera: mat4x4<f32> }
@group(0) @binding(0) var<uniform> shadow_camera: ShadowCamera;
@group(1) @binding(0) var<uniform> palette: Palette;
@vertex fn vs_caster(@location(0) position: vec3<f32>, @location(2) weights: vec4<u32>,
                     @location(3) indices: vec4<u32>) -> @builtin(position) vec4<f32> {
    return shadow_camera.clip_from_camera * skin_world(position, weights, indices);
}
";

/// One triangle covering the target; the filters address texels by pixel.
const QUAD: &str = r"
@vertex fn vs_quad(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
}
// Xenos `mul` keeps 0 * inf at 0: without a covered neighbour the depth is 0.
fn mean_depth(sum: f32, count: f32) -> f32 {
    return select(0.0, sum * (1.0 / count), count > 0.0);
}
";

/// World depthsampler_PS resolve, with uncovered stencil and edge attenuation.
const WORLD_RESOLVE: &str = r"
@group(0) @binding(0) var caster_depth: texture_depth_2d;
@group(0) @binding(1) var caster_stencil: texture_2d<u32>;
// depthsampler_PS pc3..10. The source rounds the three resolved depth
// bytes with floor(byte*255+.5) and dots them with (1,65536,256)/16777215.
// Host depth-aspect loads already expose that normalized depth.
@fragment fn fs_world_depth(@builtin(position) position: vec4<f32>) -> @location(0) vec2<f32> {
    let pixel = vec2<i32>(position.xy);
    let uv = position.xy / 512.0;
    let edge = vec2<f32>(1.0) - clamp(abs(uv - vec2<f32>(0.5)) - vec2<f32>(0.449999988), vec2<f32>(0.0), vec2<f32>(1.0)) * 20.0;
    let uncovered = 1.0 - f32(textureLoad(caster_stencil, pixel, 0).r) / 255.0;
    return vec2<f32>(textureLoad(caster_depth, pixel, 0), (uncovered * edge.x) * edge.y);
}
";

/// `Variance_HBlur_PS` / `Variance_HBlur_3_PS`: whole-texel x taps, clamped.
/// Covered mean depth or covered centre depth; tent weights 1:2:3:2:1 or 3:8:3.
const HORIZONTAL: &str = r"
@group(0) @binding(0) var caster_depth: texture_depth_2d;
@group(0) @binding(1) var caster_stencil: texture_2d<u32>;
// x the depth, y the coverage (stencil 255 / 255).
fn caster(pixel: vec2<i32>, dx: i32) -> vec2<f32> {
    let at = vec2<i32>(clamp(pixel.x + dx, 0, 511), pixel.y);
    let stencil = f32(textureLoad(caster_stencil, at, 0).r);
    return vec2<f32>(textureLoad(caster_depth, at, 0), stencil / 255.0);
}
@fragment fn fs_five_taps(@builtin(position) position: vec4<f32>) -> @location(0) vec2<f32> {
    let pixel = vec2<i32>(position.xy);
    let right2 = caster(pixel, 2);
    let left1 = caster(pixel, -1);
    let right1 = caster(pixel, 1);
    let left2 = caster(pixel, -2);
    let centre = caster(pixel, 0);
    let count = right1.y + (left2.y + left1.y) + right2.y;
    var sum = right2.x * right2.y;
    sum = right1.x * right1.y + sum;
    sum = left1.x * left1.y + sum;
    sum = left2.x * left2.y + sum;
    var depth = mean_depth(sum, count);
    if (centre.y > 0.0) {
        depth = centre.x;
    }
    let taps = right2.y * 0.33333334 + right1.y * 0.6666667 + left2.y * 0.33333334
        + left1.y * 0.6666667;
    return vec2<f32>(depth, (taps + centre.y) * 0.33333334);
}
@fragment fn fs_three_taps(@builtin(position) position: vec4<f32>) -> @location(0) vec2<f32> {
    let pixel = vec2<i32>(position.xy);
    let left1 = caster(pixel, -1);
    let right1 = caster(pixel, 1);
    let centre = caster(pixel, 0);
    let count = right1.y + left1.y;
    var sum = right1.x * right1.y;
    sum = left1.x * left1.y + sum;
    var depth = mean_depth(sum, count);
    if (centre.y > 0.0) {
        depth = centre.x;
    }
    let coverage = right1.y * 3.0 + left1.y * 3.0 + centre.y * 8.0;
    return vec2<f32>(depth, coverage * 0.071428575);
}
";

/// `Variance_VBlur_PS` / `Variance_VBlur_3_PS` into one atlas half: y taps
/// of the horizontal result, clamped. A neighbour counts toward the depth
/// once its coverage is positive (`saturate(y * 9999999)`); the coverage
/// tent is (2 4 6 4 2)/18 or (3 8 3)/14.
const VERTICAL: &str = r"
@group(0) @binding(0) var horizontal: texture_2d<f32>;
fn blurred(pixel: vec2<i32>, dy: i32) -> vec2<f32> {
    return textureLoad(horizontal, vec2<i32>(pixel.x, clamp(pixel.y + dy, 0, 511)), 0).xy;
}
fn counted(coverage: f32) -> f32 {
    return clamp(coverage * 9999999.0, 0.0, 1.0);
}
@fragment fn fs_five_taps(@builtin(position) position: vec4<f32>) -> @location(0) vec2<f32> {
    let pixel = vec2<i32>(i32(position.x) & 511, i32(position.y));
    let centre = blurred(pixel, 0);
    let up2 = blurred(pixel, -2);
    let down2 = blurred(pixel, 2);
    let up1 = blurred(pixel, -1);
    let down1 = blurred(pixel, 1);
    let near = counted(down1.y) * down1.x + counted(up1.y) * up1.x;
    let far = counted(down2.y) * down2.x + counted(up2.y) * up2.x;
    let count = counted(up2.y) + counted(up1.y) + counted(down1.y) + counted(down2.y);
    var depth = mean_depth(near + far, count);
    if (centre.y > 0.0) {
        depth = centre.x;
    }
    let middle = down1.y * 4.0 + centre.y * 6.0 + up1.y * 4.0;
    let coverage = (up2.y * 2.0 + middle + down2.y * 2.0) * 0.055555556;
    return vec2<f32>(depth, coverage);
}
@fragment fn fs_three_taps(@builtin(position) position: vec4<f32>) -> @location(0) vec2<f32> {
    let pixel = vec2<i32>(i32(position.x) & 511, i32(position.y));
    let centre = blurred(pixel, 0);
    let down1 = blurred(pixel, 1);
    let up1 = blurred(pixel, -1);
    let count = counted(up1.y) + counted(down1.y);
    let sum = counted(up1.y) * up1.x + counted(down1.y) * down1.x;
    var depth = mean_depth(sum, count);
    if (centre.y > 0.0) {
        depth = centre.x;
    }
    let coverage = down1.y * 3.0 + up1.y * 3.0 + centre.y * 8.0;
    return vec2<f32>(depth, coverage * 0.071428575);
}
";

#[cfg(test)]
mod tests {
    use super::ShadowCascades;

    #[test]
    fn world_image_requires_exact_commands_and_projection_without_retaining_geometry() {
        use super::{Arc, WorldImageKey};
        let instance = wgpu::Instance::default();
        let Ok(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        else {
            return;
        };
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("test device");
        let bundle = || {
            Arc::new(
                device
                    .create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                        label: None,
                        color_formats: &[],
                        // A bundle needs an attachment to target.
                        depth_stencil: Some(wgpu::RenderBundleDepthStencil {
                            format: wgpu::TextureFormat::Depth32Float,
                            depth_read_only: false,
                            stencil_read_only: true,
                        }),
                        sample_count: 1,
                        multiview: None,
                    })
                    .finish(&wgpu::RenderBundleDescriptor { label: None }),
            )
        };
        let commands = bundle();
        let rows = [[0; 4]; 3];
        let key = WorldImageKey {
            commands: Arc::downgrade(&commands),
            rows,
        };
        assert!(key.matches(&commands, rows));
        assert!(!key.matches(&bundle(), rows));
        let mut moved = rows;
        moved[2][3] = 1;
        assert!(!key.matches(&commands, moved));
        drop(commands);
        assert!(key.commands.upgrade().is_none());
    }

    #[test]
    fn caster_clip_rows_are_the_cascade_rows_then_unit_w() {
        let cascades = ShadowCascades {
            rows: [
                [
                    [1.0, 2.0, 3.0, 4.0],
                    [5.0, 6.0, 7.0, 8.0],
                    [9.0, 10.0, 11.0, 12.0],
                ],
                [[0.0; 4]; 3],
            ],
            second_from_first: [0.25, 0.25, 0.5, -0.5],
            casts: [true, false],
        };
        let clip = cascades.clip_from_world(0);
        // Column-major: element (row r, column c) at c * 4 + r.
        assert_eq!(&clip[0..4], &[1.0, 5.0, 9.0, 0.0]);
        assert_eq!(&clip[12..16], &[4.0, 8.0, 12.0, 1.0]);
        assert_eq!(
            cascades.globals(),
            [
                [1.0, 2.0, 3.0, 4.0],
                [0.25, 0.25, 0.5, -0.5],
                [5.0, 6.0, 7.0, 8.0],
                [9.0, 10.0, 11.0, 12.0],
            ]
        );
    }
}
