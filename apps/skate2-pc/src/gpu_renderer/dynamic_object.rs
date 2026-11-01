//! Original LivingWorld DMO draws: the `dynamicobject` and
//! `alphatestdynamicobject` effects' `default` technique and their `shadow`
//! technique casters (docs/evidence/dmo-render-2026-09-28.md).
//!
//! The shipped programs (`render.big` `data\render\shaders\final\
//! *dynamicobject_*.fpo`/`.vpo`, read through their own constant tables) are
//! newer than the `.updb` source text in three places, and the WGSL follows
//! the programs:
//! - `defaultPS` binds `shadowTexture_World` (s1) and no
//!   `g_fresnelLookupTexture`: the fresnel is `1 + 4 * (1 - saturate(N.E))^8`
//!   by three squarings (pc 97-107), `GetFresnel`'s lerp(1, 5, ..).
//! - `PCF3X3` steps `m_params[1].z / 1024` in u and `/ 512` in v (c249).
//! - `CalculateWorldShadow` reads `shadowTexture_World` at
//!   `(0.5 x + 0.5, -0.5 y + 0.5)` and returns
//!   `(dist < 0.99) * saturate(W.y * (W.x >= z))` (pc 52-93), where
//!   `depthsampler_PS` 0x82130890 stores the world casters' depth and their
//!   uncovered coverage times an edge fade.
//!
//! Inputs and deviations:
//! - `WorldShadowView` shares ShadowMaps' real world depth/coverage target
//!   and row buffer, updated after the two dynamic cascades each frame.
//! - Distance/spawn opacity and the original two-pass ghost technique are
//!   connected. The base constructor starts spawn opacity at1; DMOs do not
//!   receive a fabricated fade from zero. Alpha-tested effects have no ghost.
//! - Positions are rebased at `g_vViewPos` (the Part row 3 in f64); the
//!   replay SLerp between simulation frames is not applied.
//! - `half` math runs in f32; `TRILINEARTHRESHOLD` has no wgpu equivalent and
//!   the manifests' anisotropy 4 is raised to 16 as for the world effects.

use super::{
    DEPTH_FORMAT, GpuTexture, RenderResult,
    environment_shading::COMMON,
    multisample,
    raster_state::{CullMode, RasterState},
};
use skate2_formats::render::{
    NativeMeshGeometry, NativeVertexAttributeValues, PRIMITIVE_TYPE_TRIANGLE_LIST, VertexSemantic,
};
use std::{ops::Range, sync::Arc};
use wgpu::util::DeviceExt;

/// D3DDECLUSAGE_BINORMAL.
const USAGE_BINORMAL: u8 = 7;

/// The vehicle shaders read BLENDINDICES.w AFTER the Xenon declaration
/// swizzle, not serialized byte3. D3D's patcher82c43ec4..3f04 turns UBYTE4
/// Type001a2286 into swizzle0530 (3,2,1,0); its W is byte0. The format
/// decoder deliberately retains original byte order. Actual vehicle meshes
/// store [palette_index,0,0,0], with chassis after the wheel palette entries.
pub(crate) const fn vehicle_bone_index(stored: &[u8; 4]) -> u8 {
    stored[0]
}

#[test]
fn vehicle_index_matches_native_declaration_swizzle() {
    // D3D patcher82c43ec4..3f04, evaluated for the actual UBYTE4 declaration.
    let t = skate2_formats::render::VERTEX_FORMAT_UBYTE4;
    let packed = (8
        * ((2 * ((8 * (((t >> 10) & 0x38) | ((t >> 16) & 7))) | ((t >> 19) & 7))) | (t & 0xfc00)))
        & 0xffff;
    assert_eq!(packed, 0x0530);
    let stored = [4, 7, 11, 13];
    let shader_w = stored[((packed >> 13) & 7) as usize];
    assert_eq!(vehicle_bone_index(&stored), shader_w);
    // Authored sedan palette4 is Chassis; palette0 is LeftFront_wheel.
    assert_eq!(vehicle_bone_index(&[4, 0, 0, 0]), 4);
}
/// POSITION, TEXCOORD0, TANGENT, BINORMAL as f32 lanes.
const VERTEX_STRIDE: wgpu::BufferAddress = 56;
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    0 => Float32x3, 1 => Float32x2, 2 => Float32x3, 3 => Float32x3, 4 => Float32x2, 5 => Float32
];
/// `simpleVS` fetches the position alone.
const CASTER_ATTRIBUTES: [wgpu::VertexAttribute; 2] = [VERTEX_ATTRIBUTES[0], VERTEX_ATTRIBUTES[5]];
/// Four `i_partArray` rows and `i_params[0]`.
const OBJECT_BYTES: usize = 80 + (3 + 9 + 61 * 3) * 16;
/// `m_params[0..2]` and `i_detailNormalUVScale`.
const MATERIAL_BYTES: usize = 9 * 16;

#[cfg(test)]
#[path = "vehicle_material_tests.rs"]
mod vehicle_material_tests;

/// The production DMO shader validates and every fragment program, the park decal's
/// included, links with `defaultVS` over the production vertex layout.
#[test]
fn dynamic_object_programs_link() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no adapter; dynamicobject program check skipped");
        return;
    };
    let (device, _queue) = pollster::block_on(adapter.request_device(&Default::default())).expect("device");
    let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{COMMON}{}{DYNAMIC_OBJECT}\nconst vehicle_diagnostic: u32 = 0u;\n{}{}",
                object_source(3),
                include_str!("vehicle_material.wgsl"),
                include_str!("vehicle_shading.wgsl")
            )
            .into(),
        ),
    });
    for fragment in ["defaultPS", "ghostPS", "alphatestPS", "decalPS", "vehiclePS"] {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some(fragment),
            layout: None,
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("defaultVS"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: VERTEX_STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &VERTEX_ATTRIBUTES,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some(fragment),
                targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
        });
    }
    let error = pollster::block_on(error_scope.pop());
    assert!(error.is_none(), "{error:?}");
}
/// `WorldShadow_MatRow[0..3]`.

