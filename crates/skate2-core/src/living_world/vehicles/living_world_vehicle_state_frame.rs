//! Vehicle pre-state suffix82d252ec..534c and BasicEntity.UpdateState82e4c1d0.
//! Reuses existing route/motion/graph owners. Caller still owes BasicEntity's
//! plugin-removal prefix and PostStateGraphUpdate; this is not a whole frame.
use super::{
    Vehicle,
    perception::PerceptionError,
    skitch_perception::{SkitchActors, SkitchPerceptionError},
};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{
        vehicle_graph::runtime::VehicleBehaviourFrame,
        vehicle_motion::{
            attributes::VehicleAttributeError,
            driving::{DrivingError, DrivingVehicles},
            obstacles::FrontEntityGeometry,
            signals::{HonkingPedestrians, HornObstacle, SignalError},
        },
    },
    living_world_mesh_store::MeshStore,
    living_world_rain::RainMan,
    native_skater_random::{
        NativeRandomGate, NativeSkaterRandom,
        rw_math::{RwRandomLockError, global_next},
    },
    reckoning_numeric::ReckoningNumeric,
};
use std::convert::Infallible;

enum GraphOperation {
    Update(f32),
    EndAll,
}

#[derive(Debug)]
pub(crate) enum VehiclePreStateError<V, A, P, N> {
    Driving(DrivingError<V, N>),
    Attribute(VehicleAttributeError<RwRandomLockError>),
    Perception(PerceptionError<N>),
    Skitch(SkitchPerceptionError<A, N>),
    Horn(SignalError<AttributeRuntimeError, N, P>),
    Lights(SignalError<AttributeRuntimeError, Infallible, Infallible>),
}

impl Vehicle {
    fn front_entity_geometry(&self) -> Option<FrontEntityGeometry> {
        self.entity_in_vision.then(|| FrontEntityGeometry {
            position: self.core.target_transform[3],
            nearest_position: self.nearest_vision_position,
            // Native RainPoint producers splat radius. Scalar consumers use
            // X; perception itself retained all4 lanes and vector comparisons.
            nearest_radius: self.nearest_vision_radius[0],
            vehicle_length: self.length(),
        })
    }

    /// Call only after LWBasicEntity.PreStateGraphUpdate82e4c248 completes.
    /// The original forced-behavior flag belongs to VehicleRoute, not another
    /// boolean here. Decisions consume rw::math global RNG, NOT Manager RNG.
    pub(crate) fn pre_state_graph_vehicle_suffix<V, A, P, N>(
        &mut self,
        dt: f32,
        rain: &mut RainMan,
        vehicles: &mut V,
        actors: &mut A,
        pedestrians: &mut P,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), VehiclePreStateError<V::Error, A::Error, P::Error, N::Error>>
    where
        V: DrivingVehicles,
        A: SkitchActors,
        P: HonkingPedestrians,
        N: ReckoningNumeric,
    {
        use VehiclePreStateError as E;
        let length = self.length();
        self.route
            .reevaluate_action_decisions(
                &mut self.motion,
                self.id,
                length,
                rain,
                vehicles,
                &self.characteristics,
                &self.driver,
                &mut global_next,
            )
            .map_err(E::Driving)?;
        if self.route.force_new_behaviours {
            self.route.force_new_behaviours = !self
                .route
                .make_new_behaviour_decision(&mut self.motion, &self.driver, &mut global_next)
                .map_err(E::Attribute)?;
        }
        self.check_entities_against_vision(rain, numeric)
            .map_err(E::Perception)?;
        self.evaluate_skitching_intents(actors, numeric)
            .map_err(E::Skitch)?;
        let observation = self.front_entity_geometry().map(|geometry| HornObstacle {
            geometry,
            compulsory_stop: self.nearest_compulsory_stop,
            pedestrian_id: self.nearest_pedestrian,
        });
        self.signals
            .update_horn(
                &self.motion,
                &self.route,
                self.id,
                observation.as_ref(),
                &self.driver,
                numeric,
                pedestrians,
            )
            .map_err(E::Horn)?;
        self.signals
            .update_lights(
                dt,
                &self.motion,
                &mut self.route.lights,
                &self.characteristics,
                cvars,
            )
            .map_err(E::Lights)?;
        Ok(())
    }

    /// BasicEntity.UpdateState82e4c1d0: no graph is a no-op; otherwise run the
    /// SAME initialized Vehicle.xml controller. No new graph or motion copies.
    pub(crate) fn update_state<V: DrivingVehicles, N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        rain: &mut RainMan,
        mesh: &MeshStore,
        vehicles: &mut V,
        cvars: &InputCvarReferences,
        random: &mut NativeSkaterRandom,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>> {
        self.run_graph_operation(
            GraphOperation::Update(dt),
            rain,
            mesh,
            vehicles,
            cvars,
            random,
            numeric,
        )
    }

    /// BasicEntity.OnCullRequested82e502a8's graph suffix. The caller must
    /// finish the behaviour-sink prefix before invoking this; no graph reset.
    pub(crate) fn end_state_behaviours<V: DrivingVehicles, N: ReckoningNumeric>(
        &mut self,
        rain: &mut RainMan,
        mesh: &MeshStore,
        vehicles: &mut V,
        cvars: &InputCvarReferences,
        random: &mut NativeSkaterRandom,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>> {
        self.run_graph_operation(
            GraphOperation::EndAll,
            rain,
            mesh,
            vehicles,
            cvars,
            random,
            numeric,
        )
    }

    fn run_graph_operation<V: DrivingVehicles, N: ReckoningNumeric>(
        &mut self,
        operation: GraphOperation,
        rain: &mut RainMan,
        mesh: &MeshStore,
        vehicles: &mut V,
        cvars: &InputCvarReferences,
        random: &mut NativeSkaterRandom,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>> {
        if self.graph.is_none() {
            return Ok(());
        }
        let length = self.length();
        let front_entity = self.front_entity_geometry();
        let gate = NativeRandomGate::production_shared();
        let mut frame = VehicleBehaviourFrame {
            motion: &mut self.motion,
            route: &mut self.route,
            entity_id: self.id,
            length,
            rain,
            mesh,
            vehicles,
            characteristics: &self.characteristics,
            driver: &self.driver,
            front_entity,
            cvars,
            random,
            random_gate: &gate,
            numeric,
        };
        let graph = self.graph.as_mut().expect("checked graph");
        match operation {
            GraphOperation::Update(dt) => graph.update(dt, &mut frame),
            GraphOperation::EndAll => graph.end_all_behaviours(&mut frame),
        }
    }
}
