//! Lazy reads of the SAME retained `physics_jump.default` instance.
//! Child of `skeleton_processed_jump_events`; no second global owner.
use super::{
    SkeletonJumpAttributes, SkeletonJumpGlobal, SkeletonJumpVaultAttributes, SkeletonJumpVaultError,
};
use crate::physics::PointGraph;

#[derive(Clone, Copy, Debug)]
pub(crate) enum GroundJumpGraph {
    VerticalResponse,
    WorldYResponse,
    TangentLimit,
    MinimumHeight,
    MaximumHeight,
}
impl SkeletonJumpVaultAttributes {
    /// Direct cached scalar arrays at native31684/31878/31a10/31a54/31d5c.
    /// Four graph metadata words preceding each eight-point array are unused.
    pub(crate) fn try_ground_jump_graph(
        &self,
        graph: GroundJumpGraph,
        input: f32,
    ) -> Result<f32, SkeletonJumpVaultError> {
        match graph {
            GroundJumpGraph::VerticalResponse => self.ground_jump_graph::<16>(0, input),
            GroundJumpGraph::WorldYResponse => self.ground_jump_graph::<8>(0x90, input),
            GroundJumpGraph::TangentLimit => self.ground_jump_graph::<8>(0xe0, input),
            GroundJumpGraph::MinimumHeight => self.ground_jump_graph::<8>(0x120, input),
            GroundJumpGraph::MaximumHeight => self.ground_jump_graph::<8>(0x160, input),
        }
    }
    fn ground_jump_graph<const N: usize>(
        &self,
        offset: usize,
        input: f32,
    ) -> Result<f32, SkeletonJumpVaultError> {
        let data = self
            .adjustment
            .try_cached_layout_field(offset, N * 8)
            .map_err(SkeletonJumpVaultError::Runtime)?;
        let word = |index: usize| {
            data.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[index * 4..index * 4 + 4]
                        .try_into()
                        .expect("checked graph range"),
                )
            })
        };
        Ok(PointGraph::<N> {
            x: std::array::from_fn(word),
            y: std::array::from_fn(|i| word(N + i)),
        }
        .evaluate(input))
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum GroundJumpAttribute {
    ReferenceSpeed,
    BaseHeight,
    MinimumHeight,
    MaximumHeight,
    VerticalResponseEnabled,
    MinimumVerticalResponse,
    MaximumYCompensation,
    SidewaysScale,
    ForwardScale,
}
impl GroundJumpAttribute {
    pub(crate) const fn key(self) -> u64 {
        match self {
            Self::ReferenceSpeed => 0x97cd_b6c3_13cd_0a1b,
            Self::BaseHeight => 0xa35e_e331_9500_4033,
            Self::MinimumHeight => 0xbb9e_9baf_ba1b_29e6,
            Self::MaximumHeight => 0x0b24_54c6_6641_0860,
            Self::VerticalResponseEnabled => 0xc1a0_16ba_4ca8_8480,
            Self::MinimumVerticalResponse => 0x8d13_4b78_61d8_8c9a,
            Self::MaximumYCompensation => 0xbe6c_6a3e_d7c6_d148,
            Self::SidewaysScale => 0x09a6_c205_c73d_d180,
            Self::ForwardScale => 0x4edd_bd69_72ee_9c56,
        }
    }
}
impl SkeletonJumpVaultAttributes {
    pub(crate) fn try_ground_jump_scalar(
        &mut self,
        field: GroundJumpAttribute,
    ) -> Result<f32, SkeletonJumpVaultError> {
        let global = self.capture(SkeletonJumpGlobal::Adjustment)?;
        let pointer = self.lookup(&global, field.key())?;
        self.float(&pointer)
    }
    pub(crate) fn try_ground_jump_vertical_response_enabled(
        &mut self,
    ) -> Result<bool, SkeletonJumpVaultError> {
        let global = self.capture(SkeletonJumpGlobal::Adjustment)?;
        let pointer = self.lookup(&global, GroundJumpAttribute::VerticalResponseEnabled.key())?;
        Ok(self.byte(&pointer)? != 0)
    }
}
