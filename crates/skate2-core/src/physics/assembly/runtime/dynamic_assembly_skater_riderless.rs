//! PC Object Dropper riderless skateboard: a port addition, not original behavior.
//!
//! An actual Vault-constructed `SkateboardBody` (the same Init, Part/Joint
//! template and truck drives as a player's board) admitted to the dense
//! `SkaterPool` with no `PhysicalPlayer` owner, no hook Assembly and no Island
//! body membership. It is stepped, published and collided like every pool
//! member, but it is not one half of flags3's adjacent board/Skeleton pair.
//!
//! Its collision group is 7, the released-board group: under the flags3
//! (`Island` flags 3) culling table it meets ground 0, carried boards 4,
//! Skeletons 6, other released boards 7 and spawned DMOs 14..16.

use super::{
    AuthoredCollisionPublication, MemberContent, SimulationSkaterAssemblyCandidate,
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
};
use crate::{
    dynamic_assembly_session::{
        board_collision_groups::BoardCollisionGroupError,
        part_transform::DynamicAssemblyPartTransformError,
    },
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_spawn::{SkateboardSpawnServices, TypedSkateboardSpawnParts},
    skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::fmt;

/// The released-board collision group (see the module documentation).
pub(crate) const RIDERLESS_BOARD_GROUP: u32 = 7;
/// Board Parts: 0..=3 wheels, 4..=5 trucks, 6 deck.
pub(crate) const RIDERLESS_BOARD_PARTS: usize = 7;

#[derive(Debug)]
pub(crate) enum RiderlessBoardError {
    Pool(SimulationSkaterPoolError),
    NotRiderless,
    Groups(BoardCollisionGroupError),
    Transform {
        part: usize,
        source: DynamicAssemblyPartTransformError,
    },
    PartCount(usize),
    NonFinite,
    Construction(String),
}
impl fmt::Display for RiderlessBoardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "riderless board pool: {error}"),
            Self::NotRiderless => f.write_str("skater pool member is not a riderless board"),
            Self::Groups(error) => write!(f, "riderless board: {error}"),
            Self::Transform { part, source } => {
                write!(f, "riderless board Part{part} transform: {source:?}")
            }
            Self::PartCount(count) => write!(f, "riderless board has {count} Parts, not 7"),
            Self::NonFinite => f.write_str("riderless board transform is not finite"),
            Self::Construction(error) => write!(f, "riderless board construction: {error}"),
        }
    }
}
impl std::error::Error for RiderlessBoardError {}

impl SimulationSkaterAssemblyCandidate {
    /// Before Insert: the released-board group, then the detached Part cache
    /// moved rigidly so the deck sits at `requested` (DMO-style row-major:
    /// axis rows, then the translation row).
    pub(crate) fn try_prepare_riderless(
        &mut self,
        requested: [f32; 16],
    ) -> Result<(), RiderlessBoardError> {
        if requested.iter().any(|value| !value.is_finite()) {
            return Err(RiderlessBoardError::NonFinite);
        }
        let count = self.session.part_transforms().len();
        if count != RIDERLESS_BOARD_PARTS {
            return Err(RiderlessBoardError::PartCount(count));
        }
        let mut current = [[0.0; 16]; RIDERLESS_BOARD_PARTS];
        for (part, matrix) in self.session.part_transforms().enumerate() {
            current[part] = *matrix.ok_or(RiderlessBoardError::Transform {
                part,
                source: DynamicAssemblyPartTransformError::UnknownCacheLanes { index: part },
            })?;
        }
        let targets = riderless_part_targets(current, requested);
        for (part, matrix) in targets.into_iter().enumerate() {
            self.session
                .try_set_detached_part_transform(part, matrix)
                .map_err(|source| RiderlessBoardError::Transform { part, source })?;
        }
        self.session
            .board_collision_groups
            .try_set_group(RIDERLESS_BOARD_GROUP)
            .map_err(RiderlessBoardError::Groups)?;
        self.riderless = true;
        Ok(())
    }
}

