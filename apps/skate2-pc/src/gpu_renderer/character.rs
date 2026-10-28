//! Original character geometry with diagnostic unlit diffuse shading.
//!
//! See `docs/evidence/character-gpu-skinning-source.md`. Supplied rows are the
//! actual per-mesh palette after native inverse-bind/world composition. This
//! consumer neither creates a pose nor substitutes missing presentation output.
//! The `ghost` technique keeps its original pass states around the same
//! diagnostic colour (`docs/evidence/skater-ghost-render-2026-09-27.md`).
//! Every pass keeps its manifest CULLMODE, and defaulthair draws its three
//! `default` passes with the original `i_alpha` coverage around that colour
//! (`docs/evidence/render-cull-bias-hair-decal-2026-09-27.md`).

use super::{
    DEPTH_FORMAT, GpuObject, GpuRenderer, GpuTexture, RenderResult,
    environment_shading::SOURCE_ALPHA_BLEND,
    multisample,
    raster_state::{CullMode, RasterState},
};
use skate2_formats::render::{
    NativeMeshGeometry, NativeVertexAttribute, NativeVertexAttributeValues, VERTEX_FORMAT_FLOAT2,
    VERTEX_FORMAT_FLOAT16_2, VERTEX_FORMAT_SHORT2N, VERTEX_FORMAT_SHORT4N, VERTEX_FORMAT_UBYTE4,
    VertexSemantic,
};
use std::{error::Error, fmt};
use wgpu::util::DeviceExt;

const MAX_BONES: usize = 61;
const ROW_BYTES: usize = (MAX_BONES * 3 + 5) * 16;
#[path = "character_stamp.rs"]
pub(crate) mod stamp;
#[path = "character_surface.rs"]
mod surface;
#[path = "character_cas.rs"]
pub(crate) mod cas;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterShaderFamily {
    /// Original `defaultcharacter` effect.
    Character,
    /// Original `defaulthair` effect.
    Hair,
    /// Original `defaulthair_ropa`: ropa vertex branch and unsquared specular mask.
    HairRopa,
    /// Original `defaultcharacter_ropa` effect; only its actual GPU branch.
    Ropa,
    /// Original defaultlivingworld effect, with its own colour/fog/tone path.
    LivingWorld,
    LivingWorldStamp,
}

impl CharacterShaderFamily {
    pub(crate) fn is_hair(self) -> bool {
        matches!(self, Self::Hair | Self::HairRopa)
    }

    /// The defaultcharacter and defaultcharacter_ropa manifests have a `ghost`
    /// technique. Neither hair manifest has one, so `cEffectList::SetCurrentEffect`
    /// 82e1c210 keeps its blended `default` technique, whose alpha is `i_alpha`
    /// times `i_params[0].x` (defaulthair.fx:61-64; `HAIR_PASSES`).
    pub fn has_ghost_technique(self) -> bool {
        !self.is_hair()
    }
}

/// Borrowed live publication, already ordered for this mesh's original palette.
pub struct CharacterPoseRows<'a> {
    /// Red, blue, fog K1, fog colour, and (material gain, exposure, 0, 0).
    pub living_world: Option<[[f32;4];5]>,
    pub rows: &'a [[f32; 4]],
    /// Column-major clip matrix for camera-relative positions
    /// (`CameraRelativeClip`), depth range 0..1.
    pub world_to_clip: &'a [f32; 16],
    /// The camera the rows' translations are rebased at on upload.
    pub camera_origin: [f32; 3],
    pub source_generation: u64,
    /// Actual `SkaterPresEntity` +910 or `cLivingWorldPresEntity` +220 X;
    /// required by both ropa families.
    pub ropa_vertex_params_x: Option<f32>,
    /// Actual retained `SkaterPresEntity` DoSimRestore output for every part.
    pub ghost: CharacterGhostState,
}

/// `SkaterPresentation::DoSimRestore`827e7298..73e0 output that the
/// `SkaterPresEntity` binds to every part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterGhostState {
    /// +0x870 `i_params[0].x`: the alpha every character pixel program writes.
    pub opacity: f32,
    /// +0x61a `mIsGhosted`: `SkaterPresEntity::Update`827e8730 then selects
    /// the `ghost` technique.
    pub ghosted: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CharacterGpuError {
    InvalidPaletteCount(u16),
    UnsupportedPositionFormat(u32),
    MissingOrDuplicateAttribute(VertexSemantic),
    UnsupportedAttributeFormat(VertexSemantic, u32),
    AttributeCount(VertexSemantic),
    PaletteIndex {
        vertex: usize,
        index: u8,
        count: u16,
    },
    RowCount {
        expected: usize,
        actual: usize,
    },
    MissingRopaParameter,
    CpuClothVerticesRequired,
    /// defaulthair must bind `i_alpha`, and no other family samples one.
    HairAlphaBinding,
    NotCharacterObject,
}

