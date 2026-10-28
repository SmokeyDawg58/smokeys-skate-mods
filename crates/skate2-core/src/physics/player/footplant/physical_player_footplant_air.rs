//! `FootPlantManager::UpdateFootplantAir` 0x82dc3928, called from KnownAir.Update
//! 0x82d9987c: CalculateFootToTest 0x82dbbd10, UpdatePrediction 0x82dc1220 with
//! CheckForGrinds 0x82dbbaf0 and CalculateLanding 0x82dbbfd0, on the actual
//! retained manager, its World-predictor future and the attached Skeleton.
use super::ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    collision_query_batches::trajectory::BallisticTrajectoryInput,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle as Handle,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_footplant::{FootPlantFields, FootPlantManager},
    physics::PointGraph,
    reckoning::air::try_clamp_air_angle,
    reckoning_numeric::{self as math, ReckoningNumeric},
    simulation_assembly_backend::constraint_session::prediction::runtime::future::PredictionTarget,
    skeleton_ik_state::external::transform_point,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum FootPlantUpdateError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Service(&'static str, String),
}
impl std::fmt::Display for FootPlantUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FootPlant: {self:?}")
    }
}
type E = FootPlantUpdateError;
pub(super) fn service(stage: &'static str, error: impl std::fmt::Debug) -> E {
    E::Service(stage, format!("{error:?}"))
}

/// `kUpdateTime` 0x823512a4 (1/60), loaded directly by the manager updates.
pub(super) const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889);
/// `DEGREES_TO_RADIANS` 0x82351274.
pub(super) const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
pub(super) const ZERO: V = V::known([0.0; 4]);

pub(super) fn processed(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<Arc<Mutex<SkeletonProcessedContact>>, E> {
    Ok(Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .processed(),
    ))
}
/// Processed +0x220 up, +0x250 COM and +0x260 COM velocity.
pub(super) fn motion(p: &Mutex<SkeletonProcessedContact>) -> Result<(V, V, V), E> {
    let p = p.lock().map_err(|_| E::Poisoned)?;
    let output = p.prior_output();
    Ok((
        output
            .reckoning_up()
            .ok_or(E::Missing("Processed up+220"))?
            .value,
        output
            .centre_of_mass()
            .ok_or(E::Missing("Processed COM+250"))?
            .value,
        output
            .centre_of_mass_velocity()
            .ok_or(E::Missing("Processed COM velocity+260"))?
            .value,
    ))
}

