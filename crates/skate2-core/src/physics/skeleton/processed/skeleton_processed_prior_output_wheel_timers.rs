//! Consecutive wheel flags and retained skitch/repush timers, 82e03130..3500.
use crate::physics_state_ids::PHYSICS_STATE_SKITCHING;
use super::SkeletonProcessedContact;
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::{PhysicalPlayerAssemblyOwner, hilod_prefix::PhysicalHiLodPrefix},
    physical_player_phys_out::PhysicalPlayerPhysOut,
    physical_player_phys_out::construct::prior_input::{PriorInputReadError, PriorWheelContact},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PriorOutputWheelTimers {
    /// Processed +930. The retained vehicle itself belongs to the `HiLOD` prefix.
    pub(crate) skitching_vehicle_id: Option<u32>,
    /// Processed +a1c; independent of the prefix's retained countdown.
    pub(crate) skitch_delay_until_reskitchable: Option<f32>,
    /// Processed +a20.
    pub(crate) pushing_delay_until_repushable: Option<f32>,
    pub(crate) complete: bool,
}
impl PriorOutputWheelTimers {
    pub(super) const fn new() -> Self {
        Self {
            skitching_vehicle_id: None,
            skitch_delay_until_reskitchable: None,
            pushing_delay_until_repushable: None,
            complete: false,
        }
    }
    pub(super) fn reset(&mut self) {
        self.skitching_vehicle_id = Some(0); //82b39540
        self.skitch_delay_until_reskitchable = Some(0.0); //82b39590
        self.pushing_delay_until_repushable = Some(0.0); //82b39594
        self.complete = false;
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum PriorOutputWheelTimersError {
    ForeignOutput,
    Poisoned,
    Missing(&'static str),
    Read(PriorInputReadError),
    Cvar(InputCvarError),
}
impl std::fmt::Display for PriorOutputWheelTimersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignOutput => f.write_str("wheel/timer output occurrence differs"),
            Self::Poisoned => f.write_str("wheel/timer owner lock poisoned"),
            Self::Missing(field) => write!(f, "wheel/timer missing {field}"),
            Self::Read(error) => write!(f, "wheel/timer guarded read: {error:?}"),
            Self::Cvar(error) => write!(f, "wheel/timer cvar: {error:?}"),
        }
    }
}

/// Each pair independently guards the first wheel, and guards the second only
/// when the first is false. No previously sampled wheel value is reused.
fn wheel_pairs<E>(
    mut read: impl FnMut(PriorWheelContact) -> Result<bool, E>,
) -> Result<[bool; 4], E> {
    use PriorWheelContact::{Wheel0, Wheel1, Wheel2, Wheel3};
    Ok([
        read(Wheel2)? || read(Wheel3)?,
        read(Wheel0)? || read(Wheel1)?,
        read(Wheel2)? || read(Wheel0)?,
        read(Wheel3)? || read(Wheel1)?,
    ])
}

impl SkeletonProcessedContact {
    /// Retained Processed+0xa20, read by the OffBoard grab-query gate.
    pub(crate) fn pushing_delay_until_repushable(&self) -> Option<f32> {
        self.prior_output
            .wheel_timers
            .pushing_delay_until_repushable
    }
    pub(crate) const fn prior_wheel_timer_fields(&self) -> &PriorOutputWheelTimers {
        &self.prior_output.wheel_timers
    }
    /// Source 82e03130..3500, stopping before the `GroundPosition` branch3504.
    pub(crate) fn try_copy_prior_output_wheel_timers(
        &mut self,
        owner: &PhysicalPlayerAssemblyOwner,
    ) -> Result<(), PriorOutputWheelTimersError> {
        use PriorOutputWheelTimersError as E;
        self.prior_output.wheel_timers.complete = false;
        let output = owner.phys_out().lock().map_err(|_| E::Poisoned)?;
        let mut prefix_owner = owner.prefix().lock().map_err(|_| E::Poisoned)?;
        let prefix = prefix_owner.as_mut().ok_or(E::Missing("HiLOD prefix"))?;
        self.try_copy_wheel_timer_fields(&output, prefix, &owner.inputs().cvars)
    }