impl fmt::Display for CharacterGpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "character GPU admission: {self:?}")
    }
}

impl Error for CharacterGpuError {}

pub(super) struct CharacterBinding {
    living_exposure: ExposureOverride,
    /// Independent LivingWorld ShadowView inputs; None denotes the existing
    /// local-skater admission retained by ShadowCascades.
    pub(super) shadow_bounds:Option<([f32;3],[[f32;3];2])>,
    pub(super) selected_identity: Option<std::sync::Arc<()>>,
    pub(super) binding: wgpu::BindGroup,
    rows: wgpu::Buffer,
    palette_count: u16,
    pub(super) family: CharacterShaderFamily,
    generation: u64,
    ghosted: bool,
    /// defaulthair's `i_alpha` (group 3).
    pub(super) alpha: Option<wgpu::BindGroup>,
    pub(super) stamp: Option<stamp::Binding>,
    /// The material's `InstanceSortKey` draw-order byte (skater 9, hair
    /// 10); zero until the selected binding supplies it.
    pub(super) draw_order: u8,
}

/// Retain the native input independently of a PC display override. Updating
/// a palette resets the upload cache; retained pause frames can switch back.
pub(super) struct ExposureOverride {
    source: std::cell::Cell<f32>,
    uploaded: std::cell::Cell<f32>,
}
impl ExposureOverride {
    fn new(value: f32) -> Self { Self { source: value.into(), uploaded: value.into() } }
    fn supplied(&self, value: f32) { self.source.set(value); self.uploaded.set(value); }
    fn apply(&self, uploads: &super::frame_uploads::FrameUploads, buffer: &wgpu::Buffer, offset: u64, enabled: bool) {
        let value = if enabled { self.source.get() } else { 1. };
        if self.uploaded.replace(value) != value { uploads.write(buffer, offset, &value.to_ne_bytes()); }
    }
}
impl CharacterBinding {
    pub(super) fn apply_display_exposure(&self, uploads: &super::frame_uploads::FrameUploads, enabled: bool) {
        if matches!(self.family, CharacterShaderFamily::LivingWorld | CharacterShaderFamily::LivingWorldStamp) {
            self.living_exposure.apply(uploads, &self.rows, ((MAX_BONES * 3 + 4) * 16 + 4) as u64, enabled);
        }
        if let Some(stamp) = &self.stamp { stamp.apply_display_exposure(uploads, enabled); }
    }
}

pub(super) struct CharacterPipelines {
    row_layout: wgpu::BindGroupLayout,
    alpha_layout: wgpu::BindGroupLayout,
    alpha_sampler: wgpu::Sampler,
    character: wgpu::RenderPipeline,
    ropa: wgpu::RenderPipeline,
    stamp: stamp::Pipelines,
    /// defaulthair `default` passes 0-2.
    hair: [wgpu::RenderPipeline; 3],
    /// defaultcharacter(_ropa) `ghost` passes 0-1.
    ghost: [wgpu::RenderPipeline; 2],
}

