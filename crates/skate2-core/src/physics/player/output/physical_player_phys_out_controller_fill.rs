//! `SkateboardController::FillPhysOut82dc8370..8664`, including unconditional yaw.
use super::super::PhysOutAccessError;
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};
use crate::{
    reckoning_numeric::ReckoningNumeric,
    skeleton_query_rotation::try_atan,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

pub(crate) trait ControllerFillSource {
    type Error;
    fn state(&mut self) -> Result<u32, Self::Error>;
    fn spine_world(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
    fn effective_animation(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
    fn retrieval_start_position(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn deck_position(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn flags_8cc(&mut self, mask: u32) -> Result<u32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum ControllerFillError<S, N> {
    Source(S),
    Numeric(N),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<std::convert::Infallible>),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for ControllerFillError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "controller Fill source: {e:?}"),
            Self::Numeric(e) => write!(f, "controller Fill numeric: {e:?}"),
            Self::Access(e) => write!(f, "controller Fill output: {e:?}"),
            Self::Component(e) => write!(f, "controller Fill component: {e:?}"),
        }
    }
}
fn subtract(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| a.xyz[i] - b.xyz[i]),
        w: a.w.zip(b.w).map(|(a, b)| a - b),
    }
}
/// Raw8408..849c. The existing estimate/ATan providers retain the numeric seam.
fn yaw_projection<N: ReckoningNumeric>(
    spine: &SkeletonRootMatrix,
    effective: &SkeletonRootMatrix,
    numeric: &mut N,
) -> Result<SkeletonRootVector, N::Error> {
    let columns = effective.columns();
    let relative = subtract(spine.columns()[3], columns[3]);
    let height = numeric.dot_vector(relative, columns[1])?;
    Ok(SkeletonRootVector {
        xyz: std::array::from_fn(|i| columns[1].xyz[i].mul_add(height, columns[3].xyz[i])),
        w: columns[1]
            .w
            .zip(columns[3].w)
            .map(|(up, p)| up.mul_add(height, p)),
    })
}
fn yaw_terms<N: ReckoningNumeric>(
    projected: SkeletonRootVector,
    effective: &SkeletonRootMatrix,
    position: SkeletonRootVector,
    numeric: &mut N,
) -> Result<(f32, f32, f32), N::Error> {
    let columns = effective.columns();
    let toward = subtract(position, projected);
    let denominator = numeric.dot_vector(toward, columns[2])?;
    let numerator = numeric.dot_vector(toward, columns[0])?;
    let seed = numeric.reciprocal_estimate(denominator)?;
    let residual = -seed.mul_add(denominator, -1.0);
    let inverse = seed.mul_add(residual, seed);
    let ratio = numerator.mul_add(inverse, 0.0);
    let atan = try_atan(ratio, numeric)?;
    Ok((numerator, denominator, atan))
}
fn select_yaw(numerator: f32, denominator: f32, atan: f32) -> f32 {
    let sign = numerator.to_bits() & 0x8000_0000;
    let behind = f32::from_bits(0x4049_0fdb | sign) + atan; //PI823557fc
    let yaw = if 0.0 > denominator { behind } else { atan };
    if denominator == 0.0 {
        f32::from_bits(0x3fc9_0fdb | sign)
    } else {
        yaw
    } //HALF_PI82355804
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_controller<S: ControllerFillSource, N: ReckoningNumeric>(
        &mut self,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), ControllerFillError<S::Error, N::Error>> {
        use ControllerFillError as E;
        let holding = source.state().map_err(E::Source)? == 1;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete
            .offboard
            .as_mut()
            .unwrap()
            .is_holding_skateboard = holding;
        let spine = source.spine_world().map_err(E::Source)?;
        let state = source.state().map_err(E::Source)?; //83e4 before matrix loads
        let effective = source.effective_animation().map_err(E::Source)?;
        let projected = yaw_projection(&spine, &effective, numeric).map_err(E::Numeric)?;
        let position = if state == 3 {
            source.retrieval_start_position().map_err(E::Source)?
        } else {
            source.deck_position().map_err(E::Source)?
        };
        let (numerator, denominator, atan) =
            yaw_terms(projected, &effective, position, numeric).map_err(E::Numeric)?;
        let guard = self.sample_write_guard().map_err(E::Access)?; //84b8 before quadrant selects
        let yaw = select_yaw(numerator, denominator, atan);
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().yaw_to_free_board = yaw;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().pitch_to_free_board = 0.0;
        let guard = self.sample_write_guard().map_err(E::Access)?; //853c
        let hiding = source.state().map_err(E::Source)? == 3;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().is_hiding_board = hiding;
        let guard = self.sample_write_guard().map_err(E::Access)?; //8578 before flags load
        let dropping = source.flags_8cc(0x400).map_err(E::Source)? & 0x400 != 0;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().is_dropping_board = dropping;
        let retrieving = source.flags_8cc(0x800).map_err(E::Source)? & 0x800 != 0
            || source.state().map_err(E::Source)? == 4;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().is_retrieving_board = retrieving;
        let flags = source.flags_8cc(0x1800).map_err(E::Source)?;
        let player_retrieving =
            flags & 0x800 != 0 || (flags & 0x1000 != 0 && source.state().map_err(E::Source)? == 4);
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete
            .offboard
            .as_mut()
            .unwrap()
            .is_player_retrieving_board = player_retrieving;
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_controller_fill_tests.rs"]
mod tests;
