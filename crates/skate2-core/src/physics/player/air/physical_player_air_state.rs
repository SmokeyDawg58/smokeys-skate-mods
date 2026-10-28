//! Retained `PhysicsAir` constructor82d8a9e0. Entry lives in its actual pair bridge.
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    physical_player_footplant::RetainedTrajectory,
    physical_player_physics_state::PhysicalStateBindings,
};
use std::sync::Arc;

pub(crate) struct PhysicsAirState {
    pub(crate) bindings: PhysicalStateBindings,
    attributes: AttributeRuntimeInstance,
    #[expect(
        dead_code,
        reason = "Air reckoning attributes belong to subsequent source Update"
    )]
    reckoning_attributes: AttributeRuntimeInstance,
    pub(crate) fields: PhysicsAirFields,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PhysicsAirFields {
    pub(crate) com_trajectory: RetainedTrajectory,
    pub(crate) landing_normal: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) time_in_state: f32,
    pub(crate) start_y: f32,
    pub(crate) max_y: f32,
    pub(crate) reached_apex: bool,
    pub(crate) use_com_velocity: bool,
    pub(crate) run_mount: Option<bool>,
    pub(crate) sprint_mount: Option<bool>,
    pub(crate) trajectory_timer: i32,
}
impl PhysicsAirState {
    pub(crate) fn new(bindings: PhysicalStateBindings, vault: &Arc<AttributeRuntimeVault>) -> Self {
        let attributes = vault.instance(
            crate::physical_player_state_changer::attributes::AIR_STATES_CLASS,
            0xd7ed_bd36_2d7d_2152,
        );
        let reckoning_attributes = vault.instance(
            crate::reckoning_attributes::RECKONING_CLASS,
            0xd7ed_bd36_2d7d_2152,
        );
        Self {
            bindings,
            attributes,
            reckoning_attributes,
            fields: PhysicsAirFields::constructed(),
        }
    }
}
impl PhysicsAirFields {
    fn constructed() -> Self {
        Self {
            com_trajectory: RetainedTrajectory::constructed(),
            landing_normal: crate::skeleton_root_transforms::SkeletonRootVector::known([
                0.0, 1.0, 0.0, 0.0,
            ]),
            time_in_state: 0.0,
            start_y: 0.0,
            max_y: 0.0,
            reached_apex: false,
            use_com_velocity: false,
            run_mount: None,
            sprint_mount: None,
            trajectory_timer: 0,
        }
    }
}

#[path = "physical_player_air_spin_target.rs"]
pub(crate) mod spin_target;

impl PhysicsAirState {
    /// Own `mAttributes`+20 `GetAttributePointer(key, 0)`; a null pointer reads
    /// the four-byte zero `DefaultDataArea(4)`. Each call fetches again.
    pub(crate) fn try_scalar(
        &self,
        key: u64,
    ) -> Result<f32, crate::attribute_runtime::AttributeRuntimeError> {
        let data = self.attributes.try_get_data(key, 0)?;
        let bytes = data.as_ref().map_or(&[0; 4][..], |data| data.bytes());
        if bytes.len() != 4 {
            return Err(
                crate::attribute_runtime::AttributeRuntimeError::PayloadSize {
                    key,
                    expected: 4,
                    actual: bytes.len(),
                },
            );
        }
        Ok(f32::from_bits(u32::from_be_bytes(
            bytes.try_into().expect("checked scalar"),
        )))
    }

    /// This own-state FETCH executes only AFTER successful Reckoning.UpdateAirStates.
    pub(crate) fn try_com_correction_enabled(
        &self,
    ) -> Result<bool, crate::attribute_runtime::AttributeRuntimeError> {
        use crate::simulation_assembly_backend::constraint_session::air_com_correction::{
            ENABLE, decode_enable,
        };
        let data = self.attributes.try_get_data(ENABLE, 0)?;
        decode_enable(
            data.as_ref()
                .map(crate::attribute_runtime::AttributeRuntimeData::bytes),
        )
    }
}

#[path = "physical_player_air_trajectory.rs"]
pub(crate) mod trajectory;
