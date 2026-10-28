//! `Skeleton::GetLaunchInfo`0x82b46098, borrowing the actual retained owners.
//! Registration requires the companion typed launch-record migration.
use super::RetainedTrajectoryLaunchInfo;
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes,
    skeleton_assembly_construction::constructor::SkeletonConstructorRuntime,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;

#[cfg(test)]
#[path = "physical_player_skeleton_launch_info_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum SkeletonLaunchInfoError {
    Missing(&'static str),
    ForeignOwner,
    Poisoned(&'static str),
    Attribute(AttributeRuntimeError),
}
impl std::fmt::Display for SkeletonLaunchInfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "launch info missing {field}"),
            Self::ForeignOwner => f.write_str("launch info uses a different physical owner"),
            Self::Poisoned(owner) => write!(f, "launch info poisoned {owner}"),
            Self::Attribute(error) => write!(f, "launch info trajectory attribute: {error}"),
        }
    }
}

/// Earlier output stores survive a later missing read. There is no prefetch of
/// Reckoning or either generated-layout access before its native consumption.
pub(crate) fn try_fill_launch_info(
    constructor: &SkeletonConstructorRuntime,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    trajectory: &WallRideTrajectoryAttributes,
    output: &mut RetainedTrajectoryLaunchInfo,
) -> Result<(), SkeletonLaunchInfoError> {
    use SkeletonLaunchInfoError::Missing;
    let binding = constructor
        .bindings
        .as_ref()
        .ok_or(Missing("Skeleton binding"))?;
    if !binding
        .owner
        .upgrade()
        .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
    {
        return Err(SkeletonLaunchInfoError::ForeignOwner);
    }
    let mut source = LiveLaunchSource {
        constructor,
        owner,
        trajectory,
        processed: &binding.processed,
    };
    fill_from_source(&mut source, output)
}

// Read boundary only: all production reads below borrow the same source owners.
// This lets focused tests fail individual reads without creating another runtime owner.
trait LaunchSource {
    type Error;
    fn board(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn com(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn animated_com(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn animated_board(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn orientation(&mut self, inverse: bool) -> Result<SkeletonRootMatrix, Self::Error>;
    fn velocity(&mut self, before_jump: bool) -> Result<SkeletonRootVector, Self::Error>;
    fn time_delta(&mut self) -> Result<f32, Self::Error>;
    fn flags(&mut self) -> Result<u32, Self::Error>;
    fn cone_angle(&mut self, offset: usize) -> Result<f32, Self::Error>;
}
fn fill_from_source<S: LaunchSource>(
    source: &mut S,
    output: &mut RetainedTrajectoryLaunchInfo,
) -> Result<(), S::Error> {
    output.board_position = source.board()?;
    output.com_position = source.com()?;
    output.animated_com_position = source.animated_com()?;
    output.animated_board_position = source.animated_board()?;
    output.reckoning_orientation = source.orientation(false)?;
    output.reckoning_orientation_inverse = source.orientation(true)?;
    output.start_velocity = source.velocity(false)?;
    output.velocity_before_jump = source.velocity(true)?;
    output.time_delta = source.time_delta()?;
    output.num_trajectories = if source.flags()? & 0x2000 == 0 { 1 } else { 7 };
    output.cone_angle_x = source.cone_angle(0x248)?;
    output.cone_angle_z = source.cone_angle(0x240)?;
    Ok(())
}
struct LiveLaunchSource<'a> {
    constructor: &'a SkeletonConstructorRuntime,
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    trajectory: &'a WallRideTrajectoryAttributes,
    processed:
        &'a Arc<std::sync::Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>>,
}
impl LaunchSource for LiveLaunchSource<'_> {
    type Error = SkeletonLaunchInfoError;
    fn board(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        use SkeletonLaunchInfoError::{Missing, Poisoned};
        // 460c8 then460e0: distinct full-vector copies, including retained W knowledge.
        Ok(self
            .processed
            .lock()
            .map_err(|_| Poisoned("Processed"))?
            .deck_transform()
            .ok_or(Missing("Processed.deck"))?
            .columns()[3])
    }
    fn com(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        use SkeletonLaunchInfoError::{Missing, Poisoned};
        Ok(self
            .processed
            .lock()
            .map_err(|_| Poisoned("Processed"))?
            .prior_output()
            .centre_of_mass()
            .ok_or(Missing("Processed.COM"))?
            .value)
    }
    fn animated_com(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        let outer = self
            .constructor
            .outer
            .as_ref()
            .ok_or(SkeletonLaunchInfoError::Missing("Skeleton outer"))?;
        Ok(outer.centre_of_mass_local)
    }
    fn animated_board(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self
            .constructor
            .outer
            .as_ref()
            .ok_or(SkeletonLaunchInfoError::Missing("Skeleton outer"))?
            .deck_local)
    }
    fn orientation(&mut self, inverse: bool) -> Result<SkeletonRootMatrix, Self::Error> {
        use SkeletonLaunchInfoError::{Missing, Poisoned};
        // 46104 captures the SAME Skeleton/Reckoning pointer. Constructor values are
        // legitimate even before a completed grounded update.
        let retained = self
            .owner
            .reckoning()
            .lock()
            .map_err(|_| Poisoned("Reckoning"))?;
        if !Arc::ptr_eq(retained.processed(), self.processed) {
            return Err(SkeletonLaunchInfoError::ForeignOwner);
        }
        let reckoning = retained.reckoning().ok_or(Missing("Reckoning"))?;
        Ok(if inverse {
            *reckoning.launch_inverse_transform()
        } else {
            *reckoning.transform()
        })
    }
    fn velocity(&mut self, before_jump: bool) -> Result<SkeletonRootVector, Self::Error> {
        use SkeletonLaunchInfoError::{Missing, Poisoned};
        if before_jump {
            return Ok(self
                .processed
                .lock()
                .map_err(|_| Poisoned("Processed"))?
                .prepare_output()
                .velocity()
                .ok_or(Missing("Processed2c0"))?
                .value);
        }
        Ok(self
            .processed
            .lock()
            .map_err(|_| Poisoned("Processed"))?
            .prior_output()
            .velocity()
            .ok_or(Missing("Processed190"))?
            .value)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        use SkeletonLaunchInfoError::{Missing, Poisoned};
        self.processed
            .lock()
            .map_err(|_| Poisoned("Processed"))?
            .physics_state_fields()
            .time_delta
            .ok_or(Missing("Processed93c"))
    }
    fn flags(&mut self) -> Result<u32, Self::Error> {
        use SkeletonLaunchInfoError::{Missing, Poisoned};
        let (flags, known) = self
            .processed
            .lock()
            .map_err(|_| Poisoned("Processed"))?
            .known_flags_8c4();
        if known & 0x2000 == 0 {
            return Err(Missing("Processed8c4/2000"));
        }
        Ok(flags)
    }
    fn cone_angle(&mut self, offset: usize) -> Result<f32, Self::Error> {
        let layout = self
            .trajectory
            .capture()
            .map_err(SkeletonLaunchInfoError::Attribute)?;
        Ok(layout_word(layout.as_ref(), offset))
    }
}

fn layout_word(layout: Option<&AttributeRuntimeData>, offset: usize) -> f32 {
    layout.map_or(0.0, |data| {
        f32::from_be_bytes(
            data.bytes()[offset..offset + 4]
                .try_into()
                .expect("captured 0x24c layout"),
        )
    })
}
