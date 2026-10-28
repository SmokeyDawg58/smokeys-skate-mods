//! Native StepSelector trajectory jobs827c2910/827c2c10.
//! Uses the existing trajectory walk and the actual published ground owner.
use crate::{
    collision_query_batches::trajectory::TrajectoryResult,
    physical_player_footplant::RetainedTrajectory,
    physics::{
        BallisticTrajectory, TrajectoryCollisionWorld, TrajectoryPredictionError,
        TrajectoryPredictionQuery, predict_ballistic_trajectory_with_numeric,
    },
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector as Vector,
    static_ground_pool::StaticGroundPool,
    step_selector_geometry::{
        StepGeometryAnalyzer, StepGeometryError, StepGeometryInput, StepGeometryResult,
        StepGeometryTriangle, StepGeometryTuning,
    },
};
use skate2_formats::collision::CollisionError;

const UP: Vector = Vector::known([0.0, 1.0, 0.0, 0.0]);
// Global constructor82f6aa88; this is separate from Math::VECTOR3_GRAVITY.
pub(crate) const STEP_GRAVITY: Vector = Vector::known([0.0, f32::from_bits(0xc11c_f5c3), 0.0, 0.0]);

pub(crate) trait StepQueryTuning: StepGeometryTuning {
    fn trajectory_radius(&mut self) -> Result<f32, Self::Error>;
    fn test_height(&mut self) -> Result<f32, Self::Error>;
    fn test_radius(&mut self) -> Result<f32, Self::Error>;
    fn step_radius(&mut self) -> Result<f32, Self::Error>;
    fn normal_y_threshold(&mut self) -> Result<f32, Self::Error>;
    fn sprint_trajectory_count(&mut self) -> Result<i32, Self::Error>;
    fn maximum_time(&mut self) -> Result<f32, Self::Error>;
    /// Bind existing reckoning_sin_cos::try_sin_cos, not host sin/cos.
    fn sin_cos(&mut self, angle: f32) -> Result<(f32, f32), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum StepQueryError<G, N, T> {
    Geometry(G),
    Numeric(N),
    Tuning(T),
    IterationAssertion,
    InputCountAssertion,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StepTrajectoryResult {
    pub(crate) trajectory: RetainedTrajectory,
    pub(crate) collision: TrajectoryResult,
    pub(crate) position: Vector,
    pub(crate) normal: Vector,
    pub(crate) surface: u32,
    pub(crate) priority: i32,
    pub(crate) hit_times: [f32; 5],
    pub(crate) valid: bool,
}
impl StepTrajectoryResult {
    pub(crate) fn new() -> Self {
        Self {
            trajectory: RetainedTrajectory::constructed(),
            collision: TrajectoryResult::initialized(),
            position: Vector::known([0.0; 4]),
            normal: UP,
            surface: 0,
            priority: -1,
            hit_times: [f32::MAX; 5],
            valid: false,
        }
    }
    /// Reset827bb778 deliberately leaves the trajectory itself unchanged.
    pub(crate) fn reset(&mut self) {
        let trajectory = self.trajectory;
        *self = Self::new();
        self.trajectory = trajectory;
    }
}

/// Real static-world geometry; no synthetic plane or successful empty fallback.
pub(crate) struct StepGround<'a>(pub(crate) &'a StaticGroundPool);
impl TrajectoryCollisionWorld for StepGround<'_> {
    type Error = CollisionError;
    fn sweep_sphere_segment(
        &mut self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> Result<Option<crate::physics::TrajectorySweepHit>, Self::Error> {
        self.0.trajectory_sweep_sphere_segment(start, end, radius)
    }
    fn triangles_overlapping_aabb(
        &mut self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        _: usize,
    ) -> Result<Vec<crate::physics::TrajectoryGroundTriangle>, Self::Error> {
        self.0
            .trajectory_ground_triangles_overlapping_aabb(minimum, maximum)
    }
}

/// XYZ-only scratch for the existing analyzer. Its dot3/cross/projection paths
/// never feed W into XYZ; all geometry-derived output W is explicitly unknown.
fn xyz_scratch(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 0.0]
}
fn geometry_vector(v: [f32; 4]) -> Vector {
    Vector {
        xyz: [v[0], v[1], v[2]],
        w: None,
    }
}

/// TestTrajectory827c2910. False means no trajectory collision, NOT no step.
/// On a collision the raw trajectory/result publish before box analysis; a
/// failed step leaves the old step payload and validity untouched.
pub(crate) fn test_trajectory<
    W: TrajectoryCollisionWorld,
    N: ReckoningNumeric,
    T: StepQueryTuning,
