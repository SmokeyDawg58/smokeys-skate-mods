//! Prior-output copies within Update1, stopping at the selected Ground copy.
//! This extends the existing processed owner; no whole input-phase completion.
use super::SkeletonProcessedContact;
use crate::skeleton_root_transforms::SkeletonRootVector as WalkVector;
use crate::{
    physical_player_phys_out::{
        PhysOutAccessError, PhysOutGroundVector, PhysOutMotionScalar, PhysOutMotionVector,
        PhysOutSystemVector, PhysOutValue, PhysicalPlayerPhysOut,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;
#[path = "skeleton_processed_prior_output_body.rs"]
pub(crate) mod body;
#[path = "skeleton_processed_prior_output_runtime.rs"]
pub(crate) mod runtime;
#[path = "skeleton_processed_prior_output_tail.rs"]
pub(crate) mod tail;
#[path = "skeleton_processed_prior_output_wheel_timers.rs"]
pub(crate) mod wheel_timers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::enum_variant_names,
    reason = "each source gap precedes the named load and store"
)]
pub(crate) enum SkeletonPriorOutputGap {
    BeforeDeckVelocity,
    BeforeSurfaceVelocity,
    BeforeZSpeed,
    BeforeSurfaceSpeed,
    BeforeDeckSpeed,
    BeforeAngularVelocity,
    BeforeCentreOfMass,
    BeforeCentreOfMassVelocity,
    BeforeReckoningUp,
    BeforeGroundNormal,
}
/// These operations own the omitted Update1 instructions between selected
/// copies. They receive the same output owner so a late latch change is seen.
/// No default production implementation silently skips these instructions.
pub(crate) trait SkeletonPriorOutputGaps {
    type Error;
    fn run(
        &mut self,
        gap: SkeletonPriorOutputGap,
        output: &mut PhysicalPlayerPhysOut,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug, PartialEq)]
pub(crate) enum SkeletonPriorOutputError<E> {
    ForeignOutput,
    Gap(E),
    Output(PhysOutAccessError),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SkeletonProcessedPriorOutputState {
    output_occurrence: Option<Arc<()>>,
    velocity: Option<PhysOutValue<SkeletonRootVector>>,
    ground_position_all_states: Option<SkeletonRootVector>,
    wipeout_impulse: Option<SkeletonRootVector>,
    surface_velocity: Option<PhysOutValue<SkeletonRootVector>>,
    z_speed: Option<PhysOutValue<f32>>,
    surface_speed: Option<PhysOutValue<f32>>,
    deck_speed: Option<PhysOutValue<f32>>,
    angular_velocity: Option<PhysOutValue<SkeletonRootVector>>,
    centre_of_mass: Option<PhysOutValue<SkeletonRootVector>>,
    centre_of_mass_velocity: Option<PhysOutValue<SkeletonRootVector>>,
    reckoning_up: Option<PhysOutValue<SkeletonRootVector>>,
    ground_normal: Option<PhysOutValue<SkeletonRootVector>>,
    selected_copies_complete: bool,
    input_fields: runtime::PriorInputFields,
    tail_fields: tail::PriorOutputTailFields,
    wheel_timers: wheel_timers::PriorOutputWheelTimers,
    body_fields: body::PriorOutputBodyFields,
}
impl SkeletonProcessedPriorOutputState {
    /// Read-only pair validation. An unbound projection is not silently bound
    /// by a consumer, and no copied output value is sampled here.
    pub(crate) fn matches_output_occurrence(&self, output: &PhysicalPlayerPhysOut) -> bool {
        self.output_occurrence
            .as_ref()
            .is_some_and(|bound| Arc::ptr_eq(bound, output.occurrence()))
    }

