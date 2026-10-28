use super::{AptError, Matrix, Reader};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryKind {
    /// Factory8260a110 returns no unit; Draw82a704ec skips that null unit.
    /// Retained for asset inspection, not rendered as invented triangles.
    Line,
    Solid,
    TextureClamped,
    TextureWrapped,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeometryElement {
    pub kind: GeometryKind,
    pub colour: [f32; 4],
    pub texture_id: i32,
    pub texture_matrix: Matrix,
    /// Original triangle-list vertices, preserving order and precision.
    pub positions: Vec<[f32; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeometryShape {
    /// Character-table index, looked up by `AptGeometryInfo::GetGEO`82a6fec8.
    pub id: i32,
    pub elements: Vec<GeometryElement>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    pub shapes: Vec<GeometryShape>,
}

impl Geometry {
    /// `AptGeometryInfo`82a6fc90 and `AptRenderingUnitFactory`8260a110.
    /// Pointers are offsets from the GEO base, not the enclosing shape.
    pub fn parse(bytes: &[u8]) -> Result<Self, AptError> {
        let r = Reader(bytes);
        if r.bytes(0, 3)? != b"Sk8" {
            return Err(AptError::Magic("GEO"));
        }
        let count = r.offset(4)?;
        let mut shapes = Vec::new();
        for i in r.array(8, count, 4)? {
            let o = r.offset(8 + i * 4)?;
            if o == 0 {
                return Err(AptError::NullTable);
            }
            let id = r.u32(o)? as i32;
            let count = r.offset(o + 4)?;
            let mut elements = Vec::new();
            for j in r.array(o + 8, count, 4)? {
                let e = r.offset(o + 8 + j * 4)?;
                if e == 0 {
                    return Err(AptError::NullTable);
                }
                let kind = match r.u32(e)? {
                    0 => GeometryKind::Line,
                    1 => GeometryKind::Solid,
                    2 => GeometryKind::TextureClamped,
                    3 => GeometryKind::TextureWrapped,
                    value => {
                        return Err(AptError::Unsupported {
                            offset: e,
                            kind: "geometry element",
                            value,
                        });
                    }
                };
                let triangles = r.offset(e + 48)?;
                if (kind == GeometryKind::Solid && triangles == 0)
                    || (matches!(
                        kind,
                        GeometryKind::TextureClamped | GeometryKind::TextureWrapped
                    ) && triangles != 2)
                {
                    return Err(AptError::Invalid {
                        offset: e + 48,
                        field: "triangle count",
                        value: triangles as u32,
                    });
                }
                let vertices = triangles
                    .checked_mul(if kind == GeometryKind::Line { 2 } else { 3 })
                    .ok_or(AptError::Overflow)?;
                let mut positions = Vec::new();
                for k in r.array(e + 52, vertices, 8)? {
                    positions.push([r.f32(e + 52 + k * 8)?, r.f32(e + 56 + k * 8)?]);
                }
                elements.push(GeometryElement {
                    kind,
                    colour: [r.f32(e + 4)?, r.f32(e + 8)?, r.f32(e + 12)?, r.f32(e + 16)?],
                    texture_id: r.u32(e + 20)? as i32,
                    texture_matrix: r.matrix(e + 24)?,
                    positions,
                });
            }
            shapes.push(GeometryShape { id, elements });
        }
        Ok(Self { shapes })
    }

    pub fn shape(&self, id: i32) -> Option<&GeometryShape> {
        self.shapes.iter().find(|shape| shape.id == id)
    }
}
