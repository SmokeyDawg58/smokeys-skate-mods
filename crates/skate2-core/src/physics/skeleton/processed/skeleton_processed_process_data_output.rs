//! ProcessData82b5f258..f2e0 writes to the SAME passed processed storage.
use super::{
    SkeletonProcessedContact, jump_events::SkeletonProcessedJumpEvents,
    physics_state::ProcessedPhysicsStateFields, prior_output::SkeletonProcessedPriorOutputState,
};
use crate::skeleton_root_transforms::SkeletonRootVector;

/// These fields have no earlier represented producer. The collision and COM
/// sources remain on Skeleton; retaining their copies does not transfer ownership.
/// `mComToDeck`+2f0/`mComToDeckDelta`+300 copy Skeleton+2a30/+2a20, the
/// animation record's world COM-to-deck (`UpdateAnimRecord` State+10f0/+10e0),
/// not the animation-space centre of mass at State+10c0.
#[derive(Debug)]
pub(crate) struct ProcessDataOutput {
    pub(crate) deck_zy_delta: Option<f32>,
    pub(crate) hook_to_hips: Option<SkeletonRootVector>,
    pub(crate) com_to_deck: Option<SkeletonRootVector>,
    pub(crate) com_to_deck_delta: Option<SkeletonRootVector>,
}
impl ProcessDataOutput {
    pub(super) const fn new() -> Self {
        Self {
            deck_zy_delta: None,
            hook_to_hips: None,
            com_to_deck: None,
            com_to_deck_delta: None,
        }
    }
    pub(super) fn reset(&mut self) {
        self.deck_zy_delta = Some(0.0); //Reset82b39524 +9cc
        self.hook_to_hips = Some(SkeletonRootVector::known([0.0; 4]));
        self.com_to_deck = Some(SkeletonRootVector::known([0.0; 4]));
        self.com_to_deck_delta = Some(SkeletonRootVector::known([0.0; 4]));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProcessDataOutputError {
    UnknownCollisionFlags,
}
impl std::fmt::Display for ProcessDataOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProcessData requires the retained processed collision flags")
    }
}

impl SkeletonProcessedContact {
    /// f268 compares a splat of the adjusted deck Z-axis Y against zero.
    /// Unordered and either signed zero clear1000 and set its complement800.
    pub(crate) fn write_process_data_deck_orientation(&mut self, zy: f32) {
        let positive = zy > 0.0;
        self.jump_events.process_animation_flag(0x1000, positive);
        self.jump_events.process_animation_flag(0x800, !positive);
    }

    /// The native byte insertion consumes only bit0, unlike the later hook
    /// validity comparison which consumes any nonzero byte.
    pub(crate) fn write_process_data_collision(&mut self, captured: u32, just_collided: u8) {
        self.flags_8c8 = Some((captured & !0x40000) | (u32::from(just_collided & 1) << 18));
    }
    pub(crate) fn capture_process_data_collision_flags(
        &self,
    ) -> Result<u32, ProcessDataOutputError> {
        self.flags_8c8
            .ok_or(ProcessDataOutputError::UnknownCollisionFlags)
    }
}

use crate::{
    physical_player_grind_air_adjust::update::{
        GrindAirAdjustOut, GrindAirProcessed, GrindAirServices, GrindAirUpdateError,
    },
    physical_player_grind_manager::math::GrindNum,
    skeleton_current_animation::continuation::{
        AnimationInputError, AnimationProcessedRead, GrindAdjustRead,
    },
};

