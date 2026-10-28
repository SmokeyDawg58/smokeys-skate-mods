//! S3 FootPlantManager on the existing S2 character, IK, World predictor and
//! body solver. This binding creates no S3 player or physical world.
use super::{
    footplant_air::{FootPlantUpdateError as E, motion, processed, service},
    ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric},
};
use crate::{
    collision_query_batches::trajectory::BallisticTrajectoryInput,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle as Handle,
    grind_data_octree::OctreeBounds,
    grind_data_runtime::GrindDataQueryResult,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_footplant::{FootPlantPrediction, skate3::SourceFootplant},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector as V},
    static_ground_pool::StaticGroundPool,
};
use skate3_core::air::trajectory::QueryResult;
use std::sync::Arc;

pub(super) fn lanes(v: V) -> [f32; 4] {
    // Adapter-owned geometric W; never written back as observed S2 knowledge.
    [v.xyz[0], v.xyz[1], v.xyz[2], v.w.unwrap_or(0.)]
}
fn matrix(m: &SkeletonRootMatrix) -> [[f32; 4]; 4] {
    m.columns().map(lanes)
}
fn completed(future: &FootPlantPrediction) -> Result<QueryResult, E> {
    let time = future
        .collision_time
        .ok_or(E::Missing("S3 footplant completed time"))?;
    let mut result = QueryResult::miss();
    result.contact_time = time[0];
    result.contact_position = lanes(
        future
            .collision_position
            .ok_or(E::Missing("S3 footplant completed position"))?,
    );
    result.contact_normal = lanes(
        future
            .collision_normal
            .ok_or(E::Missing("S3 footplant completed normal"))?,
    );
    result.landing_normal = lanes(
        future
            .ground_normal
            .ok_or(E::Missing("S3 footplant completed ground normal"))?,
    );
    result.contact_frame = future
        .frame
        .ok_or(E::Missing("S3 footplant completed frame"))?;
    result.surface = future
        .surface
        .ok_or(E::Missing("S3 footplant completed surface"))?;
    // Neither contact transform nor geometry ID is consumed by FootPlantManager.
    Ok(result)
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_skate3_footplant_air(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        numeric: &mut HostGroundPreparationNumeric,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<(), E> {
        use super::prediction::runtime::future::PredictionTarget;
        use crate::skate3_native::physics::footplant::CandidatePose;
        let input =
            SourceFootplant::air_input(owner).map_err(|e| service("S3 footplant air input", e))?;
        let processed = processed(owner)?;
        let (up, _, _) = motion(&processed)?;
        let (inverse, deck) = {
            let p = processed.lock().map_err(|_| E::Poisoned)?;
            (
                p.toolkit_state()
                    .inverse_effective()
                    .ok_or(E::Missing("S3 footplant inverse board"))?,
                p.deck_transform()
                    .ok_or(E::Missing("S3 footplant deck"))?
                    .columns()[3],
            )
        };
        let (pose, world_to_animation) = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                Ok::<_, E>((
                    CandidatePose {
                        inverse_effective: matrix(&inverse),
                        physical_toes: [
                            lanes(
                                normal
                                    .try_query_part_transform(15)
                                    .map_err(|e| service("S3 footplant physical toe15", e))?
                                    .columns()[3],
                            ),
                            lanes(
                                normal
                                    .try_query_part_transform(19)
                                    .map_err(|e| service("S3 footplant physical toe19", e))?
                                    .columns()[3],
                            ),
                        ],
                        physical_com: lanes(normal.post_physics_records().centre_of_mass_typed()),
                        animation_com: lanes(normal.anim_centre_of_mass()),
                        animation_toes: [
                            lanes(
                                normal
                                    .anim_part_world_raw_position(15)
                                    .ok_or(E::Missing("animation toe15"))?,
                            ),
                            lanes(
                                normal
                                    .anim_part_world_raw_position(19)
                                    .ok_or(E::Missing("animation toe19"))?,
                            ),
                        ],
                    },
                    matrix(normal.root_transforms().world_to_animation()),
                ))
            })
            .map_err(|e| service("S3 footplant Skeleton", e))??;
        let gravity = self
            .predictor
            .value
            .as_ref()
            .ok_or(E::Missing("World predictor"))?
            .gravity;
        let request = {
            let mut foot = owner.footplant().lock().map_err(|_| E::Poisoned)?;
            let manager = foot
                .try_manager(owner, skeleton)
                .map_err(|e| service("FootPlantManager", e))?;
            if manager.source3.is_none() {
                manager.source3 = Some(Box::new(
                    SourceFootplant::load()
                        .map_err(|e| service("embedded S3 footplant data", e))?,
                ));
                crate::diagnostics::test_log::event(
                    "skate3_footplant_owner",
                    format_args!("native_s2_solver=true embedded=true"),
                );
            }
            let source = manager.source3.as_mut().expect("loaded");
            source.active = true;
            if manager.prediction.ready {
                if let Some(submitted) = source.pending.take() {
                    source
                        .manager
                        .accept_completed_query(completed(&manager.prediction)?, submitted);
                }
            }
            // The original S3 calculation consumes the preceding result only.
            let mut edges = GrindDataQueryResult::new();
            if policy != GroundPreparationGrindPolicy::ParkedByUser && manager.prediction.ready {
                let contact = manager
                    .prediction
                    .collision_position
                    .ok_or(E::Missing("completed footplant position"))?;
                self.grind_services
                    .try_box_test(
                        OctreeBounds {
                            minimum: [
                                contact.xyz[0] - 2.,
                                contact.xyz[1] - 2.,
                                contact.xyz[2] - 2.,
                                0.,
                            ],
                            maximum: [
                                contact.xyz[0] + 2.,
                                contact.xyz[1] + 2.,
                                contact.xyz[2] + 2.,
                                0.,
                            ],
                        },
                        &mut edges,
                        false,
                        &owner.inputs().cvars,
                    )
                    .map_err(|e| service("S3 footplant real edges", e))?;
            }
            let edges: Vec<_> = edges
                .results()
                .map(|edge| skate3_core::physics::grind_contact::Primitive {
                    start: edge.start,
                    end: edge.end,
                    owner: 0, // geometry-only input; FootPlantManager never reads owner IDs
                })
                .collect();
            let fp = &mut source.manager;
            fp.update_candidate_from_pose(&input.footplant, &pose);
            fp.update_launch(&input.footplant);
            if fp.consume_completed_query(&input.footplant, input.requested, &edges, lanes(deck)) {
                if let Some(target) =
                    fp.calculate_pose_target(&input.footplant, &world_to_animation)
                {
                    self.skaters
                        .as_mut()
                        .ok_or(E::Missing("skater pool"))?
                        .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                            normal
                                .try_set_anim_foot_ik_target(
                                    target.toe == 15,
                                    V::known(target.animation_position),
                                    target.blend,
                                )
                                .map_err(|e| service("S3 footplant animation foot target", e))?;
                            if target.disable_collision {
                                normal.disable_foot_collisions(target.toe == 19, 2);
                            }
                            Ok::<_, E>(())
                        })
                        .map_err(|e| service("S3 footplant Skeleton IK", e))?
                        .map_err(|e| service("S3 footplant animation foot target", e))?;
                }
            }
            let request = fp.next_query(lanes(up), gravity);
            source.pending = Some(request.trajectory);
            source.record_changes();
            request
        };
        self.predictor
            .value
            .as_ref()
            .ok_or(E::Missing("World predictor"))?
            .try_add_query(
                owner,
                PredictionTarget::FootPlant,
                BallisticTrajectoryInput {
                    position: V::known(request.trajectory.position),
                    velocity: V::known(request.trajectory.velocity),
                    gravity: V::known(request.trajectory.acceleration),
                    radius: request.radius,
                    duration: request.trajectory.duration,
                    maximum_error_start: request.start_error,
                    maximum_error_end: request.end_error,
                },
                false,
                self.world_line_queries
                    .as_mut()
                    .ok_or(E::Missing("World queries"))?,
                ground,
                self.dmos.as_ref().ok_or(E::Missing("DMO collection"))?,
                &mut numeric.reckoning,
            )
            .map_err(|e| service("S3 footplant World predictor submission", e))?;
        Ok(())
    }
}