/// Which original effect a DMO mesh draws with (the material's `effect`).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum DynamicObjectEffect {
    /// `dynamicobject` (`material_dynamicobject/default`).
    Default,
    /// `alphatestdynamicobject` (`material_dynamicobject/alphatest`).
    AlphaTest,
    /// A Skate 3 park piece's `environmentpark.decal` material: `dynamicobject` with its
    /// CLAMP decal (UV0.zw, uploaded in the UV1 lanes) laid over the diffuse.
    Decal,
    Vehicle,
    VehicleGlass,
    VehicleLight,
}

impl DynamicObjectEffect {
    pub(crate) fn from_manifest(name: &str) -> Option<Self> {
        match name {
            "dynamicobject" => Some(Self::Default),
            "alphatestdynamicobject" => Some(Self::AlphaTest),
            "vehicle" => Some(Self::Vehicle),
            "vehicle_glass" => Some(Self::VehicleGlass),
            "vehiclelight" => Some(Self::VehicleLight),
            _ => None,
        }
    }
}

/// One mesh of a DMO RModel with its material bind group.
pub(crate) struct GpuDynamicObjectMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    draws: Vec<(Range<u32>, i32)>,
    material: Arc<wgpu::BindGroup>,
    effect: DynamicObjectEffect,
    /// The material's `InstanceSortKey` draw-order byte.
    draw_order: u8,
}

/// One uploaded DMO RModel: its meshes in model order and its local bounds.
pub(crate) struct GpuDynamicObjectModel {
    meshes: Vec<GpuDynamicObjectMesh>,
    /// RModel bounding box minimum and maximum.
    bounds: [[f32; 3]; 2],
}

/// One spawned DMO entity's GPU state.
pub(crate) struct GpuDynamicObject {
    model: Arc<GpuDynamicObjectModel>,
    uniform: wgpu::Buffer,
    binding: wgpu::BindGroup,
}

/// One DMO this frame: its entity state and Part transform.
pub(crate) struct DynamicObjectDraw {
    pub(crate) object: Arc<GpuDynamicObject>,
    /// Row-vector world transform: x, y, z axis rows, then the translation.
    pub(crate) part_transform: [f32; 16],
    pub(crate) opacity: f32,
    pub(crate) ghosted: bool,
    pub(crate) vehicle: Option<VehicleDrawConstants>,
    /// A NIS vehicle's `SkaterPresEntity::GetPosition` (0x827d79c0): its
    /// callbacks' `GetHipsPos`.
    pub(crate) hips: Option<[f32; 3]>,
}

/// Per-mesh remapped palette; the shared draw/shadow paths use the same rows.
pub(crate) struct VehicleDrawConstants {
    pub(crate) colours: [[f32; 4]; 2],
    pub(crate) lights: [f32; 4],
    pub(crate) irradiance: [[f32; 4]; 9],
    /// PackAndMultiplyMatricesForUpload output, already in world space.
    pub(crate) rows: Vec<[f32; 4]>,
}

impl DynamicObjectDraw {
    /// CalculateBoundsInfo827f2708 stores the RModel midpoint with W=1.
    /// SetLocalToWorldMatrix827efd08 transforms it in X/Y/Z/W multiply order.
    /// The DMO ctor's later local-box expansion does not change this centre.
    pub(crate) fn bounds_center(&self) -> [f32; 3] {
        let [minimum, maximum] = self.object.model.bounds;
        let center: [f32; 3] = std::array::from_fn(|lane| (minimum[lane] + maximum[lane]) * 0.5);
        let m = &self.part_transform;
        std::array::from_fn(|lane| {
            let x = m[lane] * center[0];
            let y = m[4 + lane].mul_add(center[1], x);
            let z = m[8 + lane].mul_add(center[2], y);
            m[12 + lane].mul_add(1.0, z)
        })
    }
    /// `PresentationEntity::GetPosition`: the translation row, or a
    /// `SkaterPresEntity`'s hips.
    pub(crate) fn position(&self) -> [f32; 3] {
        self.hips.unwrap_or_else(|| self.translation())
    }

    /// The `m_MatLtoW` translation `SceneRenderView::Cull` adds to the box.
    fn translation(&self) -> [f32; 3] {
        [
            self.part_transform[12],
            self.part_transform[13],
            self.part_transform[14],
        ]
    }

    /// cPresDynamicObject ctor8280bb78 replaces the local box's XYZ with
    /// +/- the largest full RModel extent, centred on the entity origin.
    /// SceneRenderView.Cull828501c0 adds the entity translation, without
    /// rotating the box; ShadowView.Cull82850c30 consumes that same m_BB.
    pub(crate) fn world_bounds(&self) -> [[f32; 3]; 2] {
        let [minimum, maximum] = self.object.model.bounds;
        if self.vehicle.is_some() {
            // cPresVehicle keeps CalculateBoundsInfo's union of LOD0 parts.
            // Cull828501c0 translates this box; only cPresDynamicObject's
            // constructor applies the enlarged origin-centred cube below.
            let position = self.translation();
            return [
                std::array::from_fn(|i| minimum[i] + position[i]),
                std::array::from_fn(|i| maximum[i] + position[i]),
            ];
        }
        let mut extent = 0.0;
        for lane in 0..3 {
            let candidate = maximum[lane] - minimum[lane];
            // Ordered strict comparisons match vcmpgtfp/CR6, including NaN.
            if candidate > extent {
                extent = candidate;
            }
        }
        let position = self.translation();
        [
            std::array::from_fn(|lane| -extent + position[lane]),
            std::array::from_fn(|lane| extent + position[lane]),
        ]
    }