impl CharacterPipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        object_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let row_layout = row_layout(device);
        let alpha_layout = alpha_layout(device);
        // defaulthair.fx `sampler i_alpha;` has no sampler_state and the
        // manifest no <Sampler> block for it, so `cFxParser::EndElement("Pass")`
        // 0x82e1f778 binds the reset cSamplerStateDefinition: WRAP, LINEAR
        // min/mag, POINT mip, anisotropy 1.
        let alpha_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("defaulthair i_alpha pass-default sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("source character skinning / diagnostic diffuse"),
            source: wgpu::ShaderSource::Wgsl(format!("{SKINNING}{SHADER}").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("character palette pipeline"),
            bind_group_layouts: &[Some(object_layout), Some(texture_layout), Some(&row_layout)],
            immediate_size: 0,
        });
        let hair_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("defaulthair palette/alpha pipeline"),
            bind_group_layouts: &[Some(object_layout), Some(texture_layout), Some(&row_layout), Some(&alpha_layout)],
            immediate_size: 0,
        });
        let pipeline = |pass: CharacterPass| {
            let layout = if pass.hair { &hair_layout } else { &layout };
            character_pipeline(device, &shader, layout, format, sample_count, pass)
        };
        Self {
            character: pipeline(DEFAULT_PASS),
            ropa: pipeline(DEFAULT_PASS),
            stamp: stamp::Pipelines::new(device,format,sample_count,object_layout,texture_layout,&row_layout),
            hair: HAIR_PASSES.map(&pipeline),
            ghost: GHOST_PASSES.map(&pipeline),
            row_layout,
            alpha_layout,
            alpha_sampler,
        }
    }

    /// The pass pipelines of the technique a part draws with: `ghost` for a
    /// ghosted part whose effect has it, else its `default`.
    /// `SceneRenderView::RenderMeshes` 82850670 draws single-pass meshes in
    /// order, then every multi-pass mesh pass-major, each only for the passes
    /// it has.
    pub(super) fn technique(&self, character: &CharacterBinding) -> &[wgpu::RenderPipeline] {
        self.technique_for(character, character.ghosted)
    }

    /// The `crowd` technique `CrowdImposterTarget::RenderPresEntity` 0x827e9298 sets for the
    /// imposter atlas: livingworld_stamp's own; every other effect lacks one, so
    /// `cEffectList::SetCurrentEffect` 82e1c210 takes its `default`.
    pub(super) fn crowd_technique(&self, character: &CharacterBinding) -> &[wgpu::RenderPipeline] {
        if character.family == CharacterShaderFamily::LivingWorldStamp && !character.stamp.as_ref().is_some_and(|b| b.cas) {
            return self.stamp.crowd_technique();
        }
        self.technique_for(character, false)
    }

    fn technique_for(&self, character: &CharacterBinding, ghosted: bool) -> &[wgpu::RenderPipeline] {
        if character.stamp.as_ref().is_some_and(|b|b.cas) {
            return self.stamp.cas_technique(character.family,ghosted);
        }
        if character.stamp.is_some() {
            return self.stamp.surface_technique(character.family,ghosted);
        }
        match character.family {
            CharacterShaderFamily::LivingWorldStamp => self.stamp.technique(ghosted),
            CharacterShaderFamily::LivingWorld => self.stamp.surface_technique(character.family,ghosted),
            family if ghosted && family.has_ghost_technique() => &self.ghost,
            CharacterShaderFamily::Character => std::slice::from_ref(&self.character),
            CharacterShaderFamily::Ropa => std::slice::from_ref(&self.ropa),
            CharacterShaderFamily::Hair | CharacterShaderFamily::HairRopa => &self.hair,
        }
    }

    /// The palette layout every part's bone rows bind with.
    pub(super) fn row_layout(&self) -> &wgpu::BindGroupLayout {
        &self.row_layout
    }

    fn alpha_binding(
        &self,
        device: &wgpu::Device,
        texture: &GpuTexture,
    ) -> RenderResult<wgpu::BindGroup> {
        if texture.cube {
            return Err("defaulthair i_alpha cannot be a cube map".into());
        }
        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("defaulthair i_alpha"),
            layout: &self.alpha_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.alpha_sampler),
                },
            ],
        }))
    }
}

impl GpuRenderer {
    pub(crate) fn set_character_shadow_bounds(&self,object:&mut GpuObject,
        position:[f32;3],bounds:[[f32;3];2])->RenderResult<()> {
        object.character.as_mut().ok_or(CharacterGpuError::NotCharacterObject)?
            .shadow_bounds=Some((position,bounds));
        Ok(())
    }
    pub(crate) fn set_character_draw_order(&self,object:&mut GpuObject,order:u8)->RenderResult<()> {
        object.character.as_mut().ok_or(CharacterGpuError::NotCharacterObject)?.draw_order=order;
        Ok(())
    }
    /// Uploads original character vertices with explicitly diagnostic shading.
    /// `alpha` is defaulthair's `i_alpha` texture, required for hair only. No
    /// lighting or CPU cloth branch is implied by this draw.
    pub fn create_character_object(
        &self,
        geometry: &NativeMeshGeometry,
        palette_count: u16,
        family: CharacterShaderFamily,
        pose: &CharacterPoseRows<'_>,
        diffuse: &GpuTexture,
        alpha: Option<&GpuTexture>,
    ) -> RenderResult<GpuObject> {
        let vertices = vertex_bytes(geometry, palette_count, family)?;
        let row_data = row_bytes(palette_count, family, pose)?;
        let alpha = match (family.is_hair(), alpha) {
            (true, Some(texture)) => Some(
                self.character_pipelines
                    .alpha_binding(&self.device, texture)?,
            ),
            (true, None) | (false, Some(_)) => {
                return Err(CharacterGpuError::HairAlphaBinding.into());
            }
            (_, None) => None,
        };
        // Reuse original indexed draw/base-vertex admission and ownership.
        let mut mesh = self.resource_builder().upload_mesh_dedicated(geometry)?;
        mesh.vertices = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("original character position UV weights indices"),
                contents: &vertices,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            })
            .into();
        let mut object =
            self.create_textured_object(mesh, pose.world_to_clip, &object_color(pose), diffuse)?;
        let rows = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("explicit live character palette rows"),
                contents: &row_data,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let binding = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("live character palette"),
            layout: &self.character_pipelines.row_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: rows.as_entire_binding(),
            }],
        });
        object.character = Some(CharacterBinding {
            living_exposure: ExposureOverride::new(pose.living_world.map_or(1., |p| p[4][1])),
            shadow_bounds:None,
            selected_identity: None,
            binding,
            rows,
            palette_count,
            family,
            generation: pose.source_generation,
            ghosted: pose.ghost.ghosted,
            alpha,
            stamp: None,
            draw_order: 0,
        });
        if !matches!(family,CharacterShaderFamily::LivingWorld|CharacterShaderFamily::LivingWorldStamp) { eprintln!(
            "diagnostic unlit diffuse character: {family:?}, {palette_count} palette bones, source generation {}; original material shading pending",
            pose.source_generation
        ); }
        Ok(object)
    }

    /// Updates the same retained GPU object from a checked live publication.
    pub fn update_character_object(
        &self,
        object: &mut GpuObject,
        pose: &CharacterPoseRows<'_>,
    ) -> RenderResult<()> {
        let character = object
            .character
            .as_mut()
            .ok_or(CharacterGpuError::NotCharacterObject)?;
        let data = row_bytes(character.palette_count, character.family, pose)?;
        self.character_uploads.write(&character.rows, 0, &data);
        character.living_exposure.supplied(pose.living_world.map_or(1., |p| p[4][1]));
        character.generation = pose.source_generation;
        character.ghosted = pose.ghost.ghosted;
        self.update_object(object, pose.world_to_clip, &object_color(pose));
        Ok(())
    }
}

