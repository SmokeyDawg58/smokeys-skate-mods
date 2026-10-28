//! Selected ground Update5 output closure with an explicit grind exclusion.
//! The input token follows actual manager fills with an explicit grind exclusion.
//! Earlier `UpdateAll` callbacks remain pending.
use super::PhysicalAssemblyWorld;
use super::{
    PhysicalPlayerAssemblyOwner,
    update5_animation::{AnimationRuntimeError, try_publish_update5_animation},
    update5_decisions::{DecisionRuntimeError, try_publish_decisions},
    update5_finish_output::{FinishRuntimeError, try_finish_selected_update5},
    update5_post_category::{PostCategoryRuntimeError, try_fill_post_category_output},
    update5_push_intents::{PushIntentRuntimeError, try_publish_push_intents},
    update5_surface_check::{SurfaceCheckRuntimeError, try_do_update5_surface_check},
    update5_trick_payload::{TrickPayloadRuntimeError, try_publish_update5_trick_payload},
};
use crate::{
    debug_overlay::DebugOverlaySlot,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    input_cvar_runtime::InputCvarReferences,
    physical_player_phys_out::{
        conditioners::{
            damped_com::DampedComError, foot_side::PhysicsBeforeLandingQuality,
            physics_update::PhysicsConditionerUpdateError,
        },
        construct::{
            foot_side::FootSideError, ground_state_fill::GroundStateFillError,
            post_category::PostCategoryError, update5_animation::Update5AnimationError,
            update5_decisions::Update5DecisionOutputError, update5_finish::Update5FinishError,
            update5_push_intents::Update5PushIntentError, update5_state::Update5StateOutputError,
            update5_surface_check::Update5SurfaceCheckError,
            update5_trick_payload::Update5TrickPayloadError,
        },
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::{
        ground_state_fill::{GroundStateFillRuntimeError, try_fill_current_ground_state},
        update5_managers::GrindManagerFillPolicy,
        update5_outer_locality::Update5BeforeCurrentType,
        update5_state_publication::{
            Update5StateRuntimeError, try_fill_current_teleporting, try_publish_category_and_dt,
            try_publish_current_type,
        },
    },
    skateboard_force_runtime::push::attributes::PushAttributes,
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterPoolError,
    physical_player_phys_out::{
        conditioners::stairs::PhysicsUpdateComplete,
        construct::{landing_quality::LandingQualityError, stairs::StairsError},
    },
    simulation_assembly_backend::constraint_session::physical_island::SimulationPhysicalBodyError,
};
use std::sync::Arc;

pub(crate) struct SelectedGroundPhysicsComplete {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) physics: PhysicsUpdateComplete,
    pub(crate) grind_policy: GrindManagerFillPolicy,
}
#[derive(Debug)]
pub(crate) enum SelectedPhysicsFinishError<N> {
    Body(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    ForeignOwner,
    Poisoned,
    MissingConditioners,
    Landing(LandingQualityError<N>),
    Stairs(StairsError),
}
impl<N: std::fmt::Debug> std::fmt::Display for SelectedPhysicsFinishError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Body(error) => error.fmt(f),
            Self::Pool(error) => error.fmt(f),
            Self::Landing(error) => error.fmt(f),
            Self::Stairs(error) => error.fmt(f),
            Self::ForeignOwner => f.write_str("selected Physics finish: foreign owner"),
            Self::Poisoned => f.write_str("selected Physics finish: poisoned owner"),
            Self::MissingConditioners => {
                f.write_str("selected Physics finish: missing conditioners")
            }
        }
    }
}
impl PhysicalAssemblyWorld {
    /// Sixth and seventh native Physics callbacks on the same retained output.
    /// This does not discharge remaining `UpdateAll` scheduling.
    pub(crate) fn try_finish_selected_physics<N: ReckoningNumeric>(
        &self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: SelectedGroundBeforeLandingQuality,
        numeric: &mut N,
    ) -> Result<SelectedGroundPhysicsComplete, SelectedPhysicsFinishError<N::Error>> {
        use SelectedPhysicsFinishError as E;
        for handle in [board, skeleton] {
            let actual = self
                .physical_body_owner_for_contacts(handle)
                .map_err(E::Body)?;
            if !Arc::ptr_eq(actual, &before.owner) {
                return Err(E::ForeignOwner);
            }
        }
        let global = self.skeleton_construction(skeleton).map_err(E::Pool)?
            .and_then(crate::skeleton_assembly_construction::SkeletonAssemblyConstruction::body_reader)
            .map(crate::skeleton_body_attributes::SkeletonBodyAttributes::global_physics_animation_instance);
        let owner = before.owner;
        let physics = {
            let mut conditioners = owner.conditioners.lock().map_err(|_| E::Poisoned)?;
            let conditioners = conditioners.as_mut().ok_or(E::MissingConditioners)?;
            let mut output = owner.phys_out().lock().map_err(|_| E::Poisoned)?;
            let cvars = &owner.inputs().cvars;
            let before_stairs = conditioners
                .try_update_physics_landing_quality(
                    before.physics,
                    &mut output,
                    cvars,
                    global,
                    numeric,
                )
                .map_err(E::Landing)?;
            conditioners
                .try_finish_physics_update(before_stairs, &mut output, cvars, &self.debug_overlay)
                .map_err(E::Stairs)?
        };
        Ok(SelectedGroundPhysicsComplete {
            owner,
            physics,
            grind_policy: before.grind_policy,
        })
    }
}

