//! Skeleton Init admission (0x82b6213c) into the SAME source `SkaterPool`.
//! The complete normal owner survives membership and failed admission; initial
//! collision Create precedes the later property and `SetUpNormal` mutations.
#[path = "dynamic_assembly_force_physics_pool.rs"]
pub(crate) mod force_physics;
#[path = "dynamic_assembly_skater_wipeout.rs"]
pub(crate) mod wipeout;
use super::{
    Arc, Member, MemberContent, SimulationManagerContactDomain, SimulationSkaterAssemblyHandle,
    SimulationSkaterAssemblyPool, SimulationSkaterPoolError, SkateboardSimulationBackend,
    SkeletonBodyAttributeRead, SkeletonCollisionPublication, SkeletonNormalAssembly,
    SkeletonNormalSetupError, fmt,
};
use crate::{
    dynamic_assembly_session::{
        hook_pool::SimulationHookAssemblyPool,
        skeleton_normal::drive_owner::{SkeletonDriveConstructionError, SkeletonDriveOwnerError},
        skeleton_normal::history_owner::SkeletonHistoryManagerError,
    },
    skateboard_wheels::SkateboardWheelMassNumeric,
    skeleton_drives::{SkeletonDriveAttributeRead, SkeletonDriveCvars, SkeletonDriveNumeric},
    skeleton_physics_history::{SkeletonHistoryInput, SkeletonPhysicsHistoryNumeric},
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::convert::Infallible;

#[derive(Debug)]
pub(crate) enum SimulationSkeletonOffboardTrajectoryResetError {
    Pool(SimulationSkaterPoolError),
    MissingConstruction,
    MissingOuter,
    ForeignOwner,
    MissingAnimationInput,
    PoisonedAnimationInput,
    Pose(crate::actor_construction::anim_out_phys_in::pose_lanes::PoseAccessError),
}

#[derive(Debug)]
pub(crate) enum SimulationSkeletonTeleportResetError {
    Pool(SimulationSkaterPoolError),
    MissingConstruction,
    ForeignOwner,
    MissingOuter,
    MissingHead,
    MissingIk(crate::skeleton_current_animation::SkeletonIkUnconstructed),
    MissingWobble,
}

impl SimulationSkaterAssemblyPool {
    /// Skeleton.StateSetHeadTrackingTarget82b3ba00 -> HeadTracking.SetTarget82dca790.
    pub(crate) fn try_set_skeleton_head_target(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        target: crate::skeleton_root_transforms::SkeletonRootVector,
    ) -> Result<(), &'static str> {
        if target.xyz.iter().any(|v| v.is_nan()) {
            return Err("head target NaN assertion");
        }
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(|_| "head target member")?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            return Err("head target skeleton member");
        };
        let construction = candidate
            .construction
            .as_deref_mut()
            .ok_or("head target construction")?;
        if !Arc::ptr_eq(construction.owner(), expected_owner) {
            return Err("head target foreign owner");
        }
        let head = construction
            .constructor
            .head
            .as_mut()
            .ok_or("head tracking missing")?;
        head.target_position = Some(target);
        head.valid_target = Some(true);
        Ok(())
    }
    /// Skeleton::TeleportReset 0x82b54254..9c on one admitted candidate.
    /// Constructor-owned children remain attached beside the normal owner.
    pub(crate) fn try_teleport_reset_skeleton_live(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
    ) -> Result<(), SimulationSkeletonTeleportResetError> {
        use SimulationSkeletonTeleportResetError as E;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        {
            let MemberContent::Skeleton { candidate, .. } = &self.members[index].content else {
                unreachable!("checked Skeleton member")
            };
            let construction = candidate.construction().ok_or(E::MissingConstruction)?;
            if !Arc::ptr_eq(construction.owner(), expected_owner) {
                return Err(E::ForeignOwner);
            }
        }
        // Body::TeleportReset changes collision and disable-count state.
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("checked Skeleton member")
        };
        let construction = candidate
            .construction
            .as_deref_mut()
            .expect("validated above");
        candidate.owner.clear_teleport_deck_zy(); // 54254
        construction
            .constructor
            .outer
            .as_mut()
            .ok_or(E::MissingOuter)?
            .teleport_reset = Some(true); // 54258
        candidate.owner.clear_teleport_want_respawn(); // 5425c
        candidate.owner.enable_physics_truck_tilt(); // 54260
        candidate.owner.teleport_reset(); // 54268 SkeletonBody::TeleportReset
        candidate.owner.set_teleport_follow_animation_data(); // 5426c
        construction
            .constructor
            .head
            .as_mut()
            .ok_or(E::MissingHead)?
            .reset_after_teleport(); // 54274
        candidate
            .owner
            .try_reset_teleport_ik()
            .map_err(E::MissingIk)?; // 5427c
        construction
            .constructor
            .wobble
            .as_mut()
            .ok_or(E::MissingWobble)?
            .reset_after_teleport(); // 5428c..9c
        Ok(())
    }
    /// UpdateRootTransformsOffBoard 0x82b45420..454bc: current trajectory,
    /// then only global/local pose matrix zero, on the attached Skeleton.
    pub(crate) fn try_reset_offboard_trajectory_and_root_poses(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
    ) -> Result<(), SimulationSkeletonOffboardTrajectoryResetError> {
        use crate::actor_construction::anim_out_phys_in::pose_lanes::PoseBuffer;
        use SimulationSkeletonOffboardTrajectoryResetError as E;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("checked Skeleton member")
        };
        let construction = candidate.construction_mut().ok_or(E::MissingConstruction)?;
        if !Arc::ptr_eq(construction.owner(), expected_owner) {
            return Err(E::ForeignOwner);
        }
        construction
            .constructor
            .outer
            .as_mut()
            .ok_or(E::MissingOuter)?
            .trajectory_transform = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
        let animation_input = construction
            .constructor
            .bindings
            .as_ref()
            .ok_or(E::MissingAnimationInput)?
            .animation_input
            .as_ref()
            .ok_or(E::MissingAnimationInput)?;
        let mut animation_input = animation_input
            .lock()
            .map_err(|_| E::PoisonedAnimationInput)?;
        let identity = crate::skeleton_root_transforms::SkeletonRootMatrix::known(
            crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM,
        );
        animation_input
            .set_pose_matrix_typed(PoseBuffer::Global, 0, &identity)
            .map_err(E::Pose)?;
        animation_input
            .set_pose_matrix_typed(PoseBuffer::Local, 0, &identity)
            .map_err(E::Pose)?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum SkeletonLandingWobbleError {
    Pool(SimulationSkaterPoolError),
    MissingConstruction,
    ForeignOwner,
    Attribute(crate::attribute_runtime::AttributeRuntimeError),
}