fn attribute(
    geometry: &NativeMeshGeometry,
    semantic: VertexSemantic,
) -> Result<&NativeVertexAttribute, CharacterGpuError> {
    let mut matches = geometry
        .attributes
        .iter()
        .filter(|a| a.semantic == semantic);
    let attribute = matches
        .next()
        .ok_or(CharacterGpuError::MissingOrDuplicateAttribute(semantic))?;
    if matches.next().is_some() {
        return Err(CharacterGpuError::MissingOrDuplicateAttribute(semantic));
    }
    if attribute.values.len() != geometry.positions.len() {
        return Err(CharacterGpuError::AttributeCount(semantic));
    }
    Ok(attribute)
}

fn byte_attribute(
    geometry: &NativeMeshGeometry,
    semantic: VertexSemantic,
) -> Result<&[[u8; 4]], CharacterGpuError> {
    let attribute = attribute(geometry, semantic)?;
    if attribute.original_format == VERTEX_FORMAT_UBYTE4 {
        if let NativeVertexAttributeValues::UnsignedByte4(values) = &attribute.values {
            return Ok(values);
        }
    }
    Err(CharacterGpuError::UnsupportedAttributeFormat(
        semantic,
        attribute.original_format,
    ))
}

fn check_palette(count: u16) -> Result<(), CharacterGpuError> {
    if count == 0 || usize::from(count) > MAX_BONES {
        return Err(CharacterGpuError::InvalidPaletteCount(count));
    }
    Ok(())
}

fn uv_attribute(geometry: &NativeMeshGeometry, set: u8) -> Result<&[[f32; 2]], CharacterGpuError> {
    let uv = attribute(geometry, VertexSemantic::TextureCoordinate(set))?;
    let NativeVertexAttributeValues::Float2(values) = &uv.values else {
        return Err(CharacterGpuError::UnsupportedAttributeFormat(
            uv.semantic,
            uv.original_format,
        ));
    };
    if !matches!(
        uv.original_format,
        VERTEX_FORMAT_FLOAT16_2 | VERTEX_FORMAT_FLOAT2 | VERTEX_FORMAT_SHORT2N
    ) {
        return Err(CharacterGpuError::UnsupportedAttributeFormat(
            uv.semantic,
            uv.original_format,
        ));
    }
    Ok(values)
}

/// A UV set the mesh's vertex declaration may lack: `D3D::PatchVertexShaderToMatchVertexDeclaration`
/// 82c43eb0..bc fetches (0,0,0,1) for a shader input with no declaration element, so an absent
/// set reads as zero XY (`None`). An authored set is validated as usual, duplicates included.
fn declared_uv(geometry: &NativeMeshGeometry, set: u8) -> Result<Option<&[[f32; 2]]>, CharacterGpuError> {
    if geometry.attributes.iter().any(|a| a.semantic == VertexSemantic::TextureCoordinate(set)) {
        uv_attribute(geometry, set).map(Some)
    } else {
        Ok(None)
    }
}

