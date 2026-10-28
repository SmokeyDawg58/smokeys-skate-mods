//! Moves the real pre-admission board constructor into existing `SkaterPool`
//! membership. No new body manager or Entity identity is introduced.

use super::{
    SimulationSkaterAssemblyCandidate, SimulationSkaterAssemblyHandle,
    SimulationSkaterAssemblyPool, SimulationSkaterPoolError, SkateboardAssemblyDriveAdmission,
    SkateboardAttributeRead, SkateboardSimulationBackend, SkateboardTruckDriveError,
    SkateboardTruckDrives, TruckAdmission,
};
use crate::{
    skateboard_attributes::SkateboardAttributeError,
    skateboard_body_construction::{SkateboardBodyConstruction, SkateboardBodyConstructionError},
    skateboard_initial_transforms::{
        HostSkateboardInitialTransformNumeric, SkateboardInitialTransformError,
        try_initialize_skateboard_transforms_with_writer,
    },
    skateboard_joints::{
        HostSkateboardJointNumeric, SkateboardJointBuildError, SkateboardTruckTransforms,
        try_calc_truck_transforms,
    },
    skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
};
use std::{convert::Infallible, fmt};

pub(crate) struct SimulationConstructedBoardCandidateFailure {
    pub(crate) error: SkateboardBodyConstructionError,
    pub(crate) construction: Box<SkateboardBodyConstruction>,
}
impl fmt::Debug for SimulationConstructedBoardCandidateFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationConstructedBoardCandidateFailure")
            .field("error", &self.error)
            .field("phase", &self.construction.phase())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) enum SimulationConstructedBoardError {
    Pool(SimulationSkaterPoolError),
    MissingConstruction,
    TruckDriveHandoff,
    Joint(SkateboardJointBuildError<SkateboardAttributeError, Infallible>),
    Drive(
        SkateboardTruckDriveError<SkateboardAttributeError, Infallible, SimulationSkaterPoolError>,
    ),
    Attribute(SkateboardAttributeError),
    TeleportInitialTransforms(
        SkateboardInitialTransformError<
            SkateboardAttributeError,
            Infallible,
            crate::dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError,
        >,
    ),
    TeleportPartMotion {
        part: usize,
        source: crate::dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError,
    },
    MissingForceRuntime,
    ContactSurfacePoisoned,
    MissingPostFields,
    WheelLinesPoisoned,
    MissingHookDynamics,
    HookDynamicsPoisoned,
}

#[derive(Debug)]
pub(crate) enum SimulationConstructedSteeringError<I> {
    Owner(SimulationConstructedBoardError),
    Steering(
        crate::skateboard_steering::SkateboardSteeringError<
            crate::attribute_runtime::AttributeRuntimeError,
            I,
        >,
    ),
}

#[derive(Debug)]
pub(crate) enum SimulationConstructedMaterialError {
    Owner(SimulationConstructedBoardError),
    Part(crate::dynamic_assembly_session::DynamicAssemblyPartMaterialError),
}
impl fmt::Display for SimulationConstructedMaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "material board: {e}"),
            Self::Part(e) => write!(f, "{e}"),
        }
    }
}
impl<I: fmt::Debug> fmt::Display for SimulationConstructedSteeringError<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "steering board: {e}"),
            Self::Steering(e) => write!(f, "{e}"),
        }
    }
}
impl fmt::Display for SimulationConstructedBoardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "constructed board pool: {e}"),
            Self::MissingConstruction => f.write_str("board has no retained source constructor"),
            Self::TruckDriveHandoff => f.write_str(
                "truck-drive handoff requires one completed source owner and an empty destination",
            ),
            Self::Joint(e) => write!(f, "constructed board truck transforms: {e}"),
            Self::Drive(e) => write!(f, "constructed board drives: {e}"),
            Self::Attribute(e) => write!(f, "constructed board attribute: {e}"),
            Self::TeleportInitialTransforms(e) => {
                write!(f, "teleport board initial transforms: {e}")
            }
            Self::TeleportPartMotion { part, source } => {
                write!(f, "teleport board Part {part} motion: {source}")
            }
            Self::MissingForceRuntime => f.write_str("board has no retained force runtime"),
            Self::ContactSurfacePoisoned => f.write_str("board contact surface lock poisoned"),
            Self::MissingPostFields => f.write_str("board constructor has no retained post fields"),
            Self::WheelLinesPoisoned => f.write_str("board wheel-line output lock poisoned"),
            Self::MissingHookDynamics => f.write_str("board has no retained hook dynamics"),
            Self::HookDynamicsPoisoned => f.write_str("board hook dynamics lock poisoned"),
        }
    }
}

