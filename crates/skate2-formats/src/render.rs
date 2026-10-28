//! Human-readable forms of Skate 2's serialized presentation geometry.
//!
//! Type IDs come from the named arena registration routines. Pointer kinds
//! and inline-table rules come from the matching `Fixup` implementations.
//! Xbox GPU handles are retained only as serialized metadata needed to locate
//! and decode the original buffers; no Xbox rendering API is exposed here.

use std::{collections::BTreeMap, fmt};

use crate::{pegasus::ObjectId, rw4::Arena};

pub const R_MESH_DATA_TYPE_ID: u32 = 0x00eb_0000;
pub const R_MODEL_DATA_TYPE_ID: u32 = 0x00eb_0001;
pub const R_BLEND_SHAPE_DATA_TYPE_ID: u32 = 0x00eb_000e;
pub const R_OPTI_MESH_DATA_TYPE_ID: u32 = 0x00eb_0023;
pub const MESH_HELPER_TYPE_ID: u32 = 0x0002_0081;
pub const VERTEX_DESCRIPTOR_TYPE_ID: u32 = 0x0002_00e9;
pub const VERTEX_BUFFER_TYPE_ID: u32 = 0x0002_00ea;
pub const INDEX_BUFFER_TYPE_ID: u32 = 0x0002_00eb;
pub const GPU_BUFFER_RESOURCE_TYPE_ID: u32 = 0x0001_0031;

