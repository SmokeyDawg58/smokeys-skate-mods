//! Actual same-pair bail prerequisites; full entry ordering belongs to the caller.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError,
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkeletonJumpPairError,
    dynamic_assembly_session::skeleton_normal::velocity::SkeletonVelocityError,
    reckoning_numeric::ReckoningNumeric, skateboard_assembly::SkateboardSimulationBackend,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::convert::Infallible;

#[derive(Debug)]
pub(crate) enum WipeoutSkeletonError<N> {
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    MissingIk,
    GlobalAttribute(crate::attribute_runtime::AttributeRuntimeError),
    Torque(crate::dynamic_assembly_session::skeleton_normal::wipeout_torque::WipeoutTorqueError<N>),
    Velocity(SkeletonVelocityError<N>),
    Drag(SkeletonVelocityError<Infallible>),
    Displacement(crate::dynamic_assembly_session::part_transform::DynamicAssemblyPartDisplacementError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for WipeoutSkeletonError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(e) => write!(f, "wipeout pair: {e}"),
            Self::Pool(e) => write!(f, "wipeout pool: {e}"),
            Self::Torque(e) => write!(f, "wipeout torque: {e:?}"),
            Self::GlobalAttribute(e) => write!(f, "wipeout global attribute: {e}"),
            Self::MissingIk => f.write_str("wipeout IK unavailable"),
            Self::Velocity(e) => write!(f, "wipeout velocity: {e}"),
            Self::Drag(e) => write!(f, "wipeout drag: {e}"),
            Self::Displacement(e)=>write!(f,"wipeout displacement: {e:?}"),
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) enum WipeoutSkeletonOperation {
    ClearTruckTilt,
    DisableIk,
    ScaleVelocity {
        direction: SkeletonRootVector,
        lower: f32,
        upper: f32,
    },
    ClampVelocity(f32),
    LinearDrag(f32),
    PushRagdoll([f32;4]),
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_wipeout_skeleton_operation<N: ReckoningNumeric>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        operation: WipeoutSkeletonOperation,
        numeric: &mut N,
    ) -> Result<(), WipeoutSkeletonError<N::Error>> {
        use WipeoutSkeletonError as E;
        self.try_with_jump_pair(backend, board, skeleton, |_, _| ())
            .map_err(E::Pair)?;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("validated Skeleton")
        };
        match operation {
            //Skeleton.PushRagdoll82b59868 uses PART_SPINE2 (PDB index12).
            WipeoutSkeletonOperation::PushRagdoll(displacement)=>candidate.owner.session()
                .try_apply_part_displacement(backend,12,displacement,numeric).map_err(E::Displacement),
            WipeoutSkeletonOperation::ClearTruckTilt => {
                candidate.owner.clear_wipeout_truck_tilt();
                Ok(())
            }
            WipeoutSkeletonOperation::DisableIk => candidate
                .owner
                .try_disable_wipeout_ik()
                .map_err(|_| E::MissingIk),
            WipeoutSkeletonOperation::ScaleVelocity {
                direction,
                lower,
                upper,
            } => candidate
                .owner
                .try_scale_velocity_along_vector(backend, direction, lower, upper, numeric)
                .map_err(E::Velocity),
            WipeoutSkeletonOperation::ClampVelocity(maximum) => candidate
                .owner
                .try_clamp_velocity_magnitude(backend, maximum, numeric)
                .map_err(E::Velocity),
            WipeoutSkeletonOperation::LinearDrag(value) => candidate
                .owner
                .try_set_wipeout_linear_drag(backend, value)
                .map_err(E::Drag),
        }
    }
}

impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_apply_pair_wipeout_torque<N: ReckoningNumeric>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        torque: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), WipeoutSkeletonError<N::Error>> {
        use WipeoutSkeletonError as E;
        self.try_with_jump_pair(backend, board, skeleton, |_, _| ())
            .map_err(E::Pair)?;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("validated Skeleton")
        };
        candidate
            .owner
            .try_apply_wipeout_torque(backend, torque, numeric)
            .map_err(E::Torque)
    }
}

impl SimulationSkaterAssemblyPool {
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "same retained pair and numerical services"
    )]
    pub(crate) fn try_update_paired_skeleton_wipeout<N,S,H,C>(&mut self,backend:&mut SkateboardSimulationBackend,hooks:&mut crate::dynamic_assembly_session::hook_pool::SimulationHookAssemblyPool<skate2_formats::collision::AssemblyPartMaterial,H>,board:&SimulationSkaterAssemblyHandle,skeleton:&SimulationSkaterAssemblyHandle,expected:&std::sync::Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,processed:&std::sync::Arc<std::sync::Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>>,weights:crate::skeleton_drives::ragdoll_update::RagdollDriveWeights,refs:&crate::input_cvar_runtime::InputCvarReferences,numeric:&mut N,rounding:&mut S,cvars:&mut C)->Result<(),crate::dynamic_assembly_session::skeleton_normal::general_owner::wipeout::WipeoutSkeletonUpdateError<N::Error,S::Error,<C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error>>
    where N:ReckoningNumeric,S:crate::reckoning_sin_cos::ReckoningSinCosNumeric,C:crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralCvars+crate::skeleton_ik::SkeletonIkSuffixServices+crate::skeleton_ik_state::contact::SkeletonIkContactServices<Error=<C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error>{
        use crate::dynamic_assembly_session::skeleton_normal::general_owner::wipeout::WipeoutSkeletonUpdateError as E;
        let actual = self
            .try_with_jump_pair(backend, board, skeleton, |_, owner| {
                std::sync::Arc::clone(owner)
            })
            .map_err(E::Pair)?;
        if !std::sync::Arc::ptr_eq(&actual, expected) {
            return Err(E::ForeignOwner);
        }
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(|e| E::Pool(SimulationSkaterPoolError::Hook(e)))?;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("checked Skeleton")
        };
        let bindings = candidate
            .construction
            .as_ref()
            .and_then(|v| v.constructor.bindings.as_ref())
            .ok_or(E::Missing("Skeleton bindings"))?;
        if !std::sync::Arc::ptr_eq(&bindings.processed, processed)
            || !bindings
                .owner
                .upgrade()
                .is_some_and(|v| std::sync::Arc::ptr_eq(&v, expected))
        {
            return Err(E::ForeignOwner);
        }
        let mut guard = processed.lock().map_err(|_| E::Poisoned)?;
        let input =
            crate::skeleton_processed_contact::grounded_runtime::GroundedProcessed::new(&mut guard);
        candidate.owner.try_update_wipeout_complete(
            backend, hooks, &input, weights, refs, numeric, rounding, cvars,
        )
    }
}

impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_pair_global_wipeout_float(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        key: u64,
    ) -> Result<f32, WipeoutSkeletonError<Infallible>> {
        use crate::skeleton_body_attributes::{
            SkeletonBodyAttributeRead, SkeletonBodyAttributeRoute,
        };
        use WipeoutSkeletonError as E;
        self.try_with_jump_pair(backend, board, skeleton, |_, _| ())
            .map_err(E::Pair)?;
        let index = self.member_index(skeleton).map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("validated Skeleton")
        };
        let reader = candidate
            .construction
            .as_mut()
            .and_then(|c| c.body_reader_mut())
            .ok_or(E::Pool(
                SimulationSkaterPoolError::MissingPhysicalConstruction,
            ))?;
        reader
            .float(SkeletonBodyAttributeRoute::GlobalWipeout, key)
            .map_err(E::GlobalAttribute)
    }
}
