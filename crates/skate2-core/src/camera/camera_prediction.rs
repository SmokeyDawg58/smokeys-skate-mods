//! Retained camera predictors (`Create82e37a88`) use the actual World predictor.
use crate::{
    camera_subject::ZERO,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_grind_services::{LivingWorldGrindServices, PedestrianNearQueryError},
    physical_player_footplant::FootPlantPrediction,
    reckoning_numeric::{ReckoningHostNumericError, ReckoningVector, host_dot3},
    simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices,
    simulation_assembly_backend::constraint_session::prediction::{
        PhysicalPredictionOwner, runtime::PredictionRuntimeError,
    },
    skater_manager_local_identity::{LocalIdentityError, SkaterManagerLocalIdentity},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

pub(crate) struct CameraPrediction {
    pub(crate) world_predictor: Arc<PhysicalPredictionOwner>,
    pub(crate) future: FootPlantPrediction,
}
impl CameraPrediction {
    pub(crate) fn constructed(world_predictor: Arc<PhysicalPredictionOwner>) -> Self {
        Self {
            world_predictor,
            future: FootPlantPrediction::new(),
        }
    }

    /// `TrajectoryPredictor::StartQuery82e32ac0`. The lock is released before
    /// `AddQuery` resets the camera's own retained future through the queue.
    pub(crate) fn try_start_query(
        owner: &Arc<Mutex<Self>>,
        input: &CameraPathInput,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), PredictionRuntimeError<ReckoningHostNumericError>> {
        let profile = std::env::var_os("SKATE2_PROFILE_REDRAW").map(|_| std::time::Instant::now());
        let predictor = Arc::clone(
            &owner
                .lock()
                .map_err(|_| PredictionRuntimeError::Poisoned)?
                .world_predictor,
        );
        let world = services
            .queries
            .as_mut()
            .ok_or(PredictionRuntimeError::Missing("World trajectory queries"))?;
        let dmos = services
            .dmos
            .ok_or(PredictionRuntimeError::Missing("World dynamic pool"))?;
        predictor.try_add_camera_query(
            owner,
            input.position,
            input.velocity,
            input.radius,
            input.lookahead,
            input.in_air,
            world,
            services.ground,
            dmos,
            services.numeric,
        )?;
        if let Some(start) = profile {
            eprintln!(
                "{}",
                format!(
                    "diagnostic camera trajectory: submit={}ms",
                    start.elapsed().as_secs_f64() * 1000.0
                )
            );
        }
        Ok(())
    }
}
pub(crate) struct CameraObstacles {
    pub(crate) position: SkeletonRootVector,
    pub(crate) velocity: SkeletonRootVector,
    pub(crate) radius: f32,
}
impl CameraObstacles {
    /// Embedded server fields in `GameCameraManager82e37c08`.
    pub(crate) const fn constructed() -> Self {
        Self {
            position: ZERO,
            velocity: ZERO,
            radius: 0.0,
        }
    }

    /// `PathEvaluatorObstacleServer::FindObstacles82e3a7a0` stores the input
    /// before sampling the two live CVar targets, pedestrian first.
    fn try_find(
        &mut self,
        input: &CameraPathInput,
        sources: CameraObstacleSources<'_>,
    ) -> Result<CameraDynamicObstacleList, CameraObstacleError> {
        self.position = input.position;
        self.velocity = input.velocity;
        self.radius = input.radius;
        let mut found = CameraDynamicObstacleList::new();
        if sources
            .cvars
            .try_pedestrian_avoidance()
            .map_err(CameraObstacleError::Cvar)?
        {
            let radius = sources
                .cvars
                .try_actor_search_radius()
                .map_err(CameraObstacleError::Cvar)?;
            let mut count = 0;
            let mut ids = [0; 20];
            let mut distances = [0.0; 20];
            let living = sources
                .living_world
                .lock()
                .map_err(|_| CameraObstacleError::OwnerLock)?;
            living.try_get_pedestrians_within_range(
                    [self.position.xyz[0], self.position.xyz[2]],
                    radius,
                    20,
                    &mut count,
                    &mut ids,
                    &mut distances,
                )
                .map_err(CameraObstacleError::Pedestrian)?;
            for id in &ids[..count as usize] {
                let pedestrians = living.pedestrian_entities.as_ref()
                    .ok_or(CameraObstacleError::Pedestrian(PedestrianNearQueryError::MissingEntityMan))?;
                let Some(pedestrian) = pedestrians.get(*id) else { continue; };
                // Core proxy vslot0c -> IsInitialized82d1d248.
                let Some(core) = pedestrian.core.as_ref() else { continue; };
                // LWBasicEntity.GetWidth82e4ff40 returns model dimensions.X,
                // or zero without a model. Native uses the full width here.
                let radius = pedestrian.model.as_ref().map_or(0., |model| model.dimensions[0]);
                if !self.add_body(&mut found, core.transform[3], core.velocity, radius) {
                    break;
                }
            }
        }
        if sources
            .cvars
            .try_skater_avoidance()
            .map_err(CameraObstacleError::Cvar)?
        {
            let radius = sources
                .cvars
                .try_actor_search_radius()
                .map_err(CameraObstacleError::Cvar)?;
            let radius_squared = radius * radius;
            let local = sources
                .skaters
                .try_local_index()
                .map_err(CameraObstacleError::Skater)?;
            let local_sim = sources
                .skaters
                .try_local_sim_index()
                .map_err(CameraObstacleError::Skater)?;
            let count = sources
                .skaters
                .try_count()
                .map_err(CameraObstacleError::Skater)?;
            for index in 0..count {
                let actor = sources
                    .skaters
                    .try_actor(index)
                    .map_err(CameraObstacleError::Skater)?
                    .ok_or(CameraObstacleError::MissingRosterActor(index))?;
                let player = actor
                    .prefix()
                    .map_err(|_| CameraObstacleError::ActorPrefix)?
                    .fields()
                    .ok_or(CameraObstacleError::ActorPrefix)?
                    .player_index as i32;
                if player == local || player == local_sim {
                    continue;
                }
                // Actor::GetPosition 825f9df0 (Skeleton root translation) and
                // GetVelocity 825ed338 (SystemReckoning+0).
                let (position, velocity) = {
                    let physical = actor
                        .cached_physical_allocation()
                        .map_err(|e| CameraObstacleError::SkaterOutput(format!("{e:?}")))?;
                    let output = physical
                        .owner()
                        .phys_out()
                        .lock()
                        .map_err(|_| CameraObstacleError::OwnerLock)?;
                    (
                        output
                            .try_skeleton_output()
                            .map_err(|e| CameraObstacleError::SkaterOutput(format!("{e:?}")))?
                            .skeleton_root_matrix
                            .columns()[3]
                            .operand(),
                        output
                            .try_camera_system_output()
                            .map_err(|e| CameraObstacleError::SkaterOutput(format!("{e:?}")))?
                            .damped_centre_of_mass_velocity
                            .operand(),
                    )
                };
                let separation = [
                    self.position.xyz[0] - position[0],
                    self.position.xyz[1] - position[1],
                    self.position.xyz[2] - position[2],
                    0.,
                ];
                if !(host_dot3(separation, separation) <= radius_squared) {
                    continue;
                }
                if !self.add_body(&mut found, position, velocity, f32::from_bits(0x3f40_0000)) {
                    break;
                }
            }
        }
        Ok(found)
    }

    /// Raw AddPedestrianObstacles82e39fb0..a170 (the decompiler drops this
    /// loop) and AddSkaterObstacles82e3a198's tail: reject receding/behind
    /// candidates, then emit three body spheres.
    fn add_body(&self, found: &mut CameraDynamicObstacleList,
        position: [f32;4], velocity: [f32;4], radius: f32) -> bool {
        let camera_velocity = [self.velocity.xyz[0], self.velocity.xyz[1], self.velocity.xyz[2], 0.];
        let relative = std::array::from_fn(|i| velocity[i] - camera_velocity[i]);
        if host_dot3(relative, camera_velocity) > 0. { return true; }
        let separation = [self.position.xyz[0] - position[0],
            self.position.xyz[1] - position[1], self.position.xyz[2] - position[2], 0.];
        if host_dot3(camera_velocity, separation) > 0. { return true; }
        for height in [0., f32::from_bits(0x3f33_3333), f32::from_bits(0x3fb3_3333)] {
            let centre = if height == 0. { position } else {
                std::array::from_fn(|i| (if i == 1 { 1.0_f32 } else { 0.0_f32 }).mul_add(height, position[i]))
            };
            if !found.add(CameraDynamicObstacle {
                position: SkeletonRootVector::known(centre),
                velocity: SkeletonRootVector::known(velocity), radius,
            }) { return false; }
        }
        true
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CameraObstacleSources<'a> {
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) living_world: &'a Arc<Mutex<LivingWorldGrindServices>>,
    pub(crate) skaters: &'a Arc<SkaterManagerLocalIdentity>,
}

#[derive(Debug)]
pub(crate) enum CameraObstacleError {
    Cvar(InputCvarError),
    OwnerLock,
    Pedestrian(PedestrianNearQueryError),
    Skater(LocalIdentityError),
    MissingRosterActor(usize),
    ActorPrefix,
    SkaterOutput(String),
}

impl std::fmt::Display for CameraObstacleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "camera dynamic obstacle server: {self:?}")
    }
}