impl SimulationSkaterAssemblyCandidate {
    /// Explicit host recovery of a returned unregistered admission candidate.
    /// Publication failure may leave it attached. No manager ID is refunded.
    pub(crate) fn try_detach_failed_admission(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
    ) -> Result<(), SimulationSkaterPoolError> {
        if self.session.is_attached() {
            self.session
                .detach(backend)
                .map_err(SimulationSkaterPoolError::Detach)?;
        }
        Ok(())
    }

    /// The actual completed session and its continuing Vault/cache owner enter
    /// the SAME candidate. A failed or already-handed-off construction is
    /// returned intact; no unrelated prepared session may be substituted.
    pub(crate) fn try_from_construction(
        mut construction: Box<SkateboardBodyConstruction>,
    ) -> Result<Self, SimulationConstructedBoardCandidateFailure> {
        let session = match construction.take_session() {
            Ok(session) => session,
            Err(error) => {
                return Err(SimulationConstructedBoardCandidateFailure {
                    error,
                    construction,
                });
            }
        };
        Ok(Self {
            session,
            world_index: 0xffff,
            requested_state: None,
            construction: Some(construction),
            force_runtime: None,
            truck_drives: None,
            truck_drive_occurrence: None,
            hook: None,
            riderless: false,
        })
    }

    pub(crate) fn construction(&self) -> Option<&SkateboardBodyConstruction> {
        self.construction.as_deref()
    }
}

impl SimulationSkaterAssemblyPool {
    /// CollisionInfo::Reset 0x82b2369c..236cc prefix. The subsequent
    /// seven-Part loop also touches wheel/output owners and remains separate.
    pub(crate) fn try_teleport_reset_collision_prefix(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let surface = std::sync::Arc::clone(
            &candidate
                .force_runtime
                .as_ref()
                .ok_or(E::MissingForceRuntime)?
                .contact_surface,
        );
        let mut surface = surface.lock().map_err(|_| E::ContactSurfacePoisoned)?;
        surface.collision_state.begin_in_place_reset();
        surface.post_prefix.max_velocity = Some(
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
        ); // Body+250, 2369c
        surface.collision_state.average_normal = Some(crate::skateboard_contact_surface::state::UP); // Body+40, 236a4
        surface.collision_state.last_valid_normal =
            Some(crate::skateboard_contact_surface::state::UP); // Body+50, 236ac
        let fields = candidate
            .construction
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .post_fields_mut()
            .ok_or(E::MissingPostFields)?;
        fields.num_parts = 0; // Body+360, 236b0
        surface.post_prefix.max_speed = Some(0.0); // Body+354, 236b8
        fields.num_wheels = 0; // Body+361, 236bc
        surface.collision_state.flags.and_mask(0x17ff_ffff); // Body+364, 236cc
        Ok(())
    }