impl SimulationSkaterAssemblyPool {
    /// Skeleton.StartDeckLandingWobble82b420e8 on the retained pair.
    pub(crate) fn try_start_deck_landing_wobble(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        mirrored: bool,
    ) -> Result<(), SkeletonLandingWobbleError> {
        self.try_start_deck_state_wobble(backend, skeleton, expected_owner, true, mirrored)
    }
    pub(crate) fn try_start_deck_state_wobble(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        landing: bool,
        mirrored: bool,
    ) -> Result<(), SkeletonLandingWobbleError> {
        use SkeletonLandingWobbleError as E;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("checked Skeleton member")
        };
        let construction = candidate
            .construction
            .as_mut()
            .ok_or(E::MissingConstruction)?;
        if !Arc::ptr_eq(construction.owner(), expected_owner) {
            return Err(E::ForeignOwner);
        }
        construction
            .constructor
            .wobble
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .try_start(landing, mirrored, 1.0)
            .map_err(E::Attribute)
    }
}

#[path = "dynamic_assembly_skater_pool_jump.rs"]
pub(crate) mod jump;

#[path = "dynamic_assembly_skater_pool_ragdoll.rs"]
pub(crate) mod ragdoll;

#[path = "dynamic_assembly_skater_pool_hook_soft.rs"]
pub(crate) mod hip_hook_soft;

#[path = "../../player/skeleton/physical_player_skeleton_process_data.rs"]
pub(crate) mod process_data;
#[path = "dynamic_assembly_skater_pool_roots.rs"]
pub(crate) mod roots;

#[path = "dynamic_assembly_skater_pool_grounded.rs"]
pub(crate) mod grounded;

#[path = "dynamic_assembly_skater_pool_animation.rs"]
pub(crate) mod current_animation;

#[path = "dynamic_assembly_skater_pool_ik_construction.rs"]
pub(crate) mod ik_construction;

#[derive(Debug)]
pub(crate) enum SimulationSkeletonPostRecordsError<S> {
    Pool(SimulationSkaterPoolError),
    CollisionNotCurrent,
    AnimationNotCurrent,
    VolumeState(crate::skeleton_runtime_state::SkeletonRuntimeStateError),
    Update(
        crate::skeleton_post_physics_records::SkeletonPostPhysicsRecordError<
            S,
            crate::reckoning_numeric::ReckoningHostNumericError,
            Infallible,
        >,
    ),
}
impl<S: fmt::Debug> fmt::Display for SimulationSkeletonPostRecordsError<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "post-physics records owner: {e}"),
            Self::CollisionNotCurrent => {
                f.write_str("post-physics records need current completed collision")
            }
            Self::AnimationNotCurrent => {
                f.write_str("post-physics records need prepared animation targets")
            }
            Self::VolumeState(e) => write!(f, "post-physics volume states: {e}"),
            Self::Update(e) => write!(f, "post-physics records: {e:?}"),
        }
    }
}
impl<S: fmt::Debug> std::error::Error for SimulationSkeletonPostRecordsError<S> {}

impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_update_skeleton_post_physics_records<
        S: crate::skeleton_post_physics_records::SkeletonPostPhysicsRecordServices,
    >(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        frame: &Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
        time_delta: f32,
        flags_8c8: u32,
        services: &mut S,
    ) -> Result<(), SimulationSkeletonPostRecordsError<S::Error>> {
        self.try_update_skeleton_post_physics_records_with_flags(
            backend,
            skeleton,
            frame,
            time_delta,
            services,
            move || Ok(flags_8c8),
        )
    }

    pub(crate) fn try_update_skeleton_post_physics_records_with_flags<
        S: crate::skeleton_post_physics_records::SkeletonPostPhysicsRecordServices,
        F: FnOnce() -> Result<u32, S::Error>,
    >(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        frame: &Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
        time_delta: f32,
        services: &mut S,
        flags: F,
    ) -> Result<(), SimulationSkeletonPostRecordsError<S::Error>> {
        use super::super::skeleton_normal::collision_owner::SkeletonCollisionFrameProgress;
        use SimulationSkeletonPostRecordsError as E;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        self.try_physical_member_owner(backend, skeleton)
            .map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        let collision = candidate.owner.collision_frame_state();
        if collision.progress() != SkeletonCollisionFrameProgress::Complete
            || collision
                .frame()
                .is_none_or(|actual| !Arc::ptr_eq(actual, frame))
        {
            return Err(E::CollisionNotCurrent);
        }
        candidate
            .owner
            .try_update_volume_states()
            .map_err(E::VolumeState)?;
        if candidate
            .owner
            .current_animation()
            .root_input(time_delta)
            .is_none()
        {
            return Err(E::AnimationNotCurrent);
        }
        candidate
            .owner
            .try_update_post_physics_records_with_flags(time_delta, services, flags)
            .map_err(E::Update)
    }
}

pub(crate) struct SimulationSkeletonAssemblyCandidate {
    owner: Box<SkeletonNormalAssembly>,
    world_index: u32,
    construction: Option<Box<crate::skeleton_assembly_construction::SkeletonAssemblyConstruction>>,
}

impl SimulationSkeletonAssemblyCandidate {
    /// Explicit host recovery of the SAME returned candidate, never ID refund.
    pub(crate) fn try_detach_failed_admission(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
    ) -> Result<(), SimulationSkaterPoolError> {
        if self.owner.session().is_attached() {
            self.owner
                .detach(backend)
                .map_err(SimulationSkaterPoolError::Detach)?;
        }
        Ok(())
    }

    pub(crate) fn try_new(
        owner: SkeletonNormalAssembly,
    ) -> Result<Self, Box<SimulationSkeletonCandidateFailure>> {
        if owner.session().is_attached() {
            return Err(Box::new(SimulationSkeletonCandidateFailure {
                error: SimulationSkaterPoolError::AlreadyAttached,
                owner,
            }));
        }
        Ok(Self {
            owner: Box::new(owner),
            world_index: 0xffff,
            construction: None,
        })
    }
    pub(crate) fn owner(&self) -> &SkeletonNormalAssembly {
        &self.owner
    }
    pub(crate) const fn world_index(&self) -> u32 {
        self.world_index
    }
    pub(crate) fn into_owner(self) -> Result<SkeletonNormalAssembly, Box<Self>> {
        if self.construction.is_some() {
            return Err(Box::new(self));
        }
        Ok(*self.owner)
    }

    pub(crate) fn try_from_construction(
        mut construction: Box<crate::skeleton_assembly_construction::SkeletonAssemblyConstruction>,
    ) -> Result<Self, Box<crate::skeleton_assembly_construction::SkeletonAssemblyConstruction>>
    {
        let Ok(assembly) = construction.take_assembly() else {
            return Err(construction);
        };
        Ok(Self {
            owner: Box::new(assembly),
            world_index: 0xffff,
            construction: Some(construction),
        })
    }

    pub(crate) fn construction(
        &self,
    ) -> Option<&crate::skeleton_assembly_construction::SkeletonAssemblyConstruction> {
        self.construction.as_deref()
    }
    pub(super) fn construction_mut(
        &mut self,
    ) -> Option<&mut crate::skeleton_assembly_construction::SkeletonAssemblyConstruction> {
        self.construction.as_deref_mut()
    }
}

pub(crate) struct SimulationSkeletonCandidateFailure {
    pub(crate) error: SimulationSkaterPoolError,
    pub(crate) owner: SkeletonNormalAssembly,
}

#[derive(Debug)]
pub(crate) enum SimulationSkeletonHistoryUpdateError<E> {
    Pool(SimulationSkaterPoolError),
    DifferentPhysicalOwner,
    AlreadyAttempted,
    Update(super::super::skeleton_normal::history_owner::SkeletonHistoryUpdateError<E>),
}

impl<E: fmt::Debug> fmt::Display for SimulationSkeletonHistoryUpdateError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "skeleton history pair: {error}"),
            Self::DifferentPhysicalOwner => {
                f.write_str("skeleton history board belongs to a different physical player")
            }
            Self::AlreadyAttempted => {
                f.write_str("skeleton history already attempted for this solved frame")
            }
            Self::Update(error) => write!(f, "skeleton history update: {error}"),
        }
    }
}
impl<E: fmt::Debug> std::error::Error for SimulationSkeletonHistoryUpdateError<E> {}