/// `Iris::Obstacle` is three source fields (0x30 bytes): position, velocity,
/// and radius. The vector W lanes are not consumed by `SphereTest82e3c9b0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CameraDynamicObstacle {
    pub(crate) position: SkeletonRootVector,
    pub(crate) velocity: SkeletonRootVector,
    pub(crate) radius: f32,
}

/// The native dynamic evaluator uses a stack-local `fixed_vector<Obstacle,50>`.
pub(crate) struct CameraDynamicObstacleList {
    records: [CameraDynamicObstacle; 50],
    count: usize,
}

impl CameraDynamicObstacleList {
    pub(crate) const fn new() -> Self {
        const EMPTY: CameraDynamicObstacle = CameraDynamicObstacle {
            position: ZERO,
            velocity: ZERO,
            radius: 0.0,
        };
        Self {
            records: [EMPTY; 50],
            count: 0,
        }
    }

    /// `PathEvaluatorObstacleServer::AddObstacle82e39d90` returns false when
    /// the fixed list is full; the caller then stops adding that actor.
    pub(crate) fn add(&mut self, obstacle: CameraDynamicObstacle) -> bool {
        if self.count == self.records.len() {
            return false;
        }
        self.records[self.count] = obstacle;
        self.count += 1;
        true
    }

    fn records(&self) -> &[CameraDynamicObstacle] {
        &self.records[..self.count]
    }
}