const R_MESH_DATA_SIZE: usize = 80;
const R_MODEL_DATA_SIZE: usize = 64;
const R_MODEL_MESH_SIZE: usize = 8;
const R_OPTI_MESH_DATA_SIZE: usize = 96;
const BLEND_SHAPE_INFO_SIZE: usize = 16;
const VERTEX_DESCRIPTOR_HEADER_SIZE: usize = 16;
const VERTEX_ELEMENT_SIZE: usize = 16;
const VERTEX_BUFFER_SIZE: usize = 40;
const INDEX_BUFFER_SIZE: usize = 36;
const MESH_HELPER_HEADER_SIZE: usize = 8;
const DRAW_INDEXED_PARAMETERS_SIZE: usize = 16;
pub const VERTEX_FORMAT_FLOAT3: u32 = 2_761_657;
pub const VERTEX_FORMAT_FLOAT1: u32 = 2_917_284;
pub const VERTEX_FORMAT_FLOAT2: u32 = 2_892_709;
pub const VERTEX_FORMAT_UBYTE4: u32 = 1_712_774;
pub const VERTEX_FORMAT_HEND3N: u32 = 2_761_104;
pub const VERTEX_FORMAT_SHORT2N: u32 = 2_892_121;
pub const VERTEX_FORMAT_SHORT4N: u32 = 1_712_474;
pub const VERTEX_FORMAT_SHORT2: u32 = 2_892_633;
pub const VERTEX_FORMAT_FLOAT16_2: u32 = 2_892_639;
pub const VERTEX_FORMAT_FLOAT16_4: u32 = 1_712_992;
pub const VERTEX_USAGE_POSITION: u8 = 0;
pub const VERTEX_USAGE_BLEND_WEIGHT: u8 = 1;
pub const VERTEX_USAGE_BLEND_INDICES: u8 = 2;
pub const VERTEX_USAGE_NORMAL: u8 = 3;
pub const VERTEX_USAGE_TEXTURE_COORDINATE: u8 = 5;
pub const VERTEX_USAGE_TANGENT: u8 = 6;
pub const VERTEX_USAGE_SAMPLE: u8 = 13;
pub const PRIMITIVE_TYPE_TRIANGLE_LIST: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisAlignedBox {
    pub minimum: [f32; 4],
    pub maximum: [f32; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelMesh {
    pub mesh: ObjectId,
    pub material: Option<ObjectId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderModelData {
    pub bounding_box: AxisAlignedBox,
    pub inverse_bind_pose_offset: Option<u32>,
    pub inverse_bind_pose: Vec<[f32; 16]>,
    pub mesh_table_offset: Option<u32>,
    pub bone_name_table_offset: Option<u32>,
    pub bone_name_list_offset: Option<u32>,
    pub bone_names: Vec<String>,
    pub total_bone_count: u16,
    pub bone_count: u16,
    pub island_count: u16,
    pub island_bounding_boxes_offset: Option<u32>,
    /// Authored `m_pIslandAABBs`, in the model's mesh/island order.
    /// Fixup829c70f4 adds the containing RModel base, not the pointer field.
    pub island_bounding_boxes: Vec<AxisAlignedBox>,
    pub island_areas_offset: Option<u32>,
    /// Authored `m_pIslandAreas`, four floats per island in the same order; the
    /// same fixup rebases it. Empty when the model has none.
    pub island_areas: Vec<[f32; 4]>,
    pub meshes: Vec<ModelMesh>,
}

impl RenderModelData {
    /// Reads only `tRModelData::m_iNumMeshes` at `+0x32`, as used by the
    /// Environment hybrid filter in `ProcessInstanceList`
    /// (`0x82d5eec4..0x82d5eee4`).
    ///
    /// This deliberately does not validate unrelated bone or mesh tables.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when bytes through field `+0x32` are absent.
    pub fn raw_mesh_count(bytes: &[u8]) -> Result<u16, RenderError> {
        be_u16(bytes, 50)
    }

    /// Parses `pegasus::tRModelData` and its inline eight-byte mesh table.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when the object or mesh table is truncated.
    pub fn parse(bytes: &[u8]) -> Result<Self, RenderError> {
        require(bytes, 0, R_MODEL_DATA_SIZE)?;
        let inverse_bind_pose_offset = optional_u32(bytes, 32)?;
        let mesh_table_offset = optional_u32(bytes, 36)?;
        let bone_name_table_offset = optional_u32(bytes, 40)?;
        let bone_name_list_offset = optional_u32(bytes, 44)?;
        let total_bone_count = be_u16(bytes, 48)?;
        let mesh_count = be_u16(bytes, 50)?;
        let bone_count = be_u16(bytes, 52)?;
        let inverse_bind_pose =
            parse_inverse_bind_pose(bytes, inverse_bind_pose_offset, usize::from(bone_count))?;
        let bone_names = parse_bone_names(
            bytes,
            bone_name_table_offset,
            bone_name_list_offset,
            usize::from(bone_count),
        )?;
        let mut meshes = Vec::with_capacity(usize::from(mesh_count));
        if mesh_count != 0 {
            let start = required_offset(mesh_table_offset, 36)?;
            let table = counted_range(bytes, start, usize::from(mesh_count), R_MODEL_MESH_SIZE)?;
            for index in 0..usize::from(mesh_count) {
                let offset = index * R_MODEL_MESH_SIZE;
                meshes.push(ModelMesh {
                    mesh: ObjectId(be_u32(table, offset)?),
                    material: optional_object_id(table, offset + 4)?,
                });
            }
        }
        let island_count = be_u16(bytes, 54)?;
        let island_bounding_boxes_offset = optional_u32(bytes, 56)?;
        let island_bounding_boxes = if island_count == 0 {
            Vec::new()
        } else {
            let start = required_offset(island_bounding_boxes_offset, 56)?;
            let count = usize::from(island_count);
            let table = counted_range(bytes, start, count, 32)?;
            (0..count)
                .map(|index| parse_aabb(table, index * 32))
                .collect::<Result<Vec<_>, RenderError>>()?
        };
        let island_areas_offset = optional_u32(bytes, 60)?;
        let island_areas = match island_areas_offset {
            Some(_) if island_count != 0 => {
                let start = required_offset(island_areas_offset, 60)?;
                let count = usize::from(island_count);
                let table = counted_range(bytes, start, count, 16)?;
                (0..count)
                    .map(|index| be_f32_array(table, index * 16))
                    .collect::<Result<Vec<_>, RenderError>>()?
            }
            _ => Vec::new(),
        };
        Ok(Self {
            bounding_box: parse_aabb(bytes, 0)?,
            inverse_bind_pose_offset,
            inverse_bind_pose,
            mesh_table_offset,
            bone_name_table_offset,
            bone_name_list_offset,
            bone_names,
            total_bone_count,
            bone_count,
            island_count,
            island_bounding_boxes_offset,
            island_bounding_boxes,
            island_areas_offset,
            island_areas,
            meshes,
        })
    }
}

fn parse_inverse_bind_pose(
    bytes: &[u8],
    offset: Option<u32>,
    count: usize,
) -> Result<Vec<[f32; 16]>, RenderError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let start = required_offset(offset, 32)?;
    counted_range(bytes, start, count, 64)?;
    (0..count)
        .map(|index| be_f32_array_16(bytes, start + index * 64))
        .collect()
}

fn parse_bone_names(
    bytes: &[u8],
    table_offset: Option<u32>,
    list_offset: Option<u32>,
    count: usize,
) -> Result<Vec<String>, RenderError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let table_start = required_offset(table_offset, 40)?;
    let list_start = required_offset(list_offset, 44)?;
    counted_range(bytes, table_start, count, 4)?;
    require(bytes, list_start, 0)?;
    (0..count)
        .map(|index| {
            let name_offset = as_usize(be_u32(bytes, table_start + index * 4)?)?;
            nul_terminated_string(bytes, name_offset)
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DrawParametersRef {
    Inline(u32),
    ArenaObject(ObjectId),
}

impl DrawParametersRef {
    #[must_use]
    pub const fn from_serialized(value: u32) -> Option<Self> {
        if value == 0 {
            None
        } else if value & 0x0080_0000 != 0 {
            Some(Self::ArenaObject(ObjectId(value)))
        } else {
            Some(Self::Inline(value))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlendShapeInfo {
    pub blend_shape: ObjectId,
    pub name_offset: u32,
    pub name_hash: u64,
}

/// Stream links in `pegasus::tRBlendShapeData`, a distinct 24-byte header.
/// CreateBlendJob827e89d0 consumes these links; the remaining fields describe
/// mesh membership and separate blend weights, not an ordinary mesh header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderBlendShapeData {
    pub vertex_descriptor: ObjectId,
    pub mesh_helper: ObjectId,
}

impl RenderBlendShapeData {
    /// Fixup829c6c50 resolves the words at +0/+4 as arena object IDs.
    ///
    /// # Errors
    /// Returns [`RenderError`] when the native header is truncated.
    pub fn parse(bytes: &[u8]) -> Result<Self, RenderError> {
        require(bytes, 0, 24)?;
        Ok(Self {
            vertex_descriptor: ObjectId(be_u32(bytes, 0)?),
            mesh_helper: ObjectId(be_u32(bytes, 4)?),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderMeshData {
    pub bounding_box: AxisAlignedBox,
    pub vertex_descriptor: ObjectId,
    pub mesh_helper: ObjectId,
    pub index_buffer: Option<ObjectId>,
    pub vertex_buffer: Option<ObjectId>,
    pub vertex_count: u32,
    pub draw_parameters_offset: u32,
    pub remap_table: ObjectId,
    pub is_indexed: bool,
    pub bone_matrix_count: u32,
    pub blend_shape_names_offset: Option<u32>,
    pub blend_shapes: Vec<BlendShapeInfo>,
}

impl RenderMeshData {
    /// Parses `pegasus::tRMeshData`, preserving arena IDs separately from
    /// object-relative tables exactly as its reference fixup does.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] for truncated headers or inline tables.
    pub fn parse(bytes: &[u8]) -> Result<Self, RenderError> {
        require(bytes, 0, R_MESH_DATA_SIZE)?;
        let blend_shape_count = be_u32(bytes, 68)?;
        let blend_shape_table_offset = optional_u32(bytes, 72)?;
        let mut blend_shapes = Vec::with_capacity(as_usize(blend_shape_count)?);
        if blend_shape_count != 0 {
            let start = required_offset(blend_shape_table_offset, 72)?;
            let table = counted_range(
                bytes,
                start,
                as_usize(blend_shape_count)?,
                BLEND_SHAPE_INFO_SIZE,
            )?;
            for index in 0..as_usize(blend_shape_count)? {
                let offset = index * BLEND_SHAPE_INFO_SIZE;
                blend_shapes.push(BlendShapeInfo {
                    blend_shape: ObjectId(be_u32(table, offset)?),
                    name_offset: be_u32(table, offset + 4)?,
                    name_hash: be_u64(table, offset + 8)?,
                });
            }
        }
        Ok(Self {
            bounding_box: parse_aabb(bytes, 0)?,
            vertex_descriptor: ObjectId(be_u32(bytes, 32)?),
            mesh_helper: ObjectId(be_u32(bytes, 36)?),
            index_buffer: optional_object_id(bytes, 40)?,
            vertex_buffer: optional_object_id(bytes, 44)?,
            vertex_count: be_u32(bytes, 48)?,
            draw_parameters_offset: be_u32(bytes, 52)?,
            remap_table: ObjectId(be_u32(bytes, 56)?),
            is_indexed: be_u32(bytes, 60)? != 0,
            bone_matrix_count: be_u32(bytes, 64)?,
            blend_shape_names_offset: optional_u32(bytes, 76)?,
            blend_shapes,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptimizedRenderMeshData {
    pub bounding_box: AxisAlignedBox,
    pub vertex_count: u32,
    pub material: Option<ObjectId>,
    pub vertex_descriptor: Option<ObjectId>,
    pub mesh_helper: Option<ObjectId>,
    pub index_buffer: Option<ObjectId>,
    pub vertex_buffer: Option<ObjectId>,
    pub island_count: u32,
    pub island_areas: Option<DrawParametersRef>,
    pub island_bounding_boxes: Option<DrawParametersRef>,
    pub island_draw_parameters_offset: Option<u32>,
    pub island_draw_parameters: Vec<DrawIndexedParameters>,
    pub remap_index_count: u32,
    pub remap_table_offset: Option<u32>,
    pub remap_indices: Vec<u16>,
    pub blend_shape_names_offset: Option<u32>,
    pub blend_shapes: Vec<BlendShapeInfo>,
}

impl OptimizedRenderMeshData {
    /// `ClothInit827d7d88` reads particle IDs through GetUncompressedVector.
    /// SHORT4N XYZ is signed normalized, but W is explicitly unsigned and
    /// unnormalized (8282f3ec..8282f41c). Do not use decode_attribute's Float4 W.
    pub fn cloth_particle_indices(&self,arena:&Arena,resources:&[Vec<u8>;5])->Result<Vec<u32>,RenderError>{
        let descriptor=VertexDescriptor::parse(resolve_object(arena,resources,
            self.vertex_descriptor.ok_or(RenderError::MissingMeshLink("vertex descriptor"))?,VERTEX_DESCRIPTOR_TYPE_ID)?)?;
        let helper=MeshHelper::parse(resolve_object(arena,resources,
            self.mesh_helper.ok_or(RenderError::MissingMeshLink("mesh helper"))?,MESH_HELPER_TYPE_ID)?)?;
        let position=descriptor.position_element().ok_or(RenderError::MissingPositionElement)?;
        if position.format!=VERTEX_FORMAT_SHORT4N {
            return Err(RenderError::UnsupportedPositionFormat(position.format));
        }
        let stride=*descriptor.stream_strides()?.get(&position.stream).ok_or(RenderError::MissingVertexStream(position.stream))?;
        let buffer=VertexBuffer::parse(resolve_object(arena,resources,
            *helper.vertex_buffers.get(usize::from(position.stream)).ok_or(RenderError::MissingVertexStream(position.stream))?,VERTEX_BUFFER_TYPE_ID)?)?;
        if stride==0 || buffer.buffer_size%u32::from(stride)!=0 {
            return Err(RenderError::InvalidVertexStride{buffer_size:buffer.buffer_size,stride});
        }
        let raw=resolve_object(arena,resources,buffer.data,GPU_BUFFER_RESOURCE_TYPE_ID)?;
        require(raw,0,as_usize(buffer.buffer_size)?)?;
        (0..as_usize(buffer.buffer_size/u32::from(stride))?).map(|index|{
            let offset=index.checked_mul(usize::from(stride))
                .and_then(|n|n.checked_add(usize::from(position.offset)+6)).ok_or(RenderError::IntegerOverflow)?;
            Ok(u32::from(be_u16(raw,offset)?))
        }).collect()
    }

    /// DoSkinMesh_scalar82836964 uses CoordIndex.X when its native element
    /// type is26; otherwise the driver index is the position-W ClothInit index.
    pub fn cloth_driver_particle_indices(&self,arena:&Arena,resources:&[Vec<u8>;5])->Result<Vec<u32>,RenderError>{
        let descriptor=VertexDescriptor::parse(resolve_object(arena,resources,
            self.vertex_descriptor.ok_or(RenderError::MissingMeshLink("vertex descriptor"))?,VERTEX_DESCRIPTOR_TYPE_ID)?)?;
        let Some(element)=descriptor.elements.iter().rev().find(|e|e.element_type==26).copied() else {
            return self.cloth_particle_indices(arena,resources);
        };
        let helper=MeshHelper::parse(resolve_object(arena,resources,
            self.mesh_helper.ok_or(RenderError::MissingMeshLink("mesh helper"))?,MESH_HELPER_TYPE_ID)?)?;
        let stride=*descriptor.stream_strides()?.get(&element.stream).ok_or(RenderError::MissingVertexStream(element.stream))?;
        let buffer=VertexBuffer::parse(resolve_object(arena,resources,
            *helper.vertex_buffers.get(usize::from(element.stream)).ok_or(RenderError::MissingVertexStream(element.stream))?,VERTEX_BUFFER_TYPE_ID)?)?;
        let raw=resolve_object(arena,resources,buffer.data,GPU_BUFFER_RESOURCE_TYPE_ID)?;
        buffer.vertex_element_offsets(raw,element,stride)?.into_iter().map(|offset|{
            let x=match element.format {
                VERTEX_FORMAT_SHORT2=>f32::from(be_i16(raw,offset)?),
                VERTEX_FORMAT_UBYTE4=>f32::from(byte(raw,offset)?),
                VERTEX_FORMAT_FLOAT3=>be_f32(raw,offset)?,
                VERTEX_FORMAT_FLOAT16_4=>{
                    let h=be_u16(raw,offset)?;
                    // GetUncompressedVector's original conversion flushes zero
                    // exponent and reconstructs the implicit low mantissa bits.
                    if (h>>10)&31==0 {-0.}else{
                        f32::from_bits((u32::from(h&0x8000)<<16)|((u32::from((h>>10)&31)+112)<<23)|(u32::from(h&1023)<<13)|(u32::from(h&1023)<<3))
                    }
                },
                VERTEX_FORMAT_SHORT4N=>f32::from(be_i16(raw,offset)?)*f32::from_bits(0x38000100),
                _=>return Err(RenderError::UnknownVertexFormat(element.format)),
            };
            if !x.is_finite() || x<0. || f64::from(x)>f64::from(i32::MAX){return Err(RenderError::IntegerOverflow);}
            Ok(x as u32)
        }).collect()
    }

    /// Parses `pegasus::tROptiMeshData` and its mixed local/arena references.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] for truncated headers or inline tables.
    pub fn parse(bytes: &[u8]) -> Result<Self, RenderError> {
        require(bytes, 0, R_OPTI_MESH_DATA_SIZE)?;
        let blend_shape_count = be_u32(bytes, 80)?;
        let blend_shape_table_offset = optional_u32(bytes, 84)?;
        let mut blend_shapes = Vec::with_capacity(as_usize(blend_shape_count)?);
        if blend_shape_count != 0 {
            let start = required_offset(blend_shape_table_offset, 84)?;
            let table = counted_range(
                bytes,
                start,
                as_usize(blend_shape_count)?,
                BLEND_SHAPE_INFO_SIZE,
            )?;
            for index in 0..as_usize(blend_shape_count)? {
                let offset = index * BLEND_SHAPE_INFO_SIZE;
                blend_shapes.push(BlendShapeInfo {
                    name_hash: be_u64(table, offset)?,
                    blend_shape: ObjectId(be_u32(table, offset + 8)?),
                    name_offset: be_u32(table, offset + 12)?,
                });
            }
        }
        let island_count = be_u32(bytes, 56)?;
        let island_draw_parameters_offset = optional_u32(bytes, 68)?;
        let remap_index_count = be_u32(bytes, 72)?;
        let remap_table_offset = optional_u32(bytes, 76)?;
        let remap_indices =
            parse_u16_table(bytes, remap_table_offset, as_usize(remap_index_count)?, 76)?;
        let mut island_draw_parameters = Vec::with_capacity(as_usize(island_count)?);
        if island_count != 0 {
            let start = required_offset(island_draw_parameters_offset, 68)?;
            let table = counted_range(
                bytes,
                start,
                as_usize(island_count)?,
                DRAW_INDEXED_PARAMETERS_SIZE,
            )?;
            for index in 0..as_usize(island_count)? {
                let offset = index * DRAW_INDEXED_PARAMETERS_SIZE;
                island_draw_parameters.push(DrawIndexedParameters {
                    primitive_type: be_u32(table, offset)?,
                    base_vertex_index: be_i32(table, offset + 4)?,
                    start_index: be_u32(table, offset + 8)?,
                    index_count: be_u32(table, offset + 12)?,
                });
            }
        }
        Ok(Self {
            bounding_box: parse_aabb(bytes, 0)?,
            vertex_count: be_u32(bytes, 32)?,
            material: optional_object_id(bytes, 36)?,
            vertex_descriptor: optional_object_id(bytes, 40)?,
            mesh_helper: optional_object_id(bytes, 44)?,
            index_buffer: optional_object_id(bytes, 48)?,
            vertex_buffer: optional_object_id(bytes, 52)?,
            island_count,
            island_areas: DrawParametersRef::from_serialized(be_u32(bytes, 60)?),
            island_bounding_boxes: DrawParametersRef::from_serialized(be_u32(bytes, 64)?),
            island_draw_parameters_offset,
            island_draw_parameters,
            remap_index_count,
            remap_table_offset,
            remap_indices,
            blend_shape_names_offset: optional_u32(bytes, 88)?,
            blend_shapes,
        })
    }

    /// Resolves the declaration, helper, buffers, and raw-resource graph and
    /// converts this mesh to native positions and indices.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] for invalid arena links, unsupported position
    /// data, an out-of-range index, or an invalid draw range.
    pub fn decode_geometry(
        &self,
        arena: &Arena,
        resources: &[Vec<u8>; 5],
    ) -> Result<NativeMeshGeometry, RenderError> {
        let descriptor_id = self
            .vertex_descriptor
            .ok_or(RenderError::MissingMeshLink("vertex descriptor"))?;
        let helper_id = self
            .mesh_helper
            .ok_or(RenderError::MissingMeshLink("mesh helper"))?;
        let descriptor = VertexDescriptor::parse(resolve_object(
            arena,
            resources,
            descriptor_id,
            VERTEX_DESCRIPTOR_TYPE_ID,
        )?)?;
        let helper = MeshHelper::parse(resolve_object(
            arena,
            resources,
            helper_id,
            MESH_HELPER_TYPE_ID,
        )?)?;
        let position = descriptor
            .position_element()
            .ok_or(RenderError::MissingPositionElement)?;
        let stream_strides = descriptor.stream_strides()?;
        let stride = stream_strides
            .get(&position.stream)
            .copied()
            .ok_or(RenderError::MissingVertexStream(position.stream))?;
        let vertex_buffer_id = helper
            .vertex_buffers
            .get(usize::from(position.stream))
            .copied()
            .ok_or(RenderError::MissingVertexStream(position.stream))?;
        let vertex_buffer = VertexBuffer::parse(resolve_object(
            arena,
            resources,
            vertex_buffer_id,
            VERTEX_BUFFER_TYPE_ID,
        )?)?;
        let vertex_data = resolve_object(
            arena,
            resources,
            vertex_buffer.data,
            GPU_BUFFER_RESOURCE_TYPE_ID,
        )?;
        let positions = vertex_buffer.decode_positions(vertex_data, position, stride)?;
        let attributes = decode_vertex_attributes(
            &descriptor,
            &helper,
            &stream_strides,
            arena,
            resources,
            position,
            positions.len(),
        )?;

        let index_buffer_id = self
            .index_buffer
            .or_else(|| helper.index_buffers.first().copied())
            .ok_or(RenderError::MissingMeshLink("index buffer"))?;
        let index_buffer = IndexBuffer::parse(resolve_object(
            arena,
            resources,
            index_buffer_id,
            INDEX_BUFFER_TYPE_ID,
        )?)?;
        let index_data = resolve_object(
            arena,
            resources,
            index_buffer.data,
            GPU_BUFFER_RESOURCE_TYPE_ID,
        )?;
        let indices = index_buffer.decode_indices(index_data)?;
        validate_mesh_indices(
            &indices,
            positions.len(),
            &self.island_draw_parameters,
            index_buffer.index_count,
        )?;
        Ok(NativeMeshGeometry {
            positions,
            attributes,
            indices,
            draws: self.island_draw_parameters.clone(),
            position_format: position.format,
        })
    }
}

fn parse_u16_table(
    bytes: &[u8],
    offset: Option<u32>,
    count: usize,
    field_offset: usize,
) -> Result<Vec<u16>, RenderError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let start = required_offset(offset, field_offset)?;
    counted_range(bytes, start, count, 2)?;
    (0..count)
        .map(|index| be_u16(bytes, start + index * 2))
        .collect()
}

fn validate_mesh_indices(
    indices: &[u32],
    vertex_count: usize,
    draws: &[DrawIndexedParameters],
    index_count: u32,
) -> Result<(), RenderError> {
    if draws.is_empty()
        && indices.iter().copied().max().is_some_and(|maximum| {
            usize::try_from(maximum).map_or(true, |value| value >= vertex_count)
        })
    {
        return Err(RenderError::IndexOutsideVertexBuffer);
    }
    for draw in draws {
        let end = draw
            .start_index
            .checked_add(draw.index_count)
            .ok_or(RenderError::IntegerOverflow)?;
        if end > index_count {
            return Err(RenderError::DrawOutsideIndexBuffer);
        }
        let draw_indices = indices
            .get(as_usize(draw.start_index)?..as_usize(end)?)
            .ok_or(RenderError::DrawOutsideIndexBuffer)?;
        if draw_indices.iter().any(|index| {
            i64::from(*index)
                .checked_add(i64::from(draw.base_vertex_index))
                .and_then(|index| usize::try_from(index).ok())
                .is_none_or(|index| index >= vertex_count)
        }) {
            return Err(RenderError::IndexOutsideVertexBuffer);
        }
    }
    Ok(())
}

fn decode_vertex_attributes(
    descriptor: &VertexDescriptor,
    helper: &MeshHelper,
    stream_strides: &BTreeMap<u16, u16>,
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    position: VertexElement,
    vertex_count: usize,
) -> Result<Vec<NativeVertexAttribute>, RenderError> {
    let mut attributes = Vec::with_capacity(descriptor.elements.len().saturating_sub(1));
    for element in descriptor
        .elements
        .iter()
        .copied()
        .filter(|element| *element != position)
    {
        let stride = stream_strides
            .get(&element.stream)
            .copied()
            .ok_or(RenderError::MissingVertexStream(element.stream))?;
        let buffer_id = helper
            .vertex_buffers
            .get(usize::from(element.stream))
            .copied()
            .ok_or(RenderError::MissingVertexStream(element.stream))?;
        let buffer = VertexBuffer::parse(resolve_object(
            arena,
            resources,
            buffer_id,
            VERTEX_BUFFER_TYPE_ID,
        )?)?;
        let data = resolve_object(arena, resources, buffer.data, GPU_BUFFER_RESOURCE_TYPE_ID)?;
        let attribute = buffer.decode_attribute(data, element, stride)?;
        if attribute.values.len() != vertex_count {
            return Err(RenderError::VertexAttributeCountMismatch {
                semantic: attribute.semantic,
                expected: vertex_count,
                actual: attribute.values.len(),
            });
        }
        attributes.push(attribute);
    }
    Ok(attributes)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DrawIndexedParameters {
    pub primitive_type: u32,
    pub base_vertex_index: i32,
    pub start_index: u32,
    pub index_count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeMeshGeometry {
    pub positions: Vec<[f32; 3]>,
    pub attributes: Vec<NativeVertexAttribute>,
    pub indices: Vec<u32>,
    pub draws: Vec<DrawIndexedParameters>,
    pub position_format: u32,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum VertexSemantic {
    Position(u8),
    BlendWeight(u8),
    BlendIndices(u8),
    Normal(u8),
    TextureCoordinate(u8),
    Tangent(u8),
    Sample(u8),
    Other { usage: u8, index: u8 },
}

impl VertexSemantic {
    #[must_use]
    pub const fn from_usage(usage: u8, index: u8) -> Self {
        match usage {
            VERTEX_USAGE_POSITION => Self::Position(index),
            VERTEX_USAGE_BLEND_WEIGHT => Self::BlendWeight(index),
            VERTEX_USAGE_BLEND_INDICES => Self::BlendIndices(index),
            VERTEX_USAGE_NORMAL => Self::Normal(index),
            VERTEX_USAGE_TEXTURE_COORDINATE => Self::TextureCoordinate(index),
            VERTEX_USAGE_TANGENT => Self::Tangent(index),
            VERTEX_USAGE_SAMPLE => Self::Sample(index),
            _ => Self::Other { usage, index },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum NativeVertexAttributeValues {
    Float2(Vec<[f32; 2]>),
    Float3(Vec<[f32; 3]>),
    Float4(Vec<[f32; 4]>),
    UnsignedByte4(Vec<[u8; 4]>),
    SignedShort2(Vec<[i16; 2]>),
}

impl NativeVertexAttributeValues {
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Float2(values) => values.len(),
            Self::Float3(values) => values.len(),
            Self::Float4(values) => values.len(),
            Self::UnsignedByte4(values) => values.len(),
            Self::SignedShort2(values) => values.len(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn all_finite(&self) -> bool {
        match self {
            Self::Float2(values) => values.iter().flatten().all(|value| value.is_finite()),
            Self::Float3(values) => values.iter().flatten().all(|value| value.is_finite()),
            Self::Float4(values) => values.iter().flatten().all(|value| value.is_finite()),
            Self::UnsignedByte4(_) | Self::SignedShort2(_) => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeVertexAttribute {
    pub semantic: VertexSemantic,
    pub original_format: u32,
    pub values: NativeVertexAttributeValues,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VertexElement {
    pub stream: u16,
    pub offset: u16,
    pub format: u32,
    pub method: u8,
    pub usage: u8,
    pub usage_index: u8,
    pub element_type: u8,
    pub element_class: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VertexDescriptor {
    pub type_flags: u32,
    pub reference_count: i16,
    pub instance_streams: u16,
    pub elements: Vec<VertexElement>,
}

impl VertexDescriptor {
    /// Parses the flexible-array vertex declaration used to interpret native
    /// GPU buffer bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when the element array is truncated.
    pub fn parse(bytes: &[u8]) -> Result<Self, RenderError> {
        require(bytes, 0, VERTEX_DESCRIPTOR_HEADER_SIZE)?;
        let element_count = usize::from(be_u16(bytes, 8)?);
        let elements_bytes = counted_range(
            bytes,
            VERTEX_DESCRIPTOR_HEADER_SIZE,
            element_count,
            VERTEX_ELEMENT_SIZE,
        )?;
        let mut elements = Vec::with_capacity(element_count);
        for index in 0..element_count {
            let offset = index * VERTEX_ELEMENT_SIZE;
            elements.push(VertexElement {
                stream: be_u16(elements_bytes, offset)?,
                offset: be_u16(elements_bytes, offset + 2)?,
                format: be_u32(elements_bytes, offset + 4)?,
                method: byte(elements_bytes, offset + 8)?,
                usage: byte(elements_bytes, offset + 9)?,
                usage_index: byte(elements_bytes, offset + 10)?,
                element_type: byte(elements_bytes, offset + 11)?,
                element_class: be_u32(elements_bytes, offset + 12)?,
            });
        }
        Ok(Self {
            type_flags: be_u32(bytes, 4)?,
            reference_count: be_i16(bytes, 10)?,
            instance_streams: be_u16(bytes, 12)?,
            elements,
        })
    }

    /// Calculates the tightly addressed byte stride of every declared stream.
    /// Widths are the exact formats present in the BAM presentation corpus and
    /// named by the reference `renderengine::VertexFormat` enum.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] for an unknown format or arithmetic overflow.
    pub fn stream_strides(&self) -> Result<BTreeMap<u16, u16>, RenderError> {
        let mut strides: BTreeMap<u16, u16> = BTreeMap::new();
        for element in &self.elements {
            let width = vertex_format_width(element.format)
                .ok_or(RenderError::UnknownVertexFormat(element.format))?;
            let end = element
                .offset
                .checked_add(width)
                .ok_or(RenderError::IntegerOverflow)?;
            strides
                .entry(element.stream)
                .and_modify(|stride| *stride = (*stride).max(end))
                .or_insert(end);
        }
        Ok(strides)
    }

    /// Finds the primary position element identified by the reference D3D
    /// declaration usage values.
    #[must_use]
    pub fn position_element(&self) -> Option<VertexElement> {
        self.elements
            .iter()
            .copied()
            .find(|element| element.usage == VERTEX_USAGE_POSITION && element.usage_index == 0)
    }
}

/// Returns the serialized byte width of every vertex format observed in the
/// complete BAM presentation corpus.
#[must_use]
pub const fn vertex_format_width(format: u32) -> Option<u16> {
    Some(match format {
        // SHORT4N, FLOAT16_4, FLOAT2
        1_712_474 | 1_712_992 | 2_892_709 => 8,
        // UBYTE4, HEND3N, SHORT2N, SHORT2, FLOAT16_2, FLOAT1
        1_712_774 | 2_761_104 | 2_892_121 | 2_892_633 | 2_892_639 | VERTEX_FORMAT_FLOAT1 => 4,
        2_761_657 => 12, // FLOAT3
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VertexBuffer {
    pub data: ObjectId,
    pub fetch_flags: u8,
    pub buffer_size: u32,
    pub buffer_type: u32,
}

impl VertexBuffer {
    /// Parses the serializable portion of `renderengine::VertexBuffer`. The
    /// upper 30 bits of fetch word zero are an arena object ID; its lower two
    /// bits are preserved GPU fetch flags.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when the fixed record is truncated.
    pub fn parse(bytes: &[u8]) -> Result<Self, RenderError> {
        require(bytes, 0, VERTEX_BUFFER_SIZE)?;
        let fetch = be_u32(bytes, 24)?;
        Ok(Self {
            data: ObjectId(fetch >> 2),
            fetch_flags: (fetch & 3) as u8,
            buffer_size: be_u32(bytes, 32)?,
            buffer_type: be_u32(bytes, 36)?,
        })
    }

    /// Decodes a stream's primary `FLOAT3` positions into native floats.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] for unsupported position formats, invalid
    /// strides, or truncated raw GPU resource data.
    pub fn decode_positions(
        self,
        raw: &[u8],
        element: VertexElement,
        stride: u16,
    ) -> Result<Vec<[f32; 3]>, RenderError> {
        if !matches!(
            element.format,
            VERTEX_FORMAT_FLOAT3 | VERTEX_FORMAT_FLOAT16_4 | VERTEX_FORMAT_SHORT4N
        ) {
            return Err(RenderError::UnsupportedPositionFormat(element.format));
        }
        if stride == 0 || self.buffer_size % u32::from(stride) != 0 {
            return Err(RenderError::InvalidVertexStride {
                buffer_size: self.buffer_size,
                stride,
            });
        }
        let count = as_usize(self.buffer_size / u32::from(stride))?;
        require(raw, 0, as_usize(self.buffer_size)?)?;
        let mut positions = Vec::with_capacity(count);
        for index in 0..count {
            let offset = index
                .checked_mul(usize::from(stride))
                .and_then(|offset| offset.checked_add(usize::from(element.offset)))
                .ok_or(RenderError::IntegerOverflow)?;
            positions.push(match element.format {
                VERTEX_FORMAT_FLOAT3 => [
                    be_f32(raw, offset)?,
                    be_f32(raw, offset + 4)?,
                    be_f32(raw, offset + 8)?,
                ],
                VERTEX_FORMAT_FLOAT16_4 => [
                    half_to_f32(be_u16(raw, offset)?),
                    half_to_f32(be_u16(raw, offset + 2)?),
                    half_to_f32(be_u16(raw, offset + 4)?),
                ],
                VERTEX_FORMAT_SHORT4N => [
                    normalize_i16(be_i16(raw, offset)?),
                    normalize_i16(be_i16(raw, offset + 2)?),
                    normalize_i16(be_i16(raw, offset + 4)?),
                ],
                _ => unreachable!("position format was checked above"),
            });
        }
        Ok(positions)
    }

    /// Decodes any vertex element format present in the complete BAM world
    /// corpus into an ordinary native value array.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] for unknown formats, invalid strides, or
    /// truncated raw resource data.
    pub fn decode_attribute(
        self,
        raw: &[u8],
        element: VertexElement,
        stride: u16,
    ) -> Result<NativeVertexAttribute, RenderError> {
        let offsets = self.vertex_element_offsets(raw, element, stride)?;
        let values = match element.format {
            VERTEX_FORMAT_FLOAT2 => NativeVertexAttributeValues::Float2(
                offsets
                    .into_iter()
                    .map(|offset| Ok([be_f32(raw, offset)?, be_f32(raw, offset + 4)?]))
                    .collect::<Result<Vec<_>, RenderError>>()?,
            ),
            VERTEX_FORMAT_FLOAT3 => NativeVertexAttributeValues::Float3(
                offsets
                    .into_iter()
                    .map(|offset| {
                        Ok([
                            be_f32(raw, offset)?,
                            be_f32(raw, offset + 4)?,
                            be_f32(raw, offset + 8)?,
                        ])
                    })
                    .collect::<Result<Vec<_>, RenderError>>()?,
            ),
            VERTEX_FORMAT_FLOAT16_2 => NativeVertexAttributeValues::Float2(
                offsets
                    .into_iter()
                    .map(|offset| {
                        Ok([
                            half_to_f32(be_u16(raw, offset)?),
                            half_to_f32(be_u16(raw, offset + 2)?),
                        ])
                    })
                    .collect::<Result<Vec<_>, RenderError>>()?,
            ),
            VERTEX_FORMAT_FLOAT16_4 => NativeVertexAttributeValues::Float4(
                offsets
                    .into_iter()
                    .map(|offset| {
                        Ok([
                            half_to_f32(be_u16(raw, offset)?),
                            half_to_f32(be_u16(raw, offset + 2)?),
                            half_to_f32(be_u16(raw, offset + 4)?),
                            half_to_f32(be_u16(raw, offset + 6)?),
                        ])
                    })
                    .collect::<Result<Vec<_>, RenderError>>()?,
            ),
            VERTEX_FORMAT_SHORT2N => NativeVertexAttributeValues::Float2(
                offsets
                    .into_iter()
                    .map(|offset| {
                        Ok([
                            normalize_i16(be_i16(raw, offset)?),
                            normalize_i16(be_i16(raw, offset + 2)?),
                        ])
                    })
                    .collect::<Result<Vec<_>, RenderError>>()?,
            ),
            VERTEX_FORMAT_SHORT4N => NativeVertexAttributeValues::Float4(
                offsets
                    .into_iter()
                    .map(|offset| {
                        Ok([
                            normalize_i16(be_i16(raw, offset)?),
                            normalize_i16(be_i16(raw, offset + 2)?),
                            normalize_i16(be_i16(raw, offset + 4)?),
                            normalize_i16(be_i16(raw, offset + 6)?),
                        ])
                    })
                    .collect::<Result<Vec<_>, RenderError>>()?,
            ),
            VERTEX_FORMAT_HEND3N => NativeVertexAttributeValues::Float3(
                offsets
                    .into_iter()
                    .map(|offset| be_u32(raw, offset).map(decode_hend3n))
                    .collect::<Result<Vec<_>, RenderError>>()?,
            ),
            VERTEX_FORMAT_UBYTE4 => NativeVertexAttributeValues::UnsignedByte4(
                offsets
                    .into_iter()
                    .map(|offset| {
                        require(raw, offset, 4)
                            .map(|value| [value[0], value[1], value[2], value[3]])
                    })
                    .collect::<Result<Vec<_>, RenderError>>()?,
            ),
            VERTEX_FORMAT_SHORT2 => NativeVertexAttributeValues::SignedShort2(
                offsets
                    .into_iter()
                    .map(|offset| Ok([be_i16(raw, offset)?, be_i16(raw, offset + 2)?]))
                    .collect::<Result<Vec<_>, RenderError>>()?,
            ),
            other => return Err(RenderError::UnknownVertexFormat(other)),
        };
        Ok(NativeVertexAttribute {
            semantic: VertexSemantic::from_usage(element.usage, element.usage_index),
            original_format: element.format,
            values,
        })
    }

    fn vertex_element_offsets(
        self,
        raw: &[u8],
        element: VertexElement,
        stride: u16,
    ) -> Result<Vec<usize>, RenderError> {
        if stride == 0 || self.buffer_size % u32::from(stride) != 0 {
            return Err(RenderError::InvalidVertexStride {
                buffer_size: self.buffer_size,
                stride,
            });
        }
        require(raw, 0, as_usize(self.buffer_size)?)?;
        let count = as_usize(self.buffer_size / u32::from(stride))?;
        (0..count)
            .map(|index| {
                index
                    .checked_mul(usize::from(stride))
                    .and_then(|offset| offset.checked_add(usize::from(element.offset)))
                    .ok_or(RenderError::IntegerOverflow)
            })
            .collect()
    }
}

fn normalize_i16(value: i16) -> f32 {
    (f32::from(value) / f32::from(i16::MAX)).max(-1.0)
}

fn decode_hend3n(packed: u32) -> [f32; 3] {
    let x = sign_extend(packed & 0x07ff, 11);
    let y = sign_extend((packed >> 11) & 0x07ff, 11);
    let z = sign_extend(packed >> 22, 10);
    [
        (f32::from(x) / 1023.0).max(-1.0),
        (f32::from(y) / 1023.0).max(-1.0),
        (f32::from(z) / 511.0).max(-1.0),
    ]
}

fn sign_extend(value: u32, bits: u32) -> i16 {
    let raw = i32::try_from(value).expect("packed vertex component fits i32");
    let sign = 1_i32 << (bits - 1);
    let signed = if raw & sign == 0 {
        raw
    } else {
        raw - (1_i32 << bits)
    };
    i16::try_from(signed).expect("packed vertex component fits i16")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndexBuffer {
    pub data: ObjectId,
    pub aligned_buffer_size: u32,
    pub index_count: u32,
    pub index_depth: u8,
}

impl IndexBuffer {
    /// Parses `renderengine::IndexBuffer`, including the reference 16/32-bit
    /// depth encoding in the D3D common word.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when the fixed record is truncated.
    pub fn parse(bytes: &[u8]) -> Result<Self, RenderError> {
        require(bytes, 0, INDEX_BUFFER_SIZE)?;
        let common = be_u32(bytes, 0)?;
        Ok(Self {
            data: ObjectId(be_u32(bytes, 24)?),
            aligned_buffer_size: be_u32(bytes, 28)?,
            index_count: be_u32(bytes, 32)?,
            index_depth: if common & 0xc000_0000 == 0xc000_0000 {
                32
            } else {
                16
            },
        })
    }

    /// Converts the original big-endian 16- or 32-bit index stream to native
    /// `u32` indices. Alignment padding is excluded by `index_count`.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when the raw GPU resource is truncated.
    pub fn decode_indices(self, raw: &[u8]) -> Result<Vec<u32>, RenderError> {
        let width = usize::from(self.index_depth / 8);
        let count = as_usize(self.index_count)?;
        require(
            raw,
            0,
            count
                .checked_mul(width)
                .ok_or(RenderError::IntegerOverflow)?,
        )?;
        (0..count)
            .map(|index| {
                let offset = index * width;
                if self.index_depth == 16 {
                    be_u16(raw, offset).map(u32::from)
                } else {
                    be_u32(raw, offset)
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeshHelper {
    pub index_buffers: Vec<ObjectId>,
    pub vertex_buffers: Vec<ObjectId>,
}

impl MeshHelper {
    /// Parses the flexible arena-ID table fixed by `MeshHelper::Fixup`.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when the ID array is truncated.
    pub fn parse(bytes: &[u8]) -> Result<Self, RenderError> {
        require(bytes, 0, MESH_HELPER_HEADER_SIZE)?;
        let index_count = as_usize(be_u32(bytes, 0)?)?;
        let vertex_count = as_usize(be_u32(bytes, 4)?)?;
        let total = index_count
            .checked_add(vertex_count)
            .ok_or(RenderError::IntegerOverflow)?;
        let ids = counted_range(bytes, MESH_HELPER_HEADER_SIZE, total, 4)?;
        let parsed = (0..total)
            .map(|index| be_u32(ids, index * 4).map(ObjectId))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            index_buffers: parsed[..index_count].to_vec(),
            vertex_buffers: parsed[index_count..].to_vec(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderError {
    Truncated {
        offset: usize,
        required: usize,
        actual: usize,
    },
    IntegerOverflow,
    MissingOffset {
        field_offset: usize,
    },
    UnknownVertexFormat(u32),
    UnsupportedPositionFormat(u32),
    InvalidVertexStride {
        buffer_size: u32,
        stride: u16,
    },
    MissingMeshLink(&'static str),
    MissingArenaObject(ObjectId),
    WrongArenaObjectType {
        id: ObjectId,
        expected: u32,
        actual: u32,
    },
    MissingPositionElement,
    MissingVertexStream(u16),
    VertexAttributeCountMismatch {
        semantic: VertexSemantic,
        expected: usize,
        actual: usize,
    },
    IndexOutsideVertexBuffer,
    DrawOutsideIndexBuffer,
    InvalidString {
        offset: usize,
    },
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Skate 2 render object: {self:?}")
    }
}

impl std::error::Error for RenderError {}

fn parse_aabb(bytes: &[u8], offset: usize) -> Result<AxisAlignedBox, RenderError> {
    Ok(AxisAlignedBox {
        minimum: be_f32_array(bytes, offset)?,
        maximum: be_f32_array(bytes, offset + 16)?,
    })
}

fn resolve_object<'a>(
    arena: &Arena,
    resources: &'a [Vec<u8>; 5],
    id: ObjectId,
    expected_type: u32,
) -> Result<&'a [u8], RenderError> {
    let entry = arena
        .local_entry(id)
        .ok_or(RenderError::MissingArenaObject(id))?;
    if entry.type_id != expected_type {
        return Err(RenderError::WrongArenaObjectType {
            id,
            expected: expected_type,
            actual: entry.type_id,
        });
    }
    arena
        .object_bytes(resources, entry)
        .ok_or(RenderError::MissingArenaObject(id))
}

fn optional_object_id(bytes: &[u8], offset: usize) -> Result<Option<ObjectId>, RenderError> {
    Ok(optional_u32(bytes, offset)?.map(ObjectId))
}

fn optional_u32(bytes: &[u8], offset: usize) -> Result<Option<u32>, RenderError> {
    Ok(match be_u32(bytes, offset)? {
        0 => None,
        value => Some(value),
    })
}

fn required_offset(offset: Option<u32>, field_offset: usize) -> Result<usize, RenderError> {
    offset
        .ok_or(RenderError::MissingOffset { field_offset })
        .and_then(as_usize)
}

fn counted_range(
    bytes: &[u8],
    start: usize,
    count: usize,
    stride: usize,
) -> Result<&[u8], RenderError> {
    let size = count
        .checked_mul(stride)
        .ok_or(RenderError::IntegerOverflow)?;
    require(bytes, start, size)
}

fn require(bytes: &[u8], offset: usize, size: usize) -> Result<&[u8], RenderError> {
    let end = offset
        .checked_add(size)
        .ok_or(RenderError::IntegerOverflow)?;
    bytes.get(offset..end).ok_or(RenderError::Truncated {
        offset,
        required: end,
        actual: bytes.len(),
    })
}

fn as_usize(value: u32) -> Result<usize, RenderError> {
    usize::try_from(value).map_err(|_| RenderError::IntegerOverflow)
}

fn byte(bytes: &[u8], offset: usize) -> Result<u8, RenderError> {
    bytes.get(offset).copied().ok_or(RenderError::Truncated {
        offset,
        required: offset.saturating_add(1),
        actual: bytes.len(),
    })
}

fn be_u16(bytes: &[u8], offset: usize) -> Result<u16, RenderError> {
    require(bytes, offset, 2).map(|field| u16::from_be_bytes(field.try_into().unwrap()))
}

fn be_i16(bytes: &[u8], offset: usize) -> Result<i16, RenderError> {
    require(bytes, offset, 2).map(|field| i16::from_be_bytes(field.try_into().unwrap()))
}

fn be_i32(bytes: &[u8], offset: usize) -> Result<i32, RenderError> {
    require(bytes, offset, 4).map(|field| i32::from_be_bytes(field.try_into().unwrap()))
}

fn be_u32(bytes: &[u8], offset: usize) -> Result<u32, RenderError> {
    require(bytes, offset, 4).map(|field| u32::from_be_bytes(field.try_into().unwrap()))
}

fn be_u64(bytes: &[u8], offset: usize) -> Result<u64, RenderError> {
    require(bytes, offset, 8).map(|field| u64::from_be_bytes(field.try_into().unwrap()))
}

fn be_f32(bytes: &[u8], offset: usize) -> Result<f32, RenderError> {
    be_u32(bytes, offset).map(f32::from_bits)
}

fn half_to_f32(value: u16) -> f32 {
    let sign = u32::from(value & 0x8000) << 16;
    let exponent = u32::from((value >> 10) & 0x1f);
    let mantissa = u32::from(value & 0x03ff);
    let bits = match exponent {
        0 if mantissa == 0 => sign,
        0 => {
            let leading = mantissa.leading_zeros() - 22;
            let normalized = (mantissa << (leading + 1)) & 0x03ff;
            let f32_exponent = 112_u32 - leading;
            sign | (f32_exponent << 23) | (normalized << 13)
        }
        31 => sign | 0x7f80_0000 | (mantissa << 13),
        _ => sign | ((exponent + 112) << 23) | (mantissa << 13),
    };
    f32::from_bits(bits)
}

fn be_f32_array(bytes: &[u8], offset: usize) -> Result<[f32; 4], RenderError> {
    let mut values = [0.0; 4];
    for (index, value) in values.iter_mut().enumerate() {
        *value = f32::from_bits(be_u32(bytes, offset + index * 4)?);
    }
    Ok(values)
}

fn be_f32_array_16(bytes: &[u8], offset: usize) -> Result<[f32; 16], RenderError> {
    let mut values = [0.0; 16];
    for (index, value) in values.iter_mut().enumerate() {
        *value = be_f32(bytes, offset + index * 4)?;
    }
    Ok(values)
}

fn nul_terminated_string(bytes: &[u8], offset: usize) -> Result<String, RenderError> {
    let tail = bytes
        .get(offset..)
        .ok_or(RenderError::InvalidString { offset })?;
    let length = tail
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(RenderError::InvalidString { offset })?;
    std::str::from_utf8(&tail[..length])
        .map(str::to_owned)
        .map_err(|_| RenderError::InvalidString { offset })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_f32(bytes: &mut [u8], offset: usize, value: f32) {
        put_u32(bytes, offset, value.to_bits());
    }

    #[test]
    fn parses_model_mesh_arena_ids() {
        let mut bytes = vec![0; 72];
        put_u32(&mut bytes, 36, 64);
        put_u16(&mut bytes, 50, 1);
        put_u32(&mut bytes, 64, 3);
        put_u32(&mut bytes, 68, 4);
        let model = RenderModelData::parse(&bytes).unwrap();
        assert_eq!(
            model.meshes,
            vec![ModelMesh {
                mesh: ObjectId(3),
                material: Some(ObjectId(4)),
            }]
        );
        assert_eq!(RenderModelData::raw_mesh_count(&bytes), Ok(1));
    }

    #[test]
    fn render_model_raw_mesh_count_does_not_validate_unread_fields() {
        let mut bytes = vec![0; 52];
        put_u16(&mut bytes, 50, 7);
        assert_eq!(RenderModelData::raw_mesh_count(&bytes), Ok(7));
        assert!(RenderModelData::parse(&bytes).is_err());
    }

    #[test]
    fn model_island_bounds_use_object_relative_table_and_preserve_w() {
        let mut bytes = vec![0; 160];
        put_u16(&mut bytes, 54, 2);
        put_u32(&mut bytes, 56, 96);
        // Both boxes are distinct from the model box at zero. Native culling
        // reads the authored table, not a box reconstructed from vertices.
        for (index, value) in [
            -3.0, -2.0, -1.0, 7.0, 3.0, 2.0, 1.0, 8.0, 10.0, 20.0, 30.0, 9.0, 40.0, 50.0, 60.0,
            10.0,
        ]
        .into_iter()
        .enumerate()
        {
            put_f32(&mut bytes, 96 + index * 4, value);
        }
        let model = RenderModelData::parse(&bytes).unwrap();
        assert_eq!(
            model.island_bounding_boxes,
            vec![
                AxisAlignedBox {
                    minimum: [-3.0, -2.0, -1.0, 7.0],
                    maximum: [3.0, 2.0, 1.0, 8.0]
                },
                AxisAlignedBox {
                    minimum: [10.0, 20.0, 30.0, 9.0],
                    maximum: [40.0, 50.0, 60.0, 10.0]
                },
            ]
        );
        assert!(RenderModelData::parse(&bytes[..159]).is_err());
        put_u32(&mut bytes, 56, 0);
        assert!(RenderModelData::parse(&bytes).is_err());
        put_u16(&mut bytes, 54, 0);
        assert!(
            RenderModelData::parse(&bytes)
                .unwrap()
                .island_bounding_boxes
                .is_empty()
        );
    }

    #[test]
    fn parses_model_inverse_bind_pose_and_bone_names() {
        let mut bytes = vec![0; 224];
        put_u32(&mut bytes, 32, 64);
        put_u32(&mut bytes, 36, 192);
        put_u32(&mut bytes, 40, 200);
        put_u32(&mut bytes, 44, 208);
        put_u16(&mut bytes, 48, 1);
        put_u16(&mut bytes, 50, 1);
        put_u16(&mut bytes, 52, 2);
        put_f32(&mut bytes, 64, 1.0);
        put_f32(&mut bytes, 64 + 64 + 5 * 4, 2.0);
        put_u32(&mut bytes, 192, 3);
        put_u32(&mut bytes, 200, 208);
        put_u32(&mut bytes, 204, 213);
        bytes[208..224].copy_from_slice(b"Root\0SkateBoard\0");

        let model = RenderModelData::parse(&bytes).unwrap();
        assert_eq!(model.total_bone_count, 1);
        assert_eq!(model.inverse_bind_pose.len(), 2);
        assert_eq!(model.inverse_bind_pose[0][0].to_bits(), 1.0_f32.to_bits());
        assert_eq!(model.inverse_bind_pose[1][5].to_bits(), 2.0_f32.to_bits());
        assert_eq!(model.bone_names, ["Root", "SkateBoard"]);
    }

    #[test]
    fn parses_optimized_mesh_skin_palette_indices() {
        let mut bytes = vec![0; 102];
        put_u32(&mut bytes, 72, 3);
        put_u32(&mut bytes, 76, 96);
        put_u16(&mut bytes, 96, 7);
        put_u16(&mut bytes, 98, 2);
        put_u16(&mut bytes, 100, 19);

        let mesh = OptimizedRenderMeshData::parse(&bytes).unwrap();
        assert_eq!(mesh.remap_index_count, 3);
        assert_eq!(mesh.remap_indices, [7, 2, 19]);
    }

    #[test]
    fn decodes_vertex_fetch_object_id_and_flags() {
        let mut bytes = vec![0; VERTEX_BUFFER_SIZE];
        put_u32(&mut bytes, 24, (0x0080_0002 << 2) | 3);
        put_u32(&mut bytes, 32, 4096);
        let vertex_buffer = VertexBuffer::parse(&bytes).unwrap();
        assert_eq!(vertex_buffer.data, ObjectId(0x0080_0002));
        assert_eq!(vertex_buffer.fetch_flags, 3);
        assert_eq!(vertex_buffer.buffer_size, 4096);
    }

    #[test]
    fn parses_mesh_helper_buffer_groups() {
        let mut bytes = vec![0; 20];
        put_u32(&mut bytes, 0, 1);
        put_u32(&mut bytes, 4, 2);
        put_u32(&mut bytes, 8, 7);
        put_u32(&mut bytes, 12, 8);
        put_u32(&mut bytes, 16, 9);
        let helper = MeshHelper::parse(&bytes).unwrap();
        assert_eq!(helper.index_buffers, vec![ObjectId(7)]);
        assert_eq!(helper.vertex_buffers, vec![ObjectId(8), ObjectId(9)]);
    }

    #[test]
    fn blend_shape_has_its_own_header_and_scalar_weight_stride() {
        let mut bytes = vec![0; 28];
        put_u32(&mut bytes, 0, 13); // vertex descriptor
        put_u32(&mut bytes, 4, 12); // mesh helper
        put_u32(&mut bytes, 8, 1); // mesh membership count
        put_u32(&mut bytes, 12, 24); // relative membership table
        // Unused pointer word when blend-weight count at +16 is zero.
        put_u32(&mut bytes, 20, 0xea05_97e0);
        put_u32(&mut bytes, 24, 0x0040_0000);
        assert_eq!(RenderBlendShapeData::parse(&bytes).unwrap(), RenderBlendShapeData {
            vertex_descriptor: ObjectId(13),
            mesh_helper: ObjectId(12),
        });
        assert!(RenderBlendShapeData::parse(&bytes[..23]).is_err());

        let mut descriptor = vec![0; 16 + 3 * 16];
        put_u16(&mut descriptor, 8, 3);
        for (index, (offset, format, usage)) in [
            (0, VERTEX_FORMAT_SHORT4N, VERTEX_USAGE_POSITION),
            (8, VERTEX_FORMAT_HEND3N, VERTEX_USAGE_NORMAL),
            (12, VERTEX_FORMAT_FLOAT1, VERTEX_USAGE_BLEND_WEIGHT),
        ].into_iter().enumerate() {
            let base = 16 + index * 16;
            put_u16(&mut descriptor, base + 2, offset);
            put_u32(&mut descriptor, base + 4, format);
            descriptor[base + 9] = usage;
        }
        let descriptor = VertexDescriptor::parse(&descriptor).unwrap();
        assert_eq!(descriptor.stream_strides().unwrap().get(&0), Some(&16));
    }

    #[test]
    fn derives_vertex_stream_strides_from_reference_formats() {
        let descriptor = VertexDescriptor {
            type_flags: 0,
            reference_count: 1,
            instance_streams: 0,
            elements: vec![
                VertexElement {
                    stream: 0,
                    offset: 0,
                    format: 2_761_657,
                    method: 0,
                    usage: 0,
                    usage_index: 0,
                    element_type: 0,
                    element_class: 0,
                },
                VertexElement {
                    stream: 0,
                    offset: 12,
                    format: 2_761_104,
                    method: 0,
                    usage: 3,
                    usage_index: 0,
                    element_type: 0,
                    element_class: 0,
                },
            ],
        };
        assert_eq!(descriptor.stream_strides().unwrap().get(&0), Some(&16));
    }

    #[test]
    fn decodes_big_endian_positions_and_indices() {
        let vertex_buffer = VertexBuffer {
            data: ObjectId(1),
            fetch_flags: 0,
            buffer_size: 24,
            buffer_type: 0,
        };
        let element = VertexElement {
            stream: 0,
            offset: 0,
            format: VERTEX_FORMAT_FLOAT3,
            method: 0,
            usage: VERTEX_USAGE_POSITION,
            usage_index: 0,
            element_type: 0,
            element_class: 0,
        };
        let mut vertices = Vec::new();
        for value in [1.0_f32, 2.0, 3.0, -4.0, 5.0, 6.0] {
            vertices.extend_from_slice(&value.to_be_bytes());
        }
        assert_eq!(
            vertex_buffer
                .decode_positions(&vertices, element, 12)
                .unwrap(),
            vec![[1.0, 2.0, 3.0], [-4.0, 5.0, 6.0]]
        );

        let index_buffer = IndexBuffer {
            data: ObjectId(2),
            aligned_buffer_size: 16,
            index_count: 3,
            index_depth: 16,
        };
        assert_eq!(
            index_buffer.decode_indices(&[0, 1, 0, 2, 0, 3]).unwrap(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn decodes_float16_positions() {
        let buffer = VertexBuffer {
            data: ObjectId(1),
            fetch_flags: 0,
            buffer_size: 8,
            buffer_type: 0,
        };
        let element = VertexElement {
            stream: 0,
            offset: 0,
            format: VERTEX_FORMAT_FLOAT16_4,
            method: 0,
            usage: VERTEX_USAGE_POSITION,
            usage_index: 0,
            element_type: 0,
            element_class: 0,
        };
        assert_eq!(
            buffer
                .decode_positions(
                    &[0x3c, 0x00, 0xc0, 0x00, 0x42, 0x00, 0x3c, 0x00],
                    element,
                    8
                )
                .unwrap(),
            vec![[1.0, -2.0, 3.0]]
        );
    }

    #[test]
    fn decodes_normalized_short_and_packed_hend_attributes() {
        let buffer = VertexBuffer {
            data: ObjectId(1),
            fetch_flags: 0,
            buffer_size: 8,
            buffer_type: 0,
        };
        let short_element = VertexElement {
            stream: 0,
            offset: 0,
            format: VERTEX_FORMAT_SHORT4N,
            method: 0,
            usage: VERTEX_USAGE_TEXTURE_COORDINATE,
            usage_index: 1,
            element_type: 0,
            element_class: 1,
        };
        let mut shorts = Vec::new();
        for value in [i16::MIN, -16_384, 0, i16::MAX] {
            shorts.extend_from_slice(&value.to_be_bytes());
        }
        let attribute = buffer.decode_attribute(&shorts, short_element, 8).unwrap();
        assert_eq!(attribute.semantic, VertexSemantic::TextureCoordinate(1));
        let NativeVertexAttributeValues::Float4(values) = attribute.values else {
            panic!("expected normalized four-component values");
        };
        let expected = [-1.0, -16_384.0 / 32_767.0, 0.0, 1.0];
        for (actual, expected) in values[0].into_iter().zip(expected) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }

        let hend_buffer = VertexBuffer {
            buffer_size: 4,
            ..buffer
        };
        let hend_element = VertexElement {
            format: VERTEX_FORMAT_HEND3N,
            usage: VERTEX_USAGE_TANGENT,
            usage_index: 0,
            ..short_element
        };
        let packed = (u32::from_ne_bytes((-1024_i32).to_ne_bytes()) & 0x07ff)
            | ((1023_u32 & 0x07ff) << 11)
            | ((u32::from_ne_bytes((-512_i32).to_ne_bytes()) & 0x03ff) << 22);
        let attribute = hend_buffer
            .decode_attribute(&packed.to_be_bytes(), hend_element, 4)
            .unwrap();
        let NativeVertexAttributeValues::Float3(values) = attribute.values else {
            panic!("expected packed three-component values");
        };
        assert_eq!(values, vec![[-1.0, 1.0, -1.0]]);
    }

    #[test]
    fn decodes_normalized_short4_positions_used_by_characters() {
        let buffer = VertexBuffer {
            data: ObjectId(1),
            fetch_flags: 0,
            buffer_size: 8,
            buffer_type: 0,
        };
        let element = VertexElement {
            stream: 0,
            offset: 0,
            format: VERTEX_FORMAT_SHORT4N,
            method: 0,
            usage: VERTEX_USAGE_POSITION,
            usage_index: 0,
            element_type: 0,
            element_class: 0,
        };
        let mut bytes = Vec::new();
        for value in [i16::MIN, 0, i16::MAX, i16::MAX] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        assert_eq!(
            buffer.decode_positions(&bytes, element, 8).unwrap(),
            [[-1.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn decodes_float2_attributes_used_by_characters() {
        let buffer = VertexBuffer {
            data: ObjectId(1),
            fetch_flags: 0,
            buffer_size: 8,
            buffer_type: 0,
        };
        let element = VertexElement {
            stream: 0,
            offset: 0,
            format: VERTEX_FORMAT_FLOAT2,
            method: 0,
            usage: VERTEX_USAGE_TEXTURE_COORDINATE,
            usage_index: 0,
            element_type: 0,
            element_class: 0,
        };
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1.25_f32.to_be_bytes());
        bytes.extend_from_slice(&(-0.5_f32).to_be_bytes());
        let attribute = buffer.decode_attribute(&bytes, element, 8).unwrap();
        assert_eq!(
            attribute.values,
            NativeVertexAttributeValues::Float2(vec![[1.25, -0.5]])
        );
    }

    #[test]
    fn decodes_half_uvs_blend_indices_and_signed_samples() {
        let base = VertexElement {
            stream: 0,
            offset: 0,
            format: VERTEX_FORMAT_FLOAT16_2,
            method: 0,
            usage: VERTEX_USAGE_TEXTURE_COORDINATE,
            usage_index: 0,
            element_type: 0,
            element_class: 1,
        };
        let four_byte_buffer = VertexBuffer {
            data: ObjectId(1),
            fetch_flags: 0,
            buffer_size: 4,
            buffer_type: 0,
        };
        let half = four_byte_buffer
            .decode_attribute(&[0x3c, 0x00, 0xc0, 0x00], base, 4)
            .unwrap();
        assert_eq!(
            half.values,
            NativeVertexAttributeValues::Float2(vec![[1.0, -2.0]])
        );

        let half_infinity = four_byte_buffer
            .decode_attribute(&[0x3c, 0x00, 0xfc, 0x00], base, 4)
            .unwrap();
        assert_eq!(
            half_infinity.values,
            NativeVertexAttributeValues::Float2(vec![[1.0, f32::NEG_INFINITY]])
        );

        let indices = four_byte_buffer
            .decode_attribute(
                &[1, 2, 3, 4],
                VertexElement {
                    format: VERTEX_FORMAT_UBYTE4,
                    usage: VERTEX_USAGE_BLEND_INDICES,
                    ..base
                },
                4,
            )
            .unwrap();
        assert_eq!(
            indices.values,
            NativeVertexAttributeValues::UnsignedByte4(vec![[1, 2, 3, 4]])
        );

        let samples = four_byte_buffer
            .decode_attribute(
                &[0xff, 0xfe, 0x01, 0x02],
                VertexElement {
                    format: VERTEX_FORMAT_SHORT2,
                    usage: VERTEX_USAGE_SAMPLE,
                    ..base
                },
                4,
            )
            .unwrap();
        assert_eq!(
            samples.values,
            NativeVertexAttributeValues::SignedShort2(vec![[-2, 258]])
        );
    }
}
