//! Actual pair `Update2_SystemLogic82e01c40`: select first, clear requests last.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GRIND, PHYSICS_STATE_BONELESS, PHYSICS_STATE_FOLLOW_PATH,
    PHYSICS_STATE_FOOTPLANT, PHYSICS_STATE_GRIND_5050, PHYSICS_STATE_GRIND_5_O,
    PHYSICS_STATE_GRIND_BACKSLASH, PHYSICS_STATE_GRIND_BOARDSLIDE, PHYSICS_STATE_GRIND_TIPSLIDE,
    PHYSICS_STATE_GROUND_ANIMATION, PHYSICS_STATE_HANDPLANT, PHYSICS_STATE_KNOWN_AIR,
    PHYSICS_STATE_LANDINGONDECK, PHYSICS_STATE_NONSPECIFIC, PHYSICS_STATE_OFFBOARD,
    PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_OFFBOARD_JUMP, PHYSICS_STATE_OFFBOARD_LOCOMOTION,
    PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_STATE_PHYSICS_AIR, PHYSICS_STATE_PHYSICS_GROUND,
    PHYSICS_STATE_REVERT_GROUND, PHYSICS_STATE_SKITCHING, PHYSICS_STATE_SLEEPING,
    PHYSICS_STATE_SLIDE_GROUND, PHYSICS_STATE_TELEPORTING, PHYSICS_STATE_WIPEOUT_GROUND,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_state_changer::{StateChangerError, StateChangerFields},
    physical_player_wipeout::WipeoutOwnerError,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::Arc;

#[cfg(test)]
#[path = "physical_player_system_logic_corpus.rs"]
pub(crate) mod original_corpus;