/// `PathEvaluator::SphereTest82e3c9b0`. It reports time of closest approach,
/// rather than the first point where two sphere surfaces touch. The PPC uses
/// `fcmpu` followed by `bge` at each rejection, so unordered values take the
/// branch because LT is clear. `host_dot3` has the established host reduction policy;
/// exact Xenon SIMD rounding remains an explicit numeric limitation.
fn sphere_collision_time(input: &CameraPathInput, obstacle: &CameraDynamicObstacle) -> Option<f32> {
    let separation =
        std::array::from_fn::<_, 3, _>(|i| obstacle.position.xyz[i] - input.position.xyz[i]);
    let relative_velocity =
        std::array::from_fn::<_, 3, _>(|i| obstacle.velocity.xyz[i] - input.velocity.xyz[i]);
    let lanes = |v: [f32; 3]| [v[0], v[1], v[2], 0.0];
    let radius = input.radius + obstacle.radius;
    let distance_minus_radius = host_dot3(lanes(separation), lanes(separation)) - radius * radius;
    if distance_minus_radius < 0.0 {
        return Some(0.0);
    }
    let closing = host_dot3(lanes(separation), lanes(relative_velocity));
    if !(closing < 0.0) {
        return None;
    }
    let speed_squared = host_dot3(lanes(relative_velocity), lanes(relative_velocity));
    let time = (-1.0 / speed_squared) * closing;
    let closest_distance = time.mul_add(closing, distance_minus_radius);
    if closest_distance < 0.0 {
        Some(time)
    } else {
        None
    }
}
pub(crate) struct CameraPathInput {
    pub(crate) position: SkeletonRootVector,
    pub(crate) velocity: SkeletonRootVector,
    pub(crate) radius: f32,
    pub(crate) lookahead: f32,
    pub(crate) debug: bool,
    pub(crate) in_air: bool,
}
pub(crate) struct CameraPathEvaluator {
    pub(crate) obstacles: Arc<Mutex<CameraObstacles>>,
    pub(crate) prediction: Arc<Mutex<CameraPrediction>>,
    pub(crate) static_query_state: u32,
    pub(crate) input: Option<CameraPathInput>,
    pub(crate) collision_time: f32,
    pub(crate) last_valid_collision_time: f32,
    pub(crate) valid: bool,
    pub(crate) output_was_in_air: Option<bool>,
    pub(crate) was_in_air: bool,
}