pub(crate) struct ProcessDataRead<'a> {
    physics: &'a ProcessedPhysicsStateFields,
    prior: &'a SkeletonProcessedPriorOutputState,
    flags8c8: &'a Option<u32>,
    flags8c4: &'a SkeletonProcessedJumpEvents,
    deck: &'a Option<crate::skeleton_root_transforms::SkeletonRootMatrix>,
}
fn need<T>(value: Option<T>, field: &'static str) -> Result<T, AnimationInputError> {
    value.ok_or(AnimationInputError::Missing(field))
}
impl AnimationProcessedRead for ProcessDataRead<'_> {
    fn com_velocity_operand(&self) -> Result<[f32; 4], AnimationInputError> {
        need(self.prior.centre_of_mass_velocity(), "COM velocity+260")
            .map(|value| operand(value.value))
    }
    fn up_operand(&self) -> Result<[f32; 4], AnimationInputError> {
        need(self.prior.reckoning_up(), "up+220").map(|value| operand(value.value))
    }
    fn category(&self) -> Result<i32, AnimationInputError> {
        need(self.physics.category, "category+8ec")
    }
    fn state(&self) -> Result<i32, AnimationInputError> {
        need(self.physics.current, "state+8e4")
    }
    fn filtered_state(&self) -> Result<i32, AnimationInputError> {
        need(
            self.prior.body_fields().filtered_physics_state,
            "filtered state+8f4",
        )
        .map(|value| value as i32)
    }
    fn flags(&self, offset: u32, mask: u32) -> Result<u32, AnimationInputError> {
        let word = match offset {
            0x8c4 => {
                let (value, known) = self.flags8c4.known_word();
                if known & mask != mask {
                    return Err(AnimationInputError::UnknownFlags { offset, mask });
                }
                value
            }
            0x8c8 => need(*self.flags8c8, "flags8c8")?,
            0x8cc => need(self.physics.flags_8cc, "flags8cc")?,
            0x8d0 => need(self.physics.flags_8d0, "flags8d0")?,
            _ => return Err(AnimationInputError::UnknownFlags { offset, mask }),
        };
        Ok(word & mask)
    }
    fn balance(&self) -> Result<f32, AnimationInputError> {
        need(self.physics.balance, "balance+9a0")
    }
    fn com_velocity_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        need(self.prior.centre_of_mass_velocity(), "COM velocity+260").map(|value| value.value.xyz)
    }
    fn up_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        need(self.prior.reckoning_up(), "up+220").map(|value| value.value.xyz)
    }
    fn deck_y(&self) -> Result<f32, AnimationInputError> {
        need(*self.deck, "deck+40").map(|value| value.columns()[3].xyz[1])
    }
    fn deck_matrix(&self) -> Result<crate::animation::AnimationMatrix, AnimationInputError> {
        // LandingOnBoard composes WorldToAnim OUTER with deck INNER. Inner W
        // is never read as a coefficient; desired W is produced by the outer.
        need(*self.deck, "deck+40").map(|value| value.xyz_scratch())
    }
    fn landing_adjust_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        need(
            self.physics.landing_on_deck_adjust,
            "landing adjustment+360",
        )
        .map(|value| value.xyz)
    }
    fn time_to_land(&self) -> Result<f32, AnimationInputError> {
        need(
            self.prior.body_fields().offboard_time_to_land,
            "time to land+a0c",
        )
    }
    fn grind_air_processed(&self) -> Result<GrindAirProcessed, AnimationInputError> {
        let deck = need(*self.deck, "deck+40")?;
        Ok(GrindAirProcessed {
            deck: deck.columns().map(operand),
            velocity: operand(need(self.prior.velocity(), "velocity+190")?.value),
            up: operand(need(self.prior.reckoning_up(), "up+220")?.value),
            angular_velocity: operand(
                need(self.prior.angular_velocity(), "deck angular velocity+2d0")?.value,
            ),
            time_delta: need(self.physics.time_delta, "time delta+93c")?,
        })
    }
}

fn operand(value: SkeletonRootVector) -> [f32; 4] {
    // Preserve every known lane; only an unknown, unused dot3 lane gets scratch
    // zero. This operand is never stored into the retained source vector.
    [
        value.xyz[0],
        value.xyz[1],
        value.xyz[2],
        value.w.unwrap_or(0.0),
    ]
}