    /// CollisionInfo::Reset 0x82b236d4..23728, reusing the existing
    /// per-Part contact projection and constructor-owned wheel-height output.
    pub(crate) fn try_teleport_reset_collision_part_loop(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let surface = std::sync::Arc::clone(
            &candidate
                .force_runtime
                .as_ref()
                .ok_or(E::MissingForceRuntime)?
                .contact_surface,
        );
        let wheel_lines = std::sync::Arc::clone(
            candidate
                .construction
                .as_ref()
                .ok_or(E::MissingConstruction)?
                .wheel_lines(),
        );
        let mut surface = surface.lock().map_err(|_| E::ContactSurfacePoisoned)?;
        let zero = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
        for part in 0..7 {
            // Native vperm swizzles prior maxContactVel with zero every trip;
            // the prefix established a full zero and no nonzero source enters.
            surface.post_prefix.max_velocity = Some(zero); // 236ec
            surface.reset_collision_part_before_height(part); // 236f0..23708
            wheel_lines
                .lock()
                .map_err(|_| E::WheelLinesPoisoned)?
                .height = Some(0.0); // 23710
            surface.collision_state.reset_part_after_height(part); // 23714..23720
        }
        Ok(())
    }

    /// CollisionInfo::Reset 0x82b2372c..23760 resets each wheel's line
    /// normal, distance, then validity byte, completing this in-place Reset.
    pub(crate) fn try_teleport_reset_collision_wheel_loop(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let surface = std::sync::Arc::clone(
            &candidate
                .force_runtime
                .as_ref()
                .ok_or(E::MissingForceRuntime)?
                .contact_surface,
        );
        let wheel_lines = std::sync::Arc::clone(
            candidate
                .construction
                .as_ref()
                .ok_or(E::MissingConstruction)?
                .wheel_lines(),
        );
        let mut surface = surface.lock().map_err(|_| E::ContactSurfacePoisoned)?;
        for wheel in 0..4 {
            wheel_lines
                .lock()
                .map_err(|_| E::WheelLinesPoisoned)?
                .reset_collision_wheel_output(wheel); // 23740/48
            surface.collision_state.surfaces.set_validity(wheel, 0); // 2374c
        }
        surface.collision_state.complete_in_place_reset();
        Ok(())
    }

    /// Final in-place Body::TeleportReset stores after CollisionInfo::Reset.
    pub(crate) fn try_teleport_reset_board_grip(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        self.members[index]
            .content
            .board_mut()
            .map_err(E::Pool)?
            .construction
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .post_fields_mut()
            .ok_or(E::MissingPostFields)?
            .reset_teleport_grip();
        Ok(())
    }

    /// Outer `Skateboard::TeleportReset` 0x82b3613c..1c4 on the same board.
    /// Its repeated turn stores and drive dynamics differ in order from the
    /// earlier Body reset and DisableBoardAnimation; neither is reused here.
    pub(crate) fn try_teleport_reset_outer_board(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let runtime = candidate
            .force_runtime
            .as_mut()
            .ok_or(E::MissingForceRuntime)?;
        runtime.steering.reset_teleport_deck_tilt(); // 3613c
        candidate
            .truck_drives
            .as_mut()
            .ok_or(E::TruckDriveHandoff)?
            .reset_teleport_outer_turn_fields(); // 36170..78
        runtime.steering.reset_teleport_physics_error(); // 36180
        runtime.entry.reset_teleport_animated(); // 36184
        let dynamics = runtime
            .hook_dynamics
            .as_ref()
            .ok_or(E::MissingHookDynamics)?;
        {
            let mut dynamics = dynamics.write().map_err(|_| E::HookDynamicsPoisoned)?;
            dynamics.linear.spring = 0.0; // 3618c
            dynamics.linear.damping = 0.0; // 36190
            dynamics.linear.raw_type = 2; // 36194
            dynamics.linear.strength = 0.0; // 36198
        }
        {
            let mut dynamics = dynamics.write().map_err(|_| E::HookDynamicsPoisoned)?;
            dynamics.angular.spring = 0.0; // 361a0
            dynamics.angular.raw_type = 2; // 361a4
            dynamics.angular.damping = 0.0; // 361a8
            dynamics.angular.strength = 0.0; // 361ac
        }
        runtime.queue.reset_teleport(); // 361b0..bc
        runtime.entry.reset_teleport_target_speed(); // 361c0
        runtime.set_jump_normal(crate::skeleton_root_transforms::SkeletonRootVector::known(
            [0.0, 1.0, 0.0, 0.0],
        )); // 361c4
        Ok(())
    }

