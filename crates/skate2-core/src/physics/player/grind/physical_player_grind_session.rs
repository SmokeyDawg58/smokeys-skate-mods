//! Update1 `GrindManager::Update` 0x82de1798 (called at 82e038ec): the
//! recovered prefix through `FindNearbyPrimitives` 82de1828, then Materials
//! 82de1830, the investigation, geometry, coping, balance, engagement,
//! controls, jumper and the copy into `ProcessedPhysIn` +420 at 82de1924.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GRIND, PHYSICS_CATEGORY_GROUND, PHYSICS_CATEGORY_OFFBOARD,
    PHYSICS_STATE_KNOWN_AIR, PHYSICS_STATE_NONSPECIFIC, PHYSICS_STATE_PHYSICS_AIR,
    PHYSICS_STATE_WIPEOUT_GROUND,
};
use super::grind_world::PhysicalGrindError;
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    collision_mesh_line::CollisionMeshLineNearest,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_grind_manager::{
        geometry::{GeometryLineBatch, GeometryLines, LineHit, calc_grind_geometry},
        investigator::InvestigatorContext,
        math::{GrindNum, V4},
        suffix::{self, SuffixEffects},
        update_prefix::GrindPrefixError,
        view::{GrindAttributes, GrindCvars, GrindError, GrindView, error, lanes, root},
    },
    physical_player_wipeout::ground_access::GroundOwnerError,
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_dmo_assembly_part_session::dynamic_pool::{SimulationDmoAssemblyHandle, SimulationDmoAssemblyPool},
    skeleton_processed_contact::SkeletonProcessedContact,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum PhysicalGrindPrefixError {
    Missing(&'static str),
    Poisoned,
    Pair(SimulationSkeletonJumpPairError),
    Prefix(GrindPrefixError<PhysicalGrindError>),
    Materials(super::physical_board_forces::board_wipeout::PhysicalBoardWipeoutError),
    Wipeout(GroundOwnerError),
    Grind(GrindError),
}
impl std::fmt::Display for PhysicalGrindPrefixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "physical grind prefix missing {field}"),
            Self::Poisoned => f.write_str("physical grind manager lock poisoned"),
            Self::Pair(error) => write!(f, "physical grind pair: {error:?}"),
            Self::Prefix(error) => error.fmt(f),
            Self::Materials(error) => write!(f, "grind materials: {error}"),
            Self::Wipeout(error) => write!(f, "grind wipeout request: {error}"),
            Self::Grind(error) => error.fmt(f),
        }
    }
}