#[derive(Debug)]
pub(crate) enum CameraPathEvaluatorError {
    MissingInput,
    OwnerLock,
    MissingCollisionTime,
    CollisionTimeOutOfRange(f32),
    Prediction(PredictionRuntimeError<ReckoningHostNumericError>),
    Obstacle(CameraObstacleError),
    PendingDynamicEvaluator,
}

impl std::fmt::Display for CameraPathEvaluatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingInput => f.write_str("camera PathEvaluator input missing"),
            Self::OwnerLock => f.write_str("camera trajectory future lock poisoned"),
            Self::MissingCollisionTime => f.write_str("ready camera future has no collision time"),
            Self::CollisionTimeOutOfRange(value) => {
                write!(
                    f,
                    "camera predicted collision time outside source assertion: {value}"
                )
            }
            Self::Prediction(error) => write!(f, "camera PathEvaluator query: {error}"),
            Self::Obstacle(error) => write!(f, "{error}"),
            Self::PendingDynamicEvaluator => {
                f.write_str("camera PathEvaluator dynamic obstacle query pending")
            }
        }
    }
}

impl CameraPathEvaluator {
    /// `Rig82e45704..5724` and its two following allocations. Input and output's
    /// second bool are untouched; the separate owner's `was_in_air` is false.
    pub(crate) fn constructed(
        obstacles: Arc<Mutex<CameraObstacles>>,
        prediction: Arc<Mutex<CameraPrediction>>,
    ) -> Self {
        Self {
            obstacles,
            prediction,
            static_query_state: 0,
            input: None,
            collision_time: f32::MAX,
            last_valid_collision_time: f32::MAX,
            valid: false,
            output_was_in_air: None,
            was_in_air: false,
        }
    }

    /// Prefix of `PathEvaluator::Update82e46420`: reset the current result,
    /// update the static predictor, then stop at the missing dynamic evaluator.
    /// The source copies `last_valid_collision_time` only after dynamic update.
    pub(crate) fn try_update_prefix(
        &mut self,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), CameraPathEvaluatorError> {
        self.begin_update(services)?;
        Err(CameraPathEvaluatorError::PendingDynamicEvaluator)
    }

    fn begin_update(
        &mut self,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), CameraPathEvaluatorError> {
        self.collision_time = f32::MAX;
        self.valid = false;
        self.try_update_static_evaluator(services)
    }

    /// `PathEvaluator::Update82e46420` once the actual obstacle server has
    /// produced its pedestrian-first, skater-second fixed list. The selected
    /// live camera still calls `try_update_prefix` until that server is owned.
    pub(crate) fn try_update_with_obstacles(
        &mut self,
        services: &mut CameraLineServices<'_>,
        obstacles: &CameraDynamicObstacleList,
    ) -> Result<(), CameraPathEvaluatorError> {
        self.begin_update(services)?;
        self.update_dynamic_evaluator(obstacles)?;
        if self.valid {
            self.last_valid_collision_time = self.collision_time;
        }
        Ok(())
    }