/// `GetAttributePointer`, `DefaultDataArea(4)` zero when absent, then the word.
pub(super) fn scalar(attributes: &AttributeRuntimeInstance, key: u64) -> Result<f32, E> {
    Ok(f32::from_bits(
        attributes
            .try_scalar_word(key, 0)
            .map_err(|e| service("FootPlant attribute", e))?
            .unwrap_or(0),
    ))
}
/// Sixteen-byte vector attribute, `DefaultDataArea(0x10)` zero when absent.
fn vector(attributes: &AttributeRuntimeInstance, key: u64) -> Result<V, E> {
    let data = attributes
        .try_get_data(key, 0)
        .map_err(|e| service("FootPlant vector attribute", e))?;
    let Some(data) = data else {
        return Ok(ZERO);
    };
    let bytes = data
        .bytes()
        .get(..16)
        .ok_or(E::Missing("FootPlant vector attribute data"))?;
    Ok(V::known(std::array::from_fn(|lane| {
        f32::from_be_bytes(
            bytes[lane * 4..lane * 4 + 4]
                .try_into()
                .expect("four vector bytes"),
        )
    })))
}
/// `GetAttributePointer`, zeroed `DefaultDataArea(0x50)` when absent, then the
/// eight-point `PointGraphEval::Evaluate` with X at +0x10 and Y at +0x30.
pub(super) fn graph(attributes: &AttributeRuntimeInstance, key: u64, input: f32) -> Result<f32, E> {
    let data = attributes
        .try_get_data(key, 0)
        .map_err(|e| service("FootPlant graph attribute", e))?;
    let word = |offset: usize| -> Result<f32, E> {
        match &data {
            None => Ok(0.0),
            Some(d) => Ok(f32::from_be_bytes(
                d.bytes()
                    .get(offset..offset + 4)
                    .ok_or(E::Missing("FootPlant graph data"))?
                    .try_into()
                    .expect("four graph bytes"),
            )),
        }
    };
    let mut x = [0.0; 8];
    let mut y = [0.0; 8];
    for (i, (x_point, y_point)) in x.iter_mut().zip(&mut y).enumerate() {
        *x_point = word(16 + i * 4)?;
        *y_point = word(48 + i * 4)?;
    }
    Ok(PointGraph::<8> { x, y }.evaluate(input))
}
/// The inline VMX angle wrap after `XMVectorACos` and `AngleBetweenVectorsOnPlane`:
/// times `g2PIReciprocal`, minus its `vrfim` floor, minus one above one half
/// (`vcmpgtfp` mask AND 1.0), times `g2PI`.
pub(super) fn wrap_angle(angle: f32) -> f32 {
    let turns = angle * f32::from_bits(0x3e22_f983);
    let fraction = turns - turns.floor();
    let fraction = fraction - if fraction > 0.5 { 1.0 } else { 0.0 };
    fraction * f32::from_bits(0x40c9_0fdb)
}
/// `fsel(-x, 0, x)` then `fsel(1 - r, r, 1)`: an unordered input is retained.
pub(super) fn select_unit(value: f32) -> f32 {
    let low = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - low >= 0.0 { low } else { 1.0 }
}
fn cvar<T>(
    read: Result<T, crate::input_cvar_runtime::InputCvarError>,
    stage: &'static str,
) -> Result<T, E> {
    read.map_err(|e| service(stage, e))
}
/// Native stores of the zero vector, 0.0 and 0 when the landing is rejected.
fn clear_landing(fields: &mut FootPlantFields) {
    fields.surface_id = Some(0);
    fields.time_to_land = 0.0;
    fields.results_valid = false;
    fields.landing_position = Some(ZERO);
}

