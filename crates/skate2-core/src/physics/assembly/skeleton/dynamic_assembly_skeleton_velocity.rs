//! Actual Skeleton Body velocity operations used by `WipeoutGround.Enter`.
//! Native packed W is a body link and is preserved by the existing XYZ setter.
use super::{SkeletonNormalAssembly, SkeletonNormalSetupError};
use crate::{
    reckoning_numeric::ReckoningNumeric, simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_assembly::SkateboardSimulationBackend, skeleton_root_transforms::SkeletonRootVector,
};
use std::{cmp::Ordering, convert::Infallible};

#[derive(Debug)]
pub(crate) enum SkeletonVelocityError<N> {
    Owner(SkeletonNormalSetupError<Infallible, Infallible>),
    MissingPart(usize),
    Body {
        part: usize,
        source: SimulationRigidBodyPoolError,
    },
    Numeric {
        part: usize,
        source: N,
    },
    InvalidMaximum {
        bits: u32,
    },
}
impl<N: std::fmt::Debug> std::fmt::Display for SkeletonVelocityError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "Skeleton velocity owner: {e}"),
            Self::MissingPart(part) => write!(f, "Skeleton velocity missing Part {part}"),
            Self::Body { part, source } => write!(f, "Skeleton velocity Part {part}: {source}"),
            Self::Numeric { part, source } => {
                write!(f, "Skeleton velocity numeric Part {part}: {source:?}")
            }
            Self::InvalidMaximum { bits } => write!(
                f,
                "Skeleton velocity maximum fails positive assertion: {bits:#010x}"
            ),
        }
    }
}
impl SkeletonNormalAssembly {
    /// `WipeoutGround.Enter82d8eb64` clears truck-tilt before state-local stores.
    pub(crate) fn clear_wipeout_truck_tilt(&mut self) {
        self.use_physics_truck_tilt = Some(false);
    }
    /// `Skeleton::DisableIK` 0x82b462f0, as WipeoutGround, LandingOnDeck and
    /// `OffBoardFeetIK::ConsiderReset` call it.
    pub(crate) fn try_disable_wipeout_ik(
        &mut self,
    ) -> Result<(), crate::skeleton_current_animation::SkeletonIkUnconstructed> {
        self.current_animation.try_disable_ik()
    }
    /// `Skeleton.SetLinearDrag82b3ba38` forwards all Parts (-1) to Body.
    pub(crate) fn try_set_wipeout_linear_drag(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        value: f32,
    ) -> Result<(), SkeletonVelocityError<Infallible>> {
        use SkeletonVelocityError as E;
        self.validate_attached(backend).map_err(E::Owner)?;
        let handles = std::array::from_fn(|part| {
            self.session.part_attachment.parts[part]
                .body
                .expect("attached Skeleton retains all24 bodies")
        });
        backend
            .try_set_skeleton_drag(&handles, self.state.binding.inertias(), false, value)
            .map_err(|(part, source)| E::Body { part, source })
    }
    /// Body82b3c000: Parts1..14 use upper, then Parts15..23 use lower.
    /// Supplied direction is not normalized. Each real Part read/store is late.
    pub(crate) fn try_scale_velocity_along_vector<N: ReckoningNumeric>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        direction: SkeletonRootVector,
        lower: f32,
        upper: f32,
        numeric: &mut N,
    ) -> Result<(), SkeletonVelocityError<N::Error>> {
        use SkeletonVelocityError as E;
        self.validate_attached(backend).map_err(E::Owner)?;
        for part in 1..24 {
            let handle = self.session.part_attachment.parts[part]
                .body
                .ok_or(E::MissingPart(part))?;
            let observation = backend
                .collision_body_pool()
                .try_observe(handle)
                .map_err(|source| E::Body { part, source })?;
            let velocity = backend
                .collision_body_pool()
                .try_contact_spy_linear_velocity(observation)
                .map_err(|source| E::Body { part, source })?;
            let scalar = if part <= 14 { upper } else { lower };
            let result = scale_velocity(velocity, direction.xyz, scalar, numeric)
                .map_err(|source| E::Numeric { part, source })?;
            backend
                .try_set_part_linear_velocity(observation, result)
                .map_err(|source| E::Body { part, source })?;
        }
        Ok(())
    }

    /// Body82b3c160. This selected caller supplies `maxMagnitude` splatted from
    /// the fresh runtime cvar. Positive assertion precedes every Part read.
    pub(crate) fn try_clamp_velocity_magnitude<N: ReckoningNumeric>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        maximum: f32,
        numeric: &mut N,
    ) -> Result<(), SkeletonVelocityError<N::Error>> {
        use SkeletonVelocityError as E;
        self.validate_attached(backend).map_err(E::Owner)?;
        if maximum.partial_cmp(&0.0) != Some(Ordering::Greater) {
            return Err(E::InvalidMaximum {
                bits: maximum.to_bits(),
            });
        }
        let maximum_squared = maximum * maximum;
        for part in 1..24 {
            let handle = self.session.part_attachment.parts[part]
                .body
                .ok_or(E::MissingPart(part))?;
            let observation = backend
                .collision_body_pool()
                .try_observe(handle)
                .map_err(|source| E::Body { part, source })?;
            let velocity = backend
                .collision_body_pool()
                .try_contact_spy_linear_velocity(observation)
                .map_err(|source| E::Body { part, source })?;
            if let Some(result) = clamp_velocity(velocity, maximum, maximum_squared, numeric)
                .map_err(|source| E::Numeric { part, source })?
            {
                backend
                    .try_set_part_linear_velocity(observation, result)
                    .map_err(|source| E::Body { part, source })?;
            }
        }
        Ok(())
    }
}
fn scale_velocity<N: ReckoningNumeric>(
    velocity: [f32; 3],
    direction: [f32; 3],
    scalar: f32,
    numeric: &mut N,
) -> Result<[f32; 3], N::Error> {
    let dot = numeric.dot3(
        [direction[0], direction[1], direction[2], 0.0],
        [velocity[0], velocity[1], velocity[2], 0.0],
    )?;
    let projected = direction.map(|lane| lane * dot);
    Ok(std::array::from_fn(|lane| {
        projected[lane].mul_add(scalar, velocity[lane] - projected[lane])
    }))
}
fn clamp_velocity<N: ReckoningNumeric>(
    velocity: [f32; 3],
    maximum: f32,
    maximum_squared: f32,
    numeric: &mut N,
) -> Result<Option<[f32; 3]>, N::Error> {
    let value = [velocity[0], velocity[1], velocity[2], 0.0];
    let squared = numeric.dot3(value, value)?;
    if squared.partial_cmp(&maximum_squared) != Some(Ordering::Greater) {
        return Ok(None);
    }
    let seed = numeric.reciprocal_sqrt_estimate(squared)?;
    let square = seed * seed;
    let half = seed * 0.5;
    let residual = -squared.mul_add(square, -1.0);
    let once = half.mul_add(residual, seed);
    let square = once * once;
    let half = once * 0.5;
    let residual = -squared.mul_add(square, -1.0);
    let twice = half.mul_add(residual, once);
    let root = squared * twice;
    let length = if squared == 0.0 { 0.0 } else { root };
    let reciprocal = numeric.reciprocal_estimate(length)?;
    let residual = -reciprocal.mul_add(length, -1.0);
    let once = reciprocal.mul_add(residual, reciprocal);
    let residual = -once.mul_add(length, -1.0);
    let twice = once.mul_add(residual, once);
    let scalar = twice * maximum;
    Ok(Some(velocity.map(|lane| lane * scalar)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    #[test]
    fn scale_uses_supplied_direction_without_normalizing() {
        assert_eq!(
            scale_velocity(
                [3.0, 4.0, 5.0],
                [0.0, 2.0, 0.0],
                0.5,
                &mut HostReckoningNumeric
            )
            .unwrap()
            .map(f32::to_bits),
            [3.0_f32, -4.0, 5.0].map(f32::to_bits)
        );
    }
    #[test]
    fn clamp_skips_equal_and_nan_but_limits_large_velocity() {
        assert_eq!(
            clamp_velocity([3.0, 4.0, 0.0], 5.0, 25.0, &mut HostReckoningNumeric).unwrap(),
            None
        );
        assert_eq!(
            clamp_velocity([f32::NAN, 0.0, 0.0], 5.0, 25.0, &mut HostReckoningNumeric).unwrap(),
            None
        );
        assert_eq!(
            clamp_velocity([0.0, 8.0, 0.0], 4.0, 16.0, &mut HostReckoningNumeric)
                .unwrap()
                .map(|v| v.map(f32::to_bits)),
            Some([0.0_f32, 4.0, 0.0].map(f32::to_bits))
        );
    }
}