/// Fresh linked integer target at82ddbf68..70. A missing registry must fail.
pub(crate) trait StateChangerCvars {
    type Error;
    fn fall_through_world_reset_frames(&mut self) -> Result<i32, Self::Error>;
    fn disable_skitching(&mut self) -> Result<bool, Self::Error>;
    fn minimum_air_time(&mut self) -> Result<f32, Self::Error>;
    fn grind_jump_air_frames(&mut self) -> Result<i32, Self::Error>;
    fn known_air_warning_time(&mut self) -> Result<f32, Self::Error>;
    fn state_debug_warnings(&mut self) -> Result<bool, Self::Error>;
    /// `kNonspecificToGroundNearGrindFrames`, `kNonspecificCollFreeStickyFrames`
    /// and `kNonspecificCollExitFrames` of the Nonspecific701 case.
    fn nonspecific_frames(&mut self, which: NonspecificFrames) -> Result<i32, Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NonspecificFrames {
    ToGroundNearGrind,
    CollFreeSticky,
    CollExit,
}

#[derive(Debug)]
pub(crate) enum SystemLogicError<C> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    Poisoned,
    StateChanger(StateChangerError),
    ForeignOwner,
    Missing(&'static str),
    Cvar(C),
    Overlay(crate::debug_overlay::DebugOverlayError),
    Attribute(crate::attribute_runtime::AttributeRuntimeError),
    SkeletonAnimation(crate::dynamic_assembly_session::skeleton_normal::animation_query::SkeletonAnimationQueryError),
    UnsupportedCurrentState(i32),
    PendingTransition(i32),
    Wipeout(WipeoutOwnerError),
}

impl<C: std::fmt::Debug> std::fmt::Display for SystemLogicError<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("system logic requires skater pool"),
            Self::Pair(e) => write!(f, "system logic pair: {e:?}"),
            Self::Pool(e) => write!(f, "system logic pool: {e}"),
            Self::Poisoned => f.write_str("system logic owner lock poisoned"),
            Self::StateChanger(e) => write!(f, "system logic StateChanger: {e:?}"),
            Self::ForeignOwner => f.write_str("system logic StateChanger belongs to another pair"),
            Self::Missing(field) => write!(f, "system logic source {field} unavailable"),
            Self::Cvar(e) => write!(f, "system logic cvar: {e:?}"),
            Self::Overlay(e) => write!(f, "system logic debug overlay: {e}"),
            Self::Attribute(e) => write!(f, "system logic attribute: {e}"),
            Self::SkeletonAnimation(e) => write!(f, "system logic animation query: {e}"),
            Self::UnsupportedCurrentState(state) => {
                write!(f, "system logic current state {state} pending")
            }
            Self::PendingTransition(state) => {
                write!(f, "system logic transition to {state} pending")
            }
            Self::Wipeout(e) => write!(f, "system logic Wipeout: {e:?}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    /// This consumes SAME retained state, board-body count and Processed owner.
    /// Failed selection/transition retains its source prefix and leaves Wipeout
    /// requests untouched. No input phase or completed skating tick is implied.
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered state selection and retained request publication"
    )]
    pub(crate) fn try_update_pair_system_logic<C: StateChangerCvars>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &mut C,
    ) -> Result<(), SystemLogicError<C::Error>> {
        use SystemLogicError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let current = match owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .current
        {
            CurrentPhysicalState::Unselected => return Err(E::Missing("current physics state")),
            CurrentPhysicalState::Sleeping => PHYSICS_STATE_SLEEPING,
            CurrentPhysicalState::Ground => PHYSICS_STATE_PHYSICS_GROUND,
            CurrentPhysicalState::SlideGround => PHYSICS_STATE_SLIDE_GROUND,
            CurrentPhysicalState::RevertGround => PHYSICS_STATE_REVERT_GROUND,
            CurrentPhysicalState::GroundAnimation => PHYSICS_STATE_GROUND_ANIMATION,
            CurrentPhysicalState::Skitching => PHYSICS_STATE_SKITCHING,
            CurrentPhysicalState::FollowPath => PHYSICS_STATE_FOLLOW_PATH,
            CurrentPhysicalState::Air => PHYSICS_STATE_PHYSICS_AIR,
            CurrentPhysicalState::KnownAir => PHYSICS_STATE_KNOWN_AIR,
            CurrentPhysicalState::WipeoutGround => PHYSICS_STATE_WIPEOUT_GROUND,
            CurrentPhysicalState::Teleporting => PHYSICS_STATE_TELEPORTING,
            CurrentPhysicalState::OffBoard => PHYSICS_STATE_OFFBOARD,
            CurrentPhysicalState::OffBoardPushing => PHYSICS_STATE_OFFBOARD_PUSHING,
            CurrentPhysicalState::OffBoardAir => PHYSICS_STATE_OFFBOARD_AIR,
            // CheckForStateChange82e005a8 passes GetType: OffBoard_Jump82d98590 is
            // the folded li r3,0x1f6, so the OffBoard_Air case selects for it.
            CurrentPhysicalState::OffBoardJump => PHYSICS_STATE_OFFBOARD_AIR,
            CurrentPhysicalState::LandingOnDeck => PHYSICS_STATE_LANDINGONDECK,
            CurrentPhysicalState::OffBoardLocomotion => PHYSICS_STATE_OFFBOARD_LOCOMOTION,
            CurrentPhysicalState::Boneless => PHYSICS_STATE_BONELESS,
            CurrentPhysicalState::FootPlant => PHYSICS_STATE_FOOTPLANT,
            CurrentPhysicalState::HandPlant => PHYSICS_STATE_HANDPLANT,
            CurrentPhysicalState::Grind(grind) => grind.id(),
            CurrentPhysicalState::Nonspecific => PHYSICS_STATE_NONSPECIFIC,
        };
        let mut changer = owner.state_changer().lock().map_err(|_| E::Poisoned)?;
        if !changer
            .owner
            .as_ref()
            .and_then(std::sync::Weak::upgrade)
            .is_some_and(|bound| Arc::ptr_eq(&bound, &owner))
        {
            return Err(if changer.fields.is_none() {
                E::StateChanger(StateChangerError::NotConstructed)
            } else {
                E::ForeignOwner
            });
        }
        let fields = changer
            .fields
            .as_mut()
            .ok_or(E::StateChanger(StateChangerError::NotConstructed))?;
        fields.begin(current);
        // Native virtual GetSkateboardBody precedes byte+361 and Processed reads.
        self.board(board).map_err(E::Pool)?;
        let wheels = self
            .board_construction(board)
            .map_err(E::Pool)?
            .and_then(|construction| construction.post_fields())
            .ok_or(E::Missing("board Body post fields"))?
            .num_wheels;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let selection = {
            let processed = processed.lock().map_err(|_| E::Poisoned)?;
            select_state(fields, current, wheels, &processed, cvars)?
        };
        let next = match selection {
            Selection::State(next) => next,
            Selection::RevertGround { colliding } => {
                if !colliding && self.state_changer_is_off_ground(board, &changer)? {
                    PHYSICS_STATE_PHYSICS_AIR
                } else {
                    let p = processed.lock().map_err(|_| E::Poisoned)?;
                    let flags = read_flags(&p, 0x50000)?;
                    let second = p.known_flags_8c8().ok_or(E::Missing("revert flags8c8"))?;
                    if flags & 0x10000 != 0 || second & 0x40000 != 0 {
                        PHYSICS_STATE_PHYSICS_GROUND
                    } else if second & 0x400000 == 0 {
                        changer
                            .fields
                            .as_mut()
                            .ok_or(E::Missing("state changer"))?
                            .revert_exited_normally = true;
                        PHYSICS_STATE_PHYSICS_GROUND
                    } else if flags & 0x40000 != 0 {
                        PHYSICS_STATE_WIPEOUT_GROUND
                    } else {
                        PHYSICS_STATE_REVERT_GROUND
                    }
                }
            }
            Selection::SlideGround { colliding } => {
                //82ddc04c..c08c: unlike ordinary Ground, no teleport-countdown
                //gate and no IsSkateboardAnimated query on this path.
                if !colliding && self.state_changer_is_off_ground(board, &changer)? {
                    PHYSICS_STATE_PHYSICS_AIR
                } else if processed
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .physics_state_fields()
                    .slide
                    .ok_or(E::Missing("SlideGround slide"))?
                    == 0.0
                {
                    PHYSICS_STATE_PHYSICS_GROUND
                } else {
                    PHYSICS_STATE_SLIDE_GROUND
                }
            }
            Selection::GroundAnimation { colliding } => {
                if !colliding && self.state_changer_is_off_ground(board, &changer)? {
                    PHYSICS_STATE_PHYSICS_AIR
                } else if self
                    .skeleton(skeleton)
                    .map_err(E::Pool)?
                    .try_is_skateboard_animated()
                    .map_err(E::SkeletonAnimation)?
                {
                    PHYSICS_STATE_GROUND_ANIMATION
                } else {
                    PHYSICS_STATE_PHYSICS_GROUND
                }
            }
            Selection::Skitching => {
                // ConditionIsOffGroundSkitching82dd76f8 uses the GLOBAL
                // skitch collection, with different keys from Update's in-air flag.
                let attrs = changer
                    .attributes
                    .as_ref()
                    .ok_or(E::Missing("state changer attributes"))?;
                let body = self
                    .board_construction(board)
                    .map_err(E::Pool)?
                    .ok_or(E::Missing("board Body"))?;
                let height = body
                    .wheel_lines()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .height
                    .ok_or(E::Missing("wheel height"))?;
                let high = height
                    > attrs
                        .skitch_scalar(
                            crate::physical_player_state_changer::attributes::CLEAR_HEIGHT,
                        )
                        .map_err(E::Attribute)?;
                let low = height
                    > attrs
                        .skitch_scalar(
                            crate::physical_player_state_changer::attributes::RECENT_HEIGHT,
                        )
                        .map_err(E::Attribute)?;
                let time = body
                    .post_fields()
                    .ok_or(E::Missing("board post fields"))?
                    .time_since_wheels_touching;
                let grace = attrs
                    .skitch_scalar(crate::physical_player_state_changer::attributes::CONTACT_GRACE)
                    .map_err(E::Attribute)?;
                if (high || low && time > grace)
                    && changer
                        .fields
                        .as_ref()
                        .ok_or(E::Missing("state changer"))?
                        .teleport_countdown
                        == 0
                {
                    PHYSICS_STATE_PHYSICS_GROUND
                } else {
                    PHYSICS_STATE_SKITCHING
                }
            }
            Selection::HippyJumped => {
                // 82ddc1b0: SingletonHolder<DebugOverlay>::Get asserts the
                // instance; its "Hippy Jumped!" print is the shared BLR825187c0.
                self.debug_overlay.require().map_err(E::Overlay)?;
                PHYSICS_STATE_LANDINGONDECK
            }
            Selection::AirLanding => {
                if self
                    .skeleton(skeleton)
                    .map_err(E::Pool)?
                    .try_is_skateboard_animated()
                    .map_err(E::SkeletonAnimation)?
                {
                    PHYSICS_STATE_GROUND_ANIMATION
                } else {
                    PHYSICS_STATE_PHYSICS_GROUND
                }
            }
            Selection::Ground { colliding } => self
                .finish_ground_selection(board, skeleton, &processed, &changer, colliding, cvars)?,
        };
        drop(changer);
        if matches!(current, PHYSICS_STATE_PHYSICS_AIR | PHYSICS_STATE_KNOWN_AIR) && std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
            let p = processed.lock().map_err(|_| E::Poisoned)?;
            let flags = p.known_flags_8c4().0;
            eprintln!(
                "known air: air frame state={current} frames_since_jump={:?} contact={} ready={} com_velocity={:?} velocity={:?}",
                p.physics_state_fields().frames_since_jump,
                flags & 0x8000 != 0,
                flags & 0x400 != 0,
                p.prior_output()
                    .centre_of_mass_velocity()
                    .map(|v| v.value.xyz),
                p.prior_output().velocity().map(|v| v.value.xyz)
            );
        }
        if next != current && std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
            let p = processed.lock().map_err(|_| E::Poisoned)?;
            eprintln!(
                "known air: update2 {current} -> {next} frames_since_jump={:?} selector_ready={}",
                p.physics_state_fields().frames_since_jump,
                p.known_flags_8c4().0 & 0x400 != 0
            );
        }
        if next != current {
            // CheckForStateChange82e005f8 commits before SetPhysicsState005fc.
            owner
                .physics_state()
                .lock()
                .map_err(|_| E::Poisoned)?
                .frames_since_jump = Some(100);
            return Err(E::PendingTransition(next));
        }
        owner
            .wipeout()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_clear_requests()
            .map_err(E::Wipeout)
    }

    fn finish_ground_selection<C: StateChangerCvars>(
        &self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        processed: &std::sync::Mutex<SkeletonProcessedContact>,
        changer: &crate::physical_player_state_changer::PhysicalPlayerStateChangerOwner,
        colliding: bool,
        cvars: &mut C,
    ) -> Result<i32, SystemLogicError<C::Error>> {
        use SystemLogicError as E;
        let fields = changer
            .fields
            .as_ref()
            .ok_or(E::StateChanger(StateChangerError::NotConstructed))?;
        if !colliding
            && self.state_changer_is_off_ground(board, changer)?
            && fields.teleport_countdown == 0
        {
            return Ok(PHYSICS_STATE_PHYSICS_AIR);
        }
        // Mandatory source query even when teleport countdown prevents transition.
        if self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .try_is_skateboard_animated()
            .map_err(E::SkeletonAnimation)?
            && fields.teleport_countdown == 0
        {
            return Ok(PHYSICS_STATE_GROUND_ANIMATION);
        }
        let processed = processed.lock().map_err(|_| E::Poisoned)?;
        let state = processed.physics_state_fields();
        let flags8cc = state.flags_8cc.ok_or(E::Missing("Processed flags8cc"))?;
        if flags8cc & 0x0080_0000 != 0 {
            // Raw c314 loads the first BE byte at8d0 then tests its low bit.
            let flags8d0 = state.flags_8d0.ok_or(E::Missing("Processed flags8d0"))?;
            if flags8d0 & 0x0100_0000 != 0 && !cvars.disable_skitching().map_err(E::Cvar)? {
                return Ok(PHYSICS_STATE_SKITCHING);
            }
        }
        Ok(
            if state.slide.ok_or(E::Missing("Processed slide"))? == 0.0 {
                PHYSICS_STATE_PHYSICS_GROUND
            } else {
                PHYSICS_STATE_SLIDE_GROUND
            },
        )
    }

    fn state_changer_is_off_ground<C>(
        &self,
        board: &SimulationSkaterAssemblyHandle,
        changer: &crate::physical_player_state_changer::PhysicalPlayerStateChangerOwner,
    ) -> Result<bool, SystemLogicError<C>> {
        use crate::physical_player_state_changer::attributes::{
            CLEAR_HEIGHT, CONTACT_GRACE, RECENT_HEIGHT,
        };
        use SystemLogicError as E;
        self.board(board).map_err(E::Pool)?;
        let height = self
            .board_construction(board)
            .map_err(E::Pool)?
            .ok_or(E::Missing("board Body construction"))?
            .wheel_lines()
            .lock()
            .map_err(|_| E::Poisoned)?
            .height
            .ok_or(E::Missing("Body wheel height"))?;
        let attributes = changer
            .attributes
            .as_ref()
            .ok_or(E::Missing("global air-state attributes"))?;
        let above_clear = height > attributes.scalar(CLEAR_HEIGHT).map_err(E::Attribute)?;
        let above_recent = height > attributes.scalar(RECENT_HEIGHT).map_err(E::Attribute)?;
        // Source requires a second Body capture and third attribute even when
        // above_clear is true. Do not short-circuit the three producer reads.
        self.board(board).map_err(E::Pool)?;
        let time = self
            .board_construction(board)
            .map_err(E::Pool)?
            .and_then(|construction| construction.post_fields())
            .ok_or(E::Missing("board Body post fields"))?
            .time_since_wheels_touching;
        let beyond_grace = time > attributes.scalar(CONTACT_GRACE).map_err(E::Attribute)?;
        Ok(above_clear || above_recent && beyond_grace)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Selection {
    State(i32),
    SlideGround { colliding: bool },
    RevertGround { colliding: bool },
    Ground { colliding: bool },
    AirLanding,
    GroundAnimation { colliding: bool },
    Skitching,
    HippyJumped,
}

fn read_flags<C>(source: &SkeletonProcessedContact, mask: u32) -> Result<u32, SystemLogicError<C>> {
    let (value, known) = source.known_flags_8c4();
    if known & mask == mask {
        Ok(value & mask)
    } else {
        Err(SystemLogicError::Missing("Processed flags8c4"))
    }
}

fn select_state<C: StateChangerCvars>(
    fields: &mut StateChangerFields,
    current: i32,
    wheels: u8,
    source: &SkeletonProcessedContact,
    cvars: &mut C,
) -> Result<Selection, SystemLogicError<C::Error>> {
    use SystemLogicError as E;
    let state = source.physics_state_fields();
    // Raw bdb4/bdb8/bdbc read count, flags8c8, category in this order.
    let processed_wheels = state
        .num_wheels_in_contact
        .ok_or(E::Missing("Processed wheel count"))?;
    let flags8c8 = source
        .known_flags_8c8()
        .ok_or(E::Missing("Processed flags8c8"))?;
    let category = state.category.ok_or(E::Missing("Processed category"))?;
    let colliding = processed_wheels > 0
        || read_flags(source, 0x0001_0000)? != 0
        || read_flags(source, 0x0002_0000)? != 0;
    let grind_jump = category == PHYSICS_CATEGORY_GRIND && read_flags(source, 0x0040_0000)? != 0;
    fields.update_contact_counters(current, category, wheels, colliding, grind_jump);
    if read_flags(source, 4)? != 0 || flags8c8 & 0x0008_0000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_TELEPORTING));
    }
    if fields.air_frames > cvars.fall_through_world_reset_frames().map_err(E::Cvar)? {
        fields.request_teleport = true;
        return Ok(Selection::State(current));
    }
    if state.ai_flags.1 & 0x8000_0000 == 0 {
        return Err(E::Missing("Processed AI path following"));
    }
    if state.ai_flags.0 & 0x8000_0000 != 0 {
        if state.ai_flags.1 & 0x0200_0000 == 0 {
            return Err(E::Missing("Processed AI path ended"));
        }
        if state.ai_flags.0 & 0x0200_0000 == 0 && fields.teleport_countdown == 0 {
            return Ok(Selection::State(PHYSICS_STATE_FOLLOW_PATH));
        }
    }
    match current {
        PHYSICS_STATE_PHYSICS_GROUND => select_ground(source, wheels, colliding),
        PHYSICS_STATE_SLIDE_GROUND => {
            if read_flags(source, 0x40000)? != 0 {
                return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
            }
            if state
                .start_revert
                .ok_or(E::Missing("SlideGround start revert"))?
                != 0.0
            {
                return Ok(Selection::State(PHYSICS_STATE_REVERT_GROUND));
            }
            let grind = check_for_grind(source)?;
            if grind != 0 {
                return Ok(Selection::State(grind));
            }
            Ok(Selection::SlideGround { colliding })
        }
        PHYSICS_STATE_REVERT_GROUND => Ok(Selection::RevertGround { colliding }),
        PHYSICS_STATE_PHYSICS_AIR => select_air(source, fields, colliding, flags8c8, cvars),
        PHYSICS_STATE_KNOWN_AIR => select_known_air(source, fields, colliding, flags8c8, cvars),
        PHYSICS_STATE_GROUND_ANIMATION => select_ground_animation(source, colliding),
        //104 table entry82ddbff4+10 ->82ddc114: release before wipeout.
        PHYSICS_STATE_SKITCHING => {
            if state.flags_8cc.ok_or(E::Missing("Skitch grab flags"))? & 0x0080_0000 == 0 {
                Ok(Selection::State(PHYSICS_STATE_PHYSICS_GROUND))
            } else if read_flags(source, 0x40000)? != 0 {
                Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND))
            } else {
                Ok(Selection::Skitching)
            }
        }
        // CalcSuggestedState82ddc558 table entry99 (300-201) jumps directly
        // to82ddd294, returning current after the common priority/counter prefix.
        PHYSICS_STATE_WIPEOUT_GROUND => Ok(Selection::State(current)),
        PHYSICS_STATE_OFFBOARD => select_offboard(source),
        PHYSICS_STATE_OFFBOARD_PUSHING => select_offboard_pushing(source),
        PHYSICS_STATE_OFFBOARD_AIR => select_offboard_air(source),
        PHYSICS_STATE_LANDINGONDECK => select_landing_on_deck(source),
        PHYSICS_STATE_OFFBOARD_LOCOMOTION => select_offboard_locomotion(source),
        PHYSICS_STATE_BONELESS => select_boneless(source),
        PHYSICS_STATE_FOOTPLANT => select_foot_plant(source),
        // CalcSuggestedState HANDPLANT case: contacts terminate only after0.2s;
        // animation8cc bit0 must remain set throughout the plant.
        PHYSICS_STATE_HANDPLANT => {
            if read_flags(source, 0x40000)? != 0 {
                return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
            }
            let timer = state
                .state_timer
                .ok_or(E::Missing("HandPlant state timer"))?;
            let flags = state
                .flags_8cc
                .ok_or(E::Missing("HandPlant animation flags"))?;
            Ok(Selection::State(
                if (!colliding || timer <= 0.2) && flags & 1 != 0 {
                    PHYSICS_STATE_HANDPLANT
                } else {
                    PHYSICS_STATE_PHYSICS_GROUND
                },
            ))
        }
        PHYSICS_STATE_GRIND_BOARDSLIDE..=PHYSICS_STATE_GRIND_BACKSLASH => select_grind(source),
        PHYSICS_STATE_NONSPECIFIC => select_nonspecific(source, fields, cvars),
        // CalcSuggestedState82ddbd58's FOLLOW_PATH case: until the AI input reports its local world.
        PHYSICS_STATE_FOLLOW_PATH => {
            if state.ai_flags.1 & 0x0200_0000 == 0 {
                return Err(E::Missing("Processed AI local world"));
            }
            Ok(Selection::State(if state.ai_flags.0 & 0x0200_0000 == 0 { PHYSICS_STATE_FOLLOW_PATH } else { PHYSICS_STATE_PHYSICS_GROUND }))
        }
        _ => Err(E::UnsupportedCurrentState(current)),
    }
}