    pub(super) fn bind_output_occurrence(&mut self, output: &PhysicalPlayerPhysOut) -> bool {
        if let Some(bound) = &self.output_occurrence {
            Arc::ptr_eq(bound, output.occurrence())
        } else {
            self.output_occurrence = Some(Arc::clone(output.occurrence()));
            true
        }
    }
    pub(crate) const fn new() -> Self {
        Self {
            output_occurrence: None,
            velocity: None,
            ground_position_all_states: None,
            wipeout_impulse: None,
            surface_velocity: None,
            z_speed: None,
            surface_speed: None,
            deck_speed: None,
            angular_velocity: None,
            centre_of_mass: None,
            centre_of_mass_velocity: None,
            reckoning_up: None,
            ground_normal: None,
            selected_copies_complete: false,
            input_fields: runtime::PriorInputFields::new(),
            tail_fields: tail::PriorOutputTailFields::new(),
            wheel_timers: wheel_timers::PriorOutputWheelTimers::new(),
            body_fields: body::PriorOutputBodyFields::new(),
        }
    }
    /// Readiness invalidation leaves the source fields untouched.
    pub(crate) fn invalidate(&mut self) {
        self.selected_copies_complete = false;
        self.input_fields.complete = false;
    }
    /// Processed Reset full-vector stores 0x82b39318/3931c.
    pub(super) fn reset_spatial_fields(&mut self) {
        self.input_fields.reset();
        self.tail_fields.reset();
        self.wheel_timers.reset();
        self.body_fields.reset();
        let zero = PhysOutValue {
            value: SkeletonRootVector::known([0.0; 4]),
            origin: crate::physical_player_phys_out::PhysOutOrigin::Reset,
        };
        // Reset82b3921c stores the complete zero vector to mVelocity+190.
        self.velocity = Some(zero);
        // Reset82b3924c stores mGroundPosAllStates+1f0.
        self.ground_position_all_states = Some(zero.value);
        // Reset39378, r10=&mWipeoutImpulse+330 from392cc.
        self.wipeout_impulse = Some(zero.value);
        // Reset3926c stores loaded native gJVector to mUpVector+220.
        self.reckoning_up = Some(PhysOutValue {
            value: SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
            origin: crate::physical_player_phys_out::PhysOutOrigin::Reset,
        });
        self.centre_of_mass = Some(zero);
        self.centre_of_mass_velocity = Some(zero);
        // Source Reset 82b3923c ground+1d0 and39350 deck omega+2d0.
        self.ground_normal = Some(zero);
        self.angular_velocity = Some(zero);
    }
    pub(crate) const fn body_fields(&self) -> &body::PriorOutputBodyFields {
        &self.body_fields
    }
    pub(crate) const fn centre_of_mass(&self) -> Option<PhysOutValue<SkeletonRootVector>> {
        self.centre_of_mass
    }
    pub(crate) const fn skater_look_direction(&self) -> Option<SkeletonRootVector> {
        self.input_fields.skater_look_direction
    }
    pub(crate) const fn centre_of_mass_velocity(&self) -> Option<PhysOutValue<SkeletonRootVector>> {
        self.centre_of_mass_velocity
    }
    pub(crate) const fn selected_copies_complete(&self) -> bool {
        self.selected_copies_complete
    }
    /// Processed+370, copied by Update1_ProcessInput82e028fc.
    pub(crate) const fn offboard_next_step_position(&self) -> Option<WalkVector> {
        self.input_fields.next_step_position
    }

    pub(crate) const fn offboard_locomotion_com(&self) -> Option<WalkVector> {
        self.input_fields.center_of_mass_delta
    }

    pub(crate) const fn offboard_locomotion_animation_com(&self) -> Option<WalkVector> {
        self.input_fields.center_of_mass_animation
    }

    pub(crate) fn offboard_foot_target(&self, index: usize) -> Option<WalkVector> {
        self.input_fields.foot_targets.get(index).copied().flatten()
    }

    pub(crate) fn offboard_foot_influence(&self, index: usize) -> Option<f32> {
        self.input_fields.foot_ik.get(index).copied().flatten()
    }
    /// Actual copied Processed+9b0, retained by Update1 prior-input runtime.
    pub(crate) const fn landing_surface_angle(&self) -> Option<f32> {
        self.input_fields.surface_angle
    }
    pub(crate) const fn velocity(&self) -> Option<PhysOutValue<SkeletonRootVector>> {
        self.velocity
    }
    pub(crate) const fn wipeout_impulse(&self) -> Option<SkeletonRootVector> {
        self.wipeout_impulse
    }
    pub(super) fn initialize_tail_sim_residuals(&mut self) {
        self.tail_fields.initialize_sim_residuals();
    }
    pub(super) fn publish_ground_position_all_states(&mut self, value: SkeletonRootVector) {
        self.ground_position_all_states = Some(value);
    }
    pub(crate) const fn ground_position_all_states(&self) -> Option<SkeletonRootVector> {
        self.ground_position_all_states
    }
    pub(crate) const fn surface_velocity(&self) -> Option<PhysOutValue<SkeletonRootVector>> {
        self.surface_velocity
    }
    pub(crate) const fn z_speed(&self) -> Option<PhysOutValue<f32>> {
        self.z_speed
    }
    pub(crate) fn trajectory_collision_time(&self) -> Option<f32> {
        self.tail_fields.trajectory_collision_time
    }
    pub(crate) fn store_restored_z_speed(&mut self, value: f32) -> Result<(), &'static str> {
        self.z_speed
            .as_mut()
            .ok_or("Processed ZSpeed occurrence")?
            .value = value;
        Ok(())
    }
    pub(crate) const fn surface_speed(&self) -> Option<PhysOutValue<f32>> {
        self.surface_speed
    }
    pub(crate) const fn dynamic_up(&self) -> Option<PhysOutValue<SkeletonRootVector>> {
        self.tail_fields.dynamic_up
    }
    pub(crate) const fn deck_speed(&self) -> Option<PhysOutValue<f32>> {
        self.deck_speed
    }
    pub(crate) const fn angular_velocity(&self) -> Option<PhysOutValue<SkeletonRootVector>> {
        self.angular_velocity
    }
    pub(crate) const fn reckoning_up(&self) -> Option<PhysOutValue<SkeletonRootVector>> {
        self.reckoning_up
    }
    pub(crate) const fn ground_normal(&self) -> Option<PhysOutValue<SkeletonRootVector>> {
        self.ground_normal
    }
}
impl SkeletonProcessedContact {
    /// Ground.UpdateSkateboard82d9a45c writes the takeoff velocity into the
    /// existing Processed+190 before Skeleton.GetLaunchInfo reads it.
    pub(crate) fn publish_ground_launch_velocity(&mut self, velocity: SkeletonRootVector) {
        self.prior_output.velocity = Some(PhysOutValue {
            value: velocity,
            origin: crate::physical_player_phys_out::PhysOutOrigin::Publication,
        });
    }