    /// `i_partArray` rows with the translation rebased at `camera` in f64,
    /// then `i_params[0]` with the actual LivingWorld presentation opacity.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the f64 rebase narrows to the f32 GPU lanes"
    )]
    fn object_bytes(&self, camera: [f32; 3]) -> [u8; OBJECT_BYTES] {
        let mut rows = self.part_transform;
        for lane in 0..3 {
            rows[12 + lane] = (f64::from(rows[12 + lane]) - f64::from(camera[lane])) as f32;
        }
        let params = [
            self.opacity,
            if self.vehicle.is_some() { 1. } else { 0. },
            0.0,
            0.0,
        ];
        let mut bytes = [0; OBJECT_BYTES];
        for (destination, value) in bytes.chunks_exact_mut(4).zip(rows.iter().chain(&params)) {
            destination.copy_from_slice(&value.to_ne_bytes());
        }
        if let Some(vehicle) = &self.vehicle {
            let mut values = Vec::with_capacity((3 + 9 + 61 * 3) * 4);
            values.extend(vehicle.colours.into_iter().flatten());
            values.extend(vehicle.lights);
            values.extend(vehicle.irradiance.into_iter().flatten());
            for (index, row) in vehicle.rows.iter().enumerate() {
                let mut row = *row;
                row[3] = (f64::from(row[3]) - f64::from(camera[index % 3])) as f32;
                values.extend(row);
            }
            for (destination, value) in bytes[80..].chunks_exact_mut(4).zip(values) {
                destination.copy_from_slice(&value.to_ne_bytes());
            }
        }
        bytes
    }

    /// Writes this frame's rows before either pass records.
    pub(super) fn upload(&self, queue: &wgpu::Queue, camera: [f32; 3]) {
        queue.write_buffer(&self.object.uniform, 0, &self.object_bytes(camera));
    }

    /// The draw-order byte of mesh `index`.
    pub(super) fn draw_order(&self, index: usize) -> u8 {
        self.object.model.meshes[index].draw_order
    }

    pub(super) fn mesh_count(&self) -> usize {
        self.object.model.meshes.len()
    }
}

/// The shared pipelines, layouts and frame resources.
pub(super) struct DynamicObjectPipelines {
    material_layout: wgpu::BindGroupLayout,
    object_layout: Arc<wgpu::BindGroupLayout>,
    wrap_sampler: wgpu::Sampler,
    cube_sampler: wgpu::Sampler,
    default: wgpu::RenderPipeline,
    alpha_test: wgpu::RenderPipeline,
    decal: wgpu::RenderPipeline,
    decal_sampler: wgpu::Sampler,
    ghost: [wgpu::RenderPipeline; 2],
    vehicle: wgpu::RenderPipeline,
    vehicle_glass: wgpu::RenderPipeline,
    vehicle_light: wgpu::RenderPipeline,
    vehicle_ghost: [wgpu::RenderPipeline; 2],
    /// Group 2: the CSM atlas, the world shadow map and its rows.
    frame: wgpu::BindGroup,
}