fn vertex_bytes(
    geometry: &NativeMeshGeometry,
    palette_count: u16,
    family: CharacterShaderFamily,
) -> Result<Vec<u8>, CharacterGpuError> {
    check_palette(palette_count)?;
    if geometry.position_format != VERTEX_FORMAT_SHORT4N {
        return Err(CharacterGpuError::UnsupportedPositionFormat(
            geometry.position_format,
        ));
    }
    let weights = byte_attribute(geometry, VertexSemantic::BlendWeight(0))?;
    let indices = byte_attribute(geometry, VertexSemantic::BlendIndices(0))?;
    let uv_values = uv_attribute(geometry, 0)?;
    // defaulthair.fx VS_IN `UV1 : TEXCOORD1` addresses `i_alpha`; a mesh without the set (some
    // glasses) reads zero XY.
    let hair = matches!(family, CharacterShaderFamily::Hair | CharacterShaderFamily::HairRopa);
    let alpha_uv = if hair { declared_uv(geometry, 1)? } else { None };
    let stamp = if family==CharacterShaderFamily::LivingWorldStamp { Some(stamp::vectors(geometry)?) }else{None};
    let stride = if stamp.is_some(){52}else if hair { 36 } else { 28 };
    let mut result = Vec::with_capacity(geometry.positions.len() * stride);
    for (vertex, position) in geometry.positions.iter().enumerate() {
        for &index in &indices[vertex] {
            if u16::from(index) >= palette_count {
                return Err(CharacterGpuError::PaletteIndex {
                    vertex,
                    index,
                    count: palette_count,
                });
            }
        }
        result.extend(position.iter().flat_map(|v| v.to_ne_bytes()));
        result.extend(uv_values[vertex].iter().flat_map(|v| v.to_ne_bytes()));
        result.extend(weights[vertex]);
        result.extend(indices[vertex]);
        if hair {
            result.extend(alpha_uv.map_or([0.0; 2], |uv| uv[vertex]).iter().flat_map(|v| v.to_ne_bytes()));
        }
        if let Some((tangent,binormal))=stamp {
            result.extend(tangent[vertex].iter().chain(&binormal[vertex]).flat_map(|v|v.to_ne_bytes()));
        }
    }
    Ok(result)
}

fn row_bytes(
    count: u16,
    family: CharacterShaderFamily,
    pose: &CharacterPoseRows<'_>,
) -> Result<[u8; ROW_BYTES], CharacterGpuError> {
    check_palette(count)?;
    if matches!(family, CharacterShaderFamily::Ropa | CharacterShaderFamily::HairRopa) {
        let x = pose
            .ropa_vertex_params_x
            .ok_or(CharacterGpuError::MissingRopaParameter)?;
        // Original ordered > 0: unordered takes the actual CPU vertex branch.
        if x.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(CharacterGpuError::CpuClothVerticesRequired);
        }
    }
    let expected = usize::from(count) * 3;
    if pose.rows.len() != expected {
        return Err(CharacterGpuError::RowCount {
            expected,
            actual: pose.rows.len(),
        });
    }
    // The checked maximum is fixed: 61 bones * three float4 rows. Retain the
    // same zero padding and byte order without a Vec allocation/reallocation.
    // Each row's translation lane is rebased at the camera, so the skinned
    // positions are the camera-relative ones `world_to_clip` expects.
    let mut bytes = [0; ROW_BYTES];
    let lanes = pose.rows.iter().enumerate().flat_map(|(index, row)| {
        let mut row = *row;
        row[3] -= pose.camera_origin[index % 3];
        row
    });
    for (destination, value) in bytes.chunks_exact_mut(4).zip(lanes) {
        destination.copy_from_slice(&value.to_ne_bytes());
    }
    if let Some(parameters)=pose.living_world {
        for (destination,value) in bytes[MAX_BONES*3*16..].chunks_exact_mut(4).zip(parameters.into_iter().flatten()){
            destination.copy_from_slice(&value.to_ne_bytes());
        }
    } else if matches!(family,CharacterShaderFamily::LivingWorld|CharacterShaderFamily::LivingWorldStamp) {
        return Err(CharacterGpuError::NotCharacterObject);
    }
    Ok(bytes)
}

fn alpha_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("defaulthair i_alpha layout"),
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
    })
}

