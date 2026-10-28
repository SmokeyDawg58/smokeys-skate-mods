//! Same `FootPlantManager` constructor82dc1168, Reset82db6760, `FullReset`82dbbae0.
#![expect(
    dead_code,
    reason = "retained prediction payload fields await their native readers"
)]
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Weak};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FootPlantError {
    AlreadyAttempted,
    NotConstructed,
    ForeignOwner,
}

pub(crate) struct PhysicalPlayerFootPlantOwner {
    attempted: bool,
    value: Option<FootPlantManager>,
}

/// Native tTrajectory constructs three zero vectors and splatted time=-1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RetainedTrajectory {
    pub(crate) position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) velocity: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) gravity: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) time: [f32; 4],
}
impl RetainedTrajectory {
    pub(crate) const fn constructed() -> Self {
        Self {
            position: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            velocity: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            gravity: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            time: [-1.0; 4],
        }
    }
}

/// Constructor does not initialize the prediction result's other fields.
pub(crate) struct FootPlantPrediction {
    pub(crate) ready: bool,
    pub(crate) trajectory: RetainedTrajectory,
    pub(crate) collision_position: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) collision_normal: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) ground_normal: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) collision_time: Option<[f32; 4]>,
    pub(crate) frame: Option<i32>,
    pub(crate) surface: Option<u32>,
    pub(crate) radius: Option<f32>,
    pub(crate) max_error_start: Option<f32>,
    pub(crate) max_error_end: Option<f32>,
    pub(crate) draw_true: Option<bool>,
}
impl FootPlantPrediction {
    pub(crate) fn new() -> Self {
        Self {
            ready: false,
            trajectory: RetainedTrajectory::constructed(),
            collision_position: None,
            collision_normal: None,
            ground_normal: None,
            collision_time: None,
            frame: None,
            surface: None,
            radius: None,
            max_error_start: None,
            max_error_end: None,
            draw_true: None,
        }
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "separate native FootPlant flags with different reset/producer sites"
)]
pub(crate) struct FootPlantFields {
    pub(crate) anim_foot_position: Option<SkeletonRootVector>,
    pub(crate) physics_foot_position: Option<SkeletonRootVector>,
    pub(crate) landing_position: Option<SkeletonRootVector>,
    pub(crate) landing_normal: Option<SkeletonRootVector>,
    pub(crate) leg_vector: Option<SkeletonRootVector>,
    pub(crate) exit_velocity: SkeletonRootVector,
    pub(crate) ik_foot_position: SkeletonRootVector,
    pub(crate) centre_of_mass: SkeletonRootVector,
    pub(crate) anim_centre_of_mass: SkeletonRootVector,
    pub(crate) perfect_leg_vector: SkeletonRootVector,
    pub(crate) last_ik_foot_target: SkeletonRootVector,
    pub(crate) bezier: [Option<SkeletonRootVector>; 4],
    pub(crate) time_to_land: f32,
    pub(crate) total_time: f32,
    pub(crate) time_in_footplant: f32,
    pub(crate) ik_blend_in: f32,
    pub(crate) time_targeting: f32,
    pub(crate) foot_part: i32,
    pub(crate) surface_id: Option<u32>,
    pub(crate) foot_off_board: bool,
    pub(crate) results_valid: bool,
    pub(crate) landing_next_frame: bool,
    pub(crate) foot_planting: bool,
    pub(crate) perfect_leg_valid: bool,
    pub(crate) last_target_valid: bool,
    pub(crate) targeting: bool,
}
impl FootPlantFields {
    fn constructed() -> Self {
        Self {
            anim_foot_position: None,
            physics_foot_position: None,
            landing_position: None,
            landing_normal: None,
            leg_vector: None,
            exit_velocity: ZERO,
            ik_foot_position: ZERO,
            centre_of_mass: ZERO,
            anim_centre_of_mass: ZERO,
            perfect_leg_vector: ZERO,
            last_ik_foot_target: ZERO,
            bezier: [None; 4],
            time_to_land: -1.0,
            total_time: -1.0,
            time_in_footplant: 0.0,
            ik_blend_in: 0.0,
            time_targeting: 0.0,
            foot_part: -1,
            surface_id: None,
            foot_off_board: false,
            results_valid: false,
            landing_next_frame: false,
            foot_planting: false,
            perfect_leg_valid: false,
            last_target_valid: false,
            targeting: false,
        }
    }
    /// Reset82db6760's stores; also reached from UpdateFootplantAir 0x82dc3984.
    pub(crate) fn reset(&mut self) {
        self.foot_off_board = false;
        self.time_in_footplant = 0.0;
        self.ik_blend_in = 0.0;
        self.time_targeting = 0.0;
        self.results_valid = false;
        self.landing_next_frame = false;
        self.time_to_land = -1.0;
        self.total_time = -1.0;
        self.foot_planting = false;
        self.foot_part = -1;
        self.perfect_leg_valid = false;
        self.last_target_valid = false;
        self.targeting = false;
        self.exit_velocity = ZERO;
        self.ik_foot_position = ZERO;
        self.perfect_leg_vector = ZERO;
        self.centre_of_mass = ZERO;
        self.anim_centre_of_mass = ZERO;
        self.last_ik_foot_target = ZERO;
    }
}
const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);
#[cfg(feature = "skate3-runtime")]
#[path = "skate3_footplant.rs"]
pub(crate) mod skate3;