>(
    trajectory: RetainedTrajectory,
    maximum_y: f32,
    result: &mut StepTrajectoryResult,
    valid_step: &mut bool,
    world: &mut W,
    numeric: &mut N,
    tuning: &mut T,
) -> Result<bool, StepQueryError<W::Error, N::Error, T::Error>> {
    use StepQueryError::{Geometry, Numeric, Tuning};
    let radius = tuning.trajectory_radius().map_err(Tuning)?;
    *valid_step = false;
    let hit = predict_ballistic_trajectory_with_numeric(
        TrajectoryPredictionQuery {
            trajectory: BallisticTrajectory {
                initial_position: trajectory.position.xyz,
                initial_velocity: trajectory.velocity.xyz,
                gravity: trajectory.gravity.xyz,
                duration: trajectory.time[0],
            },
            radius,
            maximum_error_start: 0.1,
            maximum_error_end: 0.1,
            calculate_ground_normal: false,
        },
        world,
        numeric,
    )
    .map_err(|e| match e {
        TrajectoryPredictionError::Geometry(e) => Geometry(e),
        TrajectoryPredictionError::Numeric(e) => Numeric(e),
    })?;
    let Some(hit) = hit else { return Ok(false) };
    result.trajectory = trajectory;
    result.collision = TrajectoryResult {
        position: Vector {
            xyz: hit.position,
            w: None,
        },
        normal: Vector {
            xyz: hit.normal,
            w: None,
        },
        ground_normal: Vector {
            xyz: hit.ground_normal,
            w: None,
        },
        time: [hit.collision_time; 4],
        frame: hit.collision_frame,
        surface: hit.surface_id,
    };
    let height = tuning.test_height().map_err(Tuning)?;
    let radius = tuning.test_radius().map_err(Tuning)?;
    let [x, y, z] = hit.position;
    let triangles = world
        .triangles_overlapping_aabb(
            [x - radius, y - radius, z - radius],
            [x + radius, y + height, z + radius],
            64,
        )
        .map_err(Geometry)?;
    if !triangles.is_empty() {
        // Source reloads radius after the world box query.
        let radius = tuning.test_radius().map_err(Tuning)?;
        let normal_y_threshold = tuning.normal_y_threshold().map_err(Tuning)?;
        let step_radius = tuning.step_radius().map_err(Tuning)?;
        let mut analyzer = StepGeometryAnalyzer::new(StepGeometryInput {
            collision_position: xyz_scratch(hit.position),
            collision_normal: xyz_scratch(hit.normal),
            radius,
            step_radius,
            normal_y_threshold,
            maximum_y,
        });
        let convert = |e| match e {
            StepGeometryError::Numeric(e) => Numeric(e),
            StepGeometryError::Tuning(e) => Tuning(e),
        };
        for triangle in triangles {
            analyzer
                .add_triangle(
                    StepGeometryTriangle {
                        vertices: triangle.vertices.map(xyz_scratch),
                        surface: triangle.surface_id,
                    },
                    numeric,
                    tuning,
                )
                .map_err(convert)?;
        }
        let mut step = StepGeometryResult::default();
        if analyzer
            .calculate(&mut step, numeric, tuning)
            .map_err(convert)?
        {
            result.position = geometry_vector(step.position);
            result.normal = geometry_vector(step.normal);
            result.surface = step.surface;
            result.valid = true;
            *valid_step = true;
        }
    }
    Ok(true)
}

/// RunTrajectoryTest827c2c10. End point is a native by-value vector. Incremental
/// candidate displacements accumulate: end += decrement*(iteration+priority).
/// If no step succeeds, only collision fields restore to the FIRST hit; the
/// retained trajectory remains the LAST hit's trajectory (native asymmetry).
pub(crate) fn run_trajectory_tests<
    W: TrajectoryCollisionWorld,
    N: ReckoningNumeric,
    T: StepQueryTuning,