/// CalcSuggestedState grind cases82ddcaa8..cae0: wipeout, OffBoardAir, the
/// grind jump into Air/KnownAir, else the investigated grind or Nonspecific701.
fn select_grind<C>(source: &SkeletonProcessedContact) -> Result<Selection, SystemLogicError<C>> {
    use SystemLogicError as E;
    if read_flags(source, 0x40000)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
    }
    let flags8cc = source
        .physics_state_fields()
        .flags_8cc
        .ok_or(E::Missing("Processed flags8cc"))?;
    if flags8cc & 0x100 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD_AIR));
    }
    if read_flags(source, 0x100)? != 0 {
        return Ok(Selection::State(
            PHYSICS_STATE_PHYSICS_AIR | i32::from(read_flags(source, 0x400)? != 0),
        ));
    }
    let grind = check_for_grind(source)?;
    Ok(Selection::State(if grind == 0 { PHYSICS_STATE_NONSPECIFIC } else { grind }))
}

/// CalcSuggestedState Nonspecific case82ddd0c4..d1a4.
fn select_nonspecific<C: StateChangerCvars>(
    source: &SkeletonProcessedContact,
    fields: &StateChangerFields,
    cvars: &mut C,
) -> Result<Selection, SystemLogicError<C::Error>> {
    use SystemLogicError as E;
    if read_flags(source, 0x40000)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
    }
    if read_flags(source, 0x100)? != 0 {
        return Ok(Selection::State(
            PHYSICS_STATE_PHYSICS_AIR | i32::from(read_flags(source, 0x400)? != 0),
        ));
    }
    if read_flags(source, 0x0040_0000)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_NONSPECIFIC));
    }
    let flags8c8 = source
        .known_flags_8c8()
        .ok_or(E::Missing("Processed flags8c8"))?;
    if flags8c8 & 0x20 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_PHYSICS_AIR));
    }
    if flags8c8 & 0x10 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_NONSPECIFIC));
    }
    let grind = check_for_grind(source)?;
    if grind != 0 {
        return Ok(Selection::State(grind));
    }
    let hang = source
        .grind_investigation()
        .ok_or(E::Missing("Processed grind investigation"))?
        .hang_exists;
    if hang {
        let frames = cvars
            .nonspecific_frames(NonspecificFrames::ToGroundNearGrind)
            .map_err(E::Cvar)?;
        if fields.three_wheel_counter > frames {
            return Ok(Selection::State(PHYSICS_STATE_PHYSICS_GROUND));
        }
    } else if fields.two_wheel_counter > 2 {
        // IsSkateboardAnimated ? 103 : 100, the same query as the Air landing.
        return Ok(Selection::AirLanding);
    }
    let sticky = cvars
        .nonspecific_frames(NonspecificFrames::CollFreeSticky)
        .map_err(E::Cvar)?;
    if fields.nonspecific_free_frames <= sticky {
        let exit = cvars
            .nonspecific_frames(NonspecificFrames::CollExit)
            .map_err(E::Cvar)?;
        return Ok(Selection::State(
            if fields.nonspecific_collision_frames > exit {
                PHYSICS_STATE_PHYSICS_GROUND
            } else {
                PHYSICS_STATE_NONSPECIFIC
            },
        ));
    }
    Ok(Selection::State(PHYSICS_STATE_PHYSICS_AIR))
}

