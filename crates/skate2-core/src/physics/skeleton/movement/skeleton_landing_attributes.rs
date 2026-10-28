//! Lazy global `physics_animation.default` reads for `UpdateLandingAdjust`
//! (0x82b401d8). This retains a separate global generated instance; it is not
//! the Skeleton-owned instance used by `LandingOnSkateboardAdjust`.
//! Scalar pointer/value loads have no intervening fallible service or store in
//! this caller. Each method therefore performs the original lookup and value
//! read together. Immutable captured allocations exclude editor/alias races.

use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    physics::PointGraph,
    skeleton_landing_adjust::{SkeletonLandingAdjustAttributes, SkeletonLandingAdjustScalar},
};
use std::sync::Arc;

#[cfg(test)]
#[path = "skeleton_landing_attributes_tests.rs"]
pub(crate) mod tests;

const CLASS: u64 = 0x74bc_9c93_affe_6aac;
const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
const LAYOUT_SIZE: usize = 0x220;
const MANUAL_GRAPH_OFFSET: usize = 0x140;
const GRAPH_SIZE: usize = 0x30;
const COFFIN_GRAPH: u64 = 0x25e6_e31b_c284_1e1f;
const GRIND_GRAPH: u64 = 0x804c_e67e_a039_46fa;

pub(crate) struct SkeletonLandingVaultAttributes {
    global: AttributeRuntimeInstance,
}
impl SkeletonLandingVaultAttributes {
    /// Generated ctor 0x828b0f00 selects this class. The retained collection's
    /// null cached-layout pointer gets `DefaultDataArea220` at 0x828b0f50..58.
    /// All first220 bytes at 0x83059d30 are proven zero. Range validation is a
    /// lazy checked-host boundary; it does not replace nonnull corrupt storage.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            global: vault.instance(CLASS, DEFAULT),
        }
    }

    fn keyed_graph(&self, key: u64, input: f32) -> Result<f32, AttributeRuntimeError> {
        let data = self.global.try_get_data(key, 0)?;
        let Some(data) = data else {
            return Ok(PointGraph::<4>::default().evaluate(input));
        };
        let bytes = data.bytes();
        if bytes.len() != GRAPH_SIZE {
            return Err(AttributeRuntimeError::PayloadSize {
                key,
                expected: GRAPH_SIZE,
                actual: bytes.len(),
            });
        }
        Ok(evaluate_graph(bytes, input))
    }
}

impl SkeletonLandingVaultAttributes {
    fn keyed_scalar(&self, key: u64) -> Result<f32, AttributeRuntimeError> {
        let data = self.global.try_get_data(key, 0)?;
        let Some(data) = data else {
            return Ok(0.0);
        }; // Actual DefaultDataArea4.
        let bytes = data.bytes();
        let word: [u8; 4] = bytes
            .try_into()
            .map_err(|_| AttributeRuntimeError::PayloadSize {
                key,
                expected: 4,
                actual: bytes.len(),
            })?;
        Ok(f32::from_be_bytes(word))
    }
}

/// Same retained GLOBAL animation Instance; source82b49c2c then49c84.
impl crate::skeleton_ik_prephysics::SkeletonIkPrePhysicsAttributes
    for SkeletonLandingVaultAttributes
{
    type Error = AttributeRuntimeError;
    fn minimum_angle_degrees(&mut self) -> Result<f32, Self::Error> {
        self.keyed_scalar(0xb76b_5101_768e_856c)
    }
    fn maximum_angle_degrees(&mut self) -> Result<f32, Self::Error> {
        self.keyed_scalar(0xa175_875d_ec9d_6db7)
    }
}

impl SkeletonLandingAdjustAttributes for SkeletonLandingVaultAttributes {
    type Error = AttributeRuntimeError;
    fn scalar(&mut self, scalar: SkeletonLandingAdjustScalar) -> Result<f32, Self::Error> {
        self.keyed_scalar(scalar_key(scalar))
    }

    fn manual_blend(&mut self, time: f32) -> Result<f32, Self::Error> {
        // 0x82b4096c..7c: direct generated Instance+4 -> layout+140. There is
        // no generic attribute search or parent-layout substitution here.
        let layout = self.global.try_cached_layout(LAYOUT_SIZE)?;
        let Some(layout) = layout else {
            return Ok(PointGraph::<4>::default().evaluate(time));
        };
        Ok(evaluate_graph(
            &layout.bytes()[MANUAL_GRAPH_OFFSET..MANUAL_GRAPH_OFFSET + GRAPH_SIZE],
            time,
        ))
    }
    fn grind_blend(&mut self, time: f32) -> Result<f32, Self::Error> {
        self.keyed_graph(GRIND_GRAPH, time)
    }
    fn coffin_bounce(&mut self, normalized_time: f32) -> Result<f32, Self::Error> {
        self.keyed_graph(COFFIN_GRAPH, normalized_time)
    }
}

fn scalar_key(scalar: SkeletonLandingAdjustScalar) -> u64 {
    use SkeletonLandingAdjustScalar as S;
    match scalar {
        S::MaximumVelocity => 0x5199_e2e7_49e7_220b,
        S::CoffinDuration => 0x1d4c_65d8_fd17_9fe2,
        S::CoffinVelocityDenominator => 0x4f4e_3f45_66ec_34e3,
        S::CoffinBase => 0x2f99_6144_e5ef_4eea,
        S::DesiredCom => 0x9091_e734_f21a_004d,
        S::MinimumYPosition => 0x2a69_0d7e_8c1e_dcf4,
        S::GroundMinimumElapsed => 0x0175_8b41_9109_fdae,
        S::CoffinFrames => 0x9aba_ff27_0d53_f138,
        S::GrindTargetSpeed => 0x74e3_fc7c_dd2f_8c97,
        S::GrindK1 => 0x1098_9560_be00_f690,
        S::GrindK2 => 0xbf50_36b2_3111_79e8,
        S::GrindTargetAfterTime => 0x8dac_44f9_0326_2ddb,
        S::GroundK1 => 0xac53_2ffc_deba_62d8,
        S::GroundK2 => 0x2471_dd04_ee7e_e637,
        S::ManualK1 => 0x01ba_609b_b65c_adb5,
        S::ManualK2 => 0x475a_3cb8_59bd_b0a6,
    }
}

fn evaluate_graph(bytes: &[u8], input: f32) -> f32 {
    // PointNegGraphData4's four metadata words do not participate in Evaluate4.
    let word = |offset| {
        f32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("checked graph region"),
        )
    };
    PointGraph::<4> {
        x: std::array::from_fn(|index| word(0x10 + index * 4)),
        y: std::array::from_fn(|index| word(0x20 + index * 4)),
    }
    .evaluate(input)
}
