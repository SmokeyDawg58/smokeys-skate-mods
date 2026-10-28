//! Four retained landing modes from `Skeleton::UpdateLandingAdjust`82b401d8.
//! Inputs borrow the actual previous records; the animation record is updated
//! later by the caller. Source scalar rounding and partial commits are retained.
//! Host FMA/dot services do not establish Xenon exceptional-value or FP status.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained landing getters support lifecycle integration"
    )
)]

use crate::skeleton_current_animation::continuation::{
    AnimationInputError, AnimationProcessedRead,
};
use crate::{
    animation::AnimationMatrix,
    skeleton_animation_record::SkeletonAnimationRecord,
    skeleton_physics_history::{SkeletonPhysicsHistoryNumeric, SkeletonPhysicsRecord},
    skeleton_skateboard_offset::SkeletonSkateboardOffset,
};

const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonLandingAdjustKind {
    Ground,
    Grind,
    Manual,
    Coffin,
}

/// Each request is sampled at its source getter/load, never prefetched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonLandingAdjustScalar {
    MaximumVelocity,
    CoffinDuration,
    CoffinVelocityDenominator,
    CoffinBase,
    DesiredCom,
    MinimumYPosition,
    GroundMinimumElapsed,
    CoffinFrames,
    GrindTargetSpeed,
    GrindK1,
    GrindK2,
    GrindTargetAfterTime,
    GroundK1,
    GroundK2,
    ManualK1,
    ManualK2,
}

pub(crate) trait SkeletonLandingAdjustAttributes {
    type Error;
    fn scalar(&mut self, attribute: SkeletonLandingAdjustScalar) -> Result<f32, Self::Error>;
    /// Cached global instance layout+140, X+150/Y+160; no attribute lookup.
    fn manual_blend(&mut self, time: f32) -> Result<f32, Self::Error>;
    /// Global graph4 getter804ce67ea03946fa, followed by Evaluate4(time).
    fn grind_blend(&mut self, time: f32) -> Result<f32, Self::Error>;
    /// Global graph4 getter25e6e31bc2841e1f and Evaluate4(time/duration).
    fn coffin_bounce(&mut self, normalized_time: f32) -> Result<f32, Self::Error>;
}

pub(crate) trait SkeletonLandingAdjustCvars {
    type Error;
    fn debug_landing_adjust(&mut self) -> Result<u8, Self::Error>;
    /// The ten debug draws are BLR stubs. Only source singleton acquisitions
    /// require a collaborator; the initial three draws share one acquisition.
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error>;
    fn use_physics_com(&mut self) -> Result<u8, Self::Error>;
    fn first_frame_hack(&mut self) -> Result<f32, Self::Error>;
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error>;
}

/// Snapshot of the source `ProcessedPhysIn`, not Skeleton+3d20 COM velocity.
pub(crate) struct SkeletonLandingAdjustInput {
    pub(crate) filtered_state: i32,
    pub(crate) flags_8c4: u32,
    pub(crate) flags_8c8: u32,
    pub(crate) flags_8cc: u32,
    pub(crate) balance: f32,
    pub(crate) centre_of_mass_velocity: [f32; 4],
    pub(crate) up: [f32; 4],
    pub(crate) processed_deck_y: f32,
}