fn row_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("character row layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// One original technique pass: its vertex/pixel programs and the states its
/// manifest sets over the per-pass reset (`cFxParser::EndElement("Pass")`
/// 0x82e1f778: no blend or alpha test, ZENABLE and ZWRITEENABLE with ZFUNC
/// LESSEQUAL, CULLMODE NONE).
#[derive(Clone, Copy)]
struct CharacterPass {
    label: &'static str,
    vertex: &'static str,
    fragment: &'static str,
    /// None: the pass leaves ALPHABLENDENABLE at its false default.
    blend: Option<wgpu::BlendState>,
    write_mask: wgpu::ColorWrites,
    /// ZWRITEENABLE, or its alias DEPTHWRITEENABLE
    /// (`cDepthStencilStateHandle::SetState` 0x82e19368).
    depth_write: bool,
    cull: CullMode,
    /// defaulthair: TEXCOORD1 in the vertex stream and `i_alpha` in group 3.
    hair: bool,
    stencil: bool,
}

/// defaultcharacter.xml / defaultcharacter_ropa.xml `default` pass 0 (only
/// CULLMODE=FRONT) around the existing diagnostic unlit diffuse.
const DEFAULT_PASS: CharacterPass = CharacterPass {
    label: "diagnostic unlit diffuse skinned character",
    vertex: "vs_main",
    fragment: "fs_main",
    blend: None,
    write_mask: wgpu::ColorWrites::ALL,
    depth_write: true,
    cull: CullMode::Front,
    hair: false,
    stencil: true,
};

/// `ghost` pass 0: the default programs (vertex 0, pixel 1) with
/// CULLMODE=FRONT and COLORWRITE 0, so depth only.
const GHOST_DEPTH: CharacterPass = CharacterPass {
    label: "character ghost pass 0",
    write_mask: wgpu::ColorWrites::empty(),
    stencil: false,
    ..DEFAULT_PASS
};

/// `ghost` pass 1: SRCALPHA/INVSRCALPHA ADD for colour and alpha, ALPHATEST
/// GREATEREQUAL 1 (in `fs_ghost`), COLORWRITE RGB, CULLMODE=FRONT. Its
/// STENCILPASS REPLACE255 writes the shared main depth/stencil attachment.
const GHOST_BLEND: CharacterPass = CharacterPass {
    label: "character ghost pass 1",
    fragment: "fs_ghost",
    blend: Some(wgpu::BlendState {
        color: SOURCE_ALPHA_BLEND,
        alpha: SOURCE_ALPHA_BLEND,
    }),
    write_mask: wgpu::ColorWrites::COLOR,
    ..DEFAULT_PASS
};

const GHOST_PASSES: [CharacterPass; 2] = [GHOST_DEPTH, GHOST_BLEND];

/// defaulthair.xml `default` pass 0 (programs 0 and 1 in every pass):
/// SRCALPHA/INVSRCALPHA ADD (alpha the same), DEPTHWRITEENABLE FALSE,
/// COLORWRITE RGB, CULLMODE BACK so the faces turned away blend first. No
/// hair pass enables ALPHATEST. All three write STENCILPASS REPLACE255.
const HAIR_BLEND: CharacterPass = CharacterPass {
    label: "defaulthair pass 0",
    vertex: "vs_hair",
    fragment: "fs_hair",
    blend: Some(wgpu::BlendState {
        color: SOURCE_ALPHA_BLEND,
        alpha: SOURCE_ALPHA_BLEND,
    }),
    write_mask: wgpu::ColorWrites::COLOR,
    depth_write: false,
    cull: CullMode::Back,
    hair: true,
    stencil: true,
};

/// SRCBLEND ZERO / DESTBLEND ONE, BLENDOP ADD: the destination is kept.
const KEEP_DESTINATION: wgpu::BlendComponent = wgpu::BlendComponent {
    src_factor: wgpu::BlendFactor::Zero,
    dst_factor: wgpu::BlendFactor::One,
    operation: wgpu::BlendOperation::Add,
};

/// Pass 1 is pass 0 with CULLMODE FRONT. Pass 2 keeps the default depth
/// write, with COLORWRITE 0 and the ZERO/ONE blend, CULLMODE FRONT.
const HAIR_PASSES: [CharacterPass; 3] = [
    HAIR_BLEND,
    CharacterPass {
        label: "defaulthair pass 1",
        cull: CullMode::Front,
        ..HAIR_BLEND
    },
    CharacterPass {
        label: "defaulthair pass 2",
        blend: Some(wgpu::BlendState {
            color: KEEP_DESTINATION,
            alpha: KEEP_DESTINATION,
        }),
        write_mask: wgpu::ColorWrites::empty(),
        depth_write: true,
        cull: CullMode::Front,
        ..HAIR_BLEND
    },
];

/// Position, UV0, blend weights and indices (`vertex_bytes`).
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Uint8x4, 3 => Uint8x4];
/// The same lanes, then defaulthair's TEXCOORD1.
const HAIR_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    0 => Float32x3, 1 => Float32x2, 2 => Uint8x4, 3 => Uint8x4, 4 => Float32x2
];

/// The Object colour alpha carries `i_params[0].x` to `fs_ghost` and
/// `fs_hair`; `fs_main` does not read the colour.
fn object_color(pose: &CharacterPoseRows<'_>) -> [f32; 4] {
    [1.0, 1.0, 1.0, pose.ghost.opacity]
}