/// The selected World island's line services: the `DoGeometryTest` batch
/// ("GrindGeometryAnalyzer::DoGeometryTest", `BF_Default` 7, fatness zero)
/// and `PhysicsUtility::LineWorldCollisionTest` of `UpdateForceExit`.
pub(crate) struct WorldLines<'a> {
    pub(crate) queries: &'a mut super::world_line_query::SimulationWorldLineQueries,
    pub(crate) dmos: &'a SimulationDmoAssemblyPool,
    pub(crate) ground: &'a StaticGroundPool,
    pub(crate) numeric: &'a mut HostCollisionLineNumeric,
}
fn line_hit(hit: CollisionMeshLineNearest<SimulationDmoAssemblyHandle>) -> LineHit {
    LineHit {
        hit: true,
        fraction: hit.line_parameter,
        position: lanes(hit.position),
        normal: lanes(hit.normal),
        surface: u32::from(hit.surface_id),
    }
}
fn line(start: V4, end: V4) -> CollisionLineInput {
    CollisionLineInput {
        start: root(start),
        end: root(end),
        fatness: 0.0,
    }
}
impl GeometryLineBatch for WorldLines<'_> {
    fn try_test(&mut self, lines: &GeometryLines) -> Result<[Option<LineHit>; 5], GrindError> {
        let batch = self
            .queries
            .try_acquire_line_batch(self.dmos, "GrindGeometryAnalyzer::DoGeometryTest", 7)
            .map_err(error)?;
        self.queries.try_begin(&batch).map_err(error)?;
        for (start, end) in lines {
            self.queries
                .try_add_line(&batch, line(*start, *end))
                .map_err(error)?;
        }
        self.queries
            .try_end(&batch, self.ground, self.dmos, self.numeric)
            .map_err(error)?;
        self.queries.try_sync(&batch).map_err(error)?;
        let mut out = [None; 5];
        for (index, slot) in (0_u32..).zip(out.iter_mut()) {
            *slot = self
                .queries
                .try_get_result(&batch, index)
                .map_err(error)?
                .map(line_hit);
        }
        self.queries.try_release(&batch).map_err(error)?;
        Ok(out)
    }
}
impl WorldLines<'_> {
    pub(crate) fn try_single(
        &mut self,
        label: &str,
        start: V4,
        end: V4,
    ) -> Result<Option<LineHit>, GrindError> {
        self.queries
            .try_line_test(
                self.ground,
                self.dmos,
                line(start, end),
                label,
                self.numeric,
            )
            .map(|hit| hit.map(line_hit))
            .map_err(error)
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_player_grind_prefix(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalGrindPrefixError> {
        use PhysicalGrindPrefixError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let mut grind = owner.grind().lock().map_err(|_| E::Poisoned)?;
        let grind = grind.as_mut().ok_or(E::Missing("GrindManager"))?;
        let cvars = &owner.inputs().cvars;
        grind
            .try_update_prefix(cvars, &self.debug_overlay, &mut |bounds, result| {
                self.grind_services
                    .try_box_test(bounds, result, false, cvars)
            })
            .map_err(E::Prefix)
    }

    /// The whole native `GrindManager::Update`.
    #[expect(clippy::too_many_lines, reason = "one ordered native manager update")]
    pub(crate) fn try_update_player_grind_manager(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        numeric: &mut HostReckoningNumeric,
        sin_cos: &mut HostReckoningSinCosNumeric,
        lines: &mut HostCollisionLineNumeric,
    ) -> Result<(), PhysicalGrindPrefixError> {
        use PhysicalGrindPrefixError as E;
        self.try_update_player_grind_prefix(board, skeleton)?;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let refs = owner.inputs().cvars.clone();
        let cvars = GrindCvars(&refs);
        let attributes = GrindAttributes::bind(&owner.inputs().vault);
        let overlay = self.debug_overlay.clone();
        let mut num = GrindNum {
            n: numeric,
            s: sin_cos,
        };
        let mut guard = owner.grind().lock().map_err(|_| E::Poisoned)?;
        let grind = guard.as_mut().ok_or(E::Missing("GrindManager"))?;
        let processed = Arc::clone(&grind.bindings.processed);
        let view =
            GrindView::try_read(&*processed.lock().map_err(|_| E::Poisoned)?).map_err(E::Grind)?;
        let prims: Vec<_> = grind
            .collision
            .as_ref()
            .ok_or(E::Missing("GrindCollision"))?
            .result
            .lock()
            .map_err(|_| E::Poisoned)?
            .results()
            .cloned()
            .collect();
        // GrindMaterialsManager::Update 0x82dd3178.
        if view.state != PHYSICS_STATE_WIPEOUT_GROUND && view.category != PHYSICS_CATEGORY_OFFBOARD {
            let materials = grind
                .materials
                .as_mut()
                .ok_or(E::Missing("GrindMaterials"))?;
            let near_in_air = !prims.is_empty() && matches!(view.state, PHYSICS_STATE_PHYSICS_AIR | PHYSICS_STATE_KNOWN_AIR);
            let grinding = view.category == PHYSICS_CATEGORY_GRIND || view.state == PHYSICS_STATE_NONSPECIFIC;
            let landing = view.category == PHYSICS_CATEGORY_GROUND && materials.was_targeting;
            let apply = near_in_air || grinding || landing;
            materials.was_targeting = view.state == PHYSICS_STATE_KNOWN_AIR && {
                let trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
                trajectories
                    .onboard
                    .as_ref()
                    .ok_or(E::Missing("onboard TrajectorySelector"))?
                    .targeting_grind
            };
            self.try_set_board_grind_materials(board, apply)
                .map_err(E::Materials)?;
        }
        let forbidden = grind.grind_forbidden;
        let mut investigation = grind
            .investigator
            .as_mut()
            .ok_or(E::Missing("GrindInvestigator"))?
            .try_update_investigation(
                &mut InvestigatorContext {
                    view: &view,
                    attributes: &attributes,
                    cvars: &cvars,
                    prims: &prims,
                    overlay: &overlay,
                    num: &mut num,
                },
                forbidden,
            )
            .map_err(E::Grind)?;
        suffix::update_friction_vs_time(&mut investigation, grind.time_in_grind, &attributes)
            .map_err(E::Grind)?;
        if investigation.possible {
            let (queries, dmos) = self.world_line_owners();
            let mut world = WorldLines {
                queries: queries.ok_or(E::Missing("World line queries"))?,
                dmos: dmos.ok_or(E::Missing("World DMO owner"))?,
                ground,
                numeric: &mut *lines,
            };
            investigation.geometry = calc_grind_geometry(
                &investigation.primitive,
                investigation.primitive_location,
                &attributes,
                &cvars,
                &mut world,
                &mut num,
            )
            .map_err(E::Grind)?;
            let balance = grind.balance.as_ref().ok_or(E::Missing("GrindBalance"))?;
            let (tilt, high_side) = (balance.exit_tilt_angle, balance.high_side_direction);
            grind
                .investigator
                .as_mut()
                .ok_or(E::Missing("GrindInvestigator"))?
                .try_tweak_for_geometry(
                    &mut InvestigatorContext {
                        view: &view,
                        attributes: &attributes,
                        cvars: &cvars,
                        prims: &prims,
                        overlay: &overlay,
                        num: &mut num,
                    },
                    &mut investigation,
                    tilt,
                    high_side,
                )
                .map_err(E::Grind)?;
        }
        grind.gravity_relief_timer = suffix::manage_coping_assistance(
            &mut investigation,
            grind.gravity_relief_timer,
            &view,
            &attributes,
            &cvars,
            &mut num,
        )
        .map_err(E::Grind)?;
        {
            let balance = grind.balance.as_mut().ok_or(E::Missing("GrindBalance"))?;
            balance
                .try_update_target_up(&mut investigation, &view, &cvars, &mut num)
                .map_err(E::Grind)?;
            balance
                .try_update_exit_lean(&mut investigation, &view, &attributes, &cvars, &mut num)
                .map_err(E::Grind)?;
            let (queries, dmos) = self.world_line_owners();
            let mut world = WorldLines {
                queries: queries.ok_or(E::Missing("World line queries"))?,
                dmos: dmos.ok_or(E::Missing("World DMO owner"))?,
                ground,
                numeric: &mut *lines,
            };
            balance
                .try_update_force_exit(
                    &mut investigation,
                    &cvars,
                    &mut |start, end| world.try_single("GrindBalance::UpdateForceExit", start, end),
                    &mut num,
                )
                .map_err(E::Grind)?;
        }
        let mut effects = SuffixEffects::default();
        grind
            .engagement
            .as_mut()
            .ok_or(E::Missing("GrindEngagement"))?
            .try_update(
                &mut investigation,
                &view,
                &attributes,
                &cvars,
                &mut num,
                &mut effects,
            )
            .map_err(E::Grind)?;
        grind
            .controls
            .as_mut()
            .ok_or(E::Missing("GrindControls"))?
            .try_update_controls(&mut investigation, &view, &cvars, &mut num)
            .map_err(E::Grind)?;
        grind
            .jumper
            .as_mut()
            .ok_or(E::Missing("GrindJumper"))?
            .try_update(&investigation, &view, &cvars, &mut effects)
            .map_err(E::Grind)?;
        grind.investigation.clone_from(&investigation);
        drop(guard);
        apply_effects(&owner, &processed, &overlay, &effects)?;
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .publish_grind_investigation(&investigation);
        Ok(())
    }
}

/// The suffix's writes into other owners: overlay singleton assertions,
/// `mIsWipingOut` (8c4 0x40000), the wipeout requests (value zero, one counter
/// step each) and the jumper's 8cc 0x08000000. Nothing later in the same
/// manager Update reads any of them, so applying them here keeps the order.
pub(crate) fn apply_effects(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    processed: &Arc<Mutex<SkeletonProcessedContact>>,
    overlay: &crate::debug_overlay::DebugOverlaySlot,
    effects: &SuffixEffects,
) -> Result<(), PhysicalGrindPrefixError> {
    use PhysicalGrindPrefixError as E;
    for _ in 0..effects.overlay_lookups {
        overlay.require().map_err(|e| E::Grind(error(e)))?;
    }
    if effects.wiping_out {
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .write_control_flag8c4(0x0004_0000, true);
    }
    if !effects.wipeout_requests.is_empty() {
        let mut wipeout = owner.wipeout().lock().map_err(|_| E::Poisoned)?;
        for &index in &effects.wipeout_requests {
            wipeout
                .try_wipeout_request_value(index, 0.0)
                .map_err(E::Wipeout)?;
        }
    }
    if effects.avoid_grinds
        && !processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .set_flag_8cc(0x0800_0000, true)
    {
        return Err(E::Missing("Processed flags8cc"));
    }
    Ok(())
}