#[derive(Debug)]
pub(crate) enum SelectedGroundConditioningError<N> {
    Skitching(crate::physical_player_grind_manager::view::GrindError),
    Revert(crate::simulation_assembly_backend::constraint_session::revert_ground::RevertError),
    State(Update5StateOutputError<Update5StateRuntimeError>),
    Decisions(Update5DecisionOutputError<DecisionRuntimeError<N>>),
    Ground(GroundStateFillError<GroundStateFillRuntimeError>),
    Air(crate::physical_player_phys_out::construct::air_post_solve::AirFillError<
        crate::physical_player_phys_out::construct::air_post_solve::AirStateAccessError>),
    KnownAir(crate::physical_player_assembly::known_air_state::KnownAirStateError),
    Wipeout(crate::physical_player_phys_out::construct::wipeout_state_fill::WipeoutFillError),
    OffBoardAir(crate::physical_player_assembly::offboard_air_state::AirStateError),
    OffBoardJump(crate::physical_player_assembly::offboard_jump_state::JumpStateError),
    LandingOnDeck(crate::physical_player_assembly::landing_on_deck_state::LandingStateError),
    OffBoardLocomotion(crate::physical_player_assembly::offboard_locomotion_state::LocomotionStateError),
    OffBoardPushing(crate::physical_player_grind_manager::view::GrindError),
    HandPlant(crate::physical_player_grind_manager::view::GrindError),
    OffBoard(crate::physical_player_phys_out::construct::offboard_state_fill::OffBoardFillError),
    Teleporting(Update5StateRuntimeError),
    UnsupportedState(crate::physical_player_physics_state::CurrentPhysicalState),
    GroundAnimation(crate::physical_player_phys_out::construct::air_post_solve::AirFillError<Update5StateRuntimeError>),
    PostCategory(PostCategoryError<PostCategoryRuntimeError>),
    TrickPayload(Update5TrickPayloadError<TrickPayloadRuntimeError>),
    Animation(Update5AnimationError<AnimationRuntimeError, <crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric as crate::skeleton_collision_numeric::SkeletonCollisionNumeric>::Error>),
    PushIntents(Update5PushIntentError<PushIntentRuntimeError>),
    Surface(Update5SurfaceCheckError<SurfaceCheckRuntimeError>),
    Finish(Update5FinishError<FinishRuntimeError>),
    Poisoned,
    MissingConditioners,
    Physics(PhysicsConditionerUpdateError<N>),
    DampedCom(DampedComError<N>),
    FootSide(FootSideError<N>),
    Grind(crate::simulation_assembly_backend::constraint_session::grind_states::GrindStateError),
    GrindsConditioner(
        crate::physical_player_phys_out::conditioners::grinds_update::GrindsConditionerError<N>,
    ),
    AudioConditioner(
        crate::physical_player_phys_out::conditioners::audio_update::AudioConditionerError<N>,
    ),
}
impl<N: std::fmt::Debug> std::fmt::Display for SelectedGroundConditioningError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::State(error) => error.fmt(f),
            Self::Decisions(error) => error.fmt(f),
            Self::Ground(error) => error.fmt(f),
            Self::Revert(error) => error.fmt(f),
            Self::KnownAir(error) => error.fmt(f),
            Self::Air(error) => error.fmt(f),
            Self::GroundAnimation(error) => error.fmt(f),
            Self::Wipeout(error) => error.fmt(f),
            Self::OffBoardAir(error) => error.fmt(f),
            Self::OffBoardJump(error) => error.fmt(f),
            Self::OffBoardLocomotion(error) => error.fmt(f),
            Self::OffBoardPushing(error) => error.fmt(f),
            Self::LandingOnDeck(error) => error.fmt(f),
            Self::OffBoard(error) => error.fmt(f),
            Self::Teleporting(error) => error.fmt(f),
            Self::UnsupportedState(state) => write!(f, "selected Fill state pending: {state:?}"),
            Self::PostCategory(error) => error.fmt(f),
            Self::TrickPayload(error) => error.fmt(f),
            Self::Animation(error) => error.fmt(f),
            Self::PushIntents(error) => error.fmt(f),
            Self::Surface(error) => error.fmt(f),
            Self::Finish(error) => error.fmt(f),
            Self::Poisoned => f.write_str("selected Ground conditioning owner poisoned"),
            Self::MissingConditioners => f.write_str("actual HiLOD conditioners not constructed"),
            Self::Physics(error) => error.fmt(f),
            Self::DampedCom(error) => error.fmt(f),
            Self::FootSide(error) => error.fmt(f),
            Self::HandPlant(error) => error.fmt(f),
            Self::Skitching(error) => error.fmt(f),
            Self::Grind(error) => error.fmt(f),
            Self::GrindsConditioner(error) => error.fmt(f),
            Self::AudioConditioner(error) => error.fmt(f),
        }
    }
}
/// The first five Physics.Update calls returned on this retained output.
/// Earlier and later `UpdateAll` stages are not discharged by this value.
pub(crate) struct SelectedGroundBeforeLandingQuality {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) physics: PhysicsBeforeLandingQuality,
    pub(crate) grind_policy: GrindManagerFillPolicy,
}