fn character_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    sample_count: u32,
    pass: CharacterPass,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        cache: None,
        label: Some(pass.label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            compilation_options: Default::default(),
            module: shader,
            entry_point: Some(pass.vertex),
            buffers: &[if matches!(pass.vertex,"vs_surface_hair"|"vs_cas") {
                Some(wgpu::VertexBufferLayout {array_stride:60,step_mode:wgpu::VertexStepMode::Vertex,
                    attributes:&surface::HAIR_ATTRIBUTES})
            } else if pass.vertex=="vs_stamp" {
                Some(wgpu::VertexBufferLayout {array_stride:52,step_mode:wgpu::VertexStepMode::Vertex,
                    attributes:&stamp::VERTEX_ATTRIBUTES})
            } else if pass.hair {
                Some(wgpu::VertexBufferLayout {
                    array_stride: 36,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &HAIR_VERTEX_ATTRIBUTES,
                })
            } else {
                Some(wgpu::VertexBufferLayout {
                    array_stride: 28,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &VERTEX_ATTRIBUTES,
                })
            }],
        },
        fragment: Some(wgpu::FragmentState {
            compilation_options: Default::default(),
            module: shader,
            entry_point: Some(pass.fragment),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: pass.blend,
                write_mask: pass.write_mask,
            })],
        }),
        primitive: RasterState::culled(pass.cull).primitive(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(pass.depth_write),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: if pass.stencil {
                let face=wgpu::StencilFaceState {compare:wgpu::CompareFunction::Always,
                    fail_op:wgpu::StencilOperation::Keep,depth_fail_op:wgpu::StencilOperation::Keep,
                    pass_op:wgpu::StencilOperation::Replace};
                wgpu::StencilState{front:face,back:face,read_mask:255,write_mask:255}
            }else{wgpu::StencilState::default()},
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: multisample(sample_count),
        multiview_mask: None,
    })
}

/// The skinning shared by every character program (`GetSkinMatrix_4` and
/// `OffsetPos(half4(0, 0.8, 0, 2), In.Pos)`); the including shader declares
/// its own `palette` binding.
pub(super) const SKINNING: &str = r"
struct Palette { rows: array<vec4<f32>, 183>, living_world: array<vec4<f32>,5> }
fn blend_row(indices: vec4<u32>, weights: vec4<f32>, row: u32) -> vec4<f32> {
    var value = weights.x * palette.rows[indices.x + row];
    value += weights.y * palette.rows[indices.y + row];
    value += weights.z * palette.rows[indices.z + row];
    value += weights.w * palette.rows[indices.w + row];
    return value;
}
fn skin_world(position: vec3<f32>, stored_weights: vec4<u32>,
              stored_indices: vec4<u32>) -> vec4<f32> {
    var weights = vec4<f32>(stored_weights) / 255.0;
    weights.w = 1.0 - dot(vec3<f32>(1.0), weights.xyz);
    let indices = stored_indices * 3u;
    let p = vec4<f32>(position * 2.0 + vec3<f32>(0.0, 0.8, 0.0), 1.0);
    return vec4<f32>(dot(p, blend_row(indices, weights, 0u)),
                     dot(p, blend_row(indices, weights, 1u)),
                     dot(p, blend_row(indices, weights, 2u)), 1.0);
}
";