    fn try_copy_wheel_timer_fields(
        &mut self,
        output: &PhysicalPlayerPhysOut,
        prefix: &mut PhysicalHiLodPrefix,
        cvars: &InputCvarReferences,
    ) -> Result<(), PriorOutputWheelTimersError> {
        use PriorOutputWheelTimersError as E;
        self.prior_output.wheel_timers.complete = false;
        if !self.prior_output.matches_output_occurrence(output) {
            return Err(E::ForeignOutput);
        }
        let [rear, front, left, right] =
            wheel_pairs(|wheel| output.try_guarded_wheel_in_contact(wheel).map_err(E::Read))?;
        for (mask, normal, flipped) in [
            (0x1000_0000, rear, front),
            (0x0800_0000, front, rear),
            (0x0400_0000, left, right),
            (0x0200_0000, right, left),
        ] {
            let flip = self.board_flipped().ok_or(E::Missing("board flip"))?;
            if !self.write_control_flag8c8(mask, if flip { flipped } else { normal }) {
                return Err(E::Missing("processed8c8"));
            }
        }
        if output
            .try_guarded_state_output()
            .map_err(E::Read)?
            .physics_state
            == PHYSICS_STATE_SKITCHING
        {
            prefix.skitch_vehicle_collision_avoid_timer = cvars
                .try_time_to_avoid_vehicle_after_skitching()
                .map_err(E::Cvar)?;
            let object_id = output
                .try_guarded_state_output()
                .map_err(E::Read)?
                .grab_spline_desc
                .object_id;
            // Native lwz341c/stw3420 copies the signed descriptor's raw word.
            prefix.skitching_vehicle_id = u32::from_ne_bytes(object_id.to_ne_bytes());
        }
        // All five native conditional branches are NOTGT. NaN skips the body.
        if prefix.skitch_vehicle_collision_avoid_timer > 0.0 {
            let dt = self
                .physics_state_fields()
                .time_delta
                .ok_or(E::Missing("processed dt"))?;
            prefix.skitch_vehicle_collision_avoid_timer -= dt;
            self.prior_output.wheel_timers.skitching_vehicle_id = Some(prefix.skitching_vehicle_id);
        }
        let incoming = output
            .try_guarded_ground_output()
            .map_err(E::Read)?
            .skitch_delay_until_reskitchable;
        if incoming > 0.0 {
            prefix.skitch_delay_until_reskitchable = incoming;
        }
        if prefix.skitch_delay_until_reskitchable > 0.0 {
            let dt = self
                .physics_state_fields()
                .time_delta
                .ok_or(E::Missing("processed dt"))?;
            prefix.skitch_delay_until_reskitchable -= dt;
            self.prior_output
                .wheel_timers
                .skitch_delay_until_reskitchable = Some(prefix.skitch_delay_until_reskitchable);
            self.prior_output.wheel_timers.skitching_vehicle_id = Some(prefix.skitching_vehicle_id);
        }
        let incoming = output
            .try_guarded_ground_output()
            .map_err(E::Read)?
            .pushing_delay_until_repushable;
        if incoming > 0.0 {
            prefix.pushing_delay_until_repushable = incoming;
        }
        if prefix.pushing_delay_until_repushable > 0.0 {
            let dt = self
                .physics_state_fields()
                .time_delta
                .ok_or(E::Missing("processed dt"))?;
            prefix.pushing_delay_until_repushable -= dt;
            self.prior_output
                .wheel_timers
                .pushing_delay_until_repushable = Some(prefix.pushing_delay_until_repushable);
        }
        self.prior_output.wheel_timers.complete = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "skeleton_processed_prior_output_wheel_timers_tests.rs"]
mod tests;
