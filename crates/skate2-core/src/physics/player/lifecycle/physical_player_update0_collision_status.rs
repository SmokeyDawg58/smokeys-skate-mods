//! `PhysicalPlayerHiLOD::Update0_ProcessCollection`82e01f18. World::Update1
//! 827ae098 gathers every physical's Update0 before the Update1 jobs:
//! UpdateCollisionStatus 82e016a0 keeps a skater world-only only while a
//! LivingWorld pedestrian, dynamic object or vehicle overlaps it, then the
//! Collision vehicle impact gates CheckPlayerInsideVehicle 82dfe690.
use super::{
    dmo_living_world::DmoWorldError,
    physical_skeleton_controller::{PhysicalSkeletonControllerError, SkeletonCollisionTransition},
    world_services::PhysicalWorldServicesError,
};
use crate::{
    debug_overlay::DebugOverlayError,
    dynamic_assembly_session::{
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            SimulationSkeletonJumpPairError, board_collision_mode::BoardCollisionModeError,
        },
        skeleton_normal::part_transform_query::SkeletonPartTransformQueryError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_grind_services::{
        VehicleNearQueryError,
        dmo_objects::{SkaterTestCvars, build_skater_test_aabbox, build_skater_test_capsule},
        vehicle_queries::{VehicleSphereQueryError, vehicle_local_point},
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner, hilod_tail::HiLodCollisionChange,
    },
    physical_player_phys_out::construct::prior_input::PriorInputReadError,
    reckoning_numeric::ReckoningHostNumericError,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;

/// 0x49742400 (1e6) seeds the nearest same-flags skater squared distance.
const NO_SKATER_NEAR: f32 = f32::from_bits(0x4974_2400);
/// 0x3fc00000: another skater inside 1.5 m keeps normal collision.
const SKATER_CLEARANCE: f32 = f32::from_bits(0x3fc0_0000);

#[derive(Debug)]
pub(crate) enum Update0Error {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Poisoned(&'static str),
    Missing(&'static str),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    WorldServices(PhysicalWorldServicesError),
    ActorQuery(crate::actor_construction::physical_mount::input_query::ActorPhysicalInputError),
    Body(crate::simulation_assembly_backend::constraint_session::physical_island::SimulationPhysicalBodyError),
    BoardTransform(crate::dynamic_assembly_session::skater_pool::board_entry::SkateboardEntrySessionError),
    Skeleton(SimulationSkaterPoolError),
    UninitializedLivingWorld,
    PedestrianSphere(crate::living_world_grind_services::scoring::pedestrian_sphere::PedestrianSphereQueryError),
    Vehicle(VehicleNearQueryError),
    VehicleSphere(VehicleSphereQueryError),
    DynamicObject(DmoWorldError),
    PartTransform(SkeletonPartTransformQueryError),
    Numeric(ReckoningHostNumericError),
    Output(PriorInputReadError),
    BoardCollision(BoardCollisionModeError),
    SkeletonCollision(PhysicalSkeletonControllerError),
}
impl std::fmt::Display for Update0Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "physical Update0_ProcessCollection: {self:?}")
    }
}

impl PhysicalAssemblyWorld {
    /// `Update0_ProcessCollection`82e01f18 for the selected pair.
    pub(crate) fn try_update0_pair_process_collection(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
    ) -> Result<bool, Update0Error> {
        use Update0Error as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        self.try_update_pair_collision_status(&owner, board, skeleton, cvars)?;
        // 82e01f30..1f80: guarded PhysOut Collision+0x94 (skeleton collision
        // audio mVehicleImpact), ordered greater than zero.
        let impact = owner
            .phys_out()
            .lock()
            .map_err(|_| E::Poisoned("PhysOut"))?
            .try_guarded_vehicle_impact()
            .map_err(E::Output)?;
        if impact > 0.0 {
            let position = self
                .skeleton(skeleton)
                .map_err(E::Skeleton)?
                .root_transforms()
                .animation_to_world()
                .columns()[3];
            return self.try_position_inside_vehicle(position.xyz, cvars);
        }
        Ok(false)
    }