    /// Full native update against the retained shared obstacle server.
    pub(crate) fn try_update_live(
        &mut self,
        services: &mut CameraLineServices<'_>,
        sources: CameraObstacleSources<'_>,
    ) -> Result<(), CameraPathEvaluatorError> {
        self.begin_update(services)?;
        let input = self
            .input
            .as_ref()
            .ok_or(CameraPathEvaluatorError::MissingInput)?;
        let obstacles = self
            .obstacles
            .lock()
            .map_err(|_| CameraPathEvaluatorError::OwnerLock)?
            .try_find(input, sources)
            .map_err(CameraPathEvaluatorError::Obstacle)?;
        self.update_dynamic_evaluator(&obstacles)?;
        if self.valid {
            self.last_valid_collision_time = self.collision_time;
        }
        Ok(())
    }

    /// `PathEvaluator::GetCollisionTime82e42048` then
    /// `UpdateDynamicEvaluator82e45f00`. Equal and later times leave the
    /// previous result; the current output becomes valid only below lookahead.
    fn update_dynamic_evaluator(
        &mut self,
        obstacles: &CameraDynamicObstacleList,
    ) -> Result<(), CameraPathEvaluatorError> {
        let input = self
            .input
            .as_ref()
            .ok_or(CameraPathEvaluatorError::MissingInput)?;
        let mut collision_time = f32::MAX;
        for obstacle in obstacles.records() {
            if let Some(time) = sphere_collision_time(input, obstacle) {
                if time < input.lookahead && time < collision_time {
                    collision_time = time;
                }
            }
        }
        if collision_time < input.lookahead {
            let previous = self.collision_time;
            self.valid = true;
            // Native `fsel` selects the candidate when previous-candidate is
            // nonnegative, including a tie.
            self.collision_time = if previous - collision_time >= 0.0 {
                collision_time
            } else {
                previous
            };
        }
        Ok(())
    }

    /// `PathEvaluator::UpdateStaticEvaluator82e3c880`: one pass submits, a later
    /// pass polls the same retained future. State values other than 0/1 do
    /// nothing, matching the source's unsigned state comparison and branch.
    fn try_update_static_evaluator(
        &mut self,
        services: &mut CameraLineServices<'_>,
    ) -> Result<(), CameraPathEvaluatorError> {
        match self.static_query_state {
            0 => {
                let input = self
                    .input
                    .as_ref()
                    .ok_or(CameraPathEvaluatorError::MissingInput)?;
                CameraPrediction::try_start_query(&self.prediction, input, services)
                    .map_err(CameraPathEvaluatorError::Prediction)?;
                self.static_query_state = 1;
                self.was_in_air = input.in_air;
            }
            1 => {
                let collision_time = {
                    let prediction = self
                        .prediction
                        .lock()
                        .map_err(|_| CameraPathEvaluatorError::OwnerLock)?;
                    if !prediction.future.ready {
                        return Ok(());
                    }
                    let collision_time = prediction
                        .future
                        .collision_time
                        .ok_or(CameraPathEvaluatorError::MissingCollisionTime)?[0];
                    // `TrajectoryPredictor::GetPredictedCollisionTime82e355f8`
                    // asserts this before returning the retained result.
                    if collision_time.abs() >= 100.0 {
                        return Err(CameraPathEvaluatorError::CollisionTimeOutOfRange(
                            collision_time,
                        ));
                    }
                    collision_time
                };
                let candidate = if collision_time < 0.0 {
                    f32::MAX
                } else {
                    collision_time
                };
                let previous = self.collision_time;
                self.valid = true;
                self.static_query_state = 0;
                self.output_was_in_air = Some(self.was_in_air);
                // `fsel` at 82e3c924 chooses the candidate on a nonnegative
                // previous-minus-candidate result, including an equal time.
                self.collision_time = if previous - candidate >= 0.0 {
                    candidate
                } else {
                    previous
                };
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "camera_prediction_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "camera_dynamic_obstacle_tests_20260923.rs"]
mod dynamic_obstacle_tests;