/// Group 3 / caster group 1: `i_partArray` rows and `i_params[0]`.
pub(super) fn object_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dynamicobject i_partArray/i_params"),
        entries: &[uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT)],
    })
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn texture_entry(
    binding: u32,
    dimension: wgpu::TextureViewDimension,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: dimension,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

impl DynamicObjectPipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        globals_layout: &wgpu::BindGroupLayout,
        object_layout: Arc<wgpu::BindGroupLayout>,
        shadow_atlas: &wgpu::TextureView,
        world_view: &wgpu::TextureView,
        world_rows: &wgpu::Buffer,
    ) -> Self {
        use wgpu::TextureViewDimension::{Cube, D2};
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dynamicobject material"),
            entries: &[
                texture_entry(0, D2),
                texture_entry(1, D2),
                texture_entry(2, D2),
                texture_entry(3, D2),
                texture_entry(4, Cube),
                sampler_entry(5),
                sampler_entry(6),
                uniform_entry(7, wgpu::ShaderStages::FRAGMENT),
                texture_entry(8, D2),
                sampler_entry(9),
            ],
        });
        // Bindings 2 and 3 are COMMON's `shadow_csm0` and `shadow_sampler`.
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dynamicobject shadow maps"),
            entries: &[
                texture_entry(2, D2),
                sampler_entry(3),
                texture_entry(4, D2),
                sampler_entry(5),
                uniform_entry(6, wgpu::ShaderStages::FRAGMENT),
            ],
        });
        // Manifest samplers: i_diffuse/i_specular/i_normal/i_detail
        // ANISOTROPIC min/mag, LINEAR mip, default WRAP; i_environment CLAMP,
        // LINEAR min/mag, POINT mip.
        let wrap_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("dynamicobject diffuse-family wrap"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        // decalenvironment's decal sampler: the diffuse family's filtering with CLAMP.
        let decal_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("park piece decal clamp"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        let cube_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("dynamicobject reflection cube"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        // `ShadowTextureSampler` as the world effects bind it: BORDER u/v
        // with a zero border (edge clamp without the feature), LINEAR.
        let border = device
            .features()
            .contains(wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER);
        let shadow_address = if border {
            wgpu::AddressMode::ClampToBorder
        } else {
            wgpu::AddressMode::ClampToEdge
        };
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("dynamicobject ShadowTextureSampler"),
            address_mode_u: shadow_address,
            address_mode_v: shadow_address,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            border_color: border.then_some(wgpu::SamplerBorderColor::TransparentBlack),
            ..Default::default()
        });
        // `m_pWorldShadowMap` is a LINEAR-filtered G16R16 target.
        let world_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("dynamicobject shadowTexture_World"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let frame = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dynamicobject shadow maps"),
            layout: &frame_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(shadow_atlas),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(world_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&world_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: world_rows.as_entire_binding(),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dynamicobject.fx / alphatestdynamicobject.fx (shipped programs)"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{COMMON}{}{DYNAMIC_OBJECT}\nconst vehicle_diagnostic: u32 = {}u;\n{}{}",
                    object_source(3),
                    std::env::var("SKATE2_VEHICLE_SHADE_DIAGNOSTIC")
                        .ok()
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(0),
                    include_str!("vehicle_material.wgsl"),
                    include_str!("vehicle_shading.wgsl")
                )
                .into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("dynamicobject pipeline layout"),
            bind_group_layouts: &[
                Some(globals_layout),
                Some(&material_layout),
                Some(&frame_layout),
                Some(&*object_layout),
            ],
            immediate_size: 0,
        });
        let pipeline = |fragment: &str, write_mask, ghost_blend: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some(fragment),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: Some("defaultVS"),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: VERTEX_STRIDE,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &VERTEX_ATTRIBUTES,
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: Some(fragment),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: ghost_blend.then_some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask,
                    })],
                }),
                // Both `default` passes: CULLMODE=FRONT, depth test and write.
                primitive: RasterState::FRONT.primitive(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    // vehicle_glass.xml: DEPTHWRITEENABLE FALSE (alias of
                    // ZWRITEENABLE in cDepthStencilStateHandle82e19368).
                    depth_write_enabled: Some(fragment != "vehicleGlassPS"),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: if ghost_blend || fragment.starts_with("vehicle") {
                        let face = wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Always,
                            fail_op: wgpu::StencilOperation::Keep,
                            depth_fail_op: wgpu::StencilOperation::Keep,
                            pass_op: wgpu::StencilOperation::Replace,
                        };
                        wgpu::StencilState {
                            front: face,
                            back: face,
                            read_mask: 255,
                            write_mask: 255,
                        }
                    } else {
                        wgpu::StencilState::default()
                    },
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: multisample(sample_count),
                multiview_mask: None,
            })
        };
        Self {
            // dynamicobject.xml `default` pass 0: only CULLMODE=FRONT.
            default: pipeline("defaultPS", wgpu::ColorWrites::ALL, false),
            // alphatestdynamicobject.xml `default` pass 0: COLORWRITE=RGB,
            // ALPHATEST GREATEREQUAL 30 (in `alphatestPS`), ZWRITEENABLE,
            // no blend.
            alpha_test: pipeline("alphatestPS", wgpu::ColorWrites::COLOR, false),
            decal: pipeline("decalPS", wgpu::ColorWrites::ALL, false),
            decal_sampler,
            vehicle: pipeline("vehiclePS", wgpu::ColorWrites::ALL, false),
            vehicle_glass: pipeline("vehicleGlassPS", wgpu::ColorWrites::COLOR, true),
            vehicle_light: pipeline("vehicleLightPS", wgpu::ColorWrites::ALL, false),
            vehicle_ghost: [
                pipeline("vehiclePS", wgpu::ColorWrites::empty(), false),
                pipeline("vehicleGhostPS", wgpu::ColorWrites::COLOR, true),
            ],
            // final/dynamicobject.xml: same programs, depth-only then blended
            // RGB with alpha >=1/255 and stencil replacement255. Both write Z.
            ghost: [
                pipeline("defaultPS", wgpu::ColorWrites::empty(), false),
                pipeline("ghostPS", wgpu::ColorWrites::COLOR, true),
            ],
            material_layout,
            object_layout,
            wrap_sampler,
            cube_sampler,
            frame,
        }
    }

    /// One material: `i_diffuse`, `i_specular`, `i_normal`, `i_detail` and
    /// the `i_environment` cube, `m_params[0..2]` and the
    /// `detailNormalUVScale` instance scalar, and a `Decal` material's decal
    /// (any other binds its diffuse there, unsampled).
    pub(super) fn material(
        &self,
        device: &wgpu::Device,
        textures: [&GpuTexture; 5],
        m_params: [[f32; 4]; 8],
        detail_normal_uv_scale: [f32; 4],
        decal: Option<&GpuTexture>,
    ) -> RenderResult<wgpu::BindGroup> {
        for (index, texture) in textures.iter().enumerate() {
            if texture.cube != (index == 4) {
                return Err(format!(
                    "dynamicobject material channel {index} has the wrong texture type"
                )
                .into());
            }
        }
        let mut contents = [0; MATERIAL_BYTES];
        for (destination, value) in contents
            .chunks_exact_mut(4)
            .zip(m_params.iter().flatten().chain(&detail_normal_uv_scale))
        {
            destination.copy_from_slice(&value.to_ne_bytes());
        }
        let constants = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dynamicobject m_params/i_detailNormalUVScale"),
            contents: &contents,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let mut entries: Vec<wgpu::BindGroupEntry<'_>> = textures
            .iter()
            .zip(0_u32..)
            .map(|(texture, binding)| wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 5,
            resource: wgpu::BindingResource::Sampler(&self.wrap_sampler),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 6,
            resource: wgpu::BindingResource::Sampler(&self.cube_sampler),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 7,
            resource: constants.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 8,
            resource: wgpu::BindingResource::TextureView(&decal.unwrap_or(textures[0]).view),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 9,
            resource: wgpu::BindingResource::Sampler(&self.decal_sampler),
        });
        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("original dynamicobject material"),
            layout: &self.material_layout,
            entries: &entries,
        }))
    }

    fn technique(&self, effect: DynamicObjectEffect, ghosted: bool) -> &[wgpu::RenderPipeline] {
        match effect {
            DynamicObjectEffect::Default | DynamicObjectEffect::Decal if ghosted => &self.ghost,
            DynamicObjectEffect::Default => std::slice::from_ref(&self.default),
            DynamicObjectEffect::Decal => std::slice::from_ref(&self.decal),
            // SetCurrentEffect82e1c210 falls back to default: this effect's
            // shipped manifest has no ghost technique and ignores opacity.
            DynamicObjectEffect::AlphaTest => std::slice::from_ref(&self.alpha_test),
            DynamicObjectEffect::Vehicle if ghosted => &self.vehicle_ghost,
            DynamicObjectEffect::Vehicle => std::slice::from_ref(&self.vehicle),
            DynamicObjectEffect::VehicleGlass => std::slice::from_ref(&self.vehicle_glass),
            DynamicObjectEffect::VehicleLight => std::slice::from_ref(&self.vehicle_light),
        }
    }

    pub(super) fn pass_count(&self, draw: &DynamicObjectDraw, index: usize) -> usize {
        self.technique(draw.object.model.meshes[index].effect, draw.ghosted)
            .len()
    }

    /// Records mesh `index` of `draw`; the globals are group 0 already or
    /// are bound here.
    pub(super) fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        globals: &'a wgpu::BindGroup,
        draw: &'a DynamicObjectDraw,
        index: usize,
        technique_pass: usize,
    ) {
        let mesh = &draw.object.model.meshes[index];
        pass.set_pipeline(&self.technique(mesh.effect, draw.ghosted)[technique_pass]);
        if draw.ghosted && matches!(mesh.effect, DynamicObjectEffect::Default | DynamicObjectEffect::Decal) {
            pass.set_stencil_reference(255);
        }
        if draw.vehicle.is_some() {
            pass.set_stencil_reference(
                if draw.ghosted || mesh.effect == DynamicObjectEffect::VehicleGlass {
                    255
                } else {
                    128
                },
            );
        }
        pass.set_bind_group(0, globals, &[]);
        pass.set_bind_group(1, mesh.material.as_ref(), &[]);
        pass.set_bind_group(2, &self.frame, &[]);
        pass.set_bind_group(3, &draw.object.binding, &[]);
        pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        for (range, base) in &mesh.draws {
            pass.draw_indexed(range.clone(), *base, 0..1);
        }
    }
}