    /// CheckPlayerInsideVehicle82dfe690 -> Manager.IsPositionInsideAnyVehicle
    /// 82703a58. The caller performs the existing Actor breadcrumb teleport
    /// immediately, before Update1; this does not request a later state change.
    fn try_position_inside_vehicle(
        &self,
        position: [f32; 3],
        cvars: &InputCvarReferences,
    ) -> Result<bool, Update0Error> {
        use Update0Error as E;
        let owner = Arc::clone(self.grind_services.living_world());
        let living = owner.lock().map_err(|_| E::Poisoned("LivingWorld"))?;
        let mut count = 0;
        let mut ids = [0; 6];
        let mut distances = [0.; 6];
        living
            .try_get_vehicles_within_range(
                [position[0], position[2]],
                8.,
                6,
                &mut count,
                &mut ids,
                &mut distances,
            )
            .map_err(E::Vehicle)?;
        let half_y = cvars.try_inside_vehicle_y_size().map_err(E::Cvar)? * 0.5;
        let offset_y = cvars.try_inside_vehicle_y_offset().map_err(E::Cvar)?;
        let vehicles = living
            .grind_vehicle_entities
            .as_ref()
            .ok_or(E::UninitializedLivingWorld)?;
        for id in &ids[..count as usize] {
            let Some(vehicle) = vehicles.get(*id) else {
                continue;
            };
            let matrix = &vehicle.core.transform;
            let centre =
                std::array::from_fn(|i| matrix[1][i].mul_add(half_y + offset_y, matrix[3][i]));
            let local = vehicle_local_point(matrix, position, centre);
            let dimensions = vehicle.dimensions();
            let half_z = dimensions[2] * 0.5 - cvars.try_inside_vehicle_z().map_err(E::Cvar)?;
            let draw = cvars.try_draw_inside_vehicle_test().map_err(E::Cvar)?;
            if draw {
                let _ = dimensions[0] * 0.5 - cvars.try_inside_vehicle_x().map_err(E::Cvar)?;
                self.debug_overlay.require().map_err(E::Overlay)?;
            }
            if half_z > local[2].abs() {
                let half_x = dimensions[0] * 0.5 - cvars.try_inside_vehicle_x().map_err(E::Cvar)?;
                if half_x > local[0].abs() && half_y > local[1].abs() {
                    if cvars.try_draw_inside_vehicle_test().map_err(E::Cvar)? {
                        self.debug_overlay.require().map_err(E::Overlay)?;
                    }
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// `UpdateCollisionStatus`82e016a0.
    fn try_update_pair_collision_status(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
    ) -> Result<(), Update0Error> {
        use Update0Error as E;
        // 82e016cc..16f4: the HiLOD+0x120 world-only bit, else AnimOutPhysIn
        // +0x2aa0 bit 0x01000000 (Actor::SetUpPhysics825ff8d8 copies the
        // LivingWorld spawned-object request from Actor+0x258).
        let world_only = owner
            .tail()
            .lock()
            .map_err(|_| E::Poisoned("HiLOD tail"))?
            .collide_with_world_only;
        let candidate = world_only || {
            let input = owner
                .animation_physics_input
                .lock()
                .map_err(|_| E::Poisoned("AnimOutPhysIn"))?
                .clone()
                .ok_or(E::Missing("AnimOutPhysIn"))?;
            let flags = input
                .lock()
                .map_err(|_| E::Poisoned("AnimOutPhysIn"))?
                .locomotion_flags;
            flags & 0x0100_0000 != 0
        };
        // 82e01730..1748: kLWTestRadius, read before the collection walk.
        let radius = cvars.try_lw_test_radius().map_err(E::Cvar)?;
        // 82e01748..17a4: GetSkeleton()+0x2d80 AnimToWorld, all four rows.
        let anim_to_world = *self
            .skeleton(skeleton)
            .map_err(E::Skeleton)?
            .root_transforms()
            .animation_to_world();
        // 82e017ac..1838: every other physical with the same GetFlags lowers the
        // nearest squared distance to its GetSkateboard translation.
        let services = self
            .physical_world_services()
            .ok_or(E::Missing("physical World services"))?;
        let flags = Self::try_physical_flags(owner)?;
        let here = anim_to_world.columns()[3].xyz;
        let mut nearest = NO_SKATER_NEAR;
        for physical in services.try_physical_objects().map_err(E::WorldServices)? {
            let other = physical.owner();
            if Arc::ptr_eq(other, owner) || Self::try_physical_flags(other)? != flags {
                continue;
            }
            let board = self.try_physical_skateboard_translation(other)?;
            let delta = [here[0] - board[0], here[1] - board[1], here[2] - board[2]];
            // vmsum3fp128: components 0+1, then 2; vminfp128 keeps a NaN.
            let distance = (delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2];
            nearest = if nearest.is_nan() || distance.is_nan() {
                f32::NAN
            } else if distance < nearest {
                distance
            } else {
                nearest
            };
        }
        // 82e0183c..1870: the nearest skater must be beyond 1.5 m in every lane,
        // and only a candidate runs the LivingWorld overlap tests.
        let overlap = if nearest > SKATER_CLEARANCE * SKATER_CLEARANCE && candidate {
            self.try_living_world_overlap(board, skeleton, &anim_to_world, radius, cvars)?
        } else {
            false
        };
        // 82e01b9c..1bf0: the flag changes first, then Skateboard
        // CollideWithWorldOnly/CollideNormally and the SkeletonController call.
        let change = owner
            .tail()
            .lock()
            .map_err(|_| E::Poisoned("HiLOD tail"))?
            .select_collision_status(overlap);
        if let Some(change) = change {
            let world_only = change == HiLodCollisionChange::WorldOnly;
            self.try_set_pair_board_collision_mode(board, skeleton, world_only)
                .map_err(E::BoardCollision)?;
            self.try_pair_skeleton_collision_transition(
                board,
                skeleton,
                if world_only {
                    SkeletonCollisionTransition::StartWorldOnly
                } else {
                    SkeletonCollisionTransition::StopWorldOnly
                },
            )
            .map_err(E::SkeletonCollision)?;
        }
        // 82e01bf4..1c20: final kbShowCollisionStateChecks overlay assertion.
        self.require_collision_state_overlay(cvars)
    }

    /// `IPhysical::GetFlags` of a skater physical.
    fn try_physical_flags(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Result<u32, Update0Error> {
        use Update0Error as E;
        let query = owner.actor_query.lock().map_err(|_| E::Poisoned("Actor query"))?;
        query
            .as_ref()
            .ok_or(E::Missing("Actor query"))?
            .try_flags()
            .map_err(E::ActorQuery)
    }

    /// `GetSkateboard()->GetTransform()`'s translation: the physical's board Part6
    /// live body position, without a Processed flip.
    fn try_physical_skateboard_translation(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Result<[f32; 3], Update0Error> {
        use Update0Error as E;
        let pool = self.skaters.as_ref().ok_or(E::MissingPool)?;
        for handle in pool.handles() {
            // A PC Object Dropper riderless board has no physical owner.
            if pool.is_riderless(handle).map_err(|e| E::Body(crate::simulation_assembly_backend::constraint_session::physical_island::SimulationPhysicalBodyError::Pool(e)))? {
                continue;
            }
            let Ok(session) = self.board(handle) else { continue };
            if Arc::ptr_eq(self.physical_body_owner_for_contacts(handle).map_err(E::Body)?, owner) {
                let transform = crate::dynamic_assembly_session::skater_pool::board_entry::try_current_board_unflipped_transform(
                    session,
                    self.backend(),
                )
                .map_err(E::BoardTransform)?;
                return Ok(transform.columns()[3].xyz);
            }
        }
        Err(E::Missing("physical's skateboard"))
    }

    /// 82e01874..1b98: the pedestrian sphere, the dynamic-object capsule, then
    /// the vehicle sphere, stopping at the first overlap.
    fn try_living_world_overlap(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        anim_to_world: &SkeletonRootMatrix,
        first_radius: f32,
        cvars: &InputCvarReferences,
    ) -> Result<bool, Update0Error> {
        use Update0Error as E;
        let position = anim_to_world.columns()[3];
        let owner = Arc::clone(self.grind_services.living_world());
        let living = owner.lock().map_err(|_| E::Poisoned("LivingWorld"))?;
        if !living.selected_entities_initialized() {
            return Err(E::UninitializedLivingWorld);
        }
        // 82e01874..18e0 reads kLWTestRadius again and raises sphere centre
        // by that radius before the real pedestrian Part0 local-box query.
        let radius = cvars.try_lw_test_radius().map_err(E::Cvar)?;
        if living.try_pedestrian_intersects_sphere(
            [position.xyz[0], position.xyz[1] + radius, position.xyz[2]],
            radius, self, cvars, &self.debug_overlay,
        ).map_err(E::PedestrianSphere)? {
            return Ok(true);
        }
        // 82e018e4..1914: kbShowCollisionStateChecks overlay assertion.
        self.require_collision_state_overlay(cvars)?;
        // 82e01934..19bc: GetSkeleton()->GetPartTransform(1, 15, 19) rows 3.
        let [head, left, right] = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                Ok::<_, SkeletonPartTransformQueryError>([
                    normal.try_query_part_transform(1)?.columns()[3],
                    normal.try_query_part_transform(15)?.columns()[3],
                    normal.try_query_part_transform(19)?.columns()[3],
                ])
            })
            .map_err(E::Pair)?
            .map_err(E::PartTransform)?;
        // The W lanes of these rows reach no consumer: every later test reads
        // X/Y/Z only (vmsum3fp, vsldoi-masked AABBs, GP instance rows).
        let lanes = |vector: SkeletonRootVector| [vector.xyz[0], vector.xyz[1], vector.xyz[2], 0.0];
        let (head, left, right, root) = (lanes(head), lanes(left), lanes(right), lanes(position));
        // 82e019c4..1a8c: the toe midpoint with Y the lowest of right, left and
        // AnimToWorld (each `vcmpgtfp.` of splats; unordered keeps the later).
        let middle: [f32; 4] = std::array::from_fn(|lane| (left[lane] + right[lane]) * 0.5);
        let lower = if right[1] > left[1] { left } else { right };
        let lowest = if lower[1] > root[1] { root } else { lower };
        let feet = [middle[0], lowest[1], middle[2], middle[3]];
        let test_cvars = SkaterTestCvars {
            width: cvars.try_spawn_test_capsule_width().map_err(E::Cvar)?,
            height: cvars.try_spawn_test_capsule_height().map_err(E::Cvar)?,
            feet_padding: cvars
                .try_spawn_test_capsule_feet_padding()
                .map_err(E::Cvar)?,
            head_offset: cvars
                .try_spawn_test_actor_aabb_head_offset()
                .map_err(E::Cvar)?,
        };
        let show = cvars.try_show_spawn_test_capsule().map_err(E::Cvar)?;
        let matrix = anim_to_world.xyz_scratch();
        // 82e01a68..1abc: BuildSkaterTestCapsule into HiLOD+0x334 (read by no
        // later consumer), then BuildSkaterTestAABBox; each asserts the
        // DebugOverlay under kShowSpawnTestCapsule.
        let capsule =
            build_skater_test_capsule(&matrix, head, feet, &test_cvars).map_err(E::Numeric)?;
        if show {
            self.debug_overlay.require().map_err(E::Overlay)?;
        }
        let bounds = build_skater_test_aabbox(head, feet, &test_cvars);
        if show {
            self.debug_overlay.require().map_err(E::Overlay)?;
        }
        // 82e01ac0..1ae8: DoesCapsuleAndAABBoxIntersectAnyDynamicObject.
        let min_distance = cvars
            .try_spawn_test_capsule_min_distance()
            .map_err(E::Cvar)?;
        let objects = living
            .dmo_objects
            .as_ref()
            .ok_or(E::UninitializedLivingWorld)?;
        if self
            .try_capsule_intersects_any_dynamic_object(
                objects,
                &capsule,
                bounds,
                min_distance,
                show,
            )
            .map_err(E::DynamicObject)?
        {
            // 82e01af0..1b00: an overlap skips the vehicle sphere and its
            // overlay assertion.
            return Ok(true);
        }
        // 82e01b04..1b64: DoesAnyVehicleIntersectSphere82710760 with the first
        // kLWTestRadius read: radius + 8, six IDs.
        let lifted = [
            position.xyz[0],
            position.xyz[1] + first_radius,
            position.xyz[2],
        ];
        if living
            .try_vehicle_intersects_sphere(lifted, first_radius, cvars, &self.debug_overlay)
            .map_err(E::VehicleSphere)?
        {
            return Ok(true);
        }
        drop(living);
        // 82e01b68..1b98: kbShowCollisionStateChecks overlay assertion.
        self.require_collision_state_overlay(cvars)?;
        Ok(false)
    }

    /// `kbShowCollisionStateChecks` asserts on the DebugOverlay singleton.
    fn require_collision_state_overlay(
        &self,
        cvars: &InputCvarReferences,
    ) -> Result<(), Update0Error> {
        if cvars
            .try_show_collision_state_checks()
            .map_err(Update0Error::Cvar)?
        {
            self.debug_overlay
                .require()
                .map_err(Update0Error::Overlay)?;
        }
        Ok(())
    }
}
