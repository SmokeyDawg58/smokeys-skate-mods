//! Physics.Update82e2f970 through its first three calls, ending at82e2f998.
//! This is not `UpdateAll` or a replacement for actual Update5 output production.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GROUND, PHYSICS_STATE_BONELESS,
    PHYSICS_STATE_FOLLOW_PATH, PHYSICS_STATE_FOOTPLANT, PHYSICS_STATE_GRIND_BACKSLASH,
    PHYSICS_STATE_GRIND_BOARDSLIDE, PHYSICS_STATE_HANDPLANT, PHYSICS_STATE_KNOWN_AIR,
    PHYSICS_STATE_LANDINGONDECK, PHYSICS_STATE_NONSPECIFIC, PHYSICS_STATE_OFFBOARD,
    PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_OFFBOARD_JUMP, PHYSICS_STATE_OFFBOARD_LOCOMOTION,
    PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_STATE_PHYSICS_AIR, PHYSICS_STATE_PHYSICS_GROUND,
    PHYSICS_STATE_SLIDE_GROUND, PHYSICS_STATE_TELEPORTING, PHYSICS_STATE_WIPEOUT_GROUND,
};
use super::{
    Conditioners, InputCvarError, InputCvarReferences, PhysicalPlayerPhysOut, SkeletonRootVector,
    ZERO, simple::Physics,
};
use crate::{
    physical_player_phys_out::{
        PhysOutAccessError, PhysOutSystemVector,
        construct::{
            conditioned_output::ConditionedStore as Store,
            prior_input::{PriorContactFlag, PriorInputReadError},
        },
    },
    physics::{FilteredPhysicsState as State, GrindType},
    play_animation::EMPTY_FAST_STRING_30,
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum PhysicsConditionerUpdateError<E> {
    MissingPhysics,
    ForeignOutput,
    Read(PriorInputReadError),
    Access(PhysOutAccessError),
    Numeric(E),
    Cvar(InputCvarError),
    InvalidPhysicsState(i32),
    InvalidGrindHistory,
}
impl<E: std::fmt::Debug> std::fmt::Display for PhysicsConditionerUpdateError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPhysics => f.write_str("Physics conditioner is not published"),
            Self::ForeignOutput => f.write_str("Physics conditioner belongs to another PhysOut"),
            Self::Read(e) => write!(f, "Physics conditioner component: {e:?}"),
            Self::Access(e) => write!(f, "Physics conditioner output access: {e:?}"),
            Self::Numeric(e) => write!(f, "Physics conditioner numeric collaborator: {e:?}"),
            Self::Cvar(e) => write!(f, "Physics conditioner cvar: {e:?}"),
            Self::InvalidPhysicsState(s) => {
                write!(f, "native Physics state assertion82e2e390: {s}")
            }
            Self::InvalidGrindHistory => {
                f.write_str("native filtered Grind requires non-NONE grind history")
            }
        }
    }
}
/// Successful selected prefix only; the next native call is `UpdateDampedComPos`.
pub(crate) struct PhysicsBeforeDampedCom {
    output: Arc<()>,
}
impl PhysicsBeforeDampedCom {
    pub(crate) fn matches_output(&self, output: &PhysicalPlayerPhysOut) -> bool {
        Arc::ptr_eq(&self.output, output.occurrence())
    }
}
impl Conditioners {
    /// The bound Physics occurrence and all histories are retained across calls.
    pub(crate) fn try_update_physics_prefix<N: ReckoningNumeric>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<PhysicsBeforeDampedCom, PhysicsConditionerUpdateError<N::Error>> {
        use PhysicsConditionerUpdateError as E;
        if !self.matches_output(output) {
            return Err(E::ForeignOutput);
        }
        let physics = self
            .physics
            .published_value_mut()
            .ok_or(E::MissingPhysics)?;
        if !Arc::ptr_eq(&physics.output, output.occurrence()) {
            return Err(E::ForeignOutput);
        }
        physics.payload.update_system_velocity(output)?;
        physics
            .payload
            .update_empirical_measurements(output, numeric)?;
        physics.payload.update_filtered_state(output, cvars)?;
        Ok(PhysicsBeforeDampedCom {
            output: Arc::clone(output.occurrence()),
        })
    }
}
impl Physics {
    ///82e25a40: raw current*.25 rounds, then old*.75 + rounded current is fused.
    fn update_system_velocity<E>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
    ) -> Result<(), PhysicsConditionerUpdateError<E>> {
        use PhysicsConditionerUpdateError as E;
        let current = output
            .system_vector(PhysOutSystemVector::CentreOfMassVelocity)
            .map_err(E::Access)?
            .value;
        self.damped_system_velocity = zip(self.damped_system_velocity, current, |old, value| {
            old.mul_add(0.75, value * 0.25)
        });
        output
            .try_store_conditioned(Store::DampedSystemVelocity(self.damped_system_velocity))
            .map_err(E::Read)
    }

    ///82e25830: two distinct estimate calls when frames>1, no time clamp.
    fn update_empirical_measurements<N: ReckoningNumeric>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        numeric: &mut N,
    ) -> Result<(), PhysicsConditionerUpdateError<N::Error>> {
        use PhysicsConditionerUpdateError as E;
        let position = output
            .try_guarded_skateboard_reckoning()
            .map_err(E::Read)?
            .deck_position;
        let dt = output
            .try_guarded_physics_output()
            .map_err(E::Read)?
            .timestep;
        let frames = self.frames_since_reset;
        let mut velocity = ZERO;
        let mut acceleration = ZERO;
        if frames > 0 {
            let delta = zip(position, self.deck_position, |a, b| a - b);
            let seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
            let inverse = refine_reciprocal(dt, seed);
            velocity = map(delta, |lane| inverse * lane);
            if frames > 1 {
                let delta_velocity = zip(velocity, self.velocity, |a, b| a - b);
                let seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
                let inverse = refine_reciprocal(dt, seed);
                acceleration = map(delta_velocity, |lane| inverse * lane);
            }
        }
        self.frames_since_reset = frames.wrapping_add(1); //2597c
        self.velocity = velocity; //25980
        self.last_deck_position = self.deck_position; //2598c
        self.deck_position = position; //25994
        output
            .try_store_conditioned(Store::EmpiricalVelocity(velocity))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::EmpiricalAcceleration(acceleration))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::FramesSinceReset(self.frames_since_reset))
            .map_err(E::Read)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "ordered native counter updates, lazy state branches and per-field publications"
    )]
    fn update_filtered_state<E>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
    ) -> Result<(), PhysicsConditionerUpdateError<E>> {
        use PhysicsConditionerUpdateError as E;
        self.last_filtered_state = self.filtered_state; //d820 before first output guard
        let current = output
            .try_guarded_state_output()
            .map_err(E::Read)?
            .physics_state;
        let category = output
            .try_guarded_state_output()
            .map_err(E::Read)?
            .physics_state_category;
        let contact = output
            .try_guarded_collision_contact(PriorContactFlag::Anything)
            .map_err(E::Read)?;
        let wall_exit = output
            .try_guarded_ground_output()
            .map_err(E::Read)?
            .wall_ride_exit;
        let stairs = output
            .try_guarded_state_output()
            .map_err(E::Read)?
            .physics_state_category
            == PHYSICS_CATEGORY_GROUND
            && output.collision_physics_type().map_err(E::Access)?.value == 8;
        self.must_change_filtered_state =
            matches!(self.filtered_state, State::Wipeout | State::Invalid)
                || (self.last_physics_state == PHYSICS_STATE_TELEPORTING && current != PHYSICS_STATE_TELEPORTING);
        self.air_count = if category == PHYSICS_CATEGORY_AIR {
            self.air_count.wrapping_add(1)
        } else {
            0
        };
        self.nonspecific_count = if current == PHYSICS_STATE_NONSPECIFIC {
            self.nonspecific_count.wrapping_add(1)
        } else {
            0
        };
        self.nonspecific_collision_free_count = if current == PHYSICS_STATE_NONSPECIFIC
            && !output
                .try_guarded_collision_contact(PriorContactFlag::Anything)
                .map_err(E::Read)?
        {
            self.nonspecific_collision_free_count.wrapping_add(1)
        } else {
            0
        };
        self.nonspecific_collision_count = if current == PHYSICS_STATE_NONSPECIFIC
            && output
                .try_guarded_collision_contact(PriorContactFlag::Anything)
                .map_err(E::Read)?
        {
            self.nonspecific_collision_count.wrapping_add(1)
        } else {
            0
        };
        self.frames_since_ground_stairs = if stairs {
            0
        } else {
            self.frames_since_ground_stairs.wrapping_add(1)
        };
        self.filtered_state = match current {
            PHYSICS_STATE_PHYSICS_GROUND => {
                if wall_exit {
                    State::Air
                } else {
                    State::Ground
                }
            }
            PHYSICS_STATE_SLIDE_GROUND..=PHYSICS_STATE_FOLLOW_PATH | PHYSICS_STATE_BONELESS => State::Ground,
            PHYSICS_STATE_PHYSICS_AIR => {
                let mut change = self.air_count > 3;
                if self.filtered_state == State::Ground {
                    change = change
                        && self.frames_since_ground_stairs
                            > cvars.try_ground_stairs_frames().map_err(E::Cvar)?;
                }
                if change || self.must_change_filtered_state {
                    State::Air
                } else {
                    self.filtered_state
                }
            }
            PHYSICS_STATE_KNOWN_AIR => {
                if contact
                    && !output
                        .try_guarded_air_output()
                        .map_err(E::Read)?
                        .is_targeting_grind
                {
                    State::Ground
                } else {
                    State::Air
                }
            }
            PHYSICS_STATE_WIPEOUT_GROUND => State::Wipeout,
            PHYSICS_STATE_GRIND_BOARDSLIDE..=PHYSICS_STATE_GRIND_BACKSLASH => {
                // 82e2dee4..dff0: the grind history copied from PhysOut_Grinds.
                let grinds = output.try_guarded_grinds_output().map_err(E::Read)?;
                self.grind_scorable_id = grinds.grind_scorable_id;
                self.skating_grind_name = grinds.skating_grind_name;
                self.scoring_grind_name = grinds.skating_grind_name_scoring;
                self.grind_type = grinds.grind_type;
                self.is_grind_on_front = grinds.is_grind_on_front;
                self.grind_crouch = grinds.grind_crouch;
                self.grind_pathed_guid = grinds.grind_pathed_guid;
                self.grind_local_guid = grinds.grind_local_guid;
                State::Grind
            }
            PHYSICS_STATE_OFFBOARD | PHYSICS_STATE_OFFBOARD_PUSHING | PHYSICS_STATE_OFFBOARD_LOCOMOTION => State::OffBoard,
            PHYSICS_STATE_OFFBOARD_AIR | PHYSICS_STATE_OFFBOARD_JUMP => {
                if output
                    .try_guarded_offboard_output()
                    .map_err(E::Read)?
                    .has_landed
                {
                    State::OffBoard
                } else {
                    State::OffBoardAir
                }
            }
            PHYSICS_STATE_LANDINGONDECK => {
                if output
                    .try_guarded_offboard_output()
                    .map_err(E::Read)?
                    .on_deck
                {
                    State::Ground
                } else {
                    State::OffBoard
                }
            }
            PHYSICS_STATE_HANDPLANT | PHYSICS_STATE_FOOTPLANT => State::Air,
            PHYSICS_STATE_NONSPECIFIC => self.filtered_state,
            PHYSICS_STATE_TELEPORTING => State::Teleport,
            _ => return Err(E::InvalidPhysicsState(current)),
        };
        self.last_physics_state = current;
        output
            .try_store_conditioned(Store::FilteredState(self.filtered_state))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::LastFilteredState(self.last_filtered_state))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::LastGrindDistance(self.last_grind_distance))
            .map_err(E::Read)?;
        let grinding = self.filtered_state == State::Grind;
        if grinding && self.grind_type == GrindType::None {
            return Err(E::InvalidGrindHistory);
        }
        output
            .try_store_conditioned(Store::GrindType(if grinding {
                self.grind_type
            } else {
                GrindType::None
            }))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::IsGrinding(grinding))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::GrindOnFront(grinding && self.is_grind_on_front))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::GrindName(if grinding {
                self.skating_grind_name
            } else {
                EMPTY_FAST_STRING_30
            }))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::GrindScoringName(if grinding {
                self.scoring_grind_name
            } else {
                EMPTY_FAST_STRING_30
            }))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::GrindScore(if grinding {
                self.grind_scorable_id
            } else {
                -1
            }))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::GrindCrouch(if grinding {
                self.grind_crouch
            } else {
                0.0
            }))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::GrindPathedGuid(if grinding {
                self.grind_pathed_guid
            } else {
                u64::MAX
            }))
            .map_err(E::Read)?;
        output
            .try_store_conditioned(Store::GrindLocalGuid(if grinding {
                self.grind_local_guid
            } else {
                u64::MAX
            }))
            .map_err(E::Read)
    }
}

// Exact VNMSUB has a final negation, including when the fused result is +0.
fn residual(value: f32, estimate: f32) -> f32 {
    -estimate.mul_add(value, -1.0)
}
fn refine_reciprocal(value: f32, seed: f32) -> f32 {
    let first = seed.mul_add(residual(value, seed), seed);
    first.mul_add(residual(value, first), first)
}
fn map(v: SkeletonRootVector, f: impl Fn(f32) -> f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: v.xyz.map(&f),
        w: v.w.map(f),
    }
}
fn zip(
    a: SkeletonRootVector,
    b: SkeletonRootVector,
    f: impl Fn(f32, f32) -> f32,
) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| f(a.xyz[i], b.xyz[i])),
        w: a.w.zip(b.w).map(|(a, b)| f(a, b)),
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_conditioner_physics_update_tests.rs"]
mod tests;
