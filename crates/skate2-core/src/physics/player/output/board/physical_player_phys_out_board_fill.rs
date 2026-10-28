//! Ordered board `FillPhysOut` entry82b2f3e8..f714; motion publication follows.
use super::super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut};
use super::ConcretePhysOutError;
use crate::{
    skeleton_collision_numeric::SkeletonCollisionNumeric,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::convert::Infallible;

/// Live reads remain separate: repeated effective transforms are observable.
pub(crate) trait BoardFillSource {
    type Error;
    fn capture_body(&mut self) -> Result<(), Self::Error>;
    fn part_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
    fn effective_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
    fn sample_linear_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn look_direction_flipped(&mut self) -> Result<bool, Self::Error>;
    fn draw_velocity_direction(&mut self) -> Result<bool, Self::Error>;
    fn draw_look_direction(&mut self) -> Result<bool, Self::Error>;
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub(crate) enum BoardFillOutputError<S, N> {
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
    Source(S),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for BoardFillOutputError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(e) => write!(f, "board output guard: {e:?}"),
            Self::Component(e) => write!(f, "board output component: {e:?}"),
            Self::Source(e) => write!(f, "board output source: {e:?}"),
            Self::Numeric(e) => write!(f, "board output numeric: {e:?}"),
        }
    }
}

impl PhysicalPlayerPhysOut {
    /// 82b2f440..f714. Every failed late read retains preceding output stores.
    pub(crate) fn try_fill_board_transform_directions<
        S: BoardFillSource,
        N: SkeletonCollisionNumeric,
    >(
        &mut self,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), BoardFillOutputError<S::Error, N::Error>> {
        use BoardFillOutputError as E;
        source.capture_body().map_err(E::Source)?;
        let transform = source.part_transform().map_err(E::Source)?;
        self.guard_board_fill()?;
        self.concrete
            .skateboard_reckoning
            .as_mut()
            .expect("guarded")
            .transform = transform;

        // Unlike the following read, this effective transform follows its guard.
        self.guard_board_fill()?;
        let effective = source.effective_transform().map_err(E::Source)?;
        self.concrete
            .skateboard_reckoning
            .as_mut()
            .expect("guarded")
            .effective_transform = effective;
        let effective = source.effective_transform().map_err(E::Source)?;
        self.guard_board_fill()?;
        self.concrete
            .skateboard_reckoning
            .as_mut()
            .expect("guarded")
            .deck_nose_dir = effective.columns()[2];

        let linear = source.sample_linear_velocity().map_err(E::Source)?;
        let forward = source.effective_transform().map_err(E::Source)?.columns()[2];
        let right = source.effective_transform().map_err(E::Source)?.columns()[0];
        let projection = numeric.dot3(forward.xyz, linear.xyz).map_err(E::Numeric)?;
        // BLT82b2f59c: unordered takes the positive sign.
        let velocity_sign = if projection < f32::from_bits(0xb8d1_b717) {
            -1.0
        } else {
            1.0
        };
        let look_sign = if source.look_direction_flipped().map_err(E::Source)? {
            -1.0
        } else {
            1.0
        };
        // The latch is sampled before both full-vector multiplications, and
        // checked only afterwards (f5d8, f610/f614, f618).
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let velocity_direction = scale(forward, velocity_sign);
        let look_direction = scale(right, look_sign);
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::SkateboardReckoning)
            .map_err(E::Component)?;
        self.concrete
            .skateboard_reckoning
            .as_mut()
            .expect("guarded")
            .skater_look_direction = look_direction;
        self.guard_board_fill()?;
        self.concrete
            .skateboard_reckoning
            .as_mut()
            .expect("guarded")
            .vel_direction = velocity_direction;

        if source.draw_velocity_direction().map_err(E::Source)? {
            source.effective_transform().map_err(E::Source)?;
            source.effective_transform().map_err(E::Source)?;
            source.require_debug_overlay().map_err(E::Source)?;
        }
        if source.draw_look_direction().map_err(E::Source)? {
            source.effective_transform().map_err(E::Source)?;
            source.effective_transform().map_err(E::Source)?;
            source.require_debug_overlay().map_err(E::Source)?;
        }
        Ok(())
    }

    fn guard_board_fill<S, N>(&self) -> Result<(), BoardFillOutputError<S, N>> {
        self.check_current_guard()
            .map_err(BoardFillOutputError::Access)?;
        self.require_complete_component(PhysOutComponent::SkateboardReckoning)
            .map_err(BoardFillOutputError::Component)
    }
}

fn scale(vector: SkeletonRootVector, scalar: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: vector.xyz.map(|lane| lane * scalar),
        w: vector.w.map(|lane| lane * scalar),
    }
}

/// Native VNMSUB82b2f7b4/f7c4 negates the rounded fused result. Keeping the
/// negation outside `mul_add` preserves negative zero for an exact product one.
fn board_speed_residual(squared: f32, inverse_square: f32) -> f32 {
    -squared.mul_add(inverse_square, -1.0)
}

/// Local board length graph82b2f794..f7d0. The caller obtains the estimate
/// unconditionally before this graph; zero selection follows both refinements.
pub(crate) fn board_speed_from_squared_and_estimate(squared: f32, estimate: f32) -> f32 {
    let square = estimate * estimate;
    let residual = board_speed_residual(squared, square);
    let refined = (estimate * 0.5).mul_add(residual, estimate);
    let square = refined * refined;
    let residual = board_speed_residual(squared, square);
    let refined = (refined * 0.5).mul_add(residual, refined);
    let length = squared * refined;
    if squared == 0.0 { 0.0 } else { length }
}

#[cfg(test)]
#[path = "physical_player_phys_out_board_fill_tests.rs"]
mod tests;