/// Input of one mesh upload.
pub(crate) struct DynamicObjectMeshInput<'a> {
    pub(crate) geometry: &'a NativeMeshGeometry,
    pub(crate) material: Arc<wgpu::BindGroup>,
    pub(crate) effect: DynamicObjectEffect,
    pub(crate) draw_order: u8,
}

fn float3(geometry: &NativeMeshGeometry, semantic: VertexSemantic) -> RenderResult<&[[f32; 3]]> {
    let attribute = geometry
        .attributes
        .iter()
        .find(|attribute| attribute.semantic == semantic)
        .ok_or_else(|| format!("dynamicobject mesh lacks {semantic:?}"))?;
    let NativeVertexAttributeValues::Float3(values) = &attribute.values else {
        return Err(format!("dynamicobject {semantic:?} is not three lanes").into());
    };
    if values.len() != geometry.positions.len() {
        return Err(format!("dynamicobject {semantic:?} count differs from positions").into());
    }
    Ok(values)
}

/// VS_IN `POSITION`, `TEXCOORD0`, `TANGENT`, `BINORMAL` as f32 lanes, and
/// the original indexed draws.
pub(super) fn upload_model(
    device: &wgpu::Device,
    meshes: Vec<DynamicObjectMeshInput<'_>>,
    bounds: [[f32; 3]; 2],
) -> RenderResult<GpuDynamicObjectModel> {
    let mut uploaded = Vec::with_capacity(meshes.len());
    for input in meshes {
        let geometry = input.geometry;
        if geometry.positions.is_empty() || geometry.indices.is_empty() {
            return Err("cannot upload an empty dynamicobject mesh".into());
        }
        let uv = geometry
            .attributes
            .iter()
            .find(|attribute| attribute.semantic == VertexSemantic::TextureCoordinate(0))
            .ok_or("dynamicobject mesh lacks TEXCOORD0")?;
        let NativeVertexAttributeValues::Float2(uv) = &uv.values else {
            return Err("dynamicobject TEXCOORD0 is not two lanes".into());
        };
        if uv.len() != geometry.positions.len() {
            return Err("dynamicobject TEXCOORD0 count differs from positions".into());
        }
        let unused_vectors = vec![[0.; 3]; geometry.positions.len()];
        let tangent = if input.effect == DynamicObjectEffect::VehicleLight {
            &unused_vectors
        } else {
            float3(geometry, VertexSemantic::Tangent(0))?
        };
        let binormal = if input.effect == DynamicObjectEffect::VehicleLight {
            &unused_vectors
        } else {
            float3(
                geometry,
                VertexSemantic::Other {
                    usage: USAGE_BINORMAL,
                    index: 0,
                },
            )?
        };
        let vehicle = matches!(
            input.effect,
            DynamicObjectEffect::Vehicle
                | DynamicObjectEffect::VehicleGlass
                | DynamicObjectEffect::VehicleLight
        );
        let mut extra = vec![[0.; 3]; geometry.positions.len()];
        if input.effect == DynamicObjectEffect::Decal {
            let decal = geometry
                .attributes
                .iter()
                .find(|a| a.semantic == VertexSemantic::TextureCoordinate(1))
                .ok_or("decal UV absent")?;
            let NativeVertexAttributeValues::Float2(decal) = &decal.values else {
                return Err("decal UV is not float2".into());
            };
            if decal.len() != extra.len() {
                return Err("decal UV count".into());
            }
            for (row, uv) in extra.iter_mut().zip(decal) {
                row[..2].copy_from_slice(uv);
            }
        }
        if vehicle {
            let indices = geometry
                .attributes
                .iter()
                .find(|a| a.semantic == VertexSemantic::BlendIndices(0))
                .ok_or("vehicle BLENDINDICES absent")?;
            let NativeVertexAttributeValues::UnsignedByte4(indices) = &indices.values else {
                return Err("vehicle BLENDINDICES are not bytes".into());
            };
            if indices.len() != extra.len() {
                return Err("vehicle blend index count".into());
            }
            for (row, index) in extra.iter_mut().zip(indices) {
                row[2] = f32::from(vehicle_bone_index(index));
            }
            if input.effect == DynamicObjectEffect::Vehicle {
                let uv1 = geometry
                    .attributes
                    .iter()
                    .find(|a| a.semantic == VertexSemantic::TextureCoordinate(1))
                    .ok_or("vehicle light-selection UV1 absent")?;
                let NativeVertexAttributeValues::Float2(uv1) = &uv1.values else {
                    return Err("vehicle UV1 is not float2".into());
                };
                if uv1.len() != extra.len() {
                    return Err("vehicle UV1 count".into());
                }
                for (row, uv) in extra.iter_mut().zip(uv1) {
                    row[..2].copy_from_slice(uv);
                }
            }
        }
        let mut draws = Vec::with_capacity(geometry.draws.len());
        for draw in &geometry.draws {
            if draw.primitive_type != PRIMITIVE_TYPE_TRIANGLE_LIST {
                return Err("dynamicobject draws must be original triangle lists".into());
            }
            let end = draw
                .start_index
                .checked_add(draw.index_count)
                .ok_or("dynamicobject index range overflow")?;
            let indices = geometry
                .indices
                .get(usize::try_from(draw.start_index)?..usize::try_from(end)?)
                .ok_or("dynamicobject draw outside its index buffer")?;
            for &index in indices {
                let vertex = i64::from(index) + i64::from(draw.base_vertex_index);
                if vertex < 0 || usize::try_from(vertex)? >= geometry.positions.len() {
                    return Err("dynamicobject draw references a missing vertex".into());
                }
            }
            draws.push((draw.start_index..end, draw.base_vertex_index));
        }
        let mut vertices = Vec::with_capacity(geometry.positions.len() * VERTEX_STRIDE as usize);
        for (index, position) in geometry.positions.iter().enumerate() {
            for value in position
                .iter()
                .chain(&uv[index])
                .chain(&tangent[index])
                .chain(&binormal[index])
                .chain(&extra[index])
            {
                vertices.extend_from_slice(&value.to_ne_bytes());
            }
        }
        let indices = geometry
            .indices
            .iter()
            .flat_map(|index| index.to_ne_bytes())
            .collect::<Vec<_>>();
        uploaded.push(GpuDynamicObjectMesh {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("original dynamicobject vertices"),
                contents: &vertices,
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("original dynamicobject indices"),
                contents: &indices,
                usage: wgpu::BufferUsages::INDEX,
            }),
            draws,
            material: input.material,
            effect: input.effect,
            draw_order: input.draw_order,
        });
    }
    Ok(GpuDynamicObjectModel {
        meshes: uploaded,
        bounds,
    })
}

