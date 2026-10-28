//! Same-owner ProcessData82b5efd0..f348, after the retained trajectory prefix.
use super::SkeletonNormalAssembly;
use crate::{
    actor_construction::anim_out_phys_in::AnimOutPhysInOwner,
    attribute_runtime::AttributeRuntimeError,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    skeleton_assembly_construction::constructor::SkeletonConstructorRuntime,
    skeleton_current_animation::{SkeletonCurrentAnimationCvars, SkeletonCurrentAnimationError},
    skeleton_processed_contact::{
        SkeletonProcessedContact,
        process_data_output::{PhysicalGrind, ProcessDataOutputError},
    },
};

#[derive(Debug)]
pub(crate) enum ProcessDataContinuationError<C> {
    Missing(&'static str),
    Animation(SkeletonCurrentAnimationError<AttributeRuntimeError, C>),
    Output(ProcessDataOutputError),
    Poisoned,
    Pose(crate::actor_construction::anim_out_phys_in::pose_lanes::PoseAccessError),
}
impl<C: std::fmt::Debug> std::fmt::Display for ProcessDataContinuationError<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "ProcessData missing {field}"),
            Self::Animation(error) => error.fmt(f),
            Self::Output(error) => error.fmt(f),
            Self::Poisoned => f.write_str("ProcessData hook dynamics lock poisoned"),
            Self::Pose(error) => error.fmt(f),
        }
    }
}

impl SkeletonNormalAssembly {
    /// The surrounding pool has checked the three actual pointer identities.
    /// Every source output is committed before entering the following stage.
    pub(crate) fn try_process_data_continuation<C: SkeletonCurrentAnimationCvars>(
        &mut self,
        constructor: &mut SkeletonConstructorRuntime,
        input: &AnimOutPhysInOwner,
        processed: &mut SkeletonProcessedContact,
        owner: &PhysicalPlayerAssemblyOwner,
        cvars: &mut C,
    ) -> Result<(), ProcessDataContinuationError<C::Error>> {
        use ProcessDataContinuationError as E;
        let pose = input
            .pose_known_lanes(
                crate::actor_construction::anim_out_phys_in::pose_lanes::PoseBuffer::Global,
            )
            .map_err(E::Pose)?;
        let attributes = self
            .current_animation_attributes
            .as_mut()
            .ok_or(E::Missing("constructor animation attributes"))?;
        let mut grind = PhysicalGrind(owner);
        let animation_to_world = self
            .root_transforms
            .animation_to_world()
            .columns()
            .map(|c| {
                // Only the X, Y and Z lanes of AnimToWorld are splatted.
                [c.xyz[0], c.xyz[1], c.xyz[2], c.w.unwrap_or(0.0)]
            });
        processed
            .with_process_data_read(|read, delta| {
                self.current_animation.try_process_read(
                    &mut self.animation_targets,
                    pose,
                    read,
                    delta,
                    &self.physics_history,
                    self.root_transforms.animation_to_board(),
                    &animation_to_world,
                    &self.root_transforms.world_to_animation_lanes(),
                    self.post_physics_records.centre_of_mass_velocity()[1],
                    attributes,
                    cvars,
                    &mut crate::skeleton_physics_history::HostSkeletonPhysicsHistoryNumeric,
                    &mut grind,
                )
            })
            .map_err(E::Animation)?;

        // f278 captures old8c8 BEFORE f288 publishes the orientation bits.
        let old_collision = processed
            .capture_process_data_collision_flags()
            .map_err(E::Output)?;
        processed.write_process_data_deck_orientation(self.current_animation.last_deck_zy());
        processed.write_process_data_collision(
            old_collision,
            self.state.collision.compliant_just_collided,
        );
        if self.state.collision.is_hook_to_hips_valid != 0 {
            processed.write_process_data_hook(self.post_physics_records.hook_to_hips());
        }
        // f2c4..f2e0: Skeleton+2a30 (State+10f0 world COM-to-deck) -> +2f0,
        // then +2a20 (its delta) -> +300; not the anim-space COM at State+10c0.
        processed.write_process_data_com_to_deck(self.current_animation.record().com_to_deck());
        processed.write_process_data_com_to_deck_delta(
            self.current_animation.record().com_to_deck_delta(),
        );

        let outer = constructor
            .outer
            .as_mut()
            .ok_or(E::Missing("constructor remainder"))?;
        if outer
            .teleport_reset
            .ok_or(E::Missing("teleport-reset byte"))?
        {
            outer.teleport_reset = Some(false); //f2f8
            self.root_transforms
                .invalidate_prediction_after_process_data_teleport(); //f300
            self.current_animation
                .try_enable_ik()
                .map_err(|_| E::Missing("actual SkeletonIK"))?; //f30c
            // Two fresh retained drive-owner reads, with source field-store order.
            {
                let drives = self.drives.as_ref().ok_or(E::Missing("SkeletonDrives"))?;
                let mut dynamics = drives
                    .prepared()
                    .hook_dynamics(0)
                    .write()
                    .map_err(|_| E::Poisoned)?;
                dynamics.angular.damping = 0.0;
                dynamics.angular.raw_type = 2;
                dynamics.angular.spring = f32::from_bits(0x4415_ffff);
                dynamics.angular.strength = f32::from_bits(0x470c_9fff);
            }
            let drives = self.drives.as_ref().ok_or(E::Missing("SkeletonDrives"))?;
            let mut dynamics = drives
                .prepared()
                .hook_dynamics(0)
                .write()
                .map_err(|_| E::Poisoned)?;
            dynamics.linear.spring = f32::from_bits(0x4415_ffff);
            dynamics.linear.damping = 0.0;
            dynamics.linear.raw_type = 2;
            dynamics.linear.strength = f32::from_bits(0x470c_9fff);
        }
        Ok(())
    }
}