#[derive(Debug)]
pub(crate) enum SimulationSkeletonCollisionError<C> {
    Pool(SimulationSkaterPoolError),
    HistoryNotCurrent,
    AlreadyAttempted,
    ContactOwnerMismatch,
    Update(super::super::skeleton_normal::collision_owner::SkeletonNormalCollisionError<C>),
}
impl<C: fmt::Debug> fmt::Display for SimulationSkeletonCollisionError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "skeleton collision member: {error}"),
            Self::HistoryNotCurrent => {
                f.write_str("skeleton collision requires completed history for this same frame")
            }
            Self::AlreadyAttempted => {
                f.write_str("skeleton collision already attempted for this frame")
            }
            Self::ContactOwnerMismatch => {
                f.write_str("skeleton collision list belongs to a different Body")
            }
            Self::Update(error) => write!(f, "{error}"),
        }
    }
}
impl<C: fmt::Debug> std::error::Error for SimulationSkeletonCollisionError<C> {}
impl fmt::Debug for SimulationSkeletonCandidateFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationSkeletonCandidateFailure")
            .field("error", &self.error)
            .field("attached", &self.owner.session().is_attached())
            .finish_non_exhaustive()
    }
}

pub(crate) struct SimulationSkeletonInsertFailure {
    pub(crate) error: SimulationSkaterPoolError,
    pub(crate) candidate: SimulationSkeletonAssemblyCandidate,
}
impl fmt::Debug for SimulationSkeletonInsertFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationSkeletonInsertFailure")
            .field("error", &self.error)
            .field("world_index", &self.candidate.world_index)
            .field("attached", &self.candidate.owner.session().is_attached())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) enum SimulationSkeletonMutationError<A, C> {
    Pool(SimulationSkaterPoolError),
    Setup(SkeletonNormalSetupError<A, C>),
}

impl<A: fmt::Debug, C: fmt::Debug> fmt::Display for SimulationSkeletonMutationError<A, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(source) => write!(f, "Skeleton member: {source}"),
            Self::Setup(source) => write!(f, "Skeleton member setup: {source}"),
        }
    }
}
impl<A: fmt::Debug, C: fmt::Debug> std::error::Error for SimulationSkeletonMutationError<A, C> {}

impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_retain_skeleton_drive_attributes(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        attributes: &mut Option<crate::skeleton_drive_attributes::SkeletonDriveAttributes>,
    ) -> Result<(), SimulationSkaterPoolError> {
        let index = self.skeleton_mutation_index(backend, skeleton)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate
            .owner
            .try_retain_drive_update_attributes(backend, attributes)
            .map_err(SimulationSkaterPoolError::SkeletonDrive)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "same current frame, actual owners and explicit controlled pose inputs"
    )]
    pub(crate) fn try_update_skeleton_drives_from_controlled_pose<
        C: crate::skeleton_drives::update::SkeletonDrivesUpdateCvars,
        H,
    >(
        &mut self,
        backend: &SkateboardSimulationBackend,
        hooks: &SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        skeleton: &SimulationSkaterAssemblyHandle,
        frame: &Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
        pose: &[crate::animation::AnimationMatrix],
        partial_mask: u8,
        cvars: &mut C,
    ) -> Result<
        (),
        super::super::skeleton_normal::drive_owner::update_owner::SkeletonDriveFeedbackError<
            C::Error,
        >,
    > {
        use super::super::skeleton_normal::drive_owner::update_owner::SkeletonDriveFeedbackError as Error;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(Error::Pool)?;
        self.try_physical_member_owner(backend, skeleton)
            .map_err(Error::Pool)?;
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(|e| Error::Pool(SimulationSkaterPoolError::Hook(e)))?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate.owner.try_update_drives_from_controlled_pose(
            backend,
            hooks,
            frame,
            pose,
            partial_mask,
            cvars,
        )
    }

    pub(crate) fn try_update_skeleton_normal_collision<
        C: crate::skeleton_collision_data::update::SkeletonCollisionCvars,
    >(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        frame: &Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
        contacts:&[crate::simulation_assembly_backend::constraint_session::physical_contact_list::PhysicalContactInfo],
        info: &crate::skeleton_collision_data::update::SkeletonCollisionUpdateInput,
        cvars: &mut C,
        vehicle_pool: Option<
            &crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
        >,
        dmo_pool: Option<
            &crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
        >,
        pedestrian_pool: Option<
            &crate::simulation_dmo_assembly_part_session::dynamic_pool::character::pedestrian_pool::PedestrianAssemblyPool,
        >,
    ) -> Result<(), SimulationSkeletonCollisionError<C::Error>> {
        use super::super::skeleton_normal::history_owner::SkeletonHistoryFrameProgress;
        use SimulationSkeletonCollisionError as Error;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(Error::Pool)?;
        let (_, owner) = self
            .try_physical_member_owner(backend, skeleton)
            .map_err(Error::Pool)?;
        let vault = Arc::clone(&owner.inputs().vault);
        if contacts.iter().any(|contact| {
            !contact.this_body().same_as(skeleton) || !Arc::ptr_eq(contact.spy_frame(), frame)
        }) {
            return Err(Error::ContactOwnerMismatch);
        }
        if [vehicle_pool, dmo_pool]
            .into_iter()
            .flatten()
            .any(|pool| !pool.matches_authorities(&self.manager, &self.body_pool))
            || pedestrian_pool.is_some_and(|pool| !pool.matches_authorities(&self.manager, &self.body_pool))
        {
            return Err(Error::Pool(SimulationSkaterPoolError::ForeignBackend));
        }
        let (before, selected_and_after) = self.members.split_at_mut(index);
        let (selected, after) = selected_and_after
            .split_first_mut()
            .expect("checked member");
        let MemberContent::Skeleton { candidate, .. } = &mut selected.content else {
            unreachable!("kind checked")
        };
        let history = candidate.owner.physics_history_frame_state();
        if history.progress() != SkeletonHistoryFrameProgress::Complete
            || history
                .frame()
                .is_none_or(|actual| !Arc::ptr_eq(actual, frame))
        {
            return Err(Error::HistoryNotCurrent);
        }
        if !candidate.owner.try_begin_collision_frame(Arc::clone(frame)) {
            return Err(Error::AlreadyAttempted);
        }
        // Collision.Update never mutates Assembly group. This self-group read
        // is stable under the exclusive owner borrow; all OTHER groups are
        // read lazily from their actual retained owner at each source contact.
        let own_group = candidate.owner.state().assembly_group;
        candidate
            .owner
            .try_update_normal_collision(backend, contacts, info, &vault, cvars, |body| {
                use crate::simulation_assembly_backend::constraint_session::physical_island::SimulationIslandBodyHandle;
                let body = match body {
                    SimulationIslandBodyHandle::Skater(body) => body,
                    SimulationIslandBodyHandle::Vehicle(body) => {
                        let pool = vehicle_pool.ok_or(SimulationSkaterPoolError::MissingMember)?;
                        return pool.character_candidate(body).map(|candidate| candidate.assembly.group)
                            .ok_or(SimulationSkaterPoolError::MissingMember);
                    }
                    SimulationIslandBodyHandle::Pedestrian(body) => {
                        let pool = pedestrian_pool.ok_or(SimulationSkaterPoolError::MissingMember)?;
                        return pool.candidate(body).map(|candidate| candidate.assembly.group)
                            .ok_or(SimulationSkaterPoolError::MissingMember);
                    }
                    SimulationIslandBodyHandle::Dmo(body) => {
                        let pool = dmo_pool.ok_or(SimulationSkaterPoolError::MissingMember)?;
                        return pool.candidate(body).map(|candidate| candidate.group())
                            .ok_or(SimulationSkaterPoolError::MissingMember);
                    }
                };
                if body.same_as(skeleton) {
                    return Ok(own_group);
                }
                let member = before
                    .iter()
                    .chain(after.iter())
                    .find(|member| member.handle.same_as(body))
                    .ok_or(SimulationSkaterPoolError::MissingMember)?;
                match &member.content {
                    MemberContent::Board { candidate, .. } => candidate
                        .construction()
                        .and_then(
                            crate::skateboard_body_construction::SkateboardBodyConstruction::group,
                        )
                        .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction),
                    MemberContent::Skeleton { candidate, .. } => {
                        candidate
                            .construction()
                            .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?;
                        Ok(candidate.owner().state().assembly_group)
                    }
                }
            })
            .map_err(Error::Update)
    }

    /// Actual physical owner resolves the paired board used at 0x82b4cc4c.
    /// No publication transform or caller-supplied pose replaces that live read.
    /// The world checks frame freshness before this private pool continuation.
    pub(crate) fn try_update_paired_skeleton_history<I: SkeletonHistoryInput>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        frame: Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
        input: &mut I,
        numeric: &mut impl SkeletonPhysicsHistoryNumeric,
    ) -> Result<(), SimulationSkeletonHistoryUpdateError<I::Error>> {
        use super::super::skeleton_normal::history_owner::{
            SkeletonHistoryInitialization, SkeletonHistoryUpdateError,
        };
        use super::physical::SimulationPhysicalBodyKind;
        use SimulationSkeletonHistoryUpdateError as Error;
        let skeleton_index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(Error::Pool)?;
        let board_index = self.member_index(board).map_err(Error::Pool)?;
        let (kind, board_owner) = self
            .try_physical_member_owner(backend, board)
            .map_err(Error::Pool)?;
        if kind != SimulationPhysicalBodyKind::Board {
            return Err(Error::Pool(SimulationSkaterPoolError::WrongMemberKind));
        }
        let (_, skeleton_owner) = self
            .try_physical_member_owner(backend, skeleton)
            .map_err(Error::Pool)?;
        if !Arc::ptr_eq(board_owner, skeleton_owner) {
            return Err(Error::DifferentPhysicalOwner);
        }
        // Both retained constructors and actual membership are checked before
        // taking a disjoint borrow of the authoritative owners.
        let (board_member, skeleton_member) = if board_index < skeleton_index {
            let (before, after) = self.members.split_at_mut(skeleton_index);
            (&before[board_index], &mut after[0])
        } else {
            let (before, after) = self.members.split_at_mut(board_index);
            (&after[0], &mut before[skeleton_index])
        };
        let MemberContent::Board {
            candidate: board, ..
        } = &board_member.content
        else {
            unreachable!("board kind checked")
        };
        let MemberContent::Skeleton {
            candidate: skeleton,
            ..
        } = &mut skeleton_member.content
        else {
            unreachable!("skeleton kind checked")
        };
        if skeleton.owner.physics_history_initialization()
            != SkeletonHistoryInitialization::Complete
        {
            return Err(Error::Update(
                SkeletonHistoryUpdateError::InitializationIncomplete,
            ));
        }
        if !skeleton.owner.try_begin_physics_history_frame(frame) {
            return Err(Error::AlreadyAttempted);
        }
        skeleton
            .owner
            .try_update_physics_history_from_board(backend, board.session(), input, numeric)
            .map_err(Error::Update)?;
        skeleton.owner.complete_physics_history_frame();
        Ok(())
    }

    pub(crate) fn skeleton_construction(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        Option<&crate::skeleton_assembly_construction::SkeletonAssemblyConstruction>,
        SimulationSkaterPoolError,
    > {
        match &self.members[self.member_index(handle)?].content {
            MemberContent::Skeleton { candidate, .. } => Ok(candidate.construction()),
            MemberContent::Board { .. } => Err(SimulationSkaterPoolError::WrongMemberKind),
        }
    }

    /// `SetUpNormal` uses the SAME body reader retained by `SkeletonBody` creation.
    pub(crate) fn try_set_constructed_skeleton_normal(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        (),
        SimulationSkeletonMutationError<
            crate::attribute_runtime::AttributeRuntimeError,
            Infallible,
        >,
    > {
        let index = self
            .skeleton_mutation_index(backend, handle)
            .map_err(SimulationSkeletonMutationError::Pool)?;
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        let reader = candidate
            .construction
            .as_mut()
            .and_then(|c| c.body_reader_mut())
            .ok_or(SimulationSkeletonMutationError::Pool(
                SimulationSkaterPoolError::MissingPhysicalConstruction,
            ))?;
        candidate
            .owner
            .try_set_up_normal(backend, reader, &mut |angle: f32| Ok(angle.cos()))
            .map_err(SimulationSkeletonMutationError::Setup)
    }

    /// Constructor `TeleportReset` on the actual admitted owner, before drives.
    pub(crate) fn try_teleport_reset_skeleton(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        let index = self.skeleton_mutation_index(backend, handle)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate.owner.teleport_reset();
        Ok(())
    }

    /// Insert 0x82e0dbb0, with Skeleton's group1/Active Add. Host Create failure
    /// returns the actual issued-index candidate and does not silently detach.
    pub(crate) fn try_insert_skeleton(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        manager: &mut SimulationManagerContactDomain,
        mut candidate: SimulationSkeletonAssemblyCandidate,
        bend: f32,
    ) -> Result<SimulationSkaterAssemblyHandle, Box<SimulationSkeletonInsertFailure>> {
        let result = (|| {
            self.check_backend(backend)?;
            if manager.assembly_pool_authority() != self.manager {
                return Err(SimulationSkaterPoolError::ForeignManager);
            }
            if candidate
                .construction()
                .is_some_and(|c| c.owner().manager() != self.manager)
            {
                return Err(SimulationSkaterPoolError::ForeignManager);
            }
            if candidate.owner.session().is_attached() {
                return Err(SimulationSkaterPoolError::AlreadyAttached);
            }
            if self.members.len() == self.capacity {
                return Err(SimulationSkaterPoolError::Full {
                    capacity: self.capacity,
                });
            }
            let handle = SimulationSkaterAssemblyHandle(Arc::new(()));
            candidate.world_index = manager
                .try_allocate_assembly_world_index(self.body_pool)
                .map_err(SimulationSkaterPoolError::Labels)?;
            candidate
                .owner
                .attach(backend)
                .map_err(SimulationSkaterPoolError::Attach)?;
            let publication =
                SkeletonCollisionPublication::try_create(&candidate.owner, backend, bend)
                    .map_err(SimulationSkaterPoolError::SkeletonPublication)?;
            Ok((handle, publication))
        })();
        match result {
            Ok((handle, publication)) => {
                let bounds = publication.bounds();
                self.members.push(Member {
                    handle: handle.clone(),
                    content: MemberContent::Skeleton {
                        candidate,
                        publication,
                    },
                    bounds,
                });
                self.active_frame = None;
                Ok(handle)
            }
            Err(error) => Err(Box::new(SimulationSkeletonInsertFailure {
                error,
                candidate,
            })),
        }
    }

    pub(crate) fn skeleton(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&SkeletonNormalAssembly, SimulationSkaterPoolError> {
        match &self.members[self.member_index(handle)?].content {
            MemberContent::Skeleton { candidate, .. } => Ok(&candidate.owner),
            MemberContent::Board { .. } => Err(SimulationSkaterPoolError::WrongMemberKind),
        }
    }

    pub(crate) fn skeleton_publication(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&SkeletonCollisionPublication, SimulationSkaterPoolError> {
        match &self.members[self.member_index(handle)?].content {
            MemberContent::Skeleton { publication, .. } => Ok(publication),
            MemberContent::Board { .. } => Err(SimulationSkaterPoolError::WrongMemberKind),
        }
    }

    fn skeleton_mutation_index(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<usize, SimulationSkaterPoolError> {
        self.check_backend(backend)?;
        let index = self.member_index(handle)?;
        self.skeleton(handle)?;
        Ok(index)
    }

    /// Checked owner-only property mutation; invalidate BEFORE possible writes.
    pub(crate) fn try_set_skeleton_pre_normal_properties(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkeletonMutationError<Infallible, Infallible>> {
        let index = self
            .skeleton_mutation_index(backend, handle)
            .map_err(SimulationSkeletonMutationError::Pool)?;
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate
            .owner
            .try_set_pre_normal_properties(backend)
            .map_err(SimulationSkeletonMutationError::Setup)
    }

    /// Native `SetUpNormal` 0x82b50508 on the SAME owner whose initial records
    /// were created at admission. Completed source prefixes survive errors.
    pub(crate) fn try_set_skeleton_normal<R: SkeletonBodyAttributeRead, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        reader: &mut R,
        cosine: &mut impl FnMut(f32) -> Result<f32, C>,
    ) -> Result<(), SimulationSkeletonMutationError<R::Error, C>> {
        let index = self
            .skeleton_mutation_index(backend, handle)
            .map_err(SimulationSkeletonMutationError::Pool)?;
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate
            .owner
            .try_set_up_normal(backend, reader, cosine)
            .map_err(SimulationSkeletonMutationError::Setup)
    }

    /// Source 24-Part placement prefix on the actual member; not full Set's
    /// two-hook suffix. Invalidation precedes even a same-frame partial failure.
    pub(crate) fn try_place_skeleton_parts(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        let index = self.skeleton_mutation_index(backend, handle)?;
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate
            .owner
            .try_place_parts_from_retained_physics_pose(backend)
            .map_err(SimulationSkaterPoolError::SkeletonPlacement)
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "explicit pool authorities and independent source-service error types"
    )]
    pub(crate) fn try_initialize_skeleton_drives<
        A: SkeletonDriveAttributeRead,
        N: SkeletonDriveNumeric,
        V: SkeletonDriveCvars,
        H,
    >(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        manager: &mut SimulationManagerContactDomain,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        handle: &SimulationSkaterAssemblyHandle,
        attributes: &mut A,
        numeric: &mut N,
        mass: &mut impl SkateboardWheelMassNumeric,
        cvars: &mut V,
    ) -> Result<(), SkeletonDriveConstructionError<A::Error, N::Error, V::Error>> {
        let index = self
            .skeleton_mutation_index(backend, handle)
            .map_err(SkeletonDriveConstructionError::Pool)?;
        if manager.assembly_pool_authority() != self.manager {
            return Err(SkeletonDriveConstructionError::Pool(
                SimulationSkaterPoolError::ForeignManager,
            ));
        }
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(|e| {
                SkeletonDriveConstructionError::Pool(SimulationSkaterPoolError::Hook(e))
            })?;
        if self
            .skeleton(handle)
            .map_err(SkeletonDriveConstructionError::Pool)?
            .drive_owner()
            .is_some()
        {
            return Err(SkeletonDriveConstructionError::Owner(
                SkeletonDriveOwnerError::AlreadyCreated,
            ));
        }
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate
            .owner
            .try_initialize_drives(backend, manager, hooks, attributes, numeric, mass, cvars)
    }

    pub(crate) fn try_initialize_skeleton_animation_buffers(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        let index = self.skeleton_mutation_index(backend, handle)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate.owner.initialize_animation_buffers();
        Ok(())
    }

    pub(crate) fn try_finish_skeleton_init(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        let index = self.skeleton_mutation_index(backend, handle)?;
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        let construction = candidate
            .construction
            .as_ref()
            .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?;
        let physical = construction.owner();
        let constructor = &construction.constructor;
        if !constructor.ready_after_transfer()
            || candidate.owner.physics_history_initialization()
                != super::super::skeleton_normal::history_owner::SkeletonHistoryInitialization::Complete
        {
            return Err(SimulationSkaterPoolError::SkeletonConstructorIncomplete);
        }
        let bindings = constructor.bindings.as_ref().expect("ready bindings");
        if bindings
            .owner
            .upgrade()
            .is_none_or(|owner| !Arc::ptr_eq(&owner, physical))
            || bindings.island != Some(physical.manager())
        {
            return Err(SimulationSkaterPoolError::SkeletonConstructorIncomplete);
        }
        let reckoning = physical
            .reckoning()
            .lock()
            .map_err(|_| SimulationSkaterPoolError::ControllerPoisoned)?;
        if !Arc::ptr_eq(&bindings.processed, reckoning.processed()) {
            return Err(SimulationSkaterPoolError::SkeletonConstructorIncomplete);
        }
        drop(reckoning);
        let query = physical
            .actor_query
            .lock()
            .map_err(|_| SimulationSkaterPoolError::ControllerPoisoned)?;
        let same_query = match (&bindings.actor_query, &*query) {
            (None, None) => true,
            (Some(constructed), Some(actual)) => Arc::ptr_eq(constructed, actual),
            _ => false,
        };
        if !same_query {
            return Err(SimulationSkaterPoolError::SkeletonConstructorIncomplete);
        }
        drop(query);
        if physical
            .skeleton_constructor_complete
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(SimulationSkaterPoolError::AlreadyInitialized);
        }
        candidate
            .owner
            .try_finish_init()
            .map_err(|_| SimulationSkaterPoolError::SkeletonIkUnconstructed)?;
        physical
            .skeleton_constructor_complete
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn admit_world_pose_hooks_for_test<H>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        handle: &SimulationSkaterAssemblyHandle,
        pose: &[crate::animation::AnimationMatrix],
    ) -> bool {
        let index = self.skeleton_mutation_index(backend, handle).unwrap();
        hooks
            .check_authorities(self.manager, self.body_pool)
            .unwrap();
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("checked Skeleton kind")
        };
        candidate
            .owner
            .admit_world_pose_hooks_for_test(backend, hooks, pose)
    }

    pub(crate) fn try_set_skeleton_pose_with_hooks<H>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        let index = self.skeleton_mutation_index(backend, handle)?;
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(SimulationSkaterPoolError::Hook)?;
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate
            .owner
            .try_set_retained_pose_with_hooks(backend, hooks)
            .map_err(SimulationSkaterPoolError::SkeletonDrive)
    }

    pub(crate) fn try_release_skeleton_drives<H>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        let index = self.skeleton_mutation_index(backend, handle)?;
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(SimulationSkaterPoolError::Hook)?;
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate
            .owner
            .try_release_drives(backend, hooks)
            .map_err(SimulationSkaterPoolError::SkeletonDrive)
    }

    /// Initial history reads live bodies but does not change body transforms or
    /// collision publication. The actual retained member owns every write.
    pub(crate) fn try_initialize_physics_history<I: SkeletonHistoryInput>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        input: &mut I,
        numeric: &mut impl SkeletonPhysicsHistoryNumeric,
    ) -> Result<(), SkeletonHistoryManagerError<I::Error>> {
        let index = self
            .skeleton_mutation_index(backend, handle)
            .map_err(SkeletonHistoryManagerError::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate
            .owner
            .try_initialize_physics_history(backend, input, numeric)
            .map_err(SkeletonHistoryManagerError::Initialization)
    }

    /// Remove 0x82e0c1c8: detach first, then move last complete correlated owner.
    pub(crate) fn try_remove_skeleton(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<SimulationSkeletonAssemblyCandidate, SimulationSkaterPoolError> {
        let index = self.skeleton_mutation_index(backend, handle)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        candidate
            .owner
            .detach(backend)
            .map_err(SimulationSkaterPoolError::Detach)?;
        self.active_frame = None;
        match self.members.swap_remove(index).content {
            MemberContent::Skeleton { candidate, .. } => Ok(candidate),
            MemberContent::Board { .. } => unreachable!("kind checked"),
        }
    }
}

#[path = "../../player/wipeout/physical_player_post_wipeout_pose_pool.rs"]
pub(crate) mod pose_publication;
#[path = "../../player/wipeout/physical_player_post_wipeout_live_pool.rs"]
pub(crate) mod post_wipeout_live;
#[path = "../../player/skeleton/physical_player_skeleton_postphysics_live_pool.rs"]
pub(crate) mod postphysics_live;

#[path = "../../player/skeleton/physical_player_skeleton_fill_suffix_pool.rs"]
pub(crate) mod skeleton_fill_suffix;

#[path = "dynamic_assembly_skater_hook_offset.rs"]
pub(crate) mod hook_offset;

#[path = "dynamic_assembly_skater_pool_animated.rs"]
mod animated;

#[path = "dynamic_assembly_skater_pool_known_air.rs"]
pub(crate) mod known_air;

#[path = "dynamic_assembly_skater_pool_fixed_bone.rs"]
pub(crate) mod fixed_bone;

#[path = "dynamic_assembly_skater_pool_hand_plant.rs"]
pub(crate) mod hand_plant;
