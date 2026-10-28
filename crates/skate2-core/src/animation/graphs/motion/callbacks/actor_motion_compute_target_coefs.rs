//! `ComputeTargetCoefsFromSpeedAndStrength::SetCoefs` 0x82ca0520.
//!
//! Create 0x82bb3da8 returns a null instance; Begin/End 0x825187c0 are empty.
//! Every Update 0x82ca8e18 invokes this solver and writes the three retained
//! target coefficients in `PushState`.

use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::skater_push_shared_attributes::{
    PushAnimationAttributes, PushAttributes, SkaterPushSharedAttributes,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TargetCoefficients {
    pub(crate) high_strength_speed: f32,
    pub(crate) low_strength_speed: f32,
    pub(crate) strength: f32,
}

fn fsel(selector: f32, nonnegative: f32, negative: f32) -> f32 {
    if selector >= 0.0 {
        nonnegative
    } else {
        negative
    }
}

/// `fneg` followed by two `fsel` operations. An unordered quotient selects 1.
fn unit_ratio(value: f32) -> f32 {
    let lower = fsel(-value, 0.0, value);
    fsel(1.0 - lower, lower, 1.0)
}

/// One of the two speed rows at 0x82ca0880..0x82ca096c. The staged products
/// and fused operations follow the PPC instruction order, including division
/// by zero and unordered `fsel` behavior.
fn speed_row(
    low: PushAnimationAttributes,
    high: PushAnimationAttributes,
    speed: f32,
) -> (f32, f32, f32) {
    let low_bound = fsel(low.begin_velocity - speed, low.begin_velocity, speed);
    let selected_begin = fsel(
        high.begin_velocity - low_bound,
        low_bound,
        high.begin_velocity,
    );

    let low_times_selected = low.length * selected_begin;
    let low_times_begin = low.length * low.begin_velocity;
    let denominator_start = high.length.mul_add(high.begin_velocity, low_times_selected);
    let numerator = low_times_selected - low_times_begin;
    let denominator_middle = denominator_start - low_times_begin;
    let denominator = (-selected_begin).mul_add(high.length, denominator_middle);
    let speed_blend = unit_ratio(numerator / denominator);

    // 0x82ca090c..0928 reclamps the ratio before weighting both lengths;
    // 0x82ca095c..098c performs the same clamp again for the target stores.
    let weight_blend = unit_ratio(speed_blend);
    let high_length = high.length * weight_blend;
    let low_length = (1.0 - weight_blend) * low.length;
    let blended_length = high_length + low_length;
    let low_end_product = low.end_velocity * low_length;
    let end_product = high.end_velocity.mul_add(high_length, low_end_product);
    let end_velocity = end_product / blended_length;
    (speed_blend, blended_length, end_velocity)
}

/// Source coefficient math. All inputs are native `f32`; there are no authored
/// tuning constants in this callback.
#[must_use]
pub(crate) fn compute(
    block: &PushAttributes,
    board_z_speed: f32,
    current_push_dv: f32,
) -> TargetCoefficients {
    let nonnegative_speed = fsel(board_z_speed, board_z_speed, 0.0);
    let speed_lower = fsel(
        block.begin_velocity_min - nonnegative_speed,
        block.begin_velocity_min,
        nonnegative_speed,
    );
    let speed = fsel(
        block.begin_velocity_max - speed_lower,
        speed_lower,
        block.begin_velocity_max,
    );
    let dv_lower = fsel(
        block.dv_min - current_push_dv,
        block.dv_min,
        current_push_dv,
    );
    let dv = fsel(block.dv_max - dv_lower, dv_lower, block.dv_max);

    let (hst_speed, hst_length, hst_end) = speed_row(block.lsp_hst, block.hsp_hst, speed);
    let (lst_speed, lst_length, lst_end) = speed_row(block.lsp_lst, block.hsp_lst, speed);

    let desired_end = dv + speed;
    let target_end_lower = fsel(lst_end - desired_end, lst_end, desired_end);
    let target_end = fsel(hst_end - target_end_lower, target_end_lower, hst_end);
    let lst_product = target_end * lst_length;
    let lst_end_product = lst_end * lst_length;
    let combined = hst_end.mul_add(hst_length, lst_product);
    let numerator = lst_product - lst_end_product;
    let denominator_middle = combined - lst_end_product;
    let denominator = (-target_end).mul_add(hst_length, denominator_middle);
    let strength = unit_ratio(numerator / denominator);

    TargetCoefficients {
        // Native repeats the same fneg/fsel clamp immediately before each
        // store, after already clamping the row's solved ratio.
        high_strength_speed: unit_ratio(hst_speed),
        low_strength_speed: unit_ratio(lst_speed),
        strength: unit_ratio(strength),
    }
}

impl ActorConstruction {
    /// Motion Update 0x82ca8e18; PushState stores are offsets +0x14,+0x18,+0x1c.
    pub(crate) fn motion_runtime_compute_target_coefs(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        regular_attributes: bool,
    ) -> Result<(), Error> {
        self.check_motion_runtime_context(context)?;
        let animation = {
            let actor = self.prefix().map_err(Error::Actor)?;
            let animation = actor
                .animation
                .as_ref()
                .filter(|_| actor.animation_published)
                .ok_or(Error::MissingOwner("ComputeTargetCoefs ISkaterAnim"))?;
            let retained = animation.lock().map_err(|_| Error::Lock)?;
            if !retained
                .entity()
                .is_same_issuance(&self.authority.entity_handle())
                || !actor.animation_interfaces[2]
                    .as_ref()
                    .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            {
                return Err(Error::ForeignOwner("ComputeTargetCoefs ISkaterAnim"));
            }
            if !actor.motion_graph_published {
                return Err(Error::MissingOwner("ComputeTargetCoefs ISkaterMotionGraph"));
            }
            let motion = actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .ok_or(Error::MissingOwner("ComputeTargetCoefs ISkaterMotionGraph"))?;
            if !actor.graph_interfaces[2]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
            {
                return Err(Error::ForeignOwner("ComputeTargetCoefs ISkaterMotionGraph"));
            }
            Arc::clone(animation)
        };
        let physical = self.cached_physical_allocation().map_err(Error::Physical)?;
        // Native obtains the PushState pointer before physical speed, then
        // loads mCurrPushDv only after switch/shared-block selection.
        {
            let actor = self.prefix().map_err(Error::Actor)?;
            actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .and_then(|motion| motion.state.as_ref())
                .ok_or(Error::MissingOwner("ComputeTargetCoefs PushState"))?;
        }
        let speed = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_motion_z_speed()
            .map_err(Error::PhysOut)?;
        let is_switch = {
            let animation = animation.lock().map_err(|_| Error::Lock)?;
            animation
                .player()
                .ok_or(Error::MissingOwner("ComputeTargetCoefs SkaterAnim player"))?
                .relative_stance
                == 1
        };
        let shared: Arc<SkaterPushSharedAttributes> = self.push_shared_attributes()?;
        let selected = shared.selected(regular_attributes, is_switch);
        let current_push_dv = {
            let actor = self.prefix().map_err(Error::Actor)?;
            actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .and_then(|motion| motion.state.as_ref())
                .ok_or(Error::MissingOwner("ComputeTargetCoefs PushState"))?
                .push[1]
        };
        let target = compute(selected, speed, current_push_dv);
        let mut actor = self.prefix().map_err(Error::Actor)?;
        let push = &mut actor
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .and_then(|motion| motion.state.as_mut())
            .ok_or(Error::MissingOwner("ComputeTargetCoefs PushState"))?
            .push;
        push[5] = target.high_strength_speed;
        push[6] = target.low_strength_speed;
        push[7] = target.strength;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_two_speed_rows_and_strength_weight() {
        let block = PushAttributes {
            lsp_hst: PushAnimationAttributes {
                length: 1.0,
                begin_velocity: 2.0,
                end_velocity: 4.0,
            },
            hsp_hst: PushAnimationAttributes {
                length: 1.0,
                begin_velocity: 6.0,
                end_velocity: 8.0,
            },
            lsp_lst: PushAnimationAttributes {
                length: 1.0,
                begin_velocity: 1.0,
                end_velocity: 1.0,
            },
            hsp_lst: PushAnimationAttributes {
                length: 1.0,
                begin_velocity: 5.0,
                end_velocity: 5.0,
            },
            begin_velocity_min: 0.0,
            begin_velocity_max: 10.0,
            dv_min: 0.0,
            dv_max: 10.0,
        };
        let coefs = compute(&block, 4.0, 1.0);
        assert_eq!(coefs.high_strength_speed, 0.5);
        assert_eq!(coefs.low_strength_speed, 0.75);
        assert_eq!(coefs.strength, 0.5);
    }

    #[test]
    fn source_fsel_preserves_unordered_ratio_selection() {
        assert_eq!(unit_ratio(f32::NAN), 1.0);
        assert_eq!(unit_ratio(-1.0), 0.0);
        assert_eq!(unit_ratio(2.0), 1.0);
    }
}
