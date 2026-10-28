//! Narrow owner-qualified effects used by the actual Skeleton Assembly session.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "normal skeleton awaits physical-player construction"
    )
)]
use super::SimulationAssemblyBackend;
use crate::{
    ragdoll::SKELETON_PART_COUNT,
    simulation_joint_pool::{
        SimulationJointFrames, SimulationJointHandle, SimulationJointLimits,
        SimulationJointPoolError,
    },
    simulation_rigid_body_pool::{
        SimulationMutableInertia, SimulationRigidBodyHandle, SimulationRigidBodyPoolError,
    },
    skeleton_attributes::SkeletonBodyAttributeData,
    skeleton_body_definition::SkeletonPartMassCache,
    skeleton_data::SkeletonData,
    skeleton_normal_joint_limits::{
        SkeletonJointLimitBinding, SkeletonNormalJointLimitsError, try_set_normal_joint_limits,
    },
};

impl<M> SimulationAssemblyBackend<M, SimulationJointFrames, SimulationJointLimits> {
    pub(crate) fn try_set_skeleton_joint_spy(
        &mut self,
        joint: SimulationJointHandle,
    ) -> Result<(), SimulationJointPoolError> {
        self.joints.try_set_spy(joint, 1)
    }

    pub(crate) fn try_set_skeleton_normal_limits<E>(
        &mut self,
        data: &SkeletonData,
        attributes: &SkeletonBodyAttributeData,
        bindings: &[SkeletonJointLimitBinding],
        cosine: &mut impl FnMut(f32) -> Result<f32, E>,
    ) -> Result<(), SkeletonNormalJointLimitsError<E>> {
        try_set_normal_joint_limits(data, attributes, &mut self.joints, bindings, cosine)
    }

    /// `Body::SetAngularDrag/SetLinearDrag`, 0x82de7f38/0x82de7de0, all24/-1.
    /// Six groups of four current owners; all angular stores precede linear.
    /// Host copies needed to preserve unrelated fields precede each group's
    /// stores; lock failures are host refusals, not native failure recovery.
    pub(crate) fn try_zero_skeleton_drag(
        &mut self,
        bodies: &[SimulationRigidBodyHandle; SKELETON_PART_COUNT],
        owners: &[SimulationMutableInertia; SKELETON_PART_COUNT],
        angular: bool,
    ) -> Result<(), (usize, SimulationRigidBodyPoolError)> {
        self.try_set_skeleton_drag(bodies, owners, angular, 0.0)
    }

    pub(crate) fn try_set_skeleton_drag(
        &mut self,
        bodies: &[SimulationRigidBodyHandle; SKELETON_PART_COUNT],
        owners: &[SimulationMutableInertia; SKELETON_PART_COUNT],
        angular: bool,
        value: f32,
    ) -> Result<(), (usize, SimulationRigidBodyPoolError)> {
        let drag = value * f32::from_bits(0x426f_ffff);
        for first in (0..SKELETON_PART_COUNT).step_by(4) {
            let read = |part: usize| owners[part].try_read().map_err(|error| (part, error));
            let values = [
                read(first)?,
                read(first + 1)?,
                read(first + 2)?,
                read(first + 3)?,
            ];
            for (offset, mut value) in values.into_iter().enumerate() {
                let part = first + offset;
                if angular {
                    value.angular_drag = drag;
                } else {
                    value.linear_drag = drag;
                }
                self.rigid_bodies
                    .try_write_mutable_inertia(bodies[part], &owners[part], value)
                    .map_err(|error| (part, error))?;
            }
        }
        Ok(())
    }

    pub(crate) fn try_set_skeleton_ragdoll_limits<C>(
        &mut self,
        data: &SkeletonData,
        attributes: &SkeletonBodyAttributeData,
        reader: &mut crate::skeleton_joint_attributes::SkeletonJointAttributeProvider,
        bindings: &[SkeletonJointLimitBinding],
        cosine: &mut impl FnMut(f32) -> Result<f32, C>,
    ) -> Result<(), crate::dynamic_assembly_session::skeleton_normal::ragdoll_owner::joint_limits::RagdollJointLimitsError<C>>{
        crate::dynamic_assembly_session::skeleton_normal::ragdoll_owner::joint_limits::try_set_ragdoll_joint_limits(
            data, attributes, reader, &mut self.joints, bindings, cosine,
        )
    }

    pub(crate) fn try_set_skeleton_ragdoll_mass(
        &mut self,
        body: SimulationRigidBodyHandle,
        owner: &SimulationMutableInertia,
        inverse_mass: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let mut value = owner.try_read()?;
        value.inverse_mass = inverse_mass;
        self.rigid_bodies
            .try_write_mutable_inertia(body, owner, value)
    }

    /// Raw `SetUpRagdoll`50328..384: vector, XY minimum scalar, reciprocal scalar.
    pub(crate) fn try_set_skeleton_ragdoll_tensor(
        &mut self,
        body: SimulationRigidBodyHandle,
        owner: &SimulationMutableInertia,
        tensor: [f32; 4],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let mut value = owner.try_read()?;
        let [x, y, z, w] = tensor;
        value.inverse_tensor = [x, y, z];
        self.rigid_bodies
            .try_write_mutable_packed_inertia(body, owner, value, w)?;
        let xy = if x < y { x } else { y };
        value.spherical = xy;
        self.rigid_bodies
            .try_write_mutable_inertia(body, owner, value)?;
        let minimum = if xy < z { xy } else { z };
        value.spherical = 1.0 / minimum;
        self.rigid_bodies
            .try_write_mutable_inertia(body, owner, value)
    }

    /// Source 0x82b505f8..0x82b50648. Preserve scalar/vector store stages;
    /// no pose, cached rotated tensors, RF, topology, tag or motion refresh.
    pub(crate) fn try_set_skeleton_animated_inertia(
        &mut self,
        body: SimulationRigidBodyHandle,
        owner: &SimulationMutableInertia,
        cache: SkeletonPartMassCache,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let mut value = owner.try_read()?;
        value.inverse_mass = cache.inverse_mass;
        self.rigid_bodies
            .try_write_mutable_inertia(body, owner, value)?;
        let [x, y, z, w] = cache.inverse_tensor;
        value.inverse_tensor = [x, y, z];
        self.rigid_bodies
            .try_write_mutable_packed_inertia(body, owner, value, w)?;
        let xy = if x < y { x } else { y };
        value.spherical = xy;
        self.rigid_bodies
            .try_write_mutable_inertia(body, owner, value)?;
        let minimum = if xy < z { xy } else { z };
        value.spherical = 1.0 / minimum;
        self.rigid_bodies
            .try_write_mutable_inertia(body, owner, value)
    }
}