    /// `SkateboardBody::TeleportReset` 0x82b2e808..824 first resets each
    /// attached Part's body motion in ascending order, then stores its three
    /// retained turn fields. CollisionInfo::Reset is the next separate call.
    pub(crate) fn try_teleport_reset_board_motion_prefix(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        self.members[index].content.board_mut().map_err(E::Pool)?;
        self.invalidate_refresh();
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let count = candidate.session.part_transforms().len();
        for part in 0..count {
            candidate
                .session
                .try_reset_part_motion_required_host(backend, part)
                .map_err(|source| E::TeleportPartMotion { part, source })?;
        }
        candidate
            .truck_drives
            .as_mut()
            .ok_or(E::TruckDriveHandoff)?
            .reset_teleport_turn_fields();
        Ok(())
    }

    /// Skateboard::TeleportReset calls the existing body's InitializeTransforms
    /// at 0x82b360ec. Reuse the original seven-write graph on this attached
    /// board; the constructor's detached writer would reject a live body.
    pub(crate) fn try_teleport_initialize_board_transforms(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        if candidate.construction.is_none() {
            return Err(E::MissingConstruction);
        }
        // The helper's first step mutates Part 6 before any attribute lookup.
        // Invalidate the pool snapshot before entering those partial writes.
        self.invalidate_refresh();
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let construction = candidate
            .construction
            .as_mut()
            .ok_or(E::MissingConstruction)?;
        try_initialize_skateboard_transforms_with_writer(
            construction.attributes_mut(),
            &mut HostSkateboardInitialTransformNumeric,
            |part, matrix| {
                candidate
                    .session
                    .try_set_part_transform_host(backend, part, matrix)
            },
        )
        .map_err(E::TeleportInitialTransforms)
    }
    pub(crate) fn try_board_wheel_lines(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        std::sync::Arc<
            std::sync::Mutex<crate::skateboard_body_construction::SkateboardRetainedWheelLines>,
        >,
        SimulationConstructedBoardError,
    > {
        use SimulationConstructedBoardError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let super::MemberContent::Board { candidate, .. } = &self.members[index].content else {
            return Err(E::Pool(SimulationSkaterPoolError::WrongMemberKind));
        };
        let construction = candidate
            .construction
            .as_ref()
            .ok_or(E::MissingConstruction)?;
        Ok(std::sync::Arc::clone(construction.wheel_lines()))
    }
    #[cfg(test)]
    pub(crate) fn with_board_materials_for_test<T>(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
        callback: impl FnOnce(
            &super::SkateboardAssemblySession,
            &crate::dynamic_assembly_session::skateboard_publication::SkateboardCollisionPublication,
            u64,
        ) -> T,
    ) -> T {
        let index = self.member_index(handle).unwrap();
        let super::MemberContent::Board {
            candidate,
            publication,
        } = &self.members[index].content
        else {
            panic!("actual board")
        };
        callback(
            &candidate.session,
            publication,
            candidate
                .session
                .part_attachment
                .material_revision
                .load(std::sync::atomic::Ordering::Acquire),
        )
    }
    pub(crate) fn try_set_constructed_part_material(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        part: usize,
        material: std::sync::Arc<skate2_formats::collision::AssemblyPartMaterial>,
    ) -> Result<(), SimulationConstructedMaterialError> {
        use SimulationConstructedMaterialError as E;
        self.check_backend(backend)
            .map_err(|e| E::Owner(SimulationConstructedBoardError::Pool(e)))?;
        let index = self
            .member_index(handle)
            .map_err(|e| E::Owner(SimulationConstructedBoardError::Pool(e)))?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(|e| E::Owner(SimulationConstructedBoardError::Pool(e)))?;
        if candidate.construction.is_none() {
            return Err(E::Owner(
                SimulationConstructedBoardError::MissingConstruction,
            ));
        }
        candidate
            .session
            .try_set_part_material(backend, part, material)
            .map_err(E::Part)
    }

    #[cfg(test)]
    pub(crate) fn check_actual_board_material_publication(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) {
        let index = self.member_index(handle).unwrap();
        let super::MemberContent::Board {
            candidate,
            publication,
        } = &mut self.members[index].content
        else {
            panic!("actual board")
        };
        crate::dynamic_assembly_session::skateboard_publication::check_material_replacement(
            &mut candidate.session,
            backend,
            publication,
        );
    }
    pub(crate) fn try_update_constructed_steering<
        I: crate::skateboard_steering::SkateboardSteeringInput,
    >(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        target: f32,
        input: &mut I,
    ) -> Result<(), SimulationConstructedSteeringError<I::Error>> {
        use SimulationConstructedSteeringError as E;
        self.check_backend(backend)
            .map_err(|e| E::Owner(SimulationConstructedBoardError::Pool(e)))?;
        let index = self
            .member_index(handle)
            .map_err(|e| E::Owner(SimulationConstructedBoardError::Pool(e)))?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(|e| E::Owner(SimulationConstructedBoardError::Pool(e)))?;
        let runtime = candidate.force_runtime.as_mut().ok_or(E::Owner(
            SimulationConstructedBoardError::MissingConstruction,
        ))?;
        let drives = candidate
            .truck_drives
            .as_mut()
            .ok_or(E::Owner(SimulationConstructedBoardError::TruckDriveHandoff))?;
        runtime
            .steering
            .try_update(
                target,
                &mut runtime.steering_attributes,
                input,
                |angle, front, back| drives.set_force_turn_angle(angle, front, back),
            )
            .map_err(E::Steering)
    }
    /// Accesses the retained truck fields and SAME continuing body Vault reader.
    /// Numerical mutation is an explicit caller stage, not implicit prephysics.
    pub(crate) fn try_with_constructed_truck_drives<T>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        operation: impl FnOnce(
            &mut SkateboardTruckDrives,
            &mut crate::skateboard_attributes::SkateboardAttributes,
        ) -> T,
    ) -> Result<T, SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as Error;
        self.check_backend(backend).map_err(Error::Pool)?;
        let index = self.member_index(handle).map_err(Error::Pool)?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(Error::Pool)?;
        let construction = candidate
            .construction
            .as_mut()
            .ok_or(Error::MissingConstruction)?;
        let drives = candidate
            .truck_drives
            .as_mut()
            .ok_or(Error::TruckDriveHandoff)?;
        Ok(operation(drives, construction.attributes_mut()))
    }
    /// Move the same initialized payload owner into its actual admitted board.
    /// Validation failures retain the factory's owner; no drives are rebuilt.
    pub(crate) fn try_retain_constructed_truck_drives(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        source: &mut Option<SkateboardTruckDrives>,
    ) -> Result<(), SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as Error;
        self.check_backend(backend).map_err(Error::Pool)?;
        let index = self.member_index(handle).map_err(Error::Pool)?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(Error::Pool)?;
        if candidate.construction.is_none() {
            return Err(Error::MissingConstruction);
        }
        if candidate.truck_drives.is_some()
            || !source.as_ref().is_some_and(|drives| {
                drives.phase()
                    == crate::skateboard_truck_drives::SkateboardTruckDrivePhase::Complete
            })
        {
            return Err(Error::TruckDriveHandoff);
        }
        let drives = source.as_ref().expect("completed source checked");
        if !candidate
            .truck_drive_occurrence
            .as_ref()
            .is_some_and(|identity| std::sync::Arc::ptr_eq(identity, drives.occurrence()))
        {
            return Err(Error::TruckDriveHandoff);
        }
        for (slot, child) in [4, 5].into_iter().enumerate() {
            let accepted = candidate
                .session
                .try_validate_post_attachment_drive(
                    backend,
                    slot,
                    6,
                    child,
                    &drives.frames()[slot],
                    &drives.dynamics()[slot],
                )
                .map_err(|error| Error::Pool(SimulationSkaterPoolError::Drive(error)))?;
            if drives.add_results()[slot] != Some(accepted) {
                return Err(Error::TruckDriveHandoff);
            }
        }
        candidate.truck_drives = source.take();
        Ok(())
    }
    pub(crate) fn board_construction(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<Option<&SkateboardBodyConstruction>, SimulationSkaterPoolError> {
        Ok(self.members[self.member_index(handle)?]
            .content
            .board()?
            .0
            .construction())
    }

    /// Source post-admission `CalcTruckTransforms` uses the SAME retained reader,
    /// not another Vault bind or an independently constructed body definition.
    pub(crate) fn try_calc_constructed_truck_transforms(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<SkateboardTruckTransforms, SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as Error;
        let index = self.member_index(handle).map_err(Error::Pool)?;
        let owner = self.members[index]
            .content
            .board_mut()
            .map_err(Error::Pool)?
            .construction
            .as_mut()
            .ok_or(Error::MissingConstruction)?;
        try_calc_truck_transforms(owner.attributes_mut(), &mut HostSkateboardJointNumeric)
            .map_err(Error::Joint)
    }

    pub(crate) fn try_initialize_constructed_truck_drives(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        drives: &mut SkateboardTruckDrives,
    ) -> Result<(), SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as Error;
        self.check_backend(backend).map_err(Error::Pool)?;
        let index = self.member_index(handle).map_err(Error::Pool)?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(Error::Pool)?;
        let construction = candidate
            .construction
            .as_mut()
            .ok_or(Error::MissingConstruction)?;
        match &candidate.truck_drive_occurrence {
            Some(identity) if !std::sync::Arc::ptr_eq(identity, drives.occurrence()) => {
                return Err(Error::TruckDriveHandoff);
            }
            Some(_) => {}
            None => {
                candidate.truck_drive_occurrence = Some(std::sync::Arc::clone(drives.occurrence()));
            }
        }
        drives
            .try_create_fresh(
                construction.attributes_mut(),
                &mut HostSkateboardTruckDriveNumeric,
                &mut TruckAdmission(SkateboardAssemblyDriveAdmission {
                    session: &mut candidate.session,
                    backend,
                }),
            )
            .map_err(Error::Drive)
    }

    /// Sampling only: the enclosing constructor must call this AFTER truck
    /// drives, then implement or explicitly refuse nonzero before creating Hook.
    pub(crate) fn try_constructed_optional_wheel_drive_byte(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<u8, SimulationConstructedBoardError> {
        use SimulationConstructedBoardError as Error;
        let index = self.member_index(handle).map_err(Error::Pool)?;
        let owner = self.members[index]
            .content
            .board_mut()
            .map_err(Error::Pool)?
            .construction
            .as_mut()
            .ok_or(Error::MissingConstruction)?;
        let attributes = owner.attributes_mut();
        let pointer = attributes
            .lookup(
                crate::skateboard_attributes::SkateboardAttributeClass::Wheels,
                0x69ce_f3ca_3f14_80b0,
            )
            .map_err(Error::Attribute)?;
        attributes.byte(&pointer).map_err(Error::Attribute)
    }
}

#[path = "../../player/ground/physical_player_ground_control_pool.rs"]
pub(crate) mod ground_control;

#[path = "../../player/ground/physical_player_ground_control_retained.rs"]
pub(crate) mod ground_control_retained;

#[path = "../../player/ground/physical_player_ground_force_board.rs"]
pub(crate) mod ground_force_board;