impl DynamicObjectPipelines {
    /// One entity's `i_partArray`/`i_params` constant and bind group.
    pub(super) fn create_object(
        &self,
        device: &wgpu::Device,
        model: Arc<GpuDynamicObjectModel>,
    ) -> GpuDynamicObject {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dynamicobject i_partArray/i_params"),
            size: OBJECT_BYTES as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dynamicobject i_partArray/i_params"),
            layout: &self.object_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        GpuDynamicObject {
            model,
            uniform,
            binding,
        }
    }
}

/// The `shadow` technique of both effects: `simpleVS` into a cascade's
/// `g_matVP`, CULLMODE BACK, COLORWRITE 0 (no fragment stage), and the
/// stencil coverage `ShadowMaps` configures for every caster.
pub(super) fn caster_pipeline(
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    object_layout: &wgpu::BindGroupLayout,
    depth_stencil: wgpu::DepthStencilState,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("dynamicobject shadow technique (simpleVS)"),
        source: wgpu::ShaderSource::Wgsl(format!("{}{CASTER}", object_source(1)).into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("dynamicobject shadow technique"),
        bind_group_layouts: &[Some(camera_layout), Some(object_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        cache: None,
        label: Some("dynamicobject shadow technique"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            compilation_options: Default::default(),
            module: &module,
            entry_point: Some("vs_caster"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &CASTER_ATTRIBUTES,
            })],
        },
        fragment: None,
        primitive: RasterState::culled(CullMode::Back).primitive(),
        depth_stencil: Some(depth_stencil),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
    })
}

/// Records every mesh of `draw` into a cascade's caster pass; the pipeline
/// and cascade camera are bound.
pub(super) fn draw_caster<'a>(pass: &mut wgpu::RenderPass<'a>, draw: &'a DynamicObjectDraw) {
    pass.set_bind_group(1, &draw.object.binding, &[]);
    for mesh in &draw.object.model.meshes {
        pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        for (range, base) in &mesh.draws {
            pass.draw_indexed(range.clone(), *base, 0..1);
        }
    }
}

/// The object constant and the Part transform both vertex programs share,
/// declared in bind group `group`.
fn object_source(group: u32) -> String {
    format!(
        r"
struct DynamicObject {{ part: array<vec4<f32>, 4>, i_params: vec4<f32>, colours: array<vec4<f32>,2>, lights:vec4<f32>, irradiance:array<vec4<f32>,9>, bones:array<vec4<f32>,183> }}
@group({group}) @binding(0) var<uniform> object: DynamicObject;
// dynamicobject_defaultVS.vpo pc 9-11 / simpleVS pc 4-6:
// mul(half4(In.Pos, 1), partMatrix) as ((z * row2 + row3) + y * row1) +
// x * row0; row 3 is rebased at g_vViewPos.
fn part_point(p: vec3<f32>) -> vec3<f32> {{
    let r = p.z * object.part[2].xyz + object.part[3].xyz;
    return p.x * object.part[0].xyz + (p.y * object.part[1].xyz + r);
}}
fn entity_point(p: vec3<f32>, bone: f32) -> vec3<f32> {{
    if object.i_params.y == 0.0 {{ return part_point(p); }}
    let i = u32(bone)*3u;
    return vec3<f32>(dot(vec4<f32>(p,1.0),object.bones[i]),dot(vec4<f32>(p,1.0),object.bones[i+1u]),dot(vec4<f32>(p,1.0),object.bones[i+2u]));
}}
"
    )
}