/// CalcSuggestedState82ddd070: wipeout, selector ready, OffBoardAir, then
/// Boneless holds while either animation jump bit (8d0 0x20000/0x10000) is set.
fn select_boneless<C>(source: &SkeletonProcessedContact) -> Result<Selection, SystemLogicError<C>> {
    use SystemLogicError as E;
    if read_flags(source, 0x40000)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
    }
    if read_flags(source, 0x400)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_KNOWN_AIR));
    }
    let state = source.physics_state_fields();
    if state.flags_8cc.ok_or(E::Missing("Processed flags8cc"))? & 0x100 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD_AIR));
    }
    let flags8d0 = state.flags_8d0.ok_or(E::Missing("Processed flags8d0"))?;
    if flags8d0 & 0x20000 != 0 || flags8d0 & 0x10000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_BONELESS));
    }
    Ok(Selection::State(PHYSICS_STATE_PHYSICS_AIR))
}

/// CalcSuggestedState82ddd24c: wipeout, then FootPlant holds while 8d0
/// 0x02000000 is set; otherwise Air, or KnownAir when 8c4 0x400 (the
/// selector-ready bit, `rlwimi` into bit0 of 200).
fn select_foot_plant<C>(
    source: &SkeletonProcessedContact,
) -> Result<Selection, SystemLogicError<C>> {
    use SystemLogicError as E;
    if read_flags(source, 0x40000)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
    }
    let flags8d0 = source
        .physics_state_fields()
        .flags_8d0
        .ok_or(E::Missing("Processed flags8d0"))?;
    if flags8d0 & 0x0200_0000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_FOOTPLANT));
    }
    Ok(Selection::State(
        PHYSICS_STATE_PHYSICS_AIR | i32::from(read_flags(source, 0x400)? != 0),
    ))
}

fn select_offboard_locomotion<C>(
    source: &SkeletonProcessedContact,
) -> Result<Selection, SystemLogicError<C>> {
    use crate::{
        physics::PhysicsState as S,
        wipeout::{OffBoardStateFlag as F, OffBoardStateInput},
    };
    use SystemLogicError as E;
    let next = OffBoardStateInput::try_suggest_from_locomotion(
        |flag| {
            if flag == F::WipingOut {
                return Ok(read_flags(source, 0x40000)? != 0);
            }
            let (word, mask) = match flag {
                F::AnimationRequestsWipeout => (source.physics_state_fields().flags_8d0, 0x40),
                F::LandOnBoard => (source.physics_state_fields().flags_8d0, 0x80),
                F::GrabbingObject => (source.physics_state_fields().flags_8cc, 0x800000),
                F::OffBoardTrajectory => (source.physics_state_fields().flags_8cc, 0x20000),
                F::OffBoardAir => (source.physics_state_fields().flags_8cc, 0x100),
                F::OffBoardComMotion => (source.physics_state_fields().flags_8cc, 0x10000),
                _ => return Err(E::Missing("unexpected OffBoardLocomotion flag")),
            };
            Ok(word.ok_or(E::Missing("Processed locomotion flags"))? & mask != 0)
        },
        || {
            Ok(source
                .physics_state_fields()
                .flags_8d0
                .ok_or(E::Missing("Processed flags8d0"))?
                & 0x40000
                != 0)
        },
    )?;
    Ok(Selection::State(match next {
        S::WipeoutGround => PHYSICS_STATE_WIPEOUT_GROUND,
        S::OffBoardPushing => PHYSICS_STATE_OFFBOARD_PUSHING,
        S::OffBoard => PHYSICS_STATE_OFFBOARD,
        S::OffBoardAir => PHYSICS_STATE_OFFBOARD_AIR,
        S::LandingOnDeck => PHYSICS_STATE_LANDINGONDECK,
        S::PhysicsGround => PHYSICS_STATE_PHYSICS_GROUND,
        S::OffBoardLocomotion => PHYSICS_STATE_OFFBOARD_LOCOMOTION,
        _ => return Err(E::Missing("unexpected OffBoardLocomotion suggestion")),
    }))
}