>(
    valid_step: &mut bool,
    count: i32,
    decrement: Vector,
    start: Vector,
    mut end: Vector,
    duration: f32,
    maximum_y: f32,
    result: &mut StepTrajectoryResult,
    priority_start: i32,
    world: &mut W,
    numeric: &mut N,
    tuning: &mut T,
) -> Result<bool, StepQueryError<W::Error, N::Error, T::Error>> {
    *valid_step = false;
    if count <= 0 || count + priority_start > 5 || priority_start < 0 {
        return Err(StepQueryError::IterationAssertion);
    }
    let mut first = None;
    let mut iteration = 0;
    while iteration < count && !*valid_step {
        let inverse = refine_drive_reciprocal(
            duration,
            numeric
                .reciprocal_estimate(duration)
                .map_err(StepQueryError::Numeric)?,
        );
        let half_gravity_time = STEP_GRAVITY.map(|g| (g * 0.5) * duration);
        let velocity = end
            .zip(start, |e, s| e - s)
            .map(|v| inverse * v)
            .zip(half_gravity_time, |v, g| v - g);
        if test_trajectory(
            RetainedTrajectory {
                position: start,
                velocity,
                gravity: STEP_GRAVITY,
                time: [duration; 4],
            },
            maximum_y,
            result,
            valid_step,
            world,
            numeric,
            tuning,
        )? {
            if first.is_none() {
                first = Some((iteration, result.collision));
            }
            result.hit_times[(iteration + priority_start) as usize] = result.collision.time[0];
        }
        iteration += 1;
        let scale = (iteration + priority_start) as f32;
        end = decrement.zip(end, |d, e| d.mul_add(scale, e));
    }
    result.priority = if *valid_step {
        iteration + priority_start - 1
    } else if let Some((index, collision)) = first {
        result.collision = collision;
        index + priority_start
    } else {
        5
    };
    Ok(first.is_some())
}

/// ComputeTrajTimeFromApexHeight827be670; literal2g=19.620001 and native
/// reciprocal-gravity word82fef0d0. Two separate estimate calls precede roots.
pub(crate) fn apex_time<N: ReckoningNumeric>(
    start: Vector,
    end: Vector,
    mut apex: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let delta = end.xyz[1] - start.xyz[1];
    if delta > apex {
        apex = delta;
    }
    let up = apex * f32::from_bits(0x419c_f5c3);
    let down = (apex - delta) * f32::from_bits(0x419c_f5c3);
    let up_seed = numeric.reciprocal_sqrt_estimate(up)?;
    let down = if down >= 0.0 { down } else { 0.0 };
    let down_seed = numeric.reciprocal_sqrt_estimate(down)?;
    let up = length_from_squared_and_estimate(up, up_seed);
    let down = length_from_squared_and_estimate(down, down_seed);
    Ok((up + down) * f32::from_bits(0x3dd0_c43d))
}

fn cross(a: Vector, b: Vector) -> Vector {
    use crate::reckoning_numeric::{ReckoningVector, cross_full4};
    let lanes = cross_full4(a.operand(), b.operand());
    Vector {
        xyz: [lanes[0], lanes[1], lanes[2]],
        w: a.w.zip(b.w).map(|_| lanes[3]),
    }
}
fn basis_vector(x: Vector, y: Vector, z: Vector, v: [f32; 3]) -> Vector {
    let first = x.map(|x| x * v[0]);
    let second = y.zip(first, |y, x| y.mul_add(v[1], x));
    z.zip(second, |z, y| z.mul_add(v[2], y))
}
fn yaw(v: Vector, sin: f32, cos: f32) -> Vector {
    // Source matrix columns repeat their X component in W, as heading_yaw.
    basis_vector(
        Vector::known([cos, 0.0, -sin, cos]),
        UP,
        Vector::known([sin, 0.0, cos, sin]),
        v.xyz,
    )
}

/// RunStepSelectorTest827c2f00. Existing animation inputs are the actual source
/// owner representation; do not construct a parallel locomotion input model.
/// All twenty result entries reset, even though only one through four inputs
/// may execute. The batch-wide geometry height is always input[0].max_y.
pub(crate) fn run_step_test<
    W: TrajectoryCollisionWorld,
    N: ReckoningNumeric,
    T: StepQueryTuning,
