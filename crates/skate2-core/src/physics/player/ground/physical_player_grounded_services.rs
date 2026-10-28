//! Lazy cvar forwarding and same-session World query binding for active IK.
use crate::{
    dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralCvars,
    simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineError, SimulationWorldLineQueries,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    skeleton_drives::update::SkeletonDrivesUpdateCvars,
    skeleton_ik::SkeletonIkSuffixServices,
    skeleton_ik_prephysics::SkeletonIkPrePhysicsServices,
    skeleton_ik_state::{
        contact::{
            SkeletonIkContactLineQuery, SkeletonIkContactLineResult, SkeletonIkContactServices,
        },
        drive_frames::SkeletonIkDriveFramesServices,
    },
    static_ground_pool::StaticGroundPool,
};
type CvarError<C> = <C as SkeletonDrivesUpdateCvars>::Error;

#[derive(Debug)]
pub(crate) enum GroundedWorldServiceError<C> {
    Cvar(C),
    QueryUnavailable,
    DmoUnavailable,
    Query(SimulationWorldLineError<crate::reckoning_numeric::ReckoningHostNumericError>),
}
impl<C: std::fmt::Debug> std::fmt::Display for GroundedWorldServiceError<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => write!(f, "grounded cvar: {error:?}"),
            Self::QueryUnavailable => f.write_str("grounded World query owner unavailable"),
            Self::DmoUnavailable => f.write_str("grounded DMO owner unavailable"),
            Self::Query(error) => write!(f, "grounded {error}"),
        }
    }
}