/// Reuses actual type publications53f0/55e4, Ground.Fill5b74 and Physics.Update.
/// This explicit selected corpus route never sets filtered state itself.
pub(crate) fn try_condition_selected_ground<N: ReckoningNumeric>(
    world: &mut PhysicalAssemblyWorld,
    before: Update5BeforeCurrentType,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    attributes: Option<&PushAttributes>,
    cvars: &InputCvarReferences,
    overlay: &DebugOverlaySlot,
    numeric: &mut N,
) -> Result<SelectedGroundBeforeLandingQuality, SelectedGroundConditioningError<N::Error>>
where
    N::Error: std::fmt::Debug,
{
    use SelectedGroundConditioningError as E;
    let grind_policy = before.grind_policy;
    let owner = before.owner;
    try_publish_current_type(&owner, board, skeleton).map_err(E::State)?;
    try_publish_decisions(&owner, overlay, numeric).map_err(E::Decisions)?;
    try_publish_category_and_dt(&owner, board, skeleton).map_err(E::State)?;
    try_fill_post_category_output(&owner, world, board, skeleton, cvars)
        .map_err(E::PostCategory)?;
    let selected = owner
        .physics_state()
        .lock()
        .map_err(|_| E::Poisoned)?
        .current;
    match selected {
        crate::physical_player_physics_state::CurrentPhysicalState::Skitching => {
            crate::simulation_assembly_backend::constraint_session::skitch_session::state(
                &owner,
                board,
                skeleton,
                |s| {
                    owner
                        .phys_out()
                        .lock()
                        .map_err(crate::physical_player_grind_manager::view::error)?
                        .try_fill_skitch(s, cvars)
                },
            )
            .map_err(E::Skitching)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::KnownAir => {
            world
                .try_fill_pair_known_air(&owner, numeric)
                .map_err(E::KnownAir)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::Ground => {
            try_fill_current_ground_state(&owner, board, skeleton, attributes)
                .map_err(E::Ground)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::RevertGround => {
            crate::physical_player_phys_out::construct::revert_fill::fill(&owner, board, skeleton)
                .map_err(E::Revert)?;
        }
        // SlideGround vtable823715a4+1c points to the shared BLR825187c0.
        crate::physical_player_physics_state::CurrentPhysicalState::SlideGround => {}
        crate::physical_player_physics_state::CurrentPhysicalState::Grind(kind) => {
            crate::physical_player_phys_out::construct::grind_state_fill::try_fill_current_grind(
                &owner, board, skeleton, kind, overlay,
            )
            .map_err(E::Grind)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::Nonspecific => {
            crate::physical_player_phys_out::construct::grind_state_fill::try_fill_current_nonspecific(
                &owner, board, skeleton,
            )
            .map_err(E::Grind)?;
        }
        // Boneless vtable8234a578+1c is the same shared BLR.
        crate::physical_player_physics_state::CurrentPhysicalState::Boneless => {}
        // FollowPath vtable82371678+1c is the same shared BLR.
        crate::physical_player_physics_state::CurrentPhysicalState::FollowPath => {}
        // FootPlant vtable8234a534+1c is the same shared BLR; its manager's
        // Fill82db6890 runs from Update5 instead.
        crate::physical_player_physics_state::CurrentPhysicalState::FootPlant => {}
        crate::physical_player_physics_state::CurrentPhysicalState::HandPlant => {
            owner
                .phys_out()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_fill_handplant_state()
                .map_err(E::HandPlant)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::GroundAnimation => {
            crate::physical_player_phys_out::construct::ground_animation_fill::try_fill_current_ground_animation(&owner,board,skeleton).map_err(E::GroundAnimation)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::Air => {
            crate::physical_player_phys_out::construct::air_post_solve::try_fill_current_air_state(
                &owner, board, skeleton,
            )
            .map_err(E::Air)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::WipeoutGround => {
            crate::physical_player_phys_out::construct::wipeout_state_fill::try_fill_current_wipeout_state(
                &owner, board, skeleton,
            ).map_err(E::Wipeout)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::OffBoardPushing => {
            world
                .try_fill_pair_offboard_pushing(&owner, board, skeleton, numeric)
                .map_err(E::OffBoardPushing)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::OffBoardLocomotion => {
            world
                .try_fill_pair_offboard_locomotion(&owner, board, skeleton, cvars)
                .map_err(E::OffBoardLocomotion)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::LandingOnDeck => {
            world
                .try_fill_pair_landing_state(&owner, board, skeleton, cvars)
                .map_err(E::LandingOnDeck)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::OffBoardAir => {
            world
                .try_fill_pair_offboard_air(&owner, board, skeleton, cvars)
                .map_err(E::OffBoardAir)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::OffBoardJump => {
            world
                .try_fill_pair_offboard_jump(&owner, board, skeleton, cvars)
                .map_err(E::OffBoardJump)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::OffBoard => {
            crate::physical_player_phys_out::construct::offboard_state_fill::try_fill_current_offboard(
                &owner, board, skeleton,
            ).map_err(E::OffBoard)?;
        }
        crate::physical_player_physics_state::CurrentPhysicalState::Teleporting => {
            try_fill_current_teleporting(&owner, board, skeleton).map_err(E::Teleporting)?;
        }
        other => return Err(E::UnsupportedState(other)),
    }
    try_publish_update5_trick_payload(&owner).map_err(E::TrickPayload)?;
    try_publish_update5_animation(&owner).map_err(E::Animation)?;
    try_publish_push_intents(&owner).map_err(E::PushIntents)?;
    try_do_update5_surface_check(&owner).map_err(E::Surface)?;
    try_finish_selected_update5(&owner).map_err(E::Finish)?;
    let physics = {
        let mut conditioners = owner.conditioners.lock().map_err(|_| E::Poisoned)?;
        let conditioners = conditioners.as_mut().ok_or(E::MissingConditioners)?;
        let mut output = owner.phys_out().lock().map_err(|_| E::Poisoned)?;
        // UpdateAll 0x82e29f88 slots 1 and 3. Intents, MissionSystem and Camera
        // share the empty Update 0x825187c0.
        conditioners
            .try_update_grinds(&mut output, cvars, overlay, numeric)
            .map_err(E::GrindsConditioner)?;
        conditioners
            .try_update_audio(&mut output, numeric)
            .map_err(E::AudioConditioner)?;
        // A failure may have advanced retained histories: never retry this call.
        let before_damped = conditioners
            .try_update_physics_prefix(&mut output, cvars, numeric)
            .map_err(E::Physics)?;
        let before_foot_side = conditioners
            .try_update_physics_damped_com(before_damped, &mut output, cvars, overlay, numeric)
            .map_err(E::DampedCom)?;
        conditioners
            .try_update_physics_foot_side(before_foot_side, &mut output, numeric)
            .map_err(E::FootSide)?
    };
    Ok(SelectedGroundBeforeLandingQuality {
        owner,
        physics,
        grind_policy,
    })
}