>(
    inputs: &[crate::actor_animation_construction::AnimationStepInput],
    results: &mut [[StepTrajectoryResult; 4]; 5],
    world: &mut W,
    numeric: &mut N,
    tuning: &mut T,
) -> Result<(), StepQueryError<W::Error, N::Error, T::Error>> {
    use crate::reckoning_numeric::try_normalize_safe;
    use StepQueryError::{Numeric, Tuning};
    if inputs.is_empty() || inputs.len() >= 5 {
        return Err(StepQueryError::InputCountAssertion);
    }
    for row in results.iter_mut() {
        for result in row {
            result.reset();
        }
    }
    let zero = Vector::known([0.0; 4]);
    let mut previous = zero;
    let mut valid = true;
    for (index, input) in inputs.iter().enumerate() {
        if !valid {
            break;
        }
        let count = if input.flags & 0x10 != 0 {
            tuning.sprint_trajectory_count().map_err(Tuning)?
        } else {
            5
        };
        if input.flags & 8 == 0 || index == 0 {
            previous = input.reference;
        }
        let start = previous.zip(input.reference_offset, |a, b| a + b);
        let parameter = input.parameter_vector;
        let (mut endpoint, duration) = if input.flags & 2 != 0 {
            let half_squared = (input.parameter * input.parameter) * 0.5;
            let linear = parameter.zip(start, |v, p| v.mul_add(input.parameter, p));
            (
                STEP_GRAVITY.zip(linear, |g, p| g.mul_add(half_squared, p)),
                input.parameter,
            )
        } else {
            let endpoint = start.zip(parameter, |a, b| a + b);
            (
                endpoint,
                apex_time(start, endpoint, input.parameter, numeric).map_err(Numeric)?,
            )
        };
        let row = &mut results[index];
        let hit = run_trajectory_tests(
            &mut valid,
            1,
            zero,
            start,
            endpoint,
            duration,
            inputs[0].max_y,
            &mut row[0],
            0,
            world,
            numeric,
            tuning,
        )?;
        if valid || input.flags & 4 == 0 {
            previous = row[0].position;
            continue;
        }
        let mut descent_valid = false;
        // Native reload is outside the no-hit conditional.
        let maximum_time = tuning.maximum_time().map_err(Tuning)?;
        let half_squared = (maximum_time * maximum_time) * 0.5;
        if !hit {
            let linear = input
                .speed_at_end
                .zip(endpoint, |v, p| v.mul_add(maximum_time, p));
            let end = STEP_GRAVITY.zip(linear, |g, p| g.mul_add(half_squared, p));
            run_trajectory_tests(
                &mut descent_valid,
                1,
                zero,
                endpoint,
                end,
                maximum_time,
                inputs[0].max_y,
                &mut row[1],
                0,
                world,
                numeric,
                tuning,
            )?;
        }
        // Shortening basis derives direction from the ORIGINAL input reference,
        // not from the optional previous-foot reference used to launch this row.
        let direction =
            try_normalize_safe(endpoint.zip(input.reference, |a, b| a - b), zero, numeric)
                .map_err(Numeric)?;
        let side = try_normalize_safe(cross(UP, direction), zero, numeric).map_err(Numeric)?;
        let local_up = cross(direction, side);
        let decrement = basis_vector(side, local_up, direction, [0.0, -0.1, -0.035]);
        endpoint = endpoint.zip(decrement, |a, b| a + b);
        run_trajectory_tests(
            &mut valid,
            count - 1,
            decrement,
            start,
            endpoint,
            duration,
            inputs[0].max_y,
            &mut row[0],
            1,
            world,
            numeric,
            tuning,
        )?;
        if !valid && !descent_valid {
            let (sin, cos) = tuning
                .sin_cos(input.turn_intent + input.spread_angle)
                .map_err(Tuning)?;
            let original_delta = endpoint
                .zip(start, |a, b| a - b)
                .zip(decrement, |a, b| a - b);
            let turned = yaw(original_delta, sin, cos);
            endpoint = start.zip(turned, |a, b| a + b);
            run_trajectory_tests(
                &mut valid,
                count,
                decrement,
                start,
                endpoint,
                duration,
                inputs[0].max_y,
                &mut row[2],
                0,
                world,
                numeric,
                tuning,
            )?;
            if valid {
                previous = row[2].position;
            } else {
                let (sin, cos) = tuning.sin_cos(input.spread_angle * -2.0).map_err(Tuning)?;
                endpoint = start.zip(yaw(turned, sin, cos), |a, b| a + b);
                run_trajectory_tests(
                    &mut valid,
                    count,
                    decrement,
                    start,
                    endpoint,
                    duration,
                    inputs[0].max_y,
                    &mut row[3],
                    0,
                    world,
                    numeric,
                    tuning,
                )?;
                if valid {
                    previous = row[3].position;
                }
            }
        } else {
            if descent_valid && valid {
                let original_endpoint = endpoint.zip(decrement, |a, b| a - b);
                let descent_delta = row[1].position.zip(original_endpoint, |a, b| a - b);
                let straight_delta = row[0].position.zip(original_endpoint, |a, b| a - b);
                let descent_distance = numeric
                    .dot_vector(descent_delta, descent_delta)
                    .map_err(Numeric)?;
                let straight_distance = numeric
                    .dot_vector(straight_delta, straight_delta)
                    .map_err(Numeric)?;
                if straight_distance > descent_distance {
                    previous = row[1].position;
                    row[0].valid = false;
                } else {
                    previous = row[0].position;
                    row[1].valid = false;
                }
            }
            // Native does NOT update previous when exactly one of the two wins.
            valid = true;
        }
    }
    Ok(())
}
