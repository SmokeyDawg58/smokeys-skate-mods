//! Selected `HiLOD` prepared-jump history and spin history, 82e0379c..38c8.
//! Actual caller binds the repeated processed captures to the SAME owner.
//! This state does not select an Actor phase, fabricate flags, or reset itself.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected prepared history awaits full Actor Update1 scheduling"
    )
)]
use crate::{
    reckoning_numeric::ReckoningNumeric, skeleton_ik_line_frame::normalize_safe,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::fmt;

const FIXED_STEP: f32 = f32::from_bits(0x3c88_8889); // kUpdateTime, 8236e734
const GRAVITY_MAGNITUDE: f32 = f32::from_bits(0x411c_cccd); // immediate 37fc

pub(crate) trait PhysicalPlayerPrepareVelocitySource {
    type Processed;
    type Error;
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error>;
    /// Actual known processed+8c4 PREPARE bit. Unknown must refuse.
    fn prepare(&mut self, processed: &Self::Processed) -> Result<bool, Self::Error>;
    fn velocity(&mut self, processed: &Self::Processed) -> Result<SkeletonRootVector, Self::Error>;
    fn surface_velocity(
        &mut self,
        processed: &Self::Processed,
    ) -> Result<SkeletonRootVector, Self::Error>;
    /// Actual known type-zero Spin producer at processed+97c, not caller tuning.
    fn spin(&mut self, processed: &Self::Processed) -> Result<f32, Self::Error>;
    fn publish_spin_count(
        &mut self,
        processed: &Self::Processed,
        count: u32,
    ) -> Result<(), Self::Error>;
    fn publish_prepare_velocity(
        &mut self,
        processed: &Self::Processed,
        velocity: SkeletonRootVector,
    ) -> Result<(), Self::Error>;
}
pub(crate) struct PhysicalPlayerPrepareVelocity {
    velocity: Option<SkeletonRootVector>,
    last_spin: Option<f32>,
    same_direction_frames: Option<u32>,
    complete: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PrepareVelocityAlreadyInitialized;
#[derive(Debug, PartialEq)]
pub(crate) enum PhysicalPlayerPrepareVelocityError<S, N> {
    Uninitialized,
    Source(S),
    Numeric(N),
}
impl<S: fmt::Debug, N: fmt::Debug> fmt::Display for PhysicalPlayerPrepareVelocityError<S, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uninitialized => f.write_str("HiLOD prepared history constructor not executed"),
            Self::Source(e) => write!(f, "prepared history source: {e:?}"),
            Self::Numeric(e) => write!(f, "prepared history numeric: {e:?}"),
        }
    }
}
impl PhysicalPlayerPrepareVelocity {
    pub(crate) const fn uninitialized() -> Self {
        Self {
            velocity: None,
            last_spin: None,
            same_direction_frames: None,
            complete: false,
        }
    }
    /// Explicit selected constructor stores: +c0 at3dac, +158 at3e08, +13c
    /// at3e24. This does not execute intervening/omitted `HiLOD` constructors.
    pub(crate) fn initialize_constructor(
        &mut self,
    ) -> Result<(), PrepareVelocityAlreadyInitialized> {
        if self.velocity.is_some() {
            return Err(PrepareVelocityAlreadyInitialized);
        }
        self.velocity = Some(SkeletonRootVector::known([0.0; 4]));
        self.last_spin = Some(0.0);
        self.same_direction_frames = Some(0);
        Ok(())
    }
    pub(crate) const fn velocity(&self) -> Option<SkeletonRootVector> {
        self.velocity
    }
    pub(crate) const fn last_spin(&self) -> Option<f32> {
        self.last_spin
    }
    pub(crate) const fn same_direction_frames(&self) -> Option<u32> {
        self.same_direction_frames
    }
    /// TeleportPlayerNow 0x82e008b0..b4 clears the retained spin history.
    pub(crate) fn reset_after_teleport(&mut self) {
        self.last_spin = Some(0.0);
        self.same_direction_frames = Some(0);
    }
    pub(crate) const fn complete(&self) -> bool {
        self.complete
    }

    pub(crate) fn try_update<S: PhysicalPlayerPrepareVelocitySource, N: ReckoningNumeric>(
        &mut self,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), PhysicalPlayerPrepareVelocityError<S::Error, N::Error>> {
        use PhysicalPlayerPrepareVelocityError as E;
        self.complete = false;
        if self.velocity.is_none() {
            return Err(E::Uninitialized);
        }
        let processed = source.capture_processed().map_err(E::Source)?;
        if source.prepare(&processed).map_err(E::Source)? {
            self.velocity = Some(source.velocity(&processed).map_err(E::Source)?);
        } else {
            let surface = source.surface_velocity(&processed).map_err(E::Source)?;
            let direction = normalize_safe(surface, numeric).map_err(E::Numeric)?;
            // Source XOR sign, separately rounded scalar product/round trip,
            // full direction product, then full-lane fused fixed-step update.
            let negative_y = f32::from_bits(direction.xyz[1].to_bits() ^ 0x8000_0000);
            let acceleration = negative_y * GRAVITY_MAGNITUDE;
            let delta = SkeletonRootVector {
                xyz: direction.xyz.map(|lane| lane * acceleration),
                w: direction.w.map(|lane| lane * acceleration),
            };
            let previous = self.velocity.expect("constructor established velocity");
            self.velocity = Some(SkeletonRootVector {
                xyz: std::array::from_fn(|lane| {
                    delta.xyz[lane].mul_add(FIXED_STEP, previous.xyz[lane])
                }),
                w: delta
                    .w
                    .zip(previous.w)
                    .map(|(delta, old)| delta.mul_add(FIXED_STEP, old)),
            });
        }
        // The history store above survives refusal of this later capture/read.
        let processed = source.capture_processed().map_err(E::Source)?;
        let old_spin = self.last_spin.expect("constructor established spin");
        let spin = source.spin(&processed).map_err(E::Source)?;
        let product = spin * old_spin;
        // Raw3898=40990010: BO4/BI25, branch if ordered-GT bit is NOT set.
        // Both signed zeros and unordered clear; only ordered positive wraps+1.
        let count = if product > 0.0 {
            self.same_direction_frames
                .expect("constructor established count")
                .wrapping_add(1)
        } else {
            0
        };
        self.last_spin = Some(spin); // 38ac
        self.same_direction_frames = Some(count); // 38b0
        source
            .publish_spin_count(&processed, count)
            .map_err(E::Source)?; // 38b4
        let processed = source.capture_processed().map_err(E::Source)?; // 38bc
        source
            .publish_prepare_velocity(&processed, self.velocity.expect("history stored"))
            .map_err(E::Source)?;
        self.complete = true;
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_prepare_velocity_tests.rs"]
mod tests;
