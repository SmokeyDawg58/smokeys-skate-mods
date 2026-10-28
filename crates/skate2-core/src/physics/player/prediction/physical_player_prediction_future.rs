//! Typed access to existing native future owners; no second result allocation.
use super::PredictionRuntimeError;
use crate::{
    camera_prediction::CameraPrediction,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_footplant::FootPlantPrediction,
    physical_player_trajectory_owners::{RetainedPredictionFuture, RetainedPredictionResult},
};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone, Copy, Debug)]
pub(crate) enum PredictionTarget {
    Onboard(usize),
    Offboard(usize),
    WipeoutGround,
    FootPlant,
    HandPlant(usize),
    HandPlantSurface,
}
/// An address of one retained result inside an existing player or camera owner.
/// The World predictor queue never owns a second future or keeps its producer alive.
#[derive(Clone)]
pub(super) enum PredictionDestination {
    Player {
        owner: Weak<PhysicalPlayerAssemblyOwner>,
        target: PredictionTarget,
    },
    Camera(Weak<Mutex<CameraPrediction>>),
}
pub(super) enum PredictionFutureMut<'a> {
    Onboard(&'a mut RetainedPredictionFuture),
    Wipeout(&'a mut FootPlantPrediction),
    Camera(&'a mut FootPlantPrediction),
    FootPlant(&'a mut FootPlantPrediction),
}
impl PredictionFutureMut<'_> {
    pub(super) fn ready(&self) -> bool {
        match self {
            Self::Onboard(f) => f.ready,
            Self::Wipeout(f) | Self::Camera(f) | Self::FootPlant(f) => f.ready,
        }
    }
    pub(super) fn set_ready(&mut self, value: bool) {
        match self {
            Self::Onboard(f) => f.ready = value,
            Self::Wipeout(f) | Self::Camera(f) | Self::FootPlant(f) => f.ready = value,
        }
    }
    pub(super) fn reset_result(&mut self, result: RetainedPredictionResult) {
        match self {
            Self::Onboard(f) => f.result = result,
            Self::Wipeout(f) | Self::Camera(f) | Self::FootPlant(f) => {
                f.collision_position = Some(result.position);
                f.collision_normal = Some(result.normal);
                f.ground_normal = Some(result.ground_normal);
                f.collision_time = Some(result.time);
                f.frame = Some(result.frame);
                f.surface = Some(result.surface);
                f.trajectory = result.trajectory;
                f.draw_true = Some(result.draw_true);
                f.radius = Some(result.radius);
                f.max_error_start = Some(result.max_error_start);
                f.max_error_end = Some(result.max_error_end);
            }
        }
    }
    pub(super) fn copy_collision(
        &mut self,
        result: crate::collision_query_batches::trajectory::TrajectoryResult,
    ) {
        match self {
            Self::Onboard(f) => {
                let r = &mut f.result;
                r.position = result.position;
                r.normal = result.normal;
                r.ground_normal = result.ground_normal;
                r.time = result.time;
                r.frame = result.frame;
                r.surface = result.surface;
            }
            Self::Wipeout(f) | Self::Camera(f) | Self::FootPlant(f) => {
                f.collision_position = Some(result.position);
                f.collision_normal = Some(result.normal);
                f.ground_normal = Some(result.ground_normal);
                f.collision_time = Some(result.time);
                f.frame = Some(result.frame);
                f.surface = Some(result.surface);
            }
        }
    }
}
pub(super) fn with_future<N, T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    target: PredictionTarget,
    operation: impl FnOnce(PredictionFutureMut<'_>) -> Result<T, PredictionRuntimeError<N>>,
) -> Result<T, PredictionRuntimeError<N>> {
    use PredictionRuntimeError as E;
    match target {
        PredictionTarget::HandPlant(slot) => {
            let mut hand = owner.handplant().lock().map_err(|_| E::Poisoned)?;
            let selector = &mut hand
                .as_mut()
                .ok_or(E::Missing("HandPlant manager"))?
                .trajectory_selector;
            let future = selector
                .queries
                .get_mut(slot)
                .ok_or(E::Missing("HandPlant future slot"))?;
            operation(PredictionFutureMut::Onboard(future))
        }
        PredictionTarget::HandPlantSurface => {
            let mut hand = owner.handplant().lock().map_err(|_| E::Poisoned)?;
            let surface = hand
                .as_mut()
                .ok_or(E::Missing("HandPlant manager"))?
                .trajectory_selector
                .surface_query
                .as_mut()
                .ok_or(E::Missing("HandPlant surface query"))?;
            operation(PredictionFutureMut::FootPlant(&mut surface.query))
        }
        PredictionTarget::Onboard(slot) => {
            let mut trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
            let selector = trajectories
                .onboard
                .as_mut()
                .ok_or(E::Missing("onboard selector"))?;
            let future = selector
                .queries
                .get_mut(slot)
                .ok_or(E::Missing("onboard future slot"))?;
            operation(PredictionFutureMut::Onboard(future))
        }
        PredictionTarget::Offboard(slot) => {
            let processed = std::sync::Arc::clone(
                owner
                    .reckoning()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .processed(),
            );
            let mut trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
            let selector = trajectories
                .offboard
                .as_mut()
                .ok_or(E::Missing("offboard selector"))?;
            if !selector.bindings.matches(owner, &processed) {
                return Err(E::Missing("foreign offboard future"));
            }
            let future = selector
                .queries
                .get_mut(slot)
                .ok_or(E::Missing("offboard future slot"))?;
            // Same RetainedPredictionFuture representation, no copied future.
            operation(PredictionFutureMut::Onboard(future))
        }
        PredictionTarget::WipeoutGround => {
            let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let wipeout = state
                .wipeout_ground
                .as_mut()
                .ok_or(E::Missing("WipeoutGround future"))?;
            if !wipeout
                .bindings
                .owner
                .upgrade()
                .is_some_and(|p| Arc::ptr_eq(&p, owner))
            {
                return Err(E::Missing("foreign WipeoutGround future"));
            }
            // The retained future survives selected-state changes and re-entry.
            operation(PredictionFutureMut::Wipeout(&mut wipeout.fields.prediction))
        }
        PredictionTarget::FootPlant => {
            // FootPlantManager+10: SetupFakeBodyPrediction82e4bb88 from
            // UpdatePrediction82dc1700; the manager outlives both plant states.
            let mut foot = owner.footplant().lock().map_err(|_| E::Poisoned)?;
            let prediction = foot
                .try_owned_prediction(owner)
                .map_err(|_| E::Missing("FootPlant future"))?;
            operation(PredictionFutureMut::FootPlant(prediction))
        }
    }
}

impl PredictionDestination {
    pub(super) fn with_future<N, T>(
        &self,
        operation: impl FnOnce(PredictionFutureMut<'_>) -> Result<T, PredictionRuntimeError<N>>,
    ) -> Result<T, PredictionRuntimeError<N>> {
        match self {
            Self::Player { owner, target } => {
                let owner = owner
                    .upgrade()
                    .ok_or(PredictionRuntimeError::Missing("future owner"))?;
                with_future(&owner, *target, operation)
            }
            Self::Camera(owner) => {
                let owner = owner
                    .upgrade()
                    .ok_or(PredictionRuntimeError::Missing("camera future owner"))?;
                let mut owner = owner.lock().map_err(|_| PredictionRuntimeError::Poisoned)?;
                operation(PredictionFutureMut::Camera(&mut owner.future))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collision_query_batches::trajectory::BallisticTrajectoryInput,
        physical_player_footplant::RetainedTrajectory,
        skeleton_root_transforms::SkeletonRootVector as V,
    };
    #[test]
    fn actual_optional_future_reset_does_not_publish_ready_and_preserves_unknown_input_w() {
        // Constructor-unwritten result fields start unknown; submission is their producer.
        let mut stored = FootPlantPrediction {
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
        };
        let input = BallisticTrajectoryInput {
            position: V {
                xyz: [1.0, 2.0, 3.0],
                w: None,
            },
            velocity: V {
                xyz: [4.0, 5.0, 6.0],
                w: None,
            },
            gravity: V::known([0.0, -9.8, 0.0, 0.0]),
            radius: 0.3,
            duration: 2.0,
            maximum_error_start: 1.0,
            maximum_error_end: 1.0,
        };
        super::super::reset_future(&mut PredictionFutureMut::Wipeout(&mut stored), input, false);
        assert!(!stored.ready);
        assert_eq!(
            stored.collision_time.unwrap().map(f32::to_bits),
            [(-1.0_f32).to_bits(); 4]
        );
        assert!(stored.trajectory.position.w.is_none());
        assert!(stored.trajectory.velocity.w.is_none());
        assert_eq!(stored.collision_position.unwrap().w, Some(0.0));
        let collision = crate::collision_query_batches::trajectory::TrajectoryResult {
            position: V {
                xyz: [7.0, 8.0, 9.0],
                w: None,
            },
            normal: V::known([0.0, 1.0, 0.0, 0.0]),
            ground_normal: V::known([0.0, 1.0, 0.0, 0.0]),
            time: [0.5; 4],
            frame: 2,
            surface: 3,
        };
        let mut future = PredictionFutureMut::Wipeout(&mut stored);
        future.copy_collision(collision);
        assert!(!future.ready());
        future.set_ready(true);
        assert!(stored.ready);
        assert!(stored.collision_position.unwrap().w.is_none());
        assert_eq!(stored.frame, Some(2));
        assert_eq!(stored.surface, Some(3));
        assert_eq!(
            stored.trajectory.position.xyz.map(f32::to_bits),
            input.position.xyz.map(f32::to_bits)
        );
    }
}