// Explicit controlled-snapshot contract retained for numerical helper callers.
impl AnimationProcessedRead for SkeletonLandingAdjustInput {
    fn com_velocity_operand(&self) -> Result<[f32; 4], AnimationInputError> {
        Ok(self.centre_of_mass_velocity)
    }
    fn up_operand(&self) -> Result<[f32; 4], AnimationInputError> {
        Ok(self.up)
    }
    fn filtered_state(&self) -> Result<i32, AnimationInputError> {
        Ok(self.filtered_state)
    }
    fn flags(&self, offset: u32, mask: u32) -> Result<u32, AnimationInputError> {
        let word = match offset {
            0x8c4 => self.flags_8c4,
            0x8c8 => self.flags_8c8,
            0x8cc => self.flags_8cc,
            _ => return Err(AnimationInputError::UnknownFlags { offset, mask }),
        };
        Ok(word & mask)
    }
    fn balance(&self) -> Result<f32, AnimationInputError> {
        Ok(self.balance)
    }
    fn com_velocity_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        Ok(self.centre_of_mass_velocity[..3].try_into().expect("XYZ"))
    }
    fn up_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        Ok(self.up[..3].try_into().expect("XYZ"))
    }
    fn deck_y(&self) -> Result<f32, AnimationInputError> {
        Ok(self.processed_deck_y)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonLandingAdjustError<A, C> {
    Input(AnimationInputError),
    Attribute(A),
    Cvar(C),
    DebugOverlay(C),
    ZeroCoffinDuration,
}
impl<A: std::fmt::Display, C: std::fmt::Display> std::fmt::Display
    for SkeletonLandingAdjustError<A, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(e) => e.fmt(f),
            Self::Attribute(e) => write!(f, "Skeleton landing attribute: {e}"),
            Self::Cvar(e) => write!(f, "Skeleton landing cvar: {e}"),
            Self::DebugOverlay(e) => write!(f, "Skeleton landing debug singleton: {e}"),
            Self::ZeroCoffinDuration => f.write_str("Skeleton landing coffin duration equals zero"),
        }
    }
}
impl<A: std::error::Error + 'static, C: std::error::Error + 'static> std::error::Error
    for SkeletonLandingAdjustError<A, C>
{
}

/// Non-Clone projection of Skeleton+3e48, initialized at82b574b4..574d4.
/// Only the constructor and this producer write these nine fields. Native
/// `TeleportReset82b54228` preserves them, so no reset method is introduced.
pub(crate) struct SkeletonLandingAdjust {
    adjusting: bool,
    kind: SkeletonLandingAdjustKind,
    time: f32,
    y_position: f32,
    y_velocity: f32,
    second_last_com_velocity: f32,
    last_animation_position: f32,
    last_filtered_state: i32,
    desired_com_for_grind: f32,
}

impl SkeletonLandingAdjust {
    pub(crate) const fn new() -> Self {
        Self {
            adjusting: false,
            kind: SkeletonLandingAdjustKind::Ground,
            time: 0.0,
            y_position: 0.0,
            y_velocity: 0.0,
            second_last_com_velocity: 0.0,
            last_animation_position: f32::from_bits(0x7f7f_ffff),
            last_filtered_state: 0,
            desired_com_for_grind: 0.0,
        }
    }

    pub(crate) fn adjusting(&self) -> bool {
        self.adjusting
    }
    pub(crate) fn kind(&self) -> SkeletonLandingAdjustKind {
        self.kind
    }
    pub(crate) fn time(&self) -> f32 {
        self.time
    }
    pub(crate) fn y_position(&self) -> f32 {
        self.y_position
    }
    pub(crate) fn y_velocity(&self) -> f32 {
        self.y_velocity
    }
    pub(crate) fn second_last_com_velocity(&self) -> f32 {
        self.second_last_com_velocity
    }
    pub(crate) fn last_animation_position(&self) -> f32 {
        self.last_animation_position
    }
    pub(crate) fn last_filtered_state(&self) -> i32 {
        self.last_filtered_state
    }
    pub(crate) fn desired_com_for_grind(&self) -> f32 {
        self.desired_com_for_grind
    }

    #[cfg(test)]
    pub(crate) fn seed_history(&mut self, adjusting: bool, state: i32, velocity: f32) {
        self.adjusting = adjusting;
        self.last_filtered_state = state;
        self.second_last_com_velocity = velocity;
    }