fn select_offboard_air<C>(
    source: &SkeletonProcessedContact,
) -> Result<Selection, SystemLogicError<C>> {
    use crate::{
        physics::PhysicsState as S,
        wipeout::{OffBoardStateFlag as F, OffBoardStateInput},
    };
    use SystemLogicError as E;
    let next = OffBoardStateInput::try_suggest_from_air(|flag| match flag {
        F::WipingOut => Ok(read_flags(source, 0x40000)? != 0),
        F::OffBoardComMotion | F::OffBoardTrajectory | F::OffBoardAir => {
            let mask = match flag {
                F::OffBoardComMotion => 0x10000,
                F::OffBoardTrajectory => 0x20000,
                _ => 0x100,
            };
            Ok(source
                .physics_state_fields()
                .flags_8cc
                .ok_or(E::Missing("Processed flags8cc"))?
                & mask
                != 0)
        }
        F::LandOnBoard => Ok(source
            .physics_state_fields()
            .flags_8d0
            .ok_or(E::Missing("Processed flags8d0"))?
            & 0x80
            != 0),
        F::BoardMountInvalid => {
            let (bits, known) = source.physics_state_fields().flags_8d4;
            if known & 0x800000 == 0 {
                return Err(E::Missing("Processed flags8d4"));
            }
            Ok(bits & 0x800000 != 0)
        }
        _ => Err(E::Missing("unexpected OffBoardAir flag")),
    })?;
    Ok(Selection::State(match next {
        S::WipeoutGround => PHYSICS_STATE_WIPEOUT_GROUND,
        S::OffBoardLocomotion => PHYSICS_STATE_OFFBOARD_LOCOMOTION,
        S::OffBoard => PHYSICS_STATE_OFFBOARD,
        S::LandingOnDeck => PHYSICS_STATE_LANDINGONDECK,
        S::PhysicsAir => PHYSICS_STATE_PHYSICS_AIR,
        S::OffBoardAir => PHYSICS_STATE_OFFBOARD_AIR,
        _ => return Err(E::Missing("unexpected OffBoardAir suggestion")),
    }))
}

/// CalcSuggestedState82ddcc60..cc8c: wipeout takes precedence over
/// the animation's keep-pushing request. Otherwise return to OffBoard500.
fn select_offboard_pushing<C>(
    source: &SkeletonProcessedContact,
) -> Result<Selection, SystemLogicError<C>> {
    if read_flags(source, 0x40000)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
    }
    let flags = source
        .physics_state_fields()
        .flags_8cc
        .ok_or(SystemLogicError::Missing("Processed flags8cc"))?;
    Ok(Selection::State(if flags & 0x800000 != 0 {
        PHYSICS_STATE_OFFBOARD_PUSHING
    } else {
        PHYSICS_STATE_OFFBOARD
    }))
}

fn select_offboard<C>(source: &SkeletonProcessedContact) -> Result<Selection, SystemLogicError<C>> {
    use crate::{
        physics::PhysicsState,
        wipeout::{OffBoardStateFlag as F, OffBoardStateInput},
    };
    use SystemLogicError as E;
    // The policy already lives in OffBoardStateInput. This adapter only reads
    // the retained Processed occurrence when that policy reaches each flag.
    let state = OffBoardStateInput::try_suggest_from_offboard(
        |flag| match flag {
            F::WipingOut => Ok(read_flags(source, 0x40000)? != 0),
            F::GrabbingObject
            | F::OffBoardComMotion
            | F::OffBoardStandingJump
            | F::OffBoardAir
            | F::OffBoardTrajectory => {
                let mask = match flag {
                    F::GrabbingObject => 0x800000,
                    F::OffBoardComMotion => 0x10000,
                    F::OffBoardStandingJump => 0x100000,
                    F::OffBoardAir => 0x100,
                    F::OffBoardTrajectory => 0x20000,
                    _ => unreachable!(),
                };
                Ok(source
                    .physics_state_fields()
                    .flags_8cc
                    .ok_or(E::Missing("Processed flags8cc"))?
                    & mask
                    != 0)
            }
            F::OffBoardJumpInfoReady | F::BoardMountInvalid => {
                let mask = if flag == F::OffBoardJumpInfoReady {
                    0x2000_0000
                } else {
                    0x0080_0000
                };
                let (bits, known) = source.physics_state_fields().flags_8d4;
                if known & mask == 0 {
                    return Err(E::Missing("Processed flags8d4"));
                }
                Ok(bits & mask != 0)
            }
            _ => Err(E::Missing("unexpected OffBoard flag")),
        },
        || {
            Ok(source
                .physics_state_fields()
                .flags_8d0
                .ok_or(E::Missing("Processed flags8d0"))?
                & 0x40000
                != 0)
        },
    )?;
    let next = match state {
        PhysicsState::OffBoard => PHYSICS_STATE_OFFBOARD,
        PhysicsState::OffBoardPushing => PHYSICS_STATE_OFFBOARD_PUSHING,
        PhysicsState::OffBoardJump => PHYSICS_STATE_OFFBOARD_JUMP,
        PhysicsState::OffBoardLocomotion => PHYSICS_STATE_OFFBOARD_LOCOMOTION,
        PhysicsState::PhysicsAir => PHYSICS_STATE_PHYSICS_AIR,
        PhysicsState::WipeoutGround => PHYSICS_STATE_WIPEOUT_GROUND,
        _ => return Err(E::Missing("unexpected OffBoard suggestion")),
    };
    Ok(Selection::State(next))
}