pub(crate) struct FootPlantManager {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) source3: Option<Box<skate3::SourceFootplant>>,
    owner: Weak<PhysicalPlayerAssemblyOwner>,
    skeleton: SimulationSkaterAssemblyHandle,
    attributes: AttributeRuntimeInstance,
    pub(crate) prediction: FootPlantPrediction,
    pub(crate) fields: FootPlantFields,
}
impl FootPlantManager {
    /// Manager+0 `physics_footplantmanager` Instance, read by the air/ground updates.
    pub(crate) fn attributes(&self) -> &AttributeRuntimeInstance {
        &self.attributes
    }
}
impl PhysicalPlayerFootPlantOwner {
    pub(crate) fn is_constructed(&self) -> bool {
        self.value.is_some()
    }

    pub(crate) const fn new() -> Self {
        Self {
            attempted: false,
            value: None,
        }
    }
    pub(crate) fn try_construct(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
        vault: &Arc<AttributeRuntimeVault>,
        allocated: bool,
    ) -> Result<(), FootPlantError> {
        if self.attempted {
            return Err(FootPlantError::AlreadyAttempted);
        }
        self.attempted = true;
        if allocated {
            let attributes = vault.instance(0x172d_5539_37d8_e07c, 0xd7ed_bd36_2d7d_2152);
            let prediction = FootPlantPrediction::new();
            self.value = Some(FootPlantManager {
                #[cfg(feature = "skate3-runtime")]
                source3: None,
                owner: Arc::downgrade(owner),
                skeleton: skeleton.clone(),
                attributes,
                prediction,
                fields: FootPlantFields::constructed(),
            });
        }
        Ok(())
    }
    pub(crate) fn try_full_reset(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), FootPlantError> {
        let value = self.value.as_mut().ok_or(FootPlantError::NotConstructed)?;
        if !value
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
            || !value.skeleton.same_as(skeleton)
        {
            return Err(FootPlantError::ForeignOwner);
        }
        value.prediction.ready = false;
        value.fields.reset();
        #[cfg(feature = "skate3-runtime")]
        if let Some(source)=value.source3.as_mut() { source.reset(true,false); }
        Ok(())
    }
    /// The actual manager bound to this owner and Skeleton (Manager+1b0/1b4).
    pub(crate) fn try_manager(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<&mut FootPlantManager, FootPlantError> {
        let value = self.value.as_mut().ok_or(FootPlantError::NotConstructed)?;
        if !value
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
            || !value.skeleton.same_as(skeleton)
        {
            return Err(FootPlantError::ForeignOwner);
        }
        Ok(value)
    }
    /// Manager+10 future, addressed by the World predictor queue by owner only.
    pub(crate) fn try_owned_prediction(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Result<&mut FootPlantPrediction, FootPlantError> {
        let value = self.value.as_mut().ok_or(FootPlantError::NotConstructed)?;
        if !value
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
        {
            return Err(FootPlantError::ForeignOwner);
        }
        Ok(&mut value.prediction)
    }
    /// Reset82db6760 leaves the prediction readiness untouched (unlike FullReset).
    pub(crate) fn try_reset(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), FootPlantError> {
        let value = self.value.as_mut().ok_or(FootPlantError::NotConstructed)?;
        if !value
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
            || !value.skeleton.same_as(skeleton)
        {
            return Err(FootPlantError::ForeignOwner);
        }
        value.fields.reset();
        #[cfg(feature = "skate3-runtime")]
        if let Some(source)=value.source3.as_mut() {
            source.reset(false,owner.skate3_footplants.load(std::sync::atomic::Ordering::Relaxed));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_preserves_unwritten_vectors_surface_and_prediction_payload() {
        let mut fields = FootPlantFields::constructed();
        fields.anim_foot_position = Some(SkeletonRootVector::known([2.0; 4]));
        fields.bezier[3] = Some(SkeletonRootVector::known([3.0; 4]));
        fields.surface_id = Some(0xfeed);
        fields.exit_velocity = SkeletonRootVector::known([4.0; 4]);
        fields.foot_planting = true;
        fields.reset();
        assert_eq!(
            fields.anim_foot_position,
            Some(SkeletonRootVector::known([2.0; 4]))
        );
        assert_eq!(fields.bezier[3], Some(SkeletonRootVector::known([3.0; 4])));
        assert_eq!(fields.surface_id, Some(0xfeed));
        assert_eq!(
            fields
                .exit_velocity
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
        assert!(!fields.foot_planting);
        assert_eq!(fields.foot_part, -1);
    }
}

#[path = "physical_player_footplant_fill_access.rs"]
pub(crate) mod fill_access;