impl SimulationSkaterAssemblyPool {
    /// False for Skeletons and player boards; an unknown handle is an error.
    pub(crate) fn is_riderless(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<bool, SimulationSkaterPoolError> {
        Ok(match &self.members[self.member_index(handle)?].content {
            MemberContent::Board { candidate, .. } => candidate.riderless,
            MemberContent::Skeleton { .. } => false,
        })
    }

    pub(crate) fn riderless_count(&self) -> usize {
        self.members
            .iter()
            .filter(|member| {
                matches!(&member.content, MemberContent::Board { candidate, .. } if candidate.riderless)
            })
            .count()
    }

    /// The dense publications in pool order, each with its riderless flag.
    pub(crate) fn publications_with_riderless(
        &self,
    ) -> impl ExactSizeIterator<Item = (AuthoredCollisionPublication<'_>, bool)> {
        self.members.iter().map(|member| {
            let riderless = matches!(&member.content,
                MemberContent::Board { candidate, .. } if candidate.riderless);
            (member.content.publication(), riderless)
        })
    }

    /// Skate 3 mode's moving solids from this pool: every member when other
    /// skaters collide, and riderless boards always (they are props, not skaters).
    pub(crate) fn skate3_publications(
        &self,
        collide_other_skaters: bool,
    ) -> impl Iterator<Item = AuthoredCollisionPublication<'_>> {
        self.publications_with_riderless()
            .filter(move |(_, riderless)| collide_other_skaters || *riderless)
            .map(|(publication, _)| publication)
    }