    pub(crate) const fn prior_output(&self) -> &SkeletonProcessedPriorOutputState {
        &self.prior_output
    }
    pub(crate) fn store_known_air_restored_speed(
        &mut self,
        value: f32,
    ) -> Result<(), &'static str> {
        self.prior_output.store_restored_z_speed(value)
    }
    /// Each getter guard immediately precedes its selected source load/store.
    /// Fields retain their source provenance and unknown W. This does NOT take
    /// the much later lock store at 0x82e03ae8 or mark toolkit ready.
    pub(crate) fn try_copy_prior_output<G: SkeletonPriorOutputGaps>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        gaps: &mut G,
    ) -> Result<(), SkeletonPriorOutputError<G::Error>> {
        use SkeletonPriorOutputError::{Gap, Output};
        use SkeletonPriorOutputGap::{
            BeforeAngularVelocity, BeforeCentreOfMass, BeforeCentreOfMassVelocity, BeforeDeckSpeed,
            BeforeDeckVelocity, BeforeGroundNormal, BeforeReckoningUp, BeforeSurfaceSpeed,
            BeforeSurfaceVelocity, BeforeZSpeed,
        };
        // These copies can replace inputs of an already completed toolkit.
        // Invalidate host readiness before even the first fallible gap; retain
        // all native numerical fields until their own source stores execute.
        self.toolkit_prefix_complete = false;
        self.toolkit.invalidate();
        self.prior_output.invalidate();
        if !self.prior_output.bind_output_occurrence(output) {
            return Err(SkeletonPriorOutputError::ForeignOutput);
        }
        gaps.run(BeforeDeckVelocity, output).map_err(Gap)?;
        self.prior_output.velocity = Some(
            output
                .motion_vector(PhysOutMotionVector::DeckVelocity)
                .map_err(Output)?,
        ); // 82e0263c
        gaps.run(BeforeSurfaceVelocity, output).map_err(Gap)?;
        self.prior_output.surface_velocity = Some(
            output
                .motion_vector(PhysOutMotionVector::SurfaceVelocity)
                .map_err(Output)?,
        ); // 82e02678
        gaps.run(BeforeZSpeed, output).map_err(Gap)?;
        self.prior_output.z_speed = Some(
            output
                .motion_scalar(PhysOutMotionScalar::ZSpeed)
                .map_err(Output)?,
        ); // 82e026e0
        gaps.run(BeforeSurfaceSpeed, output).map_err(Gap)?;
        self.prior_output.surface_speed = Some(
            output
                .motion_scalar(PhysOutMotionScalar::SurfaceSpeed)
                .map_err(Output)?,
        ); // 82e02714
        gaps.run(BeforeDeckSpeed, output).map_err(Gap)?;
        self.prior_output.deck_speed = Some(
            output
                .motion_scalar(PhysOutMotionScalar::DeckSpeed)
                .map_err(Output)?,
        ); // 82e02748
        gaps.run(BeforeAngularVelocity, output).map_err(Gap)?;
        self.prior_output.angular_velocity = Some(
            output
                .motion_vector(PhysOutMotionVector::AngularVelocity)
                .map_err(Output)?,
        ); // 82e02784
        gaps.run(BeforeCentreOfMass, output).map_err(Gap)?;
        self.prior_output.centre_of_mass = Some(
            output
                .system_vector(PhysOutSystemVector::CentreOfMass)
                .map_err(Output)?,
        ); // 82e02944
        gaps.run(BeforeCentreOfMassVelocity, output).map_err(Gap)?;
        self.prior_output.centre_of_mass_velocity = Some(
            output
                .system_vector(PhysOutSystemVector::CentreOfMassVelocity)
                .map_err(Output)?,
        ); // 82e0297c
        gaps.run(BeforeReckoningUp, output).map_err(Gap)?;
        self.prior_output.reckoning_up = Some(
            output
                .system_vector(PhysOutSystemVector::Up)
                .map_err(Output)?,
        ); // 82e029b4
        gaps.run(BeforeGroundNormal, output).map_err(Gap)?;
        self.prior_output.ground_normal = Some(
            output
                .ground_vector(PhysOutGroundVector::LastValidGroundUp)
                .map_err(Output)?,
        ); // 82e02a28
        self.prior_output.selected_copies_complete = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "skeleton_processed_prior_output_tests.rs"]
mod tests;

#[path = "../postphysics/skeleton_postphysics_collision_inputs_live.rs"]
pub(crate) mod postphysics_collision_inputs;