    /// Complete four-mode producer82b401d8..82b41100. Checked services stop
    /// exactly at their acquisition, preserving already committed owner fields.
    #[expect(
        clippy::too_many_arguments,
        reason = "source owner borrows and services remain distinct"
    )]
    pub(crate) fn try_update<A, C, N>(
        &mut self,
        input: &SkeletonLandingAdjustInput,
        physics: &SkeletonPhysicsRecord,
        previous_animation: &SkeletonAnimationRecord,
        mapped: &AnimationMatrix,
        offset: &mut SkeletonSkateboardOffset,
        attributes: &mut A,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<(), SkeletonLandingAdjustError<A::Error, C::Error>>
    where
        A: SkeletonLandingAdjustAttributes,
        C: SkeletonLandingAdjustCvars,
        N: SkeletonPhysicsHistoryNumeric,
    {
        self.try_update_read(
            input,
            physics,
            previous_animation,
            mapped,
            offset,
            attributes,
            cvars,
            numeric,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "four modes share ordered retained stores and late source services"
    )]
    pub(crate) fn try_update_read<A, C, N, I: AnimationProcessedRead>(
        &mut self,
        input: &I,
        physics: &SkeletonPhysicsRecord,
        previous_animation: &SkeletonAnimationRecord,
        mapped: &AnimationMatrix,
        offset: &mut SkeletonSkateboardOffset,
        attributes: &mut A,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<(), SkeletonLandingAdjustError<A::Error, C::Error>>
    where
        A: SkeletonLandingAdjustAttributes,
        C: SkeletonLandingAdjustCvars,
        N: SkeletonPhysicsHistoryNumeric,
    {
        use SkeletonLandingAdjustError as E;
        use SkeletonLandingAdjustKind as K;
        use SkeletonLandingAdjustScalar as S;
        let max_velocity = attributes
            .scalar(S::MaximumVelocity)
            .map_err(E::Attribute)?;
        let offboard_ground =
            self.last_filtered_state == 6 && input.filtered_state().map_err(E::Input)? == 1;
        let air_ground =
            self.last_filtered_state == 2 && input.filtered_state().map_err(E::Input)? == 1;
        let ground =
            (offboard_ground || air_ground) && input.flag(0x8cc, 0x4000_0000).map_err(E::Input)?;
        let balance = input.balance().map_err(E::Input)?; //402f4 unconditional
        let manual = air_ground && balance != 0.0;
        let grind = self.last_filtered_state == 2
            && input.filtered_state().map_err(E::Input)? == 3
            && !input.flag(0x8c4, 0x20).map_err(E::Input)?;
        let flags8c8 = input.flags(0x8c8, 5).map_err(E::Input)?; //40350 captured once
        let coffin_start = flags8c8 & 4 != 0 && (grind || air_ground);
        let disable = flags8c8 & 1 != 0 && flags8c8 & 4 == 0;
        if disable {
            self.adjusting = false;
        }
        let up = input.up_operand().map_err(E::Input)?; //403c0; reused below
        let com_velocity = input.com_velocity_operand().map_err(E::Input)?; //403c8
        let velocity = numeric.dot3(com_velocity, up);
        debug(cvars, 1)?;

        let mut first_ground_frame = false;
        if !disable && !self.adjusting && (ground || coffin_start || manual || grind) {
            self.time = 0.0;
            self.adjusting = true;
            self.kind = if coffin_start {
                K::Coffin
            } else if manual {
                K::Manual
            } else if ground {
                K::Ground
            } else {
                K::Grind
            };
            let start_velocity = if grind || coffin_start {
                self.second_last_com_velocity
            } else {
                velocity
            };
            let lower = -max_velocity;
            let bounded = select(lower - start_velocity, lower, start_velocity);
            self.y_velocity = select(-bounded, bounded, 0.0);
            let centre = physics.centre_of_mass();
            let position = physics.positions()[0];
            let difference = std::array::from_fn(|lane| centre[lane] - position[lane]);
            let start_position = numeric.dot3(difference, up);
            self.last_animation_position = f32::from_bits(0x7f7f_ffff);
            self.y_position = start_position;
            first_ground_frame = self.kind == K::Ground;
            debug(cvars, 2)?;
        }

        if self.adjusting {
            let use_physics = cvars.use_physics_com().map_err(E::Cvar)?;
            let animation_position = if use_physics != 0 && offboard_ground {
                input.com_velocity_xyz().map_err(E::Input)?[1].mul_add(
                    UPDATE_TIME,
                    physics.centre_of_mass()[1] - input.deck_y().map_err(E::Input)?,
                )
            } else {
                previous_animation.centre_of_mass()[1] - mapped[13]
            };
            let duration = attributes.scalar(S::CoffinDuration).map_err(E::Attribute)?;
            if duration == 0.0 {
                return Err(E::ZeroCoffinDuration);
            }

            let (k1, k2, blend, mut stop) = match self.kind {
                K::Ground => {
                    let k1 = attributes.scalar(S::GroundK1).map_err(E::Attribute)?;
                    let k2 = attributes.scalar(S::GroundK2).map_err(E::Attribute)?;
                    (
                        k1,
                        k2,
                        1.0,
                        input.filtered_state().map_err(E::Input)? != 1
                            || !input.flag(0x8cc, 0x4000_0000).map_err(E::Input)?,
                    )
                }
                K::Grind => {
                    let k1 = attributes.scalar(S::GrindK1).map_err(E::Attribute)?;
                    let k2 = attributes.scalar(S::GrindK2).map_err(E::Attribute)?;
                    let blend = attributes.grind_blend(self.time).map_err(E::Attribute)?;
                    (
                        k1,
                        k2,
                        blend,
                        input.filtered_state().map_err(E::Input)? != 3,
                    )
                }
                K::Manual => {
                    let k1 = attributes.scalar(S::ManualK1).map_err(E::Attribute)?;
                    let k2 = attributes.scalar(S::ManualK2).map_err(E::Attribute)?;
                    let blend = attributes.manual_blend(self.time).map_err(E::Attribute)?;
                    (
                        k1,
                        k2,
                        blend,
                        input.filtered_state().map_err(E::Input)? != 1,
                    )
                }
                K::Coffin => (
                    0.0,
                    0.0,
                    1.0,
                    input.filtered_state().map_err(E::Input)? != 1
                        || self.time > duration
                        || !input.flag(0x8c8, 4).map_err(E::Input)?,
                ),
            };

            if self.kind == K::Coffin {
                let denominator = attributes
                    .scalar(S::CoffinVelocityDenominator)
                    .map_err(E::Attribute)?;
                let ratio = self.y_velocity.abs() / denominator;
                let positive = select(-ratio, 0.0, ratio);
                let bounded = select(1.0 - positive, positive, 1.0);
                let bounce = attributes
                    .coffin_bounce(self.time / duration)
                    .map_err(E::Attribute)?;
                let contribution = bounce * bounded;
                let base = attributes.scalar(S::CoffinBase).map_err(E::Attribute)?;
                self.y_position = base + contribution;
            } else {
                let mut desired = attributes.scalar(S::DesiredCom).map_err(E::Attribute)?;
                if self.kind == K::Grind {
                    let after = attributes
                        .scalar(S::GrindTargetAfterTime)
                        .map_err(E::Attribute)?;
                    if self.time > after {
                        let speed = attributes
                            .scalar(S::GrindTargetSpeed)
                            .map_err(E::Attribute)?;
                        let lower = self.desired_com_for_grind - speed;
                        let upper = self.desired_com_for_grind + speed;
                        let bounded = select(lower - animation_position, lower, animation_position);
                        desired = select(upper - bounded, bounded, upper);
                    }
                    self.desired_com_for_grind = desired;
                }
                let displacement = self.y_position - desired;
                let old_velocity = self.y_velocity;
                let negative_elastic = -(displacement * k1);
                // fnmsubs40c4c: negate AFTER the fused product/subtraction.
                let acceleration = -old_velocity.mul_add(k2, -negative_elastic);
                let candidate_velocity = acceleration.mul_add(UPDATE_TIME, old_velocity);
                let lower = -max_velocity;
                let upper = if self.kind == K::Ground {
                    0.0
                } else {
                    max_velocity
                };
                let bounded = select(lower - candidate_velocity, lower, candidate_velocity);
                let new_velocity = select(upper - bounded, bounded, upper);
                self.y_velocity = new_velocity;
                #[expect(
                    clippy::manual_midpoint,
                    reason = "source rounds the sum before multiplying, including overflow"
                )]
                let average = (new_velocity + old_velocity) * 0.5;
                let show = cvars.debug_landing_adjust().map_err(E::Cvar)?;
                let candidate_position = average.mul_add(UPDATE_TIME, displacement) + desired;
                if show != 0 {
                    acquire_debug(cvars, 3)?;
                }
                let minimum = attributes
                    .scalar(S::MinimumYPosition)
                    .map_err(E::Attribute)?;
                self.y_position = select(candidate_position - minimum, candidate_position, minimum);
            }

            let mut correction = animation_position - self.y_position;
            if first_ground_frame {
                let hack = cvars.first_frame_hack().map_err(E::Cvar)?;
                let product = hack * self.y_velocity;
                let lower = f32::from_bits(0xbdcc_cccd);
                let bounded = select(lower - product, lower, product);
                let negative = select(-bounded, bounded, 0.0);
                let adjusted = negative + correction;
                correction = select(adjusted, adjusted, 0.0);
            }
            debug(cvars, 2)?;

            let ground_time = if self.kind == K::Ground {
                let minimum = attributes
                    .scalar(S::GroundMinimumElapsed)
                    .map_err(E::Attribute)?;
                self.time > minimum
            } else {
                false
            };
            if ground_time && self.last_animation_position < animation_position && correction > 0.0
            {
                stop = true;
            }
            if self.kind == K::Grind && input.flag(0x8c4, 0x20).map_err(E::Input)? {
                stop = true;
            }
            if self.time > 1.5 {
                stop = true;
            }
            let prepare_jump = input.flag(0x8c4, 0x20_0000).map_err(E::Input)?;
            // 41030=40980018 is !CR6.LT, including unordered blend.
            #[expect(
                clippy::neg_cmp_op_on_partial_ord,
                reason = "native BO4 tests LT clear, including NaN"
            )]
            let keep = (!stop && !(blend < f32::from_bits(0x3d4c_cccd))) || prepare_jump;
            if keep {
                let mut frames = if self.kind == K::Coffin {
                    attributes.scalar(S::CoffinFrames).map_err(E::Attribute)?
                } else {
                    cvars.frames_to_blend_away_offset().map_err(E::Cvar)?
                };
                if prepare_jump {
                    self.adjusting = false;
                    frames = 6.0;
                }
                offset.transform[13] = correction * blend;
                offset.vertical_just_set = true;
                offset.vertical_count = frames;
            } else {
                self.adjusting = false;
            }
            self.time += UPDATE_TIME;
            self.last_animation_position = animation_position;
        }
        self.second_last_com_velocity = velocity;
        self.last_filtered_state = input.filtered_state().map_err(E::Input)?;
        Ok(())
    }
}

/// PPC fsel: both signed zeros select the nonnegative operand; NaN does not.
fn select(condition: f32, nonnegative: f32, negative_or_nan: f32) -> f32 {
    if condition >= 0.0 {
        nonnegative
    } else {
        negative_or_nan
    }
}

fn debug<A, C: SkeletonLandingAdjustCvars>(
    cvars: &mut C,
    acquisitions: usize,
) -> Result<(), SkeletonLandingAdjustError<A, C::Error>> {
    if cvars
        .debug_landing_adjust()
        .map_err(SkeletonLandingAdjustError::Cvar)?
        != 0
    {
        acquire_debug(cvars, acquisitions)?;
    }
    Ok(())
}

fn acquire_debug<A, C: SkeletonLandingAdjustCvars>(
    cvars: &mut C,
    acquisitions: usize,
) -> Result<(), SkeletonLandingAdjustError<A, C::Error>> {
    for _ in 0..acquisitions {
        cvars
            .require_debug_overlay()
            .map_err(SkeletonLandingAdjustError::DebugOverlay)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "skeleton_landing_adjust_tests.rs"]
mod tests;