    fn riderless_board_mut(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&mut SimulationSkaterAssemblyCandidate, RiderlessBoardError> {
        let index = self.member_index(handle).map_err(RiderlessBoardError::Pool)?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(RiderlessBoardError::Pool)?;
        if !candidate.riderless {
            return Err(RiderlessBoardError::NotRiderless);
        }
        Ok(candidate)
    }

    /// Every Part `factor` times lighter (inverse mass and tensor). Call
    /// `try_place_riderless` after: bodies refresh their cached mass there.
    pub(crate) fn try_scale_riderless_lightness(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        factor: f32,
    ) -> Result<(), RiderlessBoardError> {
        let candidate = self.riderless_board_mut(handle)?;
        for part in 0..RIDERLESS_BOARD_PARTS {
            let body = candidate
                .session
                .try_part_contact_spy_observation(backend, part)
                .map_err(|e| RiderlessBoardError::Construction(format!("{e:?}")))?;
            backend
                .try_scale_part_lightness(body, factor)
                .map_err(|e| RiderlessBoardError::Construction(format!("{e:?}")))?;
        }
        Ok(())
    }

    /// Wakes every riderless board that is only PARTLY frozen. On flat ground
    /// the deck freezes alone while the wheels stay active; contact thaw only
    /// fires for an assembly whose every Part is frozen, so the Skate 2 skater
    /// could never wake it and the board sat stuck to the ground.
    pub(crate) fn try_wake_partly_frozen_riderless(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
    ) -> Result<(), SimulationSkaterPoolError> {
        for member in &mut self.members {
            let MemberContent::Board { candidate, .. } = &mut member.content else { continue };
            if !candidate.riderless {
                continue;
            }
            let wake = |e| SimulationSkaterPoolError::RiderlessWake(format!("{e:?}"));
            let (any, all) = candidate.session.try_part_frozen_counts(backend).map_err(wake)?;
            if any && !all {
                candidate.session.try_activate(backend).map_err(wake)?;
            }
        }
        Ok(())
    }

    /// Holds every riderless board's trucks straight, each tick: the bail's
    /// zero force-turn, then the player's per-tick `SetTruckDriveDynamics` and
    /// `SetTruckDriveFrames` (Body post suffix), which a riderless board never
    /// ran, so its trucks kept the soft fresh drives and tilted under a rider.
    /// The forced drive's spring and strength are then `stiffness` times firmer.
    pub(crate) fn try_hold_riderless_trucks(
        &mut self,
        backend: &SkateboardSimulationBackend,
        stiffness: f32,
    ) -> Result<(), SimulationSkaterPoolError> {
        let handles: Vec<SimulationSkaterAssemblyHandle> = self
            .members
            .iter()
            .filter(|member| matches!(&member.content,
                MemberContent::Board { candidate, .. } if candidate.riderless))
            .map(|member| member.handle.clone())
            .collect();
        let fail = |e: &dyn fmt::Debug| SimulationSkaterPoolError::RiderlessWake(format!("trucks: {e:?}"));
        for handle in handles {
            self.try_with_constructed_truck_drives(backend, &handle, |drives, attributes| {
                drives.set_wipeout_force_turn_angle(0.0);
                drives.try_update_dynamics(attributes).map_err(|e| fail(&e))?;
                for dynamics in drives.dynamics() {
                    let mut dynamics = dynamics.write().map_err(|_| fail(&"dynamics poisoned"))?;
                    dynamics.angular.spring *= stiffness;
                    dynamics.angular.strength *= stiffness;
                }
                drives
                    .try_update_frames(&mut crate::skateboard_truck_drives::HostSkateboardTruckDriveNumeric)
                    .map_err(|e| fail(&e))
            })
            .map_err(|e| fail(&e))??;
        }
        Ok(())
    }

    /// Test probe: every Part's linear velocity set to `velocity`.
    #[cfg(test)]
    pub(crate) fn try_shove_riderless(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        velocity: [f32; 3],
    ) -> Result<(), RiderlessBoardError> {
        self.invalidate_refresh();
        let candidate = self.riderless_board_mut(handle)?;
        // Physics.Body.SetVelocity activates and clears cooldown first.
        candidate
            .session
            .try_activate(backend)
            .map_err(|e| RiderlessBoardError::Construction(format!("{e:?}")))?;
        for part in 0..RIDERLESS_BOARD_PARTS {
            let body = candidate
                .session
                .try_part_contact_spy_observation(backend, part)
                .map_err(|e| RiderlessBoardError::Construction(format!("{e:?}")))?;
            backend
                .try_set_part_linear_velocity(body, velocity)
                .map_err(|e| RiderlessBoardError::Construction(format!("{e:?}")))?;
        }
        Ok(())
    }

    /// Test probe: each board Part's solver inverse mass.
    #[cfg(test)]
    pub(crate) fn board_inverse_masses(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Vec<f32> {
        let Ok(index) = self.member_index(handle) else { return Vec::new() };
        let Ok((candidate, _)) = self.members[index].content.board() else { return Vec::new() };
        (0..candidate.session.part_transforms().len())
            .filter_map(|part| candidate.session.try_part_contact_spy_observation(backend, part).ok())
            .filter_map(|body| backend.try_capture_part_inertia(body).ok()?.try_read().ok())
            .map(|inertia| inertia.inverse_mass)
            .collect()
    }

    /// Test probe: the deck body's raw activation fields.
    #[cfg(test)]
    pub(crate) fn probe_riderless_deck(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> String {
        let Ok(index) = self.member_index(handle) else { return "no member".into() };
        let Ok((candidate, _)) = self.members[index].content.board() else { return "not board".into() };
        match candidate.session.try_part_contact_spy_observation(backend, 6) {
            Ok(body) => backend.rigid_body_pool_for_test().probe_body(body),
            Err(e) => format!("{e:?}"),
        }
    }

    /// Test probe: the deck's angular velocity set to `omega` (rad/s, world).
    #[cfg(test)]
    pub(crate) fn try_spin_riderless_deck(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        omega: [f32; 3],
    ) -> Result<(), RiderlessBoardError> {
        self.invalidate_refresh();
        let candidate = self.riderless_board_mut(handle)?;
        candidate
            .session
            .try_activate(backend)
            .map_err(|e| RiderlessBoardError::Construction(format!("{e:?}")))?;
        let body = candidate
            .session
            .try_part_contact_spy_observation(backend, 6)
            .map_err(|e| RiderlessBoardError::Construction(format!("{e:?}")))?;
        backend
            .try_set_part_angular_velocity(body, omega)
            .map_err(|e| RiderlessBoardError::Construction(format!("{e:?}")))
    }

    /// Test probe: a board member's volume enable bits.
    #[cfg(test)]
    pub(crate) fn board_volume_enables(&self, handle: &SimulationSkaterAssemblyHandle) -> String {
        let Ok(index) = self.member_index(handle) else { return "no member".into() };
        let Ok((candidate, _)) = self.members[index].content.board() else { return "not board".into() };
        candidate.construction.as_ref().map_or("no construction".into(), |c| c.volume_enables_for_test())
    }

    /// True when every Part of the board is frozen (asleep).
    pub(crate) fn try_riderless_frozen(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<bool, RiderlessBoardError> {
        self.check_backend(backend).map_err(RiderlessBoardError::Pool)?;
        let member = &self.members[self.member_index(handle).map_err(RiderlessBoardError::Pool)?];
        member.content.all_frozen(backend).map_err(RiderlessBoardError::Pool)
    }

    /// Live world Part matrices, `[0..=3]` wheels, `[4..=5]` trucks, `[6]` deck.
    pub(crate) fn try_riderless_part_transforms(
        &self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<[[f32; 16]; RIDERLESS_BOARD_PARTS], RiderlessBoardError> {
        self.check_backend(backend).map_err(RiderlessBoardError::Pool)?;
        let (candidate, _) = self.members[self.member_index(handle).map_err(RiderlessBoardError::Pool)?]
            .content
            .board()
            .map_err(RiderlessBoardError::Pool)?;
        if !candidate.riderless {
            return Err(RiderlessBoardError::NotRiderless);
        }
        let mut parts = [[0.0; 16]; RIDERLESS_BOARD_PARTS];
        for (part, matrix) in parts.iter_mut().enumerate() {
            *matrix = candidate
                .session
                .try_live_part_transform_host(backend, part)
                .map_err(|source| RiderlessBoardError::Transform { part, source })?;
        }
        Ok(parts)
    }

    /// Teleport: rigid move with the deck at `requested`, then every Part's
    /// motion reset (zero velocity) and the truck drives' turn fields, the
    /// same per-Part reset Skateboard::TeleportReset's motion prefix runs.
    pub(crate) fn try_place_riderless(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        requested: [f32; 16],
    ) -> Result<(), RiderlessBoardError> {
        if requested.iter().any(|value| !value.is_finite()) {
            return Err(RiderlessBoardError::NonFinite);
        }
        let current = self.try_riderless_part_transforms(backend, handle)?;
        let targets = riderless_part_targets(current, requested);
        // A failure can follow partial body writes: invalidate first.
        self.invalidate_refresh();
        let candidate = self.riderless_board_mut(handle)?;
        for (part, matrix) in targets.into_iter().enumerate() {
            candidate
                .session
                .try_set_part_transform_host(backend, part, matrix)
                .map_err(|source| RiderlessBoardError::Transform { part, source })?;
        }
        for part in 0..RIDERLESS_BOARD_PARTS {
            candidate
                .session
                .try_reset_part_motion_required_host(backend, part)
                .map_err(|source| RiderlessBoardError::Transform { part, source })?;
        }
        if let Some(drives) = candidate.truck_drives.as_mut() {
            drives.reset_teleport_turn_fields();
        }
        Ok(())
    }
}

/// `SkateboardBody::SetTransform`'s rigid relocation (deck = requested,
/// others = requested * inverse(old deck) * Part) without a hook Part.
fn riderless_part_targets(
    current: [[f32; 16]; RIDERLESS_BOARD_PARTS],
    requested: [f32; 16],
) -> [[f32; 16]; RIDERLESS_BOARD_PARTS] {
    struct Parts([[f32; 16]; RIDERLESS_BOARD_PARTS]);
    impl TypedSkateboardSpawnParts for Parts {
        type Error = std::convert::Infallible;
        fn live_board_part(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error> {
            Ok(SkeletonRootMatrix::known(self.0[part]))
        }
        fn set_board_part(
            &mut self,
            part: usize,
            matrix: SkeletonRootMatrix,
        ) -> Result<(), Self::Error> {
            self.0[part] = matrix.known_lanes().expect("known riderless matrices");
            Ok(())
        }
        fn set_hook_part(&mut self, _: SkeletonRootMatrix) -> Result<(), Self::Error> {
            Ok(())
        }
    }
    /// The dropper's request is not a source spawn: no orthogonality assertion.
    struct NoAssertion;
    impl SkateboardSpawnServices for NoAssertion {
        fn current_orthogonal_tolerance(&mut self) -> f32 {
            f32::MAX
        }
        fn orthogonality_assertion_failed(&mut self) {}
    }
    let mut parts = Parts(current);
    let Ok(()) = crate::skateboard_spawn::try_set_skateboard_transform_typed(
        &mut parts,
        SkeletonRootMatrix::known(requested),
        &mut NoAssertion,
        &mut HostSkateboardSpawnMathNumeric,
    ) else {
        unreachable!("infallible Parts and services")
    };
    parts.0
}