const SHADER: &str = r"
struct Object { world_to_clip: mat4x4<f32>, color: vec4<f32> }
@group(0) @binding(0) var<uniform> object: Object;
@group(1) @binding(0) var diffuse: texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler: sampler;
@group(2) @binding(0) var<uniform> palette: Palette;
@group(3) @binding(0) var hair_alpha: texture_2d<f32>;
@group(3) @binding(1) var hair_alpha_sampler: sampler;
// Invariant: a multi-pass technique runs one vertex program for every pass,
// and later passes depth-test LESSEQUAL against the depth earlier ones wrote.
// uv: UV0 in xy; defaulthair `Out.UV = half4(In.UV0.xy, In.UV1.xy)`.
struct Output { @builtin(position) @invariant position: vec4<f32>, @location(0) @interpolate(perspective, centroid) uv: vec4<f32> }
fn skin_position(position: vec3<f32>, stored_weights: vec4<u32>,
                 stored_indices: vec4<u32>) -> vec4<f32> {
    return object.world_to_clip * skin_world(position, stored_weights, stored_indices);
}
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
                  @location(2) weights: vec4<u32>, @location(3) indices: vec4<u32>) -> Output {
    var output: Output;
    output.position = skin_position(position, weights, indices);
    output.uv = vec4<f32>(uv, 0.0, 0.0);
    return output;
}
@vertex fn vs_hair(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
                  @location(2) weights: vec4<u32>, @location(3) indices: vec4<u32>,
                  @location(4) alpha_uv: vec2<f32>) -> Output {
    var output: Output;
    // The shipped defaulthair_defaultVS.vpo writes oPos with the same four
    // dp4 as every character program: `GetZBias(z, 0)` compiled away.
    output.position = skin_position(position, weights, indices);
    output.uv = vec4<f32>(uv, alpha_uv);
    return output;
}
@fragment fn fs_main(@location(0) @interpolate(perspective, centroid) uv: vec4<f32>) -> @location(0) vec4<f32> {
    let sampled = textureSample(diffuse, diffuse_sampler, uv.xy);
    // Deliberately opaque unlit diffuse inspection on the gamma-2 surface.
    return vec4<f32>(sampled.rgb, 1.0);
}
// ghost pass 1: the default pixel program writes outcolor.a = i_params[0].x
// (defaultcharacter.fx:72, defaultcharacter_ropa.fx:84), carried in the
// Object colour alpha; ALPHATESTENABLE GREATEREQUAL ALPHAREF 1 rejects first.
@fragment fn fs_ghost(@location(0) @interpolate(perspective, centroid) uv: vec4<f32>) -> @location(0) vec4<f32> {
    let sampled = textureSample(diffuse, diffuse_sampler, uv.xy);
    let alpha = object.color.a;
    if (alpha < 0.0039215689) {
        discard;
    }
    return vec4<f32>(sampled.rgb, alpha);
}
// defaulthair.fx defaultPS coverage around the diagnostic colour:
// `vDiffuse.a = tex2D(i_alpha, In.UV.zw)` keeps the fetch's first lane and
// `outcolor.a = vDiffuse.a * i_params[0].x` (the Object colour alpha).
@fragment fn fs_hair(@location(0) @interpolate(perspective, centroid) uv: vec4<f32>) -> @location(0) vec4<f32> {
    let sampled = textureSample(diffuse, diffuse_sampler, uv.xy);
    let coverage = textureSample(hair_alpha, hair_alpha_sampler, uv.zw).r;
    return vec4<f32>(sampled.rgb, coverage * object.color.a);
}
";

#[cfg(test)]
#[path = "character_tests.rs"]
mod tests;

impl GpuRenderer {
    /// Update the already allocated selected vertex stream after native CPU
    /// skinning/cloth publication. Channel/layout ownership stays on the object.
    pub(crate) fn update_cpu_character_vertices(&self,object:&GpuObject,geometry:&NativeMeshGeometry,generation:u64)->RenderResult<()> {
        let c=object.character.as_ref().ok_or(CharacterGpuError::NotCharacterObject)?;
        if generation==c.generation{return Ok(());}
        let bytes=if c.stamp.as_ref().is_some_and(|s|s.cas) {
            cas::cas_vertex_bytes(geometry,c.palette_count,c.family)?
        }else if c.stamp.is_some() && matches!(c.family,CharacterShaderFamily::Character|CharacterShaderFamily::Hair|CharacterShaderFamily::HairRopa|CharacterShaderFamily::Ropa) {
            let base=vertex_bytes(geometry,c.palette_count,c.family)?;
            let (tangent,binormal)=stamp::vectors(geometry)?;
            let hair=c.family.is_hair();
            let mut bytes=Vec::with_capacity(geometry.positions.len()*if hair{60}else{52});
            for (index,vertex) in base.chunks_exact(if hair{36}else{28}).enumerate(){
                bytes.extend_from_slice(&vertex[..28]);
                bytes.extend(tangent[index].iter().chain(&binormal[index]).flat_map(|v|v.to_ne_bytes()));
                if hair{bytes.extend_from_slice(&vertex[28..36]);}
            }
            bytes
        }else {vertex_bytes(geometry,c.palette_count,c.family)?};
        if bytes.len() as u64!=object.mesh.vertices.size(){return Err("CPU cloth vertex update changed allocated layout/size".into());}
        self.queue.write_buffer(&object.mesh.vertices,0,&bytes);
        Ok(())
    }
    pub(crate) fn validate_cpu_character_stream(&self,geometry:&NativeMeshGeometry)->RenderResult<()> {
        let weights=byte_attribute(geometry,VertexSemantic::BlendWeight(0))?;
        let indices=byte_attribute(geometry,VertexSemantic::BlendIndices(0))?;
        if weights.len()!=geometry.positions.len() || indices.len()!=geometry.positions.len()
            || weights.iter().any(|w|*w!=[255,0,0,0]) || indices.iter().any(|i|*i!=[0;4]) {
            return Err("CPU cloth stream must carry explicit rigid root weights and indices".into());
        }
        Ok(())
    }
}