const CASTER: &str = r"
struct ShadowCamera { clip_from_camera: mat4x4<f32> }
@group(0) @binding(0) var<uniform> shadow_camera: ShadowCamera;
// simpleVS: the Part position into the cascade's g_matVP; shadowPS's
// output is discarded by COLORWRITE 0.
@vertex fn vs_caster(@location(0) position: vec3<f32>, @location(5) bone: f32) -> @builtin(position) vec4<f32> {
    return shadow_camera.clip_from_camera * vec4<f32>(entity_point(position,bone), 1.0);
}
";

const DYNAMIC_OBJECT: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_detail: texture_2d<f32>;
@group(1) @binding(4) var i_environment: texture_cube<f32>;
@group(1) @binding(5) var wrap_sampler: sampler;
@group(1) @binding(6) var cube_sampler: sampler;
struct Material { m_params: array<vec4<f32>, 8>, detail_normal_uv_scale: vec4<f32> }
@group(1) @binding(7) var<uniform> material: Material;
@group(1) @binding(8) var i_decal: texture_2d<f32>;
@group(1) @binding(9) var decal_sampler: sampler;
@group(2) @binding(4) var shadow_world: texture_2d<f32>;
@group(2) @binding(5) var world_sampler: sampler;
struct WorldShadow { rows: array<vec4<f32>, 3> }
@group(2) @binding(6) var<uniform> world_shadow: WorldShadow;

struct PsIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) binormal: vec3<f32>,
    // vPos relative to g_vViewPos.
    @location(4) relative: vec3<f32>,
    @location(5) fog: vec4<f32>,
    // A park piece decal's UV (the UV1 lanes).
    @location(6) decal: vec2<f32>,
}

// dynamicobject_defaultVS.vpo pc 16-21: mul(v, partMatrix) as
// (z * row2 + y * row1) + x * row0.
fn part_vector(v: vec3<f32>) -> vec3<f32> {
    return v.x * object.part[0].xyz + (v.y * object.part[1].xyz + v.z * object.part[2].xyz);
}

// dynamicobject_params.fxh defaultVS: the Part-transformed tangent and
// binormal (not normalized), the normal their cross product (pc 23, 28),
// UV0, and ComputeFogFactor per vertex (pc 22, 30-38).
@vertex fn defaultVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) binormal: vec3<f32>,
    @location(4) uv1: vec2<f32>,
    @location(5) bone: f32
) -> PsIn {
    let relative = entity_point(position,bone);
    var output: PsIn;
    output.position = globals.clip_from_camera * vec4<f32>(relative, 1.0);
    output.relative = relative;
    output.tangent = part_vector(tangent);
    output.binormal = part_vector(binormal);
    output.normal = cross(output.tangent, output.binormal);
    output.uv = uv;
    output.decal = uv1;
    if object.i_params.y != 0.0 {
        let i = u32(bone)*3u;
        output.tangent = vec3<f32>(dot(tangent,object.bones[i].xyz),dot(tangent,object.bones[i+1u].xyz),dot(tangent,object.bones[i+2u].xyz));
        output.binormal = vec3<f32>(dot(binormal,object.bones[i].xyz),dot(binormal,object.bones[i+1u].xyz),dot(binormal,object.bones[i+2u].xyz));
        output.normal = cross(output.tangent,output.binormal);
        let selector = i32(round(uv1.x+2.0));
        var light = 0.0;
        if selector > 0 && selector < 5 { light = object.lights[u32(selector-1)]; }
        output.uv.x += uv1.y * clamp(light,0.0,1.0);
    }
    output.fog = compute_fog_factor(relative + globals.view_position.xyz);
    return output;
}

// One PCF3X3 tap: (z - bias.x >= moments.x) * (moments.y != 0).
fn pcf_tap(uv: vec2<f32>, depth: f32) -> f32 {
    let moments = textureSample(shadow_csm0, shadow_sampler, uv).xy;
    return select(0.0, 1.0, depth >= moments.x && moments.y != 0.0);
}

// CSM_ShadowMap_Self_PS as dynamicobject_defaultPS.fpo pc 17-97 compiled
// it: CSM_Shadow's uv (the first cascade wins), then nine taps stepped
// bias.z / 1024 in u and bias.z / 512 in v; 1 minus their mean.
fn csm_self_shadow(world: vec4<f32>, bias: vec3<f32>) -> f32 {
    let light_space = csm_shadow(world);
    let du = bias.z * 0.0009765625;
    let dv = bias.z * 0.001953125;
    let depth = light_space.z - bias.x;
    let uv = light_space.xy;
    var sum = pcf_tap(uv + vec2<f32>(-du, -dv), depth);
    sum += pcf_tap(uv + vec2<f32>(0.0, -dv), depth);
    sum += pcf_tap(uv + vec2<f32>(du, -dv), depth);
    sum += pcf_tap(uv + vec2<f32>(-du, 0.0), depth);
    sum += pcf_tap(uv, depth);
    sum += pcf_tap(uv + vec2<f32>(du, 0.0), depth);
    sum += pcf_tap(uv + vec2<f32>(-du, dv), depth);
    sum += pcf_tap(uv + vec2<f32>(0.0, dv), depth);
    sum += pcf_tap(uv + vec2<f32>(du, dv), depth);
    return 1.0 - sum * 0.111111112;
}

// CalculateWorldShadow as the program runs it (pc 52-93): shadowTexture_World
// at (0.5 x + 0.5, -0.5 y + 0.5); inside the map (dist < 0.99), the
// uncovered coverage where the stored depth is not in front.
fn world_shadow_value(world: vec4<f32>) -> f32 {
    let light_space = vec2<f32>(dot(world_shadow.rows[0], world), dot(world_shadow.rows[1], world));
    let depth = dot(world_shadow.rows[2], world);
    let dist = max(abs(light_space.x), abs(light_space.y)) - 0.99;
    let uv = vec2<f32>(light_space.x * 0.5 + 0.5, light_space.y * -0.5 + 0.5);
    let moments = textureSample(shadow_world, world_sampler, uv).xy;
    let in_front = select(0.0, 1.0, moments.x >= depth);
    let inside = select(0.0, 1.0, dist < 0.0);
    return inside * clamp(moments.y * in_front, 0.0, 1.0);
}

