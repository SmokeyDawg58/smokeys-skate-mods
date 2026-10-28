//! Same-owner wheel-line fallback and retained velocity updates82b36ed0..37094.
use super::SkateboardContactSurface;
use crate::{reckoning_numeric::ReckoningNumeric, skeleton_root_transforms::SkeletonRootVector};
use std::cmp::Ordering;

#[cfg(test)]
#[path = "skateboard_contact_motion_tests.rs"]
mod tests;

pub(crate) trait SkateboardContactMotionServices: ReckoningNumeric {
    type LineTarget;
    fn capture_line_threshold(&mut self) -> Result<Self::LineTarget, Self::Error>;
    fn wheel_distance(&mut self, wheel: usize) -> Result<f32, Self::Error>;
    fn line_threshold(&mut self, target: &Self::LineTarget) -> Result<f32, Self::Error>;
    fn wheel_normal(&mut self, wheel: usize) -> Result<SkeletonRootVector, Self::Error>;
    fn body_velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
    fn grip_enabled(&mut self) -> Result<bool, Self::Error>;
    fn grip_cosine(&mut self) -> Result<f32, Self::Error>;
    fn grip_vector(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}
#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardContactMotionError<E> {
    Service(E),
    Unknown(&'static str, usize),
}
impl SkateboardContactSurface {
    /// Caller has already applied the entry dt assertion and contact traversal.
    /// Completion does not vote or certify a completed normal publication.
    pub(crate) fn try_finish_line_fallback<S: SkateboardContactMotionServices>(
        &mut self,
        services: &mut S,
    ) -> Result<(), SkateboardContactMotionError<S::Error>> {
        use SkateboardContactMotionError as E;
        for wheel in 0..4 {
            if self.collision_state.in_contact[wheel].ok_or(E::Unknown("contact", wheel))? {
                continue;
            }
            let target = services.capture_line_threshold().map_err(E::Service)?;
            let distance = services.wheel_distance(wheel).map_err(E::Service)?;
            let threshold = services.line_threshold(&target).map_err(E::Service)?;
            if distance.partial_cmp(&threshold) == Some(Ordering::Less) {
                self.collision_state.normal[wheel] =
                    Some(services.wheel_normal(wheel).map_err(E::Service)?);
            } else {
                self.collision_state.surfaces.set_validity(wheel, 0);
            }
        }
        Ok(())
    }

    /// The source fdiv is evaluated once, before the seven Part iterations.
    /// Opposing-contact delta commits after the division and stair-flag clear.
    pub(crate) fn try_update_contact_motion<S: SkateboardContactMotionServices>(
        &mut self,
        timestep: f32,
        services: &mut S,
    ) -> Result<(), SkateboardContactMotionError<S::Error>> {
        use SkateboardContactMotionError as E;
        let delta = self
            .post_prefix
            .opposing_max
            .ok_or(E::Unknown("opposing maximum", 6))?
            - self
                .post_prefix
                .opposing_min
                .ok_or(E::Unknown("opposing minimum", 6))?;
        let inverse_dt = 1.0 / timestep;
        self.collision_state.flags.and_mask(0x7fff_ffff);
        let negative = f32::from_bits(delta.to_bits() ^ 0x8000_0000);
        self.collision_state.opposing_delta = Some(
            if negative
                .partial_cmp(&0.0)
                .is_some_and(|order| order != Ordering::Less)
            {
                0.0
            } else {
                delta
            },
        );
        for part in 0..7 {
            let old = self.collision_state.velocity[part].ok_or(E::Unknown("velocity", part))?;
            let new = services.body_velocity(part).map_err(E::Service)?;
            let difference = SkeletonRootVector {
                xyz: std::array::from_fn(|lane| new.xyz[lane] - old.xyz[lane]),
                w: new.w.zip(old.w).map(|(new, old)| new - old),
            };
            self.collision_state.acceleration[part] = Some(difference);
            self.collision_state.acceleration[part] = Some(SkeletonRootVector {
                xyz: difference.xyz.map(|lane| lane * inverse_dt),
                w: difference.w.map(|lane| lane * inverse_dt),
            });
            self.collision_state.velocity[part] = Some(new);
            if part == 6 && std::env::var_os("SKATE2_TRACE_PUSH_INTERRUPTION").is_some() {
                eprintln!(
                    "push deck acceleration: dt={timestep} old_velocity={:?} new_velocity={:?} acceleration={:?}",
                    old.xyz, new.xyz, difference.xyz.map(|lane| lane * inverse_dt),
                );
            }
            let in_contact =
                self.collision_state.in_contact[part].ok_or(E::Unknown("contact", part))?;
            if in_contact
                && self
                    .collision_state
                    .surfaces
                    .physics_type(part)
                    .ok_or(E::Unknown("physics type", part))?
                    == 8
            {
                self.collision_state.flags.or_mask(0x8000_0000);
            }
            if services.grip_enabled().map_err(E::Service)?
                && in_contact
                && self
                    .collision_state
                    .surfaces
                    .validity(part)
                    .ok_or(E::Unknown("validity", part))?
                    != 0
            {
                let cosine = services.grip_cosine().map_err(E::Service)?;
                let grip = services.grip_vector().map_err(E::Service)?;
                let normal = self.collision_state.normal[part].ok_or(E::Unknown("normal", part))?;
                let scratch =
                    |v: SkeletonRootVector| [v.xyz[0], v.xyz[1], v.xyz[2], v.w.unwrap_or(0.0)];
                let dot = services
                    .dot3(scratch(normal), scratch(grip))
                    .map_err(E::Service)?;
                if cosine.partial_cmp(&dot) == Some(Ordering::Greater) {
                    self.collision_state.flags.or_mask(0x0800_0000);
                }
            }
        }
        Ok(())
    }
}
