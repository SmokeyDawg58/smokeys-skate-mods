//! tRAINData/tRAINRoad/tRAINIntersection Fixup829c8558/80e0/82b8.
//! Views retain the existing streamed arena; nested offsets use the record base.
use crate::{
    simulation_inner_registry_asset::SimulationInnerRegistryAsset,
    simulation_normal_arena_plan::SimulationRegistryAssetKind,
    world_painter_layer_asset::{LayerAssetError, vector, word},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RainAssetError {
    Data(LayerAssetError),
    WrongKind,
    Index,
    NullArray,
}
impl From<LayerAssetError> for RainAssetError {
    fn from(e: LayerAssetError) -> Self {
        Self::Data(e)
    }
}
#[derive(Clone)]
pub(crate) struct RainRecord {
    pub(crate) asset: Arc<SimulationInnerRegistryAsset>,
    pub(crate) offset: usize,
}
impl RainRecord {
    pub(crate) fn new(asset: Arc<SimulationInnerRegistryAsset>) -> Result<Self, RainAssetError> {
        if asset.kind != SimulationRegistryAssetKind::Rain {
            return Err(RainAssetError::WrongKind);
        }
        let r = Self { asset, offset: 0 };
        r.word(0x34)?;
        Ok(r)
    }
    fn bytes(&self) -> Result<&[u8], RainAssetError> {
        self.asset
            .bytes()
            .map_err(|e| RainAssetError::Data(e.into()))
    }
    pub(crate) fn word(&self, at: usize) -> Result<u32, RainAssetError> {
        Ok(word(self.bytes()?, self.offset + at)?)
    }
    pub(crate) fn id(&self, at: usize) -> Result<u64, RainAssetError> {
        Ok((u64::from(self.word(at)?) << 32) | u64::from(self.word(at + 4)?))
    }
    pub(crate) fn scalar(&self, at: usize) -> Result<f32, RainAssetError> {
        Ok(f32::from_bits(self.word(at)?))
    }
    pub(crate) fn vector(&self, at: usize) -> Result<[f32; 4], RainAssetError> {
        Ok(vector(self.bytes()?, self.offset + at)?)
    }
    pub(crate) fn inline(&self, at: usize, size: usize) -> Result<Self, RainAssetError> {
        self.at(self.offset + at, size)
    }
    fn at(&self, offset: usize, size: usize) -> Result<Self, RainAssetError> {
        let bytes = self.bytes()?;
        if bytes.get(offset..offset + size).is_none() {
            return Err(LayerAssetError::Truncated {
                required: offset + size,
                actual: bytes.len(),
            }
            .into());
        }
        Ok(Self {
            asset: Arc::clone(&self.asset),
            offset,
        })
    }
    pub(crate) fn array(
        &self,
        count_at: usize,
        pointer_at: usize,
        stride: usize,
    ) -> Result<Vec<Self>, RainAssetError> {
        let count = self.word(count_at)? as usize;
        // Shipped empty arrays contain poison pointers; native never reads elements.
        if count == 0 {
            return Ok(Vec::new());
        }
        let pointer = self.word(pointer_at)? as usize;
        if pointer == 0 {
            return Err(RainAssetError::NullArray);
        }
        let start = self.offset + pointer;
        self.at(start, count * stride)?;
        Ok((0..count)
            .map(|i| Self {
                asset: Arc::clone(&self.asset),
                offset: start + i * stride,
            })
            .collect())
    }
    pub(crate) fn intersections(&self) -> Result<Vec<Self>, RainAssetError> {
        self.array(0x20, 0x2c, 608)
    }
    pub(crate) fn roads(&self) -> Result<Vec<Self>, RainAssetError> {
        self.array(0x24, 0x30, 160)
    }
    pub(crate) fn super_roads(&self) -> Result<Vec<Self>, RainAssetError> {
        self.array(0x28, 0x34, 64)
    }
    pub(crate) fn road_segments(&self) -> Result<Vec<Self>, RainAssetError> {
        self.array(0x78, 0x7c, 224)
    }
    pub(crate) fn intersection_passages(&self) -> Result<Vec<Self>, RainAssetError> {
        self.array(0x254, 0x250, 112)
    }
    pub(crate) fn pin(&self, at: usize) -> Result<Pin, RainAssetError> {
        let record = self.inline(at, 56)?;
        Ok(Pin {
            item_id: record.id(0)?,
            item_type: record.word(8)?,
            item_pin: record.word(12)?,
            lanes: record.word(16)?,
            record,
        })
    }
    pub(crate) fn super_road(&self) -> Result<SuperRoadData, RainAssetError> {
        Ok(SuperRoadData {
            id: self.id(0)?,
            destination: self.id(8)?,
            destination_pin: self.word(16)?,
            origin: self.id(24)?,
            origin_pin: self.word(32)?,
            length: self.scalar(36)?,
            width: self.scalar(40)?,
            parking_width: self.scalar(44)?,
            speed_limit: self.scalar(48)?,
            lanes: self.word(52)?,
            flags: self.word(56)?,
        })
    }
    pub(crate) fn curve(&self) -> Result<CurveData, RainAssetError> {
        let rows = [
            self.vector(0)?,
            self.vector(16)?,
            self.vector(32)?,
            self.vector(48)?,
        ];
        let parameters = self
            .array(0x44, 0x48, 4)?
            .iter()
            .map(|r| r.scalar(0))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CurveData {
            rows,
            length: self.scalar(0x40)?,
            parameters,
        })
    }
}
#[derive(Clone)]
pub(crate) struct Pin {
    pub(crate) item_id: u64,
    pub(crate) item_type: u32,
    pub(crate) item_pin: u32,
    pub(crate) lanes: u32,
    record: RainRecord,
}
impl Pin {
    pub(crate) fn available_passages(&self, lane: usize) -> Result<Vec<i32>, RainAssetError> {
        if lane >= self.lanes as usize || lane >= 4 {
            return Err(RainAssetError::Index);
        }
        self.record
            .array(0x14 + lane * 4, 0x24 + lane * 4, 4)?
            .iter()
            .map(|r| r.word(0).map(|v| v as i32))
            .collect()
    }
}
/// Native SuperRoad ctor COPIES its pegasus record; it may outlive that chunk.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SuperRoadData {
    pub(crate) id: u64,
    pub(crate) destination: u64,
    pub(crate) destination_pin: u32,
    pub(crate) origin: u64,
    pub(crate) origin_pin: u32,
    pub(crate) length: f32,
    pub(crate) width: f32,
    pub(crate) parking_width: f32,
    pub(crate) speed_limit: f32,
    pub(crate) lanes: u32,
    pub(crate) flags: u32,
}
pub(crate) struct CurveData {
    pub(crate) rows: [[f32; 4]; 4],
    pub(crate) length: f32,
    pub(crate) parameters: Vec<f32>,
}