impl PhysicalAssemblyWorld {
    /// CheckForGrinds 0x82dbbaf0: `kFP_GrindBoxSize` builds the box around the
    /// landing, `GrindData::TrajectoryBoxTest` 0x82cf9cf0 keeps the rails near the
    /// deck position, and each one the footplant prediction crosses within the
    /// best distance so far (from `kFP_MaxDistToAdjustFootPlant`), not more than
    /// `kFP_MaxDistBelowCollisionForGrind` below the current landing, becomes the
    /// landing. With grinds parked for the first test there is no rail to report
    /// and the zero-count branch 0x82dbbb7c returns before the adjust loop.
    fn try_check_for_grinds(
        &self,
        refs: &InputCvarReferences,
        manager: &mut FootPlantManager,
        processed: &Mutex<SkeletonProcessedContact>,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<(), E> {
        use crate::{
            grind_data_runtime::GrindDataQueryResult,
            input_cvar_runtime::grind_states::GrindCvar as C,
            physical_player_grind_manager::{
                math::GrindNum,
                view::{GrindCvars, error, lanes, root},
            },
            physical_player_trajectory_owners::selector_grind::{
                traj_line_plane_intersect_test, trajectory_box_test,
            },
        };
        let box_size = cvar(refs.try_fp_grind_box_size(), "kFP_GrindBoxSize")?;
        if policy == GroundPreparationGrindPolicy::ParkedByUser {
            return Ok(());
        }
        let landing = lanes(
            manager
                .fields
                .landing_position
                .ok_or(E::Missing("FootPlant landing position"))?,
        );
        let extent = [box_size, box_size, box_size, 0.0];
        let takeoff = lanes(
            processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .deck_transform()
                .ok_or(E::Missing("deck transform+40"))?
                .columns()[3],
        );
        let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
        let mut sin_cos = crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
        let mut num = GrindNum {
            n: &mut numeric,
            s: &mut sin_cos,
        };
        let cvars = GrindCvars(refs);
        let services = &self.grind_services;
        let mut box_test = |bounds, result: &mut GrindDataQueryResult| {
            services
                .try_box_test(bounds, result, false, refs)
                .map_err(error)
        };
        let mut results = GrindDataQueryResult::new();
        trajectory_box_test(
            &mut box_test,
            &cvars,
            &self.debug_overlay,
            std::array::from_fn(|i| landing[i] - extent[i]),
            std::array::from_fn(|i| landing[i] + extent[i]),
            takeoff,
            &mut results,
            &mut num,
        )
        .map_err(|e| service("TrajectoryBoxTest", e))?;
        if results.count() == 0 {
            return Ok(());
        }
        let mut best = cvars
            .f(C::kFP_MaxDistToAdjustFootPlant)
            .map_err(|e| service("kFP_MaxDistToAdjustFootPlant", e))?;
        let prims: Vec<_> = results.results().cloned().collect();
        for prim in &prims {
            let Some(hit) = traj_line_plane_intersect_test(
                &manager.prediction.trajectory,
                prim.start,
                prim.end,
                &mut num,
            )
            .map_err(|e| service("TrajLinePlaneIntersectTest", e))?
            else {
                continue;
            };
            if !(hit.distance < best) {
                continue;
            }
            let below = cvars
                .f(C::kFP_MaxDistBelowCollisionForGrind)
                .map_err(|e| service("kFP_MaxDistBelowCollisionForGrind", e))?;
            // The landing re-read follows each accepted rail.
            let landing_y = manager
                .fields
                .landing_position
                .ok_or(E::Missing("FootPlant landing position"))?
                .xyz[1];
            if !(hit.point_on_line[1] - landing_y > -below) {
                continue;
            }
            manager.fields.time_to_land = hit.time;
            best = hit.distance;
            manager.fields.landing_normal = Some(root(hit.line_normal));
            manager.fields.landing_position = Some(root(hit.point_on_line));
            manager.fields.surface_id = Some(0);
            manager.fields.results_valid = true;
        }
        if cvar(refs.try_debug_draw_foot_plant(), "kDebugDrawFootPlant")? {
            self.debug_overlay
                .require()
                .map_err(|e| service("CheckForGrinds overlay", e))?;
        }
        Ok(())
    }
}

/// `SetupFakeBodyPrediction` 0x82e4bb88 operands; gravity is the predictor's.
struct FakeBodyRequest {
    position: V,
    velocity: V,
    radius: f32,
    duration: f32,
    error: f32,
}

impl PhysicalAssemblyWorld {
    /// UpdateFootplantAir 0x82dc3928: clear the perfect-leg flag, pick the foot
    /// off the board, then UpdatePrediction or the three clears, and Reset when
    /// no result is valid (after clearing the last IK target flag).
    pub(crate) fn try_update_pair_footplant_air(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        numeric: &mut HostGroundPreparationNumeric,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<(), E> {
        #[cfg(feature = "skate3-runtime")]
        if owner.skate3_footplants.load(std::sync::atomic::Ordering::Relaxed) {
            return self.try_update_pair_skate3_footplant_air(owner,board,skeleton,ground,numeric,policy);
        }
        let processed = processed(owner)?;
        let request = {
            let mut foot = owner.footplant().lock().map_err(|_| E::Poisoned)?;
            let manager = foot
                .try_manager(owner, skeleton)
                .map_err(|e| service("FootPlantManager", e))?;
            manager.fields.perfect_leg_valid = false;
            self.try_calculate_foot_to_test(board, skeleton, manager, &processed, numeric)?;
            if manager.fields.foot_off_board {
                Some(self.try_update_footplant_prediction(
                    owner, board, skeleton, manager, &processed, numeric, policy,
                )?)
            } else {
                manager.fields.results_valid = false;
                manager.fields.targeting = false;
                manager.prediction.ready = false;
                None
            }
        };
        if let Some(request) = request {
            // SetupFakeBodyPrediction82e4bb88 -> AddQuery with Predictor+30
            // gravity, draw false, and the same error ratio for start and end.
            // UpdatePrediction asserts its true return (line 0x100).
            let predictor = self
                .predictor
                .value
                .as_ref()
                .ok_or(E::Missing("World predictor"))?;
            let input = BallisticTrajectoryInput {
                position: request.position,
                velocity: request.velocity,
                gravity: V::known(predictor.gravity),
                radius: request.radius,
                duration: request.duration,
                maximum_error_start: request.error,
                maximum_error_end: request.error,
            };
            predictor
                .try_add_query(
                    owner,
                    PredictionTarget::FootPlant,
                    input,
                    false,
                    self.world_line_queries
                        .as_mut()
                        .ok_or(E::Missing("World queries"))?,
                    ground,
                    self.dmos.as_ref().ok_or(E::Missing("DMO collection"))?,
                    &mut numeric.reckoning,
                )
                .map_err(|e| service("SetupFakeBodyPrediction", e))?;
        }
        let mut foot = owner.footplant().lock().map_err(|_| E::Poisoned)?;
        let manager = foot
            .try_manager(owner, skeleton)
            .map_err(|e| service("FootPlantManager", e))?;
        if !manager.fields.results_valid {
            manager.fields.last_target_valid = false;
            manager.fields.reset();
        }
        Ok(())
    }

    /// CalculateFootToTest 0x82dbbd10: the attribute box (extents, Y offset) in
    /// board space via Processed `InverseEffectiveTransform`+100; the left toe
    /// (Part15) is tested first, then the right (Part19). The selected toe's
    /// raw animation and physical world positions and its unit leg from the
    /// Skeleton COM (Skeleton+3d00) are retained.
    fn try_calculate_foot_to_test(
        &mut self,
        board: &Handle,
        skeleton: &Handle,
        manager: &mut FootPlantManager,
        processed: &Mutex<SkeletonProcessedContact>,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<(), E> {
        manager.fields.foot_off_board = false;
        let extents = vector(manager.attributes(), 0x5d73_c954_31b7_0c59)?;
        let offset = scalar(manager.attributes(), 0x8de4_98d0_65a4_e4a5)?;
        let bias = V::known([0.0, offset, 0.0, 0.0]);
        let low = extents.map(|v| -v).zip(bias, |a, b| a + b);
        let high = extents.zip(bias, |a, b| a + b);
        let parts = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                let anim_left = normal
                    .anim_part_world_raw_position(15)
                    .ok_or(E::Missing("AnimRecord Part15"))?;
                let anim_right = normal
                    .anim_part_world_raw_position(19)
                    .ok_or(E::Missing("AnimRecord Part19"))?;
                let physics_left = normal
                    .try_query_part_transform(15)
                    .map_err(|e| service("GetPartTransform", e))?
                    .columns()[3];
                let physics_right = normal
                    .try_query_part_transform(19)
                    .map_err(|e| service("GetPartTransform", e))?
                    .columns()[3];
                let com = normal.post_physics_records().centre_of_mass_typed();
                Ok::<_, E>((
                    anim_left,
                    anim_right,
                    physics_left,
                    physics_right,
                    com,
                    normal.anim_centre_of_mass(),
                ))
            })
            .map_err(|e| service("Skeleton pair", e))??;
        let (anim_left, anim_right, physics_left, physics_right, com, anim_com) = parts;
        let inverse = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .toolkit_state()
            .inverse_effective()
            .ok_or(E::Missing("Processed InverseEffectiveTransform+100"))?;
        let local_left = transform_point(&inverse, physics_left);
        let local_right = transform_point(&inverse, physics_right);
        manager.fields.centre_of_mass = com;
        manager.fields.anim_centre_of_mass = anim_com;
        // Per lane min>p OR p>max, XYZ only (vsldoi drops W); unordered is inside.
        let outside = |local: V| {
            (0..3).any(|axis| low.xyz[axis] > local.xyz[axis] || local.xyz[axis] > high.xyz[axis])
        };
        let selected = if outside(local_left) {
            Some((15, anim_left, physics_left))
        } else if outside(local_right) {
            Some((19, anim_right, physics_right))
        } else {
            None
        };
        if let Some((part, anim, physics)) = selected {
            let fields = &mut manager.fields;
            fields.foot_part = part;
            fields.foot_off_board = true;
            fields.anim_foot_position = Some(anim);
            fields.physics_foot_position = Some(physics);
            let leg = physics.zip(fields.centre_of_mass, |a, b| a - b);
            fields.leg_vector = Some(
                math::try_normalize_safe(leg, ZERO, &mut numeric.reckoning)
                    .map_err(|e| service("leg NormalizeSafe", e))?,
            );
        }
        Ok(())
    }