pub(super) struct GroundedWorldServices<'a, C> {
    pub(super) cvars: &'a mut C,
    pub(super) queries: Option<&'a mut SimulationWorldLineQueries>,
    pub(super) dmos: Option<&'a SimulationDmoAssemblyPool>,
    pub(super) ground: &'a StaticGroundPool,
}
impl<C> SkeletonDrivesUpdateCvars for GroundedWorldServices<'_, C>
where
    C: SkeletonGeneralCvars
        + SkeletonIkSuffixServices
        + SkeletonIkContactServices<Error = CvarError<C>>,
{
    type Error = GroundedWorldServiceError<CvarError<C>>;
    fn offboard_drive_scalar(&mut self) -> Result<f32, Self::Error> {
        SkeletonDrivesUpdateCvars::offboard_drive_scalar(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn soft_damping(&mut self) -> Result<f32, Self::Error> {
        SkeletonDrivesUpdateCvars::soft_damping(self.cvars).map_err(GroundedWorldServiceError::Cvar)
    }
}
impl<C> SkeletonGeneralCvars for GroundedWorldServices<'_, C>
where
    C: SkeletonGeneralCvars
        + SkeletonIkSuffixServices
        + SkeletonIkContactServices<Error = CvarError<C>>,
{
    fn ik_draw_feet_on_deck(&mut self) -> Result<u8, GroundedWorldServiceError<CvarError<C>>> {
        SkeletonGeneralCvars::ik_draw_feet_on_deck(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn ik_require_debug_overlay(&mut self) -> Result<(), GroundedWorldServiceError<CvarError<C>>> {
        SkeletonGeneralCvars::ik_require_debug_overlay(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn ik_maximum_external_normal_angle(
        &mut self,
    ) -> Result<f32, GroundedWorldServiceError<CvarError<C>>> {
        SkeletonGeneralCvars::ik_maximum_external_normal_angle(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn ik_target_normal_blend_speed(
        &mut self,
    ) -> Result<f32, GroundedWorldServiceError<CvarError<C>>> {
        SkeletonGeneralCvars::ik_target_normal_blend_speed(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn ik_maximum_local_delta(&mut self) -> Result<f32, GroundedWorldServiceError<CvarError<C>>> {
        SkeletonGeneralCvars::ik_maximum_local_delta(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn ik_maximum_world_offset(&mut self) -> Result<f32, GroundedWorldServiceError<CvarError<C>>> {
        SkeletonGeneralCvars::ik_maximum_world_offset(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
}
impl<C> SkeletonIkPrePhysicsServices for GroundedWorldServices<'_, C>
where
    C: SkeletonGeneralCvars
        + SkeletonIkSuffixServices
        + SkeletonIkContactServices<Error = CvarError<C>>,
{
    type Error = GroundedWorldServiceError<CvarError<C>>;
    fn draw_pre_feet_ik(&mut self) -> Result<bool, Self::Error> {
        SkeletonIkPrePhysicsServices::draw_pre_feet_ik(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        SkeletonIkPrePhysicsServices::require_debug_overlay(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
}
impl<C> SkeletonIkDriveFramesServices for GroundedWorldServices<'_, C>
where
    C: SkeletonGeneralCvars
        + SkeletonIkSuffixServices
        + SkeletonIkContactServices<Error = CvarError<C>>,
{
    fn enable_prephysics(&mut self) -> Result<bool, GroundedWorldServiceError<CvarError<C>>> {
        SkeletonIkDriveFramesServices::enable_prephysics(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn maximum_leg_length(&mut self) -> Result<f32, GroundedWorldServiceError<CvarError<C>>> {
        SkeletonIkDriveFramesServices::maximum_leg_length(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn maximum_arm_length(&mut self) -> Result<f32, GroundedWorldServiceError<CvarError<C>>> {
        SkeletonIkDriveFramesServices::maximum_arm_length(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
}
impl<C> SkeletonIkSuffixServices for GroundedWorldServices<'_, C>
where
    C: SkeletonGeneralCvars
        + SkeletonIkSuffixServices
        + SkeletonIkContactServices<Error = CvarError<C>>,
{
    fn enable_contact_offset(&mut self) -> Result<bool, GroundedWorldServiceError<CvarError<C>>> {
        SkeletonIkSuffixServices::enable_contact_offset(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
}
impl<C> SkeletonIkContactServices for GroundedWorldServices<'_, C>
where
    C: SkeletonGeneralCvars
        + SkeletonIkSuffixServices
        + SkeletonIkContactServices<Error = CvarError<C>>,
{
    type Error = GroundedWorldServiceError<CvarError<C>>;
    fn draw_ik_contact_debug(&mut self) -> Result<bool, Self::Error> {
        SkeletonIkContactServices::draw_ik_contact_debug(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn max_ik_contact_penetration_offset(&mut self) -> Result<f32, Self::Error> {
        SkeletonIkContactServices::max_ik_contact_penetration_offset(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn sideways_offset(&mut self) -> Result<f32, Self::Error> {
        SkeletonIkContactServices::sideways_offset(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn capture_sideways_offset_draw(&mut self) -> Result<(), Self::Error> {
        self.cvars
            .capture_sideways_offset_draw()
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn sideways_offset_draw(&mut self) -> Result<bool, Self::Error> {
        SkeletonIkContactServices::sideways_offset_draw(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn max_ik_contact_offset(&mut self) -> Result<f32, Self::Error> {
        SkeletonIkContactServices::max_ik_contact_offset(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn toe_contact_threshold(&mut self) -> Result<f32, Self::Error> {
        SkeletonIkContactServices::toe_contact_threshold(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        SkeletonIkContactServices::require_debug_overlay(self.cvars)
            .map_err(GroundedWorldServiceError::Cvar)
    }

    fn try_line_query(
        &mut self,
        query: &SkeletonIkContactLineQuery,
    ) -> Result<SkeletonIkContactLineResult, Self::Error> {
        let queries = self
            .queries
            .as_mut()
            .ok_or(GroundedWorldServiceError::QueryUnavailable)?;
        let dmos = self.dmos.ok_or(GroundedWorldServiceError::DmoUnavailable)?;
        let result = queries
            .try_line_test(
                self.ground,
                dmos,
                crate::collision_line_collection::CollisionLineInput {
                    start: query.start,
                    end: query.end,
                    fatness: query.fatness,
                },
                query.label,
                &mut crate::collision_line_numeric::HostCollisionLineNumeric,
            )
            .map_err(GroundedWorldServiceError::Query)?;
        Ok(match result {
            Some(hit) => SkeletonIkContactLineResult::Hit {
                position: hit.position,
            },
            None => SkeletonIkContactLineResult::Miss,
        })
    }
}