fn select_ground_animation<C>(
    source: &SkeletonProcessedContact,
    colliding: bool,
) -> Result<Selection, SystemLogicError<C>> {
    use SystemLogicError as E;
    if read_flags(source, 0x40000)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
    }
    let state = source.physics_state_fields();
    if state.flags_8d0.ok_or(E::Missing("Processed flags8d0"))? & 0x20000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_BONELESS));
    }
    let flags = state.flags_8cc.ok_or(E::Missing("Processed flags8cc"))?;
    if flags & 0x20000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD));
    }
    if flags & 0x100 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD_AIR));
    }
    if state
        .frames_since_jump
        .ok_or(E::Missing("Processed frames since jump"))?
        == 1
    {
        return Ok(Selection::State(
            PHYSICS_STATE_PHYSICS_AIR | i32::from(read_flags(source, 0x400)? != 0),
        ));
    }
    Ok(Selection::GroundAnimation { colliding })
}
/// CalcSuggestedState KnownAir case82ddc88c..caa8, not ordinary Air's case.
fn select_known_air<C: StateChangerCvars>(
    source: &SkeletonProcessedContact,
    fields: &StateChangerFields,
    colliding: bool,
    flags8c8: u32,
    cvars: &mut C,
) -> Result<Selection, SystemLogicError<C::Error>> {
    use SystemLogicError as E;
    let s = source.physics_state_fields();
    if s.flags_8d0.ok_or(E::Missing("flags8d0"))? & 0x0400_0000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_FOOTPLANT));
    }
    let flags = s.flags_8cc.ok_or(E::Missing("flags8cc"))?;
    if flags & 0x20000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD));
    }
    if flags & 0x10000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD_LOCOMOTION));
    }
    if flags & 0x100 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD_AIR));
    }
    if read_flags(source, 0x40000)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
    }
    if flags8c8 & 0x100 != 0 {
        return Ok(Selection::State(
            if colliding || read_flags(source, 0x8000)? != 0 {
                PHYSICS_STATE_WIPEOUT_GROUND
            } else {
                PHYSICS_STATE_KNOWN_AIR
            },
        ));
    }
    if s.state_timer.ok_or(E::Missing("state timer"))?
        > cvars.minimum_air_time().map_err(E::Cvar)?
        && fields.post_grind_jump_counter > cvars.grind_jump_air_frames().map_err(E::Cvar)?
    {
        let grind = check_for_grind(source)?;
        if grind != 0 {
            return Ok(Selection::State(grind));
        }
    }
    if colliding {
        let timer = s.state_timer.ok_or(E::Missing("state timer"))?;
        if timer >= cvars.known_air_warning_time().map_err(E::Cvar)? {
            return Ok(Selection::State(if flags8c8 & 0x10 != 0 {
                PHYSICS_STATE_PHYSICS_AIR
            } else {
                PHYSICS_STATE_PHYSICS_GROUND
            }));
        }
        if cvars.state_debug_warnings().map_err(E::Cvar)? {
            return Err(E::Missing("KnownAir early contact debug overlay"));
        }
    }
    if source
        .prior_output()
        .trajectory_collision_time()
        .ok_or(E::Missing("trajectory collision time9d0"))?
        < -0.2
    {
        return Ok(Selection::State(PHYSICS_STATE_PHYSICS_AIR | i32::from(flags & 0x1000_0000 != 0)));
    }
    if read_flags(source, 0x8000)? != 0 {
        return Ok(Selection::State(if flags & 0x1000_0000 != 0 {
            PHYSICS_STATE_WIPEOUT_GROUND
        } else {
            PHYSICS_STATE_PHYSICS_AIR
        }));
    }
    Ok(Selection::State(PHYSICS_STATE_KNOWN_AIR))
}
fn select_air<C: StateChangerCvars>(
    source: &SkeletonProcessedContact,
    fields: &StateChangerFields,
    colliding: bool,
    flags8c8: u32,
    cvars: &mut C,
) -> Result<Selection, SystemLogicError<C::Error>> {
    use SystemLogicError as E;
    // CalcSuggestedState82ddc420: these two priorities precede the Air mode flag.
    if read_flags(source, 0x40000)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
    }
    let state = source.physics_state_fields();
    if state.flags_8cc.ok_or(E::Missing("Processed flags8cc"))? & 0x100 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD_AIR));
    }
    if flags8c8 & 0x100 != 0 {
        if colliding || read_flags(source, 0x8000)? != 0 {
            return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
        }
    } else {
        // Raw c47c timer read precedes c484/488 target/value. BO4 BI25 at
        // c490 skips on NOT GT, including unordered; only ordered > enters.
        let timer = state
            .state_timer
            .ok_or(E::Missing("Processed state timer"))?;
        let minimum = cvars.minimum_air_time().map_err(E::Cvar)?;
        if timer > minimum {
            if colliding {
                if source
                    .known_flags_8c8()
                    .ok_or(E::Missing("Processed flags8c8"))?
                    & 0x10
                    != 0
                {
                    return Ok(Selection::State(PHYSICS_STATE_PHYSICS_AIR));
                }
                return Ok(Selection::AirLanding);
            }
            let frames = cvars.grind_jump_air_frames().map_err(E::Cvar)?;
            if fields.post_grind_jump_counter > frames {
                let grind = check_for_grind(source)?;
                if grind != 0 {
                    return Ok(Selection::State(grind));
                }
            }
        }
    }
    // Source c4fc reloads the retained Processed pointer/flags after the query.
    Ok(Selection::State(if read_flags(source, 0x400)? != 0 {
        PHYSICS_STATE_KNOWN_AIR
    } else {
        PHYSICS_STATE_PHYSICS_AIR
    }))
}

/// `CheckForGrind` 0x82dd3ac8: with a possible grind and neither 8d0
/// 0x00080000 nor 0x08000000, the investigated type's state (5050 401,
/// Boardslide 400, Tipslide 402, 5_O 403, Backslash 404); otherwise zero.
fn check_for_grind<C>(source: &SkeletonProcessedContact) -> Result<i32, SystemLogicError<C>> {
    use SystemLogicError as E;
    let state = source.physics_state_fields();
    if !state
        .grind_possible
        .ok_or(E::Missing("Processed grind possibility"))?
    {
        return Ok(0);
    }
    let flags = state.flags_8d0.ok_or(E::Missing("Processed flags8d0"))?;
    if flags & 0x0008_0000 != 0 || flags & 0x0800_0000 != 0 {
        return Ok(0);
    }
    let investigation = source
        .grind_investigation()
        .ok_or(E::Missing("Processed grind investigation"))?;
    Ok(match investigation.grind_type {
        0 => PHYSICS_STATE_GRIND_5050,
        1 => PHYSICS_STATE_GRIND_BOARDSLIDE,
        2 => PHYSICS_STATE_GRIND_TIPSLIDE,
        3 => PHYSICS_STATE_GRIND_5_O,
        4 => PHYSICS_STATE_GRIND_BACKSLASH,
        // 82dd3b9c debug FailAssert, then the shared zero return.
        _ => 0,
    })
}

fn select_ground<C>(
    source: &SkeletonProcessedContact,
    wheels: u8,
    colliding: bool,
) -> Result<Selection, SystemLogicError<C>> {
    use SystemLogicError as E;
    let state = source.physics_state_fields();
    let flags8d0 = state.flags_8d0.ok_or(E::Missing("Processed flags8d0"))?;
    // 82ddc160..c1dc: mIsHippyJumping without both feet on deck suggests
    // LandingOnDeck505 after the debug-overlay Get.
    if flags8d0 & 0x8000 != 0 && flags8d0 & 0x3000 != 0x3000 {
        return Ok(Selection::HippyJumped);
    }
    let request_handplant = flags8d0 & 0x8000_0000 != 0 && wheels <= 2;
    let flags8cc = state.flags_8cc.ok_or(E::Missing("Processed flags8cc"))?;
    if flags8cc & 2 != 0 && (request_handplant || wheels == 0) {
        return Ok(Selection::State(PHYSICS_STATE_HANDPLANT));
    }
    if flags8cc & 0x20000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD));
    }
    if flags8cc & 0x10000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD_LOCOMOTION));
    }
    if flags8cc & 0x100 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_OFFBOARD_AIR));
    }
    if read_flags(source, 0x40000)? != 0 {
        return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
    }
    if state
        .frames_since_jump
        .ok_or(E::Missing("Processed frames since jump"))?
        == 1
    {
        return Ok(Selection::State(if read_flags(source, 0x400)? != 0 {
            PHYSICS_STATE_KNOWN_AIR
        } else {
            PHYSICS_STATE_PHYSICS_AIR
        }));
    }
    if state
        .start_revert
        .ok_or(E::Missing("Processed start revert"))?
        != 0.0
    {
        return Ok(Selection::State(PHYSICS_STATE_REVERT_GROUND));
    }
    let grind = check_for_grind(source)?;
    if grind != 0 {
        return Ok(Selection::State(grind));
    }
    Ok(Selection::Ground { colliding })
}