    /// UpdatePrediction 0x82dc1220 up to its SetupFakeBodyPrediction operands:
    /// consume a ready future (TimeToCollision clamped to 0..1 by two fsel,
    /// GetContactPosition, surface), CheckForGrinds, the start/hold targeting
    /// window, CalculateLanding while targeting, then the next query start.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual pair, retained manager and source services"
    )]
    fn try_update_footplant_prediction(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        manager: &mut FootPlantManager,
        processed: &Mutex<SkeletonProcessedContact>,
        numeric: &mut HostGroundPreparationNumeric,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<FakeBodyRequest, E> {
        let refs = &owner.inputs().cvars;
        manager.fields.results_valid = false;
        if manager.prediction.ready {
            let time = manager
                .prediction
                .collision_time
                .ok_or(E::Missing("ready FootPlant collision time"))?;
            if time.iter().all(|t| *t >= 0.0) {
                // TimeToCollision82d83ca0 asserts abs(X) < 100 before returning X.
                if time[0].abs().partial_cmp(&100.0) != Some(std::cmp::Ordering::Less) {
                    return Err(E::Missing("TimeToCollision abs(X)<100"));
                }
                manager.fields.time_to_land = select_unit(time[0]);
                // GetContactPosition82db57a8: an all-lanes valid time returns
                // the stored collision position.
                manager.fields.landing_position = Some(
                    manager
                        .prediction
                        .collision_position
                        .ok_or(E::Missing("ready FootPlant contact"))?,
                );
                manager.fields.surface_id = Some(
                    manager
                        .prediction
                        .surface
                        .ok_or(E::Missing("ready FootPlant surface"))?,
                );
                manager.fields.results_valid = true;
                self.try_check_for_grinds(refs, manager, processed, policy)?;
                let time_to_land = manager.fields.time_to_land;
                if !manager.fields.targeting {
                    if time_to_land > cvar(refs.try_fp_min_time_to_start(), "kFP_MinTimeToStart")?
                        && time_to_land
                            < cvar(refs.try_fp_max_time_to_start(), "kFP_MaxTimeToStart")?
                    {
                        manager.fields.targeting = true;
                        manager.fields.time_targeting = 0.0;
                    } else {
                        clear_landing(&mut manager.fields);
                    }
                } else if manager.fields.time_targeting + time_to_land
                    < cvar(refs.try_fp_min_time_after_start(), "kFP_MinTimeAfterStart")?
                {
                    manager.fields.targeting = false;
                    clear_landing(&mut manager.fields);
                    manager.fields.time_targeting = 0.0;
                }
                if manager.fields.targeting {
                    manager.fields.time_targeting += UPDATE_TIME;
                    self.try_calculate_landing(
                        owner, board, skeleton, manager, processed, numeric,
                    )?;
                }
            }
        }
        let (up, com, velocity) = motion(processed)?;
        manager.fields.landing_normal = Some(up);
        // 0x82dc1454..14b0: GetPartVelocity82b46078 (var_80, never read) and the
        // NormalizeSafe of COM velocity x J (var_D0, overwritten at 0x82dc1610)
        // are dead; both callees are pure, so omitting them changes no state.
        let foot_offset = cvar(refs.try_fp_y_foot_offset(), "kFPYFootOffset")?;
        let offset = up.map(|u| u * foot_offset);
        if !manager.fields.perfect_leg_valid {
            manager.fields.perfect_leg_vector = V::known([0.0, -1.0, 0.0, 0.0]);
        }
        let scale = scalar(manager.attributes(), 0xdd9b_6fbf_b677_5050)?;
        let start = manager
            .fields
            .perfect_leg_vector
            .zip(com, |leg, c| leg.mul_add(scale, c));
        let weight = if manager.fields.results_valid {
            graph(
                manager.attributes(),
                0xca25_5957_17ae_af32,
                manager.fields.time_to_land,
            )?
        } else {
            1.0
        };
        let remaining = 1.0 - weight;
        let physics = manager
            .fields
            .physics_foot_position
            .ok_or(E::Missing("FootPlant physical toe+d0"))?;
        let adjust = cvar(
            refs.try_traj_start_test_time_adjust(),
            "kTrajStartTestTimeAdjust",
        )?;
        let blended = start.zip(physics.map(|p| p * remaining), |s, p| s.mul_add(weight, p));
        let stepped = velocity
            .map(|v| v * UPDATE_TIME)
            .zip(blended, |v, b| v.mul_add(adjust, b));
        let position = stepped.zip(offset, |a, b| a + b);
        let error = cvar(refs.try_foot_plant_traj_error(), "kFootPlantTrajError")?;
        let radius = cvar(refs.try_foot_plant_traj_radius(), "kFootPlantTrajRadius")?;
        let duration = cvar(refs.try_foot_plant_traj_max_time(), "kFootPlantTrajMaxTime")?;
        Ok(FakeBodyRequest {
            position,
            velocity,
            radius,
            duration,
            error,
        })
    }

    /// CalculateLanding 0x82dbbfd0: the perfect leg from the landing velocity
    /// (fixed -9.8 gravity) against the landing normal, the leg toward the
    /// landing point, both clamped to `kMaxEntryAngleDegrees`; their graph blend
    /// sets the animation-space IK target (step-clamped by `kFPM_MaxOffsetDelta`)
    /// and the next-frame landing flag; near landing the selected foot's three
    /// Parts (15..17 or 19..21) stop colliding for two frames.
    #[expect(
        clippy::too_many_lines,
        reason = "one source-ordered landing calculation with its Skeleton writes"
    )]
    fn try_calculate_landing(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        manager: &mut FootPlantManager,
        processed: &Mutex<SkeletonProcessedContact>,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<(), E> {
        let refs = &owner.inputs().cvars;
        let attributes = manager.attributes().clone();
        let fields = &mut manager.fields;
        let n = &mut numeric.reckoning;
        let s = &mut numeric.sin_cos;
        let numeric_error = |e| service("CalculateLanding numeric", e);
        let time = fields.time_to_land;
        let (_, com, velocity) = motion(processed)?;
        let landing = fields
            .landing_position
            .ok_or(E::Missing("FootPlant landing+e0"))?;
        let to_landing = landing.zip(com, |a, b| a - b);
        let gravity = V::known([0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0]);
        let landing_velocity = gravity.zip(velocity, |g, v| g.mul_add(time, v));
        let direction =
            math::try_normalize_safe(landing_velocity, ZERO, n).map_err(numeric_error)?;
        let normal = fields
            .landing_normal
            .ok_or(E::Missing("FootPlant landing normal+f0"))?;
        let leg_scalar = cvar(refs.try_com_vel_leg_scalar(), "kCOMVelLegScalar")?;
        let range =
            cvar(refs.try_max_entry_angle_degrees(), "kMaxEntryAngleDegrees")? * DEGREES_TO_RADIANS;
        let against = normal.map(|v| -v);
        let entry = direction.zip(against, |d, a| d.mul_add(leg_scalar, a));
        let clamped = try_clamp_air_angle(entry, against, range, n, s)
            .map_err(|e| service("ClampVectorWithinAngleRange", e))?;
        let perfect = math::try_normalize_safe(clamped, ZERO, n).map_err(numeric_error)?;
        fields.perfect_leg_vector = perfect;
        fields.perfect_leg_valid = true;
        let clamped = try_clamp_air_angle(to_landing, normal.map(|v| -v), range, n, s)
            .map_err(|e| service("ClampVectorWithinAngleRange", e))?;
        let toward = math::try_normalize_safe(clamped, ZERO, n).map_err(numeric_error)?;
        let weight = graph(&attributes, 0x164b_0a6d_1bfd_0a02, fields.time_to_land)?;
        let remaining = 1.0 - weight;
        let blend = toward.zip(perfect.map(|p| p * remaining), |t, p| t.mul_add(weight, p));
        let blended = math::try_normalize_safe(blend, ZERO, n).map_err(numeric_error)?;
        let leg = fields
            .leg_vector
            .ok_or(E::Missing("FootPlant leg vector+100"))?;
        let dot = n.dot_vector(blended, leg).map_err(numeric_error)?;
        let angle = wrap_angle(math::try_acos(dot, n).map_err(numeric_error)?);
        if fields.time_to_land
            < cvar(
                refs.try_ik_blend_in_max_time_to_land(),
                "kIKBlendInMaxTimeToLand",
            )?
            && cvar(refs.try_ik_foot_prepare(), "kIKFootPrepare")?
        {
            // Skeleton+2dc0 WorldToAnimTransform.
            let world_to_animation = self
                .skaters
                .as_mut()
                .ok_or(E::Missing("skater pool"))?
                .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                    *normal.root_transforms().world_to_animation()
                })
                .map_err(|e| service("Skeleton pair", e))?;
            let [row0, row1, row2, _] = *world_to_animation.columns();
            let anim_com = fields.anim_centre_of_mass;
            let anim_foot = fields
                .anim_foot_position
                .ok_or(E::Missing("FootPlant animation toe+c0"))?;
            let difference = anim_foot.zip(anim_com, |a, b| a - b);
            let squared = n
                .dot_vector(difference, difference)
                .map_err(numeric_error)?;
            let seed = n.reciprocal_sqrt_estimate(squared).map_err(numeric_error)?;
            let length = math::length_from_squared_and_estimate(squared, seed);
            let first = row0.map(|v| v * blended.xyz[0]);
            let second = row1.zip(first, |v, old| v.mul_add(blended.xyz[1], old));
            let rotated = row2.zip(second, |v, old| v.mul_add(blended.xyz[2], old));
            let centre = transform_point(&world_to_animation, anim_com);
            let mut target = rotated.zip(centre, |r, c| r.mul_add(length, c));
            if fields.last_target_valid {
                if cvar(refs.try_debug_draw_foot_plant(), "kDebugDrawFootPlant")? {
                    self.debug_overlay
                        .require()
                        .map_err(|e| service("FootPlant overlay", e))?;
                }
                let delta = target.zip(fields.last_ik_foot_target, |a, b| a - b);
                let maximum = cvar(refs.try_fpm_max_offset_delta(), "kFPM_MaxOffsetDelta")?;
                let clamped = math::try_clamp(delta, maximum, n)
                    .map_err(|e| service("ClampVectorWithinMaxLength", e))?;
                target = fields.last_ik_foot_target.zip(clamped, |a, b| a + b);
                if cvar(refs.try_debug_draw_foot_plant(), "kDebugDrawFootPlant")? {
                    self.debug_overlay
                        .require()
                        .map_err(|e| service("FootPlant overlay", e))?;
                }
            }
            fields.last_ik_foot_target = target;
            fields.last_target_valid = true;
            let left = match fields.foot_part {
                15 => true,
                19 => false,
                _ => {
                    return Err(E::Missing(
                        "CalculateLanding foot Part 15/19 (assert 0x1DD)",
                    ));
                }
            };
            let blend = fields.ik_blend_in;
            self.skaters
                .as_mut()
                .ok_or(E::Missing("skater pool"))?
                .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                    normal.try_set_anim_foot_ik_target(left, target, blend)
                })
                .map_err(|e| service("Skeleton pair", e))?
                .map_err(|e| service("SetFootIKTargetPositionAnim", e))?;
            let sum = cvar(refs.try_ik_blend_in_speed(), "kIKBlendInSpeed")? + fields.ik_blend_in;
            fields.ik_blend_in = sum;
            fields.ik_blend_in = select_unit(sum);
            if cvar(refs.try_debug_draw_foot_plant(), "kDebugDrawFootPlant")? {
                self.debug_overlay
                    .require()
                    .map_err(|e| service("FootPlant overlay", e))?;
            }
        }
        if cvar(refs.try_debug_draw_foot_plant(), "kDebugDrawFootPlant")? {
            self.debug_overlay
                .require()
                .map_err(|e| service("FootPlant overlay", e))?;
        }
        let threshold = scalar(&attributes, 0x945b_b959_a3e3_77e9)? * DEGREES_TO_RADIANS;
        // 0x82dbc608..63c: all-lanes threshold > angle (splats), results
        // valid, and time-to-land not above 0.017 (unordered passes).
        fields.landing_next_frame = fields.results_valid
            && !(fields.time_to_land > f32::from_bits(0x3c8b_4396))
            && threshold > angle;
        if fields.time_to_land
            < cvar(
                refs.try_fpm_time_to_land_to_disable_foot_collision(),
                "kFPM_TimeToLandToDisableFootCollision",
            )?
        {
            let right = fields.foot_part == 19;
            self.skaters
                .as_mut()
                .ok_or(E::Missing("skater pool"))?
                .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                    normal.disable_foot_collisions(right, 2);
                })
                .map_err(|e| service("Skeleton pair", e))?;
        }
        Ok(())
    }
}