impl SkeletonProcessedContact {
    pub(crate) fn with_process_data_read<R>(
        &mut self,
        callback: impl FnOnce(&ProcessDataRead<'_>, &mut Option<f32>) -> R,
    ) -> R {
        let input = ProcessDataRead {
            physics: &self.physics_state,
            prior: &self.prior_output,
            flags8c8: &self.flags_8c8,
            flags8c4: &self.jump_events,
            deck: &self.deck,
        };
        callback(&input, &mut self.process_data_output.deck_zy_delta)
    }
    pub(crate) fn process_data_output(&self) -> &ProcessDataOutput {
        &self.process_data_output
    }
    pub(crate) fn write_process_data_hook(&mut self, value: SkeletonRootVector) {
        self.process_data_output.hook_to_hips = Some(value);
    }
    /// f2d4..f2d8: `mComToDeck`+2f0 = Skeleton `GetCOMToDeckWorld`82b3b468 (+2a30).
    pub(crate) fn write_process_data_com_to_deck(&mut self, value: SkeletonRootVector) {
        self.process_data_output.com_to_deck = Some(value);
    }
    /// f2dc..f2e0: `mComToDeckDelta`+300 = `GetCOMToDeckWorldDelta`82b3b478 (+2a20).
    pub(crate) fn write_process_data_com_to_deck_delta(&mut self, value: SkeletonRootVector) {
        self.process_data_output.com_to_deck_delta = Some(value);
    }
}
pub(crate) struct PhysicalGrind<'a>(
    pub(crate) &'a crate::physical_player_assembly::PhysicalPlayerAssemblyOwner,
);
impl GrindAdjustRead for PhysicalGrind<'_> {
    fn update<I: AnimationProcessedRead>(
        &mut self,
        input: &I,
    ) -> Result<GrindAirAdjustOut, AnimationInputError> {
        let mut holder = self
            .0
            .grind_air()
            .lock()
            .map_err(|_| AnimationInputError::Missing("unpoisoned GrindAirAdjust owner"))?;
        let owner = holder
            .as_mut()
            .ok_or(AnimationInputError::Missing("GrindAirAdjust owner"))?;
        let services = GrindAirServices {
            cvars: &self.0.inputs().cvars,
            vault: &self.0.inputs().vault,
        };
        let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
        let mut sin_cos = crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
        owner
            .try_update_grind_adjust(
                &mut |offset, mask| {
                    input
                        .flag(offset, mask)
                        .map_err(|e| GrindAirUpdateError::Missing(input_field(&e)))
                },
                &mut || {
                    input
                        .grind_air_processed()
                        .map_err(|e| GrindAirUpdateError::Missing(input_field(&e)))
                },
                &services,
                &mut GrindNum {
                    n: &mut numeric,
                    s: &mut sin_cos,
                },
            )
            .map_err(|e| AnimationInputError::GrindAirAdjust(e.to_string()))
    }
}
fn input_field(error: &AnimationInputError) -> &'static str {
    match error {
        AnimationInputError::Missing(field) => field,
        _ => "processed grind-air input",
    }
}

#[cfg(test)]
mod tests {
    use super::{ProcessDataOutputError, SkeletonProcessedContact};
    #[test]
    fn unordered_orientation_and_byte_low_bit_preserve_neighbors() {
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        processed.flags_8c8 = Some(0xa5a5_ffff);
        processed
            .jump_events
            .process_animation_bits(u32::MAX, 0xfeed_babe);
        for (bits, expected) in [
            (0, 0x800),
            (0x8000_0000, 0x800),
            (0x7fc0_0042, 0x800),
            (0x3f80_0000, 0x1000),
            (0xbf80_0000, 0x800),
        ] {
            let old = processed.known_flags_8c4().0;
            processed.write_process_data_deck_orientation(f32::from_bits(bits));
            assert_eq!(processed.known_flags_8c4().0, (old & !0x1800) | expected);
        }
        let old = processed.capture_process_data_collision_flags().unwrap();
        processed.write_process_data_collision(old, 2);
        assert_eq!(processed.flags_8c8, Some(old & !0x40000));
        processed.write_process_data_collision(old, 3);
        assert_eq!(processed.flags_8c8, Some(old | 0x40000));
    }
    #[test]
    fn generic_unknown_outputs_and_reset_are_distinct() {
        let mut processed = SkeletonProcessedContact::new();
        assert_eq!(
            processed.capture_process_data_collision_flags(),
            Err(ProcessDataOutputError::UnknownCollisionFlags)
        );
        assert!(processed.process_data_output().hook_to_hips.is_none());
        processed.reset();
        assert_eq!(
            processed
                .process_data_output()
                .deck_zy_delta
                .unwrap()
                .to_bits(),
            0
        );
        for value in [
            processed.process_data_output().com_to_deck.unwrap(),
            processed.process_data_output().com_to_deck_delta.unwrap(),
        ] {
            assert_eq!(value.xyz.map(f32::to_bits), [0; 3]);
            assert_eq!(value.w.map(f32::to_bits), Some(0));
        }
    }
    #[test]
    fn com_to_deck_slots_hold_the_written_record_vectors() {
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        let com_to_deck = super::SkeletonRootVector::known([0.25, 0.75, -0.5, 0.0]);
        let delta = super::SkeletonRootVector::known([0.0, 0.01, 0.0, 0.0]);
        processed.write_process_data_com_to_deck(com_to_deck);
        processed.write_process_data_com_to_deck_delta(delta);
        let output = processed.process_data_output();
        assert_eq!(output.com_to_deck, Some(com_to_deck));
        assert_eq!(output.com_to_deck_delta, Some(delta));
    }
}