/// Original-pair failure probe: an unavailable integer registry must stop after
/// actual counter writes, before request clearing or transition stores.
#[cfg(test)]
pub(crate) fn check_original_system_logic_prefix(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) {
    struct MissingInteger;
    impl StateChangerCvars for MissingInteger {
        fn nonspecific_frames(&mut self, _: NonspecificFrames) -> Result<i32, Self::Error> {
            panic!("no Nonspecific state in this test")
        }
        fn known_air_warning_time(&mut self) -> Result<f32, Self::Error> {
            Ok(0.04)
        }
        fn state_debug_warnings(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        type Error = &'static str;
        fn fall_through_world_reset_frames(&mut self) -> Result<i32, Self::Error> {
            Err("integer registry unavailable")
        }
        fn disable_skitching(&mut self) -> Result<bool, Self::Error> {
            panic!("earlier integer failure must stop before skitching cvar");
        }
        fn minimum_air_time(&mut self) -> Result<f32, Self::Error> {
            panic!("earlier integer failure")
        }
        fn grind_jump_air_frames(&mut self) -> Result<i32, Self::Error> {
            panic!("earlier integer failure")
        }
    }
    let owner = world
        .skaters
        .as_mut()
        .unwrap()
        .try_with_jump_pair(&world.backend, board, skeleton, |_, owner| {
            Arc::clone(owner)
        })
        .unwrap();
    let before = owner.state_changer().lock().unwrap().fields.unwrap();
    assert!(!owner.wipeout().lock().unwrap().requests_initialized());
    let jump_before = owner.physics_state().lock().unwrap().frames_since_jump;
    let result = world.try_update_pair_system_logic(board, skeleton, &mut MissingInteger);
    assert!(matches!(
        result,
        Err(SystemLogicError::Cvar("integer registry unavailable"))
    ));
    let after = owner.state_changer().lock().unwrap().fields.unwrap();
    assert_eq!(after.current, Some(PHYSICS_STATE_PHYSICS_GROUND));
    assert_eq!(
        after.post_grind_jump_counter,
        before.post_grind_jump_counter.wrapping_add(1)
    );
    assert!(!after.request_teleport);
    assert!(!owner.wipeout().lock().unwrap().requests_initialized());
    assert_eq!(
        owner.physics_state().lock().unwrap().frames_since_jump,
        jump_before
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics_state_ids::{
        PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GROUND, PHYSICS_CATEGORY_WIPEOUT,
    };

    struct Cvars(Result<i32, ()>, usize);
    impl StateChangerCvars for Cvars {
        fn nonspecific_frames(&mut self, _: NonspecificFrames) -> Result<i32, Self::Error> {
            panic!("no Nonspecific state in this test")
        }
        fn known_air_warning_time(&mut self) -> Result<f32, Self::Error> {
            Ok(0.04)
        }
        fn state_debug_warnings(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        type Error = ();
        fn fall_through_world_reset_frames(&mut self) -> Result<i32, ()> {
            self.1 += 1;
            self.0
        }
        fn disable_skitching(&mut self) -> Result<bool, ()> {
            panic!("prefix tests do not reach the skitching cvar");
        }
        fn minimum_air_time(&mut self) -> Result<f32, ()> {
            panic!("Ground prefix")
        }
        fn grind_jump_air_frames(&mut self) -> Result<i32, ()> {
            panic!("Ground prefix")
        }
    }

    #[test]
    fn skitch_release_uses_output_grab_bit_and_retained_grab_reaches_contact_query() {
        let mut source = SkeletonProcessedContact::new();
        source.try_initialize_constructor_fields().unwrap();
        source.physics_state_fields_mut().category = Some(PHYSICS_CATEGORY_GROUND);
        source.physics_state_fields_mut().flags_8cc = Some(0);
        let mut fields = StateChangerFields::constructed();
        fields.begin(PHYSICS_STATE_SKITCHING);
        let mut cvars = Cvars(Ok(400), 0);
        // The grab-request bit alone is not the previous output's grabbing bit.
        for bits in [0, 0x0100_0000] {
            source.physics_state_fields_mut().flags_8cc = Some(bits);
            assert_eq!(
                select_state(&mut fields, PHYSICS_STATE_SKITCHING, 4, &source, &mut cvars).unwrap(),
                Selection::State(PHYSICS_STATE_PHYSICS_GROUND)
            );
        }
        source.physics_state_fields_mut().flags_8cc = Some(0x0080_0000);
        assert_eq!(
            select_state(&mut fields, PHYSICS_STATE_SKITCHING, 4, &source, &mut cvars).unwrap(),
            Selection::Skitching
        );
    }

    #[test]
    fn wipeout_stays_after_common_prefix_without_ground_or_air_reads() {
        let mut source = SkeletonProcessedContact::new();
        source.try_initialize_constructor_fields().unwrap();
        let state = source.physics_state_fields_mut();
        state.category = Some(PHYSICS_CATEGORY_WIPEOUT);
        state.flags_8cc = None;
        state.flags_8d0 = None;
        state.state_timer = None;
        state.grind_possible = None;
        let mut fields = StateChangerFields::constructed();
        fields.air_frames = 7;
        fields.begin(PHYSICS_STATE_WIPEOUT_GROUND);
        let mut cvars = Cvars(Ok(400), 0);
        assert_eq!(
            select_state(&mut fields, PHYSICS_STATE_WIPEOUT_GROUND, 4, &source, &mut cvars).unwrap(),
            Selection::State(PHYSICS_STATE_WIPEOUT_GROUND)
        );
        assert_eq!(cvars.1, 1);
        assert_eq!(fields.air_frames, 0);
        assert_eq!(fields.two_wheel_counter, 1);
        assert_eq!(fields.three_wheel_counter, 1);
        assert_eq!(fields.post_grind_jump_counter, 1);
        // Common AI priority still precedes the otherwise-sticky Wipeout arm.
        source.physics_state_fields_mut().ai_flags = (0x8000_0000, 0x8200_0000);
        assert_eq!(
            select_state(&mut fields, PHYSICS_STATE_WIPEOUT_GROUND, 4, &source, &mut cvars).unwrap(),
            Selection::State(PHYSICS_STATE_FOLLOW_PATH)
        );
    }

    #[test]
    fn cvar_failure_keeps_counter_prefix_before_any_ground_branch() {
        let mut source = SkeletonProcessedContact::new();
        source.try_initialize_constructor_fields().unwrap();
        source.physics_state_fields_mut().category = Some(PHYSICS_CATEGORY_AIR);
        let mut fields = StateChangerFields::constructed();
        fields.air_frames = 8;
        fields.begin(PHYSICS_STATE_PHYSICS_GROUND);
        let mut cvars = Cvars(Err(()), 0);
        assert!(matches!(
            select_state(&mut fields, PHYSICS_STATE_PHYSICS_GROUND, 3, &source, &mut cvars),
            Err(SystemLogicError::Cvar(()))
        ));
        assert_eq!(fields.air_frames, 9);
        assert_eq!(fields.two_wheel_counter, 1);
        assert_eq!(fields.three_wheel_counter, 1);
        assert_eq!(fields.something_colliding_frames, 0);
        assert!(!fields.request_teleport);
        assert_eq!(cvars.1, 1);
    }

    #[test]
    fn fall_reset_priority_precedes_unknown_ai_and_ground_fields() {
        let mut source = SkeletonProcessedContact::new();
        source.try_initialize_constructor_fields().unwrap();
        let state = source.physics_state_fields_mut();
        state.category = Some(PHYSICS_CATEGORY_AIR);
        state.ai_flags = (0, 0);
        state.flags_8d0 = None;
        let mut fields = StateChangerFields::constructed();
        fields.air_frames = 400;
        fields.begin(PHYSICS_STATE_PHYSICS_GROUND);
        let mut cvars = Cvars(Ok(400), 0);
        assert_eq!(
            select_state(&mut fields, PHYSICS_STATE_PHYSICS_GROUND, 0, &source, &mut cvars).unwrap(),
            Selection::State(PHYSICS_STATE_PHYSICS_GROUND)
        );
        assert!(fields.request_teleport);
        assert_eq!(fields.air_frames, 401);
        assert_eq!(cvars.1, 1);
    }

    #[test]
    fn original_reset_requires_the_actual_ground_continuation() {
        let mut source = SkeletonProcessedContact::new();
        source.try_initialize_constructor_fields().unwrap();
        let mut fields = StateChangerFields::constructed();
        fields.begin(PHYSICS_STATE_PHYSICS_GROUND);
        let mut cvars = Cvars(Ok(400), 0);
        assert_eq!(
            select_state(&mut fields, PHYSICS_STATE_PHYSICS_GROUND, 0, &source, &mut cvars).unwrap(),
            Selection::Ground { colliding: false }
        );
        assert_eq!(fields.post_grind_jump_counter, 1);
        assert_eq!(cvars.1, 1);
    }
}

#[cfg(test)]
#[path = "physical_player_system_logic_air_tests.rs"]
mod air_tests;
#[cfg(test)]
#[path = "physical_player_system_logic_ground_animation_tests.rs"]
mod ground_animation_tests;

#[cfg(test)]
mod offboard_tests {
    use super::*;
    use crate::physics_state_ids::PHYSICS_CATEGORY_OFFBOARD;

    struct Cvars;
    impl StateChangerCvars for Cvars {
        fn nonspecific_frames(&mut self, _: NonspecificFrames) -> Result<i32, Self::Error> {
            panic!("no Nonspecific state in this test")
        }
        fn known_air_warning_time(&mut self) -> Result<f32, Self::Error> {
            Ok(0.04)
        }
        fn state_debug_warnings(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        type Error = ();
        fn fall_through_world_reset_frames(&mut self) -> Result<i32, ()> {
            Ok(400)
        }
        fn disable_skitching(&mut self) -> Result<bool, ()> {
            unreachable!()
        }
        fn minimum_air_time(&mut self) -> Result<f32, ()> {
            unreachable!()
        }
        fn grind_jump_air_frames(&mut self) -> Result<i32, ()> {
            unreachable!()
        }
    }

    #[test]
    fn offboard_air_selector_preserves_priority_and_lazy_unknown_field_errors() {
        let mut source = SkeletonProcessedContact::new();
        source.try_initialize_constructor_fields().unwrap();
        source.physics_state_fields_mut().flags_8cc = Some(0x10000);
        source.physics_state_fields_mut().flags_8d0 = None;
        source.physics_state_fields_mut().flags_8d4 = (0, 0);
        assert_eq!(
            select_offboard_air::<()>(&source).unwrap(),
            Selection::State(PHYSICS_STATE_OFFBOARD_LOCOMOTION)
        );
        source.physics_state_fields_mut().flags_8cc = Some(0x20000);
        assert_eq!(
            select_offboard_air::<()>(&source).unwrap(),
            Selection::State(PHYSICS_STATE_OFFBOARD)
        );
        source.physics_state_fields_mut().flags_8cc = Some(0x100);
        assert!(matches!(
            select_offboard_air::<()>(&source),
            Err(SystemLogicError::Missing("Processed flags8d0"))
        ));
        source.physics_state_fields_mut().flags_8d0 = Some(0x80);
        assert_eq!(
            select_offboard_air::<()>(&source).unwrap(),
            Selection::State(PHYSICS_STATE_LANDINGONDECK)
        );
        source.physics_state_fields_mut().flags_8d0 = Some(0);
        assert_eq!(
            select_offboard_air::<()>(&source).unwrap(),
            Selection::State(PHYSICS_STATE_OFFBOARD_AIR)
        );
        source.physics_state_fields_mut().flags_8cc = Some(0);
        assert!(matches!(
            select_offboard_air::<()>(&source),
            Err(SystemLogicError::Missing("Processed flags8d4"))
        ));
        source.physics_state_fields_mut().flags_8d4 = (0, 0x800000);
        assert_eq!(
            select_offboard_air::<()>(&source).unwrap(),
            Selection::State(PHYSICS_STATE_PHYSICS_AIR)
        );
        source.physics_state_fields_mut().flags_8d4 = (0x800000, 0x800000);
        assert_eq!(
            select_offboard_air::<()>(&source).unwrap(),
            Selection::State(PHYSICS_STATE_WIPEOUT_GROUND)
        );
    }

    #[test]
    fn selected_offboard_branch_keeps_native_priority_and_unknown_reads() {
        let mut source = SkeletonProcessedContact::new();
        source.try_initialize_constructor_fields().unwrap();
        let state = source.physics_state_fields_mut();
        state.category = Some(PHYSICS_CATEGORY_OFFBOARD);
        state.flags_8cc = Some(0x810000);
        state.flags_8d4 = (0, 0);
        let mut fields = StateChangerFields::constructed();
        fields.begin(PHYSICS_STATE_OFFBOARD);
        let mut cvars = Cvars;
        assert_eq!(
            select_state(&mut fields, PHYSICS_STATE_OFFBOARD, 0, &source, &mut cvars).unwrap(),
            Selection::State(PHYSICS_STATE_OFFBOARD_PUSHING)
        );

        source.physics_state_fields_mut().flags_8cc = Some(0x10000);
        assert_eq!(
            select_offboard::<()>(&source).unwrap(),
            Selection::State(PHYSICS_STATE_OFFBOARD_LOCOMOTION)
        );

        source.physics_state_fields_mut().flags_8cc = Some(0x20000);
        assert!(matches!(
            select_offboard::<()>(&source),
            Err(SystemLogicError::Missing("Processed flags8d4"))
        ));
        source.physics_state_fields_mut().flags_8d4 = (0, 0x2000_0000);
        assert_eq!(
            select_offboard::<()>(&source).unwrap(),
            Selection::State(PHYSICS_STATE_OFFBOARD)
        );

        source.physics_state_fields_mut().flags_8cc = Some(0x100100);
        source.physics_state_fields_mut().flags_8d4 = (0x2000_0000, 0x2000_0000);
        assert_eq!(
            select_offboard::<()>(&source).unwrap(),
            Selection::State(PHYSICS_STATE_OFFBOARD_JUMP)
        );
    }
}

// Native82ddd1cc..1f0: priority is wipeout, on-deck ground, offboard-air.
fn select_landing_on_deck<C>(
    source: &SkeletonProcessedContact,
) -> Result<Selection, SystemLogicError<C>> {
    use SystemLogicError as E;
    let flags = source.known_flags_8c8().ok_or(E::Missing("flags8c8"))?;
    if flags & 0x40000 != 0 {
        return Ok(Selection::State(PHYSICS_STATE_WIPEOUT_GROUND));
    }
    let flags = source
        .physics_state_fields()
        .flags_8cc
        .ok_or(E::Missing("flags8cc"))?;
    Ok(Selection::State(if flags & 0x40000 != 0 {
        PHYSICS_STATE_PHYSICS_GROUND
    } else if flags & 0x100 != 0 {
        PHYSICS_STATE_OFFBOARD_AIR
    } else {
        PHYSICS_STATE_LANDINGONDECK
    }))
}
#[cfg(test)]
mod landing_state_selector_tests {
    use super::*;
    #[test]
    fn source_priority_and_lazy_missing_flag_reads() {
        let mut p = SkeletonProcessedContact::new();
        p.try_initialize_constructor_fields().unwrap();
        p.physics_state_fields_mut().flags_8cc = None;
        assert!(matches!(
            select_landing_on_deck::<()>(&p),
            Err(SystemLogicError::Missing("flags8cc"))
        ));
        for (flags, target) in [(0x40100, PHYSICS_STATE_PHYSICS_GROUND), (0x100, PHYSICS_STATE_OFFBOARD_AIR), (0, PHYSICS_STATE_LANDINGONDECK)] {
            p.physics_state_fields_mut().flags_8cc = Some(flags);
            assert_eq!(
                select_landing_on_deck::<()>(&p).unwrap(),
                Selection::State(target)
            );
        }
    }
}