// GetSpecularTerm as the program runs it: (8, 1, 5) fresnel by squaring
// (pc 97-107), reflection luminosity 0.3 * saturate(4 * (masks.b - 1.5)),
// the reflection's xy negated, power 10 + masks.g * 290, tint (18, 15, 10.5)
// and threshold 0.1 (pc 109-121).
fn dynamic_object_specular(
    light: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    masks: vec3<f32>,
    luminosity: f32
) -> vec3<f32> {
    let facing = 1.0 - clamp(dot(normal, view_dir), 0.0, 1.0);
    let facing2 = facing * facing;
    let facing4 = facing2 * facing2;
    let fresnel = 4.0 * (facing4 * facing4) + 1.0;
    let reflection_luminosity = 0.3 * clamp(4.0 * (masks.b - 1.5), 0.0, 1.0);
    var reflect_vector = reflect(view_dir, normal);
    reflect_vector = vec3<f32>(-reflect_vector.xy, reflect_vector.z);
    let environment = textureSample(i_environment, cube_sampler, reflect_vector).rgb;
    let reflected = environment * (luminosity + (1.0 - luminosity) * reflection_luminosity)
        * masks.b * 1.5;
    let power = 10.0 + masks.g * 290.0;
    let calculated = phong_specular(normal, view_dir, light, power) * vec3<f32>(18.0, 15.0, 10.5)
        * clamp(luminosity - 0.1, 0.0, 1.0);
    return (calculated * masks.r + reflected) * fresnel;
}

// dynamicobject.fx defaultPS (and alphatestdynamicobject.fx, whose program
// has the same math) through PS_Fog_Bloom_Tone; alpha is the diffuse alpha.
fn dynamic_object_lit(input: PsIn) -> vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv);
    return dynamic_object_lit_surface(input, diffuse.rgb * diffuse.rgb, diffuse.a);
}

// The lighting above over a given linear surface colour.
fn dynamic_object_lit_surface(input: PsIn, surface: vec3<f32>, alpha: f32) -> vec4<f32> {
    let masks = textureSample(i_specular, wrap_sampler, input.uv).xyz;
    let detail = 2.0 * textureSample(i_detail, wrap_sampler, input.uv * material.detail_normal_uv_scale.x).xyz - 1.0;
    let tangent_normal = 2.0 * textureSample(i_normal, wrap_sampler, input.uv).xyz - 1.0;
    let view_dir = normalize(-input.relative);
    let nd = normalize(tangent_normal + vec3<f32>(detail.x, detail.y, 0.0));
    // half3x3(tangent, binormal, normal) has those as rows.
    let world_normal = normalize(nd.x * input.tangent + (nd.y * input.binormal + nd.z * input.normal));
    let world = vec4<f32>(input.relative + globals.view_position.xyz, 1.0);
    let shadow = csm_self_shadow(world, material.m_params[1].xyz) * world_shadow_value(world);
    let light = globals.light_direction.xyz;
    // GetTangentLight(vND, (0.64, 0.64, 0.420465), TransformToTangentFromWorld(L)):
    // only the signs of the tangent-space light reach it; 1 / normalize(ldir).z
    // is the compiled 2.37354684 (c254.x).
    let tangent_light = vec3<f32>(
        dot(input.tangent, light),
        dot(input.binormal, light),
        dot(input.normal, light)
    );
    let ldir = vec3<f32>(0.64 * sign(tangent_light.x), 0.64 * sign(tangent_light.y), 0.420465);
    let kd = dot(nd, ldir) * 2.37354684;
    let key_light = clamp(dot(world_normal, light), 0.0, 1.0);
    let bounce_light = clamp(dot(world_normal, -reflect(light, vec3<f32>(0.0, 1.0, 0.0))), 0.0, 1.0);
    let lighting = vec3<f32>(0.3, 0.3, 0.4) * bounce_light + vec3<f32>(key_light * shadow)
        + vec3<f32>(0.1, 0.1, 0.2);
    let diff_term = lighting * kd;
    let spec_term = dynamic_object_specular(light, world_normal, view_dir, masks, shadow);
    let out_colour = vec4<f32>(diff_term * surface + spec_term, 1.0);
    let tone = fog_bloom_tone(out_colour * material.m_params[0].y, input.fog, globals.envattributes2.x);
    return vec4<f32>(tone.rgb, alpha);
}

// defaultPS with environment_textures.fxh GetDecalTexture/ApplyDecal (as decalenvironment
// uses them): the squared decal over the squared diffuse by the decal's alpha.
@fragment fn decalPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv);
    let decal = textureSample(i_decal, decal_sampler, input.decal);
    let surface = mix(diffuse.rgb * diffuse.rgb, decal.rgb * decal.rgb, vec3<f32>(decal.a));
    let lit = dynamic_object_lit_surface(input, surface, diffuse.a);
    return vec4<f32>(lit.rgb, object.i_params.x);
}

// dynamicobject.fx:56: the returned alpha is i_params[0].x.
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let lit = dynamic_object_lit(input);
    return vec4<f32>(lit.rgb, object.i_params.x);
}

// Same native pixel program; pass1 enables GREATEREQUAL alpha test at1.
@fragment fn ghostPS(input: PsIn) -> @location(0) vec4<f32> {
    let lit = dynamic_object_lit(input);
    if (object.i_params.x < 0.0039215689) { discard; }
    return vec4<f32>(lit.rgb, object.i_params.x);
}

// alphatestdynamicobject.fx:57: the returned alpha is the diffuse alpha, which
// the pass alpha test (GREATEREQUAL 30) rejects on.
@fragment fn alphatestPS(input: PsIn) -> @location(0) vec4<f32> {
    let lit = dynamic_object_lit(input);
    if (lit.a < 30.0 * 0.0039215689) {
        discard;
    }
    return lit;
}
";
