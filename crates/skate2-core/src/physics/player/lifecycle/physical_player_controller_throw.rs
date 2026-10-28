//! Actual LetGoOfSkateboard82dd04b8..05c8 throw continuation.
use super::{
    PhysicalAssemblyWorld,
    controller_active::{curve, scalar},
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle as Handle,
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skateboard_controller::{SkateboardController, throw::ThrowSource},
    skateboard_foot_forces::SkateboardFootForce,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum ControllerThrowError {
    Poisoned,
    Missing(&'static str),
    Service(&'static str, String),
}
use ControllerThrowError as E;
fn service(site: &'static str, error: impl std::fmt::Debug) -> E {
    E::Service(site, format!("{error:?}"))
}
struct Source<'a> {
    controller: &'a SkateboardController,
    processed: &'a Mutex<SkeletonProcessedContact>,
}
impl ThrowSource for Source<'_> {
    type Error = E;
    fn effective_forward(&mut self) -> Result<V, E> {
        Ok(self
            .processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .ground_position_fields()
            .effective_animation
            .as_ref()
            .ok_or(E::Missing("effective animation"))?
            .columns()[2])
    }
    fn com_velocity(&mut self) -> Result<V, E> {
        Ok(self
            .processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .centre_of_mass_velocity()
            .ok_or(E::Missing("COM velocity260"))?
            .value)
    }
    fn board_velocity(&mut self) -> Result<V, E> {
        Ok(self
            .processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .velocity()
            .ok_or(E::Missing("board velocity190"))?
            .value)
    }
    fn scalar(&mut self, key: u64) -> Result<f32, E> {
        scalar(self.controller, key).map_err(|e| service("throw scalar", e))
    }
    fn curve(&mut self, key: u64, input: f32) -> Result<f32, E> {
        curve(self.controller, key, input).map_err(|e| service("throw graph", e))
    }
    fn mass(&mut self) -> Result<f32, E> {
        self.processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physical_mass()
            .ok_or(E::Missing("mass970"))
    }
}
impl PhysicalAssemblyWorld {
    pub(super) fn try_throw_retained_controller(
        &mut self,
        board: &Handle,
        skeleton: &Handle,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        controller: &mut SkateboardController,
    ) -> Result<(), E> {
        let mut numeric = HostReckoningNumeric;
        let mut rounding = HostReckoningSinCosNumeric;
        let force = crate::skateboard_controller::throw::try_compute(
            &mut Source {
                controller,
                processed,
            },
            &mut numeric,
            &mut rounding,
        )
        .map_err(|e| service("ComputeSkateboardThrowParameters", e))?;
        // Righting captures Processed forward BEFORE sampling the live non-Raw
        // animation root; target publication precedes the ground-normal read.
        let forward = Source {
            controller,
            processed,
        }
        .effective_forward()?;
        let target = self
            .active_animation_root(skeleton)
            .map_err(|e| service("animation root", e))?;
        controller
            .try_active_retrieval_mut::<()>()
            .map_err(|e| service("retrieval", e))?
            .target = target;
        let normal = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .ground_normal()
            .ok_or(E::Missing("ground normal1d0"))?
            .value;
        let mut angles = [V::known([0.0; 4]); 3];
        crate::skateboard_controller::righting::try_compute(
            forward,
            target.columns()[0],
            target.columns()[2],
            normal,
            &mut angles,
            |key| scalar(controller, key),
            &mut numeric,
            &mut rounding,
        )
        .map_err(|e| service("ComputeRightingForces", e))?;
        self.try_add_board_force(
            board,
            6,
            SkateboardFootForce {
                world_space_force: force,
                local_space_position: V::known([0.0; 4]),
            },
        )
        .map_err(|e| service("speed conservation force", e))?;
        for angle in angles {
            self.try_controller_board_angular_exact(board, angle, &mut numeric)
                .map_err(|e| service("TargetAngularDisplacementExact", e))?;
        }
        Ok(())
    }
}
