//! `ComputeRepushDeadline::End` 0x82ca0d60. Create returns a null instance;
//! Begin and Update are single `blr` callbacks.

use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_construction::action_motion_intent_bridge::disable_push::PushingMotionAttributes,
    skater_push_shared_attributes::PushAttributes,
};
use std::sync::Arc;

fn fsel(selector: f32, nonnegative: f32, negative: f32) -> f32 {
    if selector >= 0.0 {
        nonnegative
    } else {
        negative
    }
}

fn unit_ratio(numerator: f32, denominator: f32) -> f32 {
    let ratio = numerator / denominator; // native fdivs
    let lower = fsel(-ratio, 0.0, ratio);
    fsel(1.0 - lower, lower, 1.0)
}

/// 0x82ca102c..0x82ca10dc precedes both fresh Vault scalar reads.
fn fractions(block: &PushAttributes, current_push_dv: f32, board_z_speed: f32) -> (f32, f32) {
    let lower_speed_bound = block.begin_velocity_min + block.dv_min;
    let speed = fsel(board_z_speed, board_z_speed, 0.0);
    let dv_floor = fsel(
        block.dv_min - current_push_dv,
        block.dv_min,
        current_push_dv,
    );
    let speed_floor = fsel(lower_speed_bound - speed, lower_speed_bound, speed);
    let dv = fsel(block.dv_max - dv_floor, dv_floor, block.dv_max);
    let bounded_speed = fsel(
        block.begin_velocity_max - speed_floor,
        speed_floor,
        block.begin_velocity_max,
    );
    let dv_fraction = unit_ratio(dv - block.dv_min, block.dv_max - block.dv_min);
    let speed_fraction = unit_ratio(
        bounded_speed - lower_speed_bound,
        block.begin_velocity_max - lower_speed_bound,
    );
    (dv_fraction, speed_fraction)
}

/// 0x82ca1184..0x82ca11a4 follows the ordered cap and weight reads.
fn finish(dv_fraction: f32, speed_fraction: f32, cap: f32, weight: f32) -> f32 {
    let weighted_speed = weight * speed_fraction;
    let mixed = (1.0 - weight).mul_add(dv_fraction, weighted_speed);
    let nonnegative = fsel(-mixed, 0.0, mixed);
    fsel(cap - nonnegative, nonnegative, cap)
}

/// Float sequence at 0x82ca102c..0x82ca11a4 for an isolated value check.
#[cfg(test)]
fn out_factor(
    block: &PushAttributes,
    current_push_dv: f32,
    board_z_speed: f32,
    cap: f32,
    weight: f32,
) -> f32 {
    let (dv_fraction, speed_fraction) = fractions(block, current_push_dv, board_z_speed);
    finish(dv_fraction, speed_fraction, cap, weight)
}

impl ActorConstruction {
    /// Source End writes only PushState.mOutFactor (+0) after both Vault reads.
    pub(crate) fn motion_runtime_compute_repush_deadline_end(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        regular_attributes: bool,
        pushing: Option<&PushingMotionAttributes>,
    ) -> Result<(), Error> {
        self.check_motion_runtime_context(context)?;
        // Native interface order: ISkaterAnim, ISkaterMotionGraph, IPhysical.
        let animation = {
            let actor = self.prefix().map_err(Error::Actor)?;
            let animation = actor
                .animation
                .as_ref()
                .filter(|_| actor.animation_published)
                .ok_or(Error::MissingOwner("ComputeRepushDeadline ISkaterAnim"))?;
            let retained = animation.lock().map_err(|_| Error::Lock)?;
            if !retained
                .entity()
                .is_same_issuance(&self.authority.entity_handle())
                || !actor.animation_interfaces[2]
                    .as_ref()
                    .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            {
                return Err(Error::ForeignOwner("ComputeRepushDeadline ISkaterAnim"));
            }
            if !actor.motion_graph_published {
                return Err(Error::MissingOwner(
                    "ComputeRepushDeadline ISkaterMotionGraph",
                ));
            }
            let motion = actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .ok_or(Error::MissingOwner(
                    "ComputeRepushDeadline ISkaterMotionGraph",
                ))?;
            if !actor.graph_interfaces[2]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
            {
                return Err(Error::ForeignOwner(
                    "ComputeRepushDeadline ISkaterMotionGraph",
                ));
            }
            Arc::clone(animation)
        };
        let physical = self.cached_physical_allocation().map_err(Error::Physical)?;
        // Source gets the PushState pointer before switch/shared/speed reads.
        {
            let actor = self.prefix().map_err(Error::Actor)?;
            actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .and_then(|motion| motion.state.as_ref())
                .ok_or(Error::MissingOwner("ComputeRepushDeadline PushState"))?;
        }
        let is_switch = {
            let animation = animation.lock().map_err(|_| Error::Lock)?;
            animation
                .player()
                .ok_or(Error::MissingOwner(
                    "ComputeRepushDeadline SkaterAnim player",
                ))?
                .relative_stance
                == 1
        };
        let shared = self.push_shared_attributes()?;
        let block = shared.selected(regular_attributes, is_switch);
        let current_push_dv = {
            let actor = self.prefix().map_err(Error::Actor)?;
            actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .and_then(|motion| motion.state.as_ref())
                .ok_or(Error::MissingOwner("ComputeRepushDeadline PushState"))?
                .push[1]
        };
        let speed = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_motion_z_speed()
            .map_err(Error::PhysOut)?;
        let (dv_fraction, speed_fraction) = fractions(block, current_push_dv, speed);
        let pushing = pushing.ok_or(Error::MissingOwner(
            "ComputeRepushDeadline anim_motion_pushing+13c",
        ))?;
        pushing.check_vault(&physical.owner().inputs().vault)?;
        let cap = pushing.repush_scalar(0x792c_66a0_ddc0_2df7)?;
        let weight = pushing.repush_scalar(0x1761_c0ae_4c9e_79c2)?;
        let result = finish(dv_fraction, speed_fraction, cap, weight);
        self.prefix()
            .map_err(Error::Actor)?
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .and_then(|motion| motion.state.as_mut())
            .ok_or(Error::MissingOwner("ComputeRepushDeadline PushState"))?
            .push[0] = result;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_blends_speed_and_delta_velocity_then_caps() {
        let block = PushAttributes {
            begin_velocity_min: 2.0,
            begin_velocity_max: 10.0,
            dv_min: 1.0,
            dv_max: 5.0,
            ..PushAttributes::default()
        };
        // dv fraction=(3-1)/(5-1)=0.5; speed fraction=(6-3)/(10-3)=3/7.
        let result = out_factor(&block, 3.0, 6.0, 0.75, 0.5);
        assert!((result - 13.0 / 28.0).abs() < 1.0e-6);
        assert_eq!(out_factor(&block, 5.0, 10.0, 0.4, 0.5), 0.4);
    }

    #[test]
    fn source_fsel_handles_unordered_fraction_and_unclamped_weight() {
        let block = PushAttributes::default();
        assert_eq!(out_factor(&block, 0.0, 0.0, 0.8, 0.5), 0.8);
        let block = PushAttributes {
            begin_velocity_min: 0.0,
            begin_velocity_max: 10.0,
            dv_min: 0.0,
            dv_max: 10.0,
            ..PushAttributes::default()
        };
        assert_eq!(out_factor(&block, 0.0, 0.0, 1.0, 2.0), 0.0);
    }
}
