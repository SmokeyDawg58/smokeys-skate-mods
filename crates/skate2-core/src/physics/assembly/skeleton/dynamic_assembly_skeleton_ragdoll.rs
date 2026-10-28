//! Actual retained Skeleton `SetUpRagdoll` 0x82b4ffd8..0x82b503fc.
use super::SkeletonNormalAssembly;
use crate::{
    attribute_runtime::AttributeRuntimeError,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    ragdoll::SKELETON_PART_COUNT,
    simulation_rigid_body_pool::{SimulationRigidBodyHandle, SimulationRigidBodyPoolError},
    skeleton_assembly::SkeletonPreNormalSimulationBackend,
    skeleton_body_attributes::{
        SkeletonBodyAttributeRead, SkeletonBodyAttributeRoute, SkeletonBodyAttributes,
    },
    skeleton_culling::SkeletonCullingResource,
    skeleton_joint_attributes::SkeletonJointAttributeProvider,
    skeleton_normal_joint_limits::SkeletonJointLimitBinding,
    skeleton_runtime_state::SkeletonPartMaterialSelection,
};
use std::sync::Arc;

#[path = "../../skeleton/movement/skeleton_ragdoll_joint_limits.rs"]
pub(crate) mod joint_limits;

pub(crate) struct RagdollInputs<'a> {
    pub(crate) bouncy: u8,
    pub(crate) culling: Arc<SkeletonCullingResource>,
    pub(crate) body_reader: &'a mut SkeletonBodyAttributes,
    pub(crate) joint_reader: &'a mut SkeletonJointAttributeProvider,
    pub(crate) cvars: &'a InputCvarReferences,
}

#[derive(Debug)]
pub(crate) enum RagdollSetupError<C> {
    Detached,
    ForeignBackend,
    BindingAllocation,
    Cvar(InputCvarError),
    Attribute(AttributeRuntimeError),
    Joint(joint_limits::RagdollJointLimitsError<C>),
    Body {
        part: usize,
        source: SimulationRigidBodyPoolError,
    },
}

impl<C: std::fmt::Debug> std::fmt::Display for RagdollSetupError<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Detached => f.write_str("ragdoll requires attached Skeleton"),
            Self::ForeignBackend => f.write_str("ragdoll backend mismatch"),
            Self::BindingAllocation => f.write_str("ragdoll Joint binding allocation failed"),
            Self::Cvar(e) => write!(f, "ragdoll cvar: {e}"),
            Self::Attribute(e) => write!(f, "ragdoll attribute: {e}"),
            Self::Joint(e) => write!(f, "ragdoll Joint limits: {e}"),
            Self::Body { part, source } => write!(f, "ragdoll Part {part}: {source}"),
        }
    }
}

impl SkeletonNormalAssembly {
    /// Fresh source reads and immediate actual owner stores; failures retain
    /// the completed prefix, including the current Part's material selection.
    #[expect(
        clippy::too_many_lines,
        reason = "one source-ordered actual Skeleton setup with partial stores"
    )]
    pub(crate) fn try_set_up_ragdoll<C>(
        &mut self,
        backend: &mut SkeletonPreNormalSimulationBackend,
        inputs: RagdollInputs<'_>,
        cosine: &mut impl FnMut(f32) -> Result<f32, C>,
    ) -> Result<(), RagdollSetupError<C>> {
        use RagdollSetupError as E;
        use SkeletonBodyAttributeRoute as R;
        match self.session.attached_authority {
            None => return Err(E::Detached),
            Some(authority) if authority != backend.authority() => return Err(E::ForeignBackend),
            Some(_) => {}
        }
        let binding = Arc::clone(self.session.template().collision());
        let body = binding.body();
        if inputs
            .cvars
            .try_use_ragdoll_joint_limits()
            .map_err(E::Cvar)?
        {
            let mut joints = Vec::new();
            joints
                .try_reserve_exact(self.session.joints.len())
                .map_err(|_| E::BindingAllocation)?;
            for (joint, limits) in self.session.joints.iter().zip(binding.joint_limits()) {
                joints.push(SkeletonJointLimitBinding {
                    joint: joint.joint.expect("attached session retains each Joint"),
                    limits: limits.clone(),
                });
            }
            backend
                .try_set_skeleton_ragdoll_limits(
                    &body.data,
                    &body.attributes,
                    inputs.joint_reader,
                    &joints,
                    cosine,
                )
                .map_err(E::Joint)?;
        }
        if inputs.bouncy != 0 || inputs.cvars.try_bouncy_ragdoll_cheat().map_err(E::Cvar)? {
            self.state.ragdoll_head_material.restitution = inputs
                .cvars
                .try_bouncy_ragdoll_restitution()
                .map_err(E::Cvar)?;
            self.state.ragdoll_material.restitution = inputs
                .cvars
                .try_bouncy_ragdoll_restitution()
                .map_err(E::Cvar)?;
        } else {
            self.state.ragdoll_material.restitution = inputs
                .body_reader
                .float(R::LocalSkeleton, 0x5e3e_f6cc_8922_8e59)
                .map_err(E::Attribute)?;
            self.state.ragdoll_head_material.restitution = inputs
                .body_reader
                .float(R::LocalSkeleton, 0xbac7_c1be_3809_0b31)
                .map_err(E::Attribute)?;
        }
        self.state.current_culling = inputs.culling;
        self.state.set_group_and_userdata(7);
        let angular = inputs
            .body_reader
            .float(R::GlobalWipeout, 0xdc15_94a6_5b48_6273)
            .map_err(E::Attribute)?;
        let linear = inputs
            .body_reader
            .float(R::GlobalWipeout, 0x8a35_2071_b894_3245)
            .map_err(E::Attribute)?;
        let handles: [SimulationRigidBodyHandle; SKELETON_PART_COUNT] =
            std::array::from_fn(|part| {
                self.session.part_attachment.parts[part]
                    .body
                    .expect("attached session retains each body")
            });
        for (is_angular, value) in [(true, angular), (false, linear)] {
            backend
                .try_set_skeleton_drag(&handles, binding.inertias(), is_angular, value)
                .map_err(|(part, source)| E::Body { part, source })?;
        }
        for (part, &handle) in handles.iter().enumerate().skip(1) {
            self.state.disable_counts[part] = Some(0);
            // The source OnEnd call between these writes is exactly BLR.
            self.state.volume_headers[part].flags |= 1;
            self.state.volume_disable_count_active = Some(false);
            self.state.material_selection[part] = SkeletonPartMaterialSelection::RagdollBody;
            if inputs
                .body_reader
                .byte(R::GlobalWipeout, 0xf189_9bbc_1605_0b02)
                .map_err(E::Attribute)?
                != 0
            {
                backend
                    .try_set_skeleton_ragdoll_mass(
                        handle,
                        &binding.inertias()[part],
                        body.parts[part].ragdoll.inverse_mass,
                    )
                    .map_err(|source| E::Body { part, source })?;
            }
            if inputs
                .body_reader
                .byte(R::GlobalWipeout, 0xd6de_5267_3515_1b3f)
                .map_err(E::Attribute)?
                != 0
            {
                backend
                    .try_set_skeleton_ragdoll_tensor(
                        handle,
                        &binding.inertias()[part],
                        body.parts[part].ragdoll.inverse_tensor,
                    )
                    .map_err(|source| E::Body { part, source })?;
            }
        }
        for part in [1, 3, 7] {
            self.state.material_selection[part] = SkeletonPartMaterialSelection::RagdollHead;
        }
        self.state.is_ragdoll = true;
        Ok(())
    }
}
