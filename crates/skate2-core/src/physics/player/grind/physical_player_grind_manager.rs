//! Actual `GrindManager82dd36e0` and its seven separately allocated children.
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::{Arc, Mutex, Weak};
#[path = "physical_player_grind_collision.rs"]
pub(crate) mod collision;
#[path = "physical_player_grind_manager_fields.rs"]
pub(crate) mod fields;
#[path = "physical_player_grind_find_nearby.rs"]
pub(crate) mod find_nearby;
#[path = "physical_player_grind_geometry.rs"]
pub(crate) mod geometry;
#[path = "physical_player_grind_investigator.rs"]
pub(crate) mod investigator;
#[path = "physical_player_grind_math.rs"]
pub(crate) mod math;
#[path = "physical_player_grind_suffix.rs"]
pub(crate) mod suffix;
#[path = "physical_player_grind_types.rs"]
pub(crate) mod types;
#[path = "physical_player_grind_update_prefix.rs"]
pub(crate) mod update_prefix;
#[path = "physical_player_grind_view.rs"]
pub(crate) mod view;
use fields::{
    GrindBalance, GrindCollision, GrindControls, GrindEngagement, GrindInvestigator, GrindJumper,
    GrindMaterials,
};
use types::Investigation;

/// Actual Wipeout, `StateChanger` and `TrajectorySelector` are reached through this
/// same physical owner. Null source links remain null in their owning slots.
#[derive(Clone)]
pub(crate) struct GrindBindings {
    pub(crate) owner: Weak<PhysicalPlayerAssemblyOwner>,
    pub(crate) processed: Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) board: SimulationSkaterAssemblyHandle,
}
impl GrindBindings {
    pub(crate) fn new(
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: Arc<Mutex<SkeletonProcessedContact>>,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Self {
        Self {
            owner: Arc::downgrade(owner),
            processed,
            board: board.clone(),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GrindAllocationSite {
    Collision,
    Materials,
    Investigator,
    Balance,
    Engagement,
    Controls,
    Jumper,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GrindChildPhase {
    Unattempted,
    AllocationNull,
    Constructing,
    Published,
}
#[derive(Debug)]
pub(crate) enum GrindConstructorError {
    AlreadyAttempted,
    Cvar(InputCvarError),
}
impl std::fmt::Display for GrindConstructorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => f.write_str("Grind children construction already attempted"),
            Self::Cvar(e) => write!(f, "Grind balance constructor cvar: {e:?}"),
        }
    }
}
pub(crate) struct GrindManager {
    pub(crate) bindings: GrindBindings,
    pub(crate) investigation: Investigation,
    pub(crate) collision: Option<Arc<GrindCollision>>,
    pub(crate) materials: Option<GrindMaterials>,
    pub(crate) investigator: Option<GrindInvestigator>,
    /// Native Balance pointer is not published until both cvar reads complete.
    /// The separate candidate retains allocation/first counter if the second fails.
    pub(crate) balance: Option<GrindBalance>,
    pub(crate) balance_candidate: Option<GrindBalance>,
    pub(crate) engagement: Option<GrindEngagement>,
    pub(crate) controls: Option<GrindControls>,
    pub(crate) jumper: Option<GrindJumper>,
    pub(crate) phases: [GrindChildPhase; 7],
    pub(crate) complete: bool,
    pub(crate) flicker_state: i32,
    pub(crate) time_in_grind: f32,
    pub(crate) gravity_relief_timer: f32,
    pub(crate) grind_forbidden: bool,
    pub(crate) too_much_flickering: bool,
    pub(crate) flicker_score: i32,
    pub(crate) flicker_avoid_frames: i32,
}
impl GrindManager {
    /// Actual outer Sim zero initializes the fields not written before children.
    /// `GrindGeometry` ctor is immediately overwritten by `Investigation.Reset`.
    pub(crate) fn new_sim_prefix(bindings: GrindBindings) -> Self {
        Self {
            bindings,
            investigation: Investigation::reset(),
            collision: None,
            materials: None,
            investigator: None,
            balance: None,
            balance_candidate: None,
            engagement: None,
            controls: None,
            jumper: None,
            phases: [GrindChildPhase::Unattempted; 7],
            complete: false,
            flicker_state: 0,
            time_in_grind: 0.0,
            gravity_relief_timer: 0.0,
            grind_forbidden: false,
            too_much_flickering: false,
            flicker_score: 0,
            flicker_avoid_frames: 0,
        }
    }
    #[expect(
        clippy::too_many_lines,
        reason = "source ordered allocations preserve null and partially constructed child prefixes"
    )]
    pub(crate) fn try_construct_children(
        &mut self,
        refs: &InputCvarReferences,
        allocate: &mut impl FnMut(GrindAllocationSite) -> bool,
    ) -> Result<(), GrindConstructorError> {
        use GrindChildPhase::{AllocationNull, Constructing, Published, Unattempted};
        if self.phases[0] != Unattempted {
            return Err(GrindConstructorError::AlreadyAttempted);
        }
        if allocate(GrindAllocationSite::Collision) {
            self.collision = Some(Arc::new(GrindCollision {
                bindings: self.bindings.clone(),
                result: Mutex::new(crate::grind_data_runtime::GrindDataQueryResult::new()),
            }));
            self.phases[0] = Published;
        } else {
            self.phases[0] = AllocationNull;
        }
        if allocate(GrindAllocationSite::Materials) {
            self.materials = Some(GrindMaterials {
                bindings: self.bindings.clone(),
                collision: self.collision.clone(),
                was_targeting: false,
            });
            self.phases[1] = Published;
        } else {
            self.phases[1] = AllocationNull;
        }
        if allocate(GrindAllocationSite::Investigator) {
            self.investigator = Some(GrindInvestigator {
                bindings: self.bindings.clone(),
                collision: self.collision.clone(),
                avoid_grind_frames: 0,
                avoid_five_o_frames: 0,
                ground_frames: 0,
                ground_to_boardslide_timer: 0,
                non_ground_frames: 0,
                hang_primitive_existed: false,
                last_velocity: [0.0; 4],
            });
            self.phases[2] = Published;
        } else {
            self.phases[2] = AllocationNull;
        }
        if allocate(GrindAllocationSite::Balance) {
            self.phases[3] = Constructing;
            self.balance_candidate = Some(GrindBalance {
                bindings: self.bindings.clone(),
                time_in_droppable_grind: 0.0,
                exit_tilt_angle: 0.0,
                exit_start_time: None,
                non_grind_frames: None,
                grind_normal: [0.0, 1.0, 0.0, 0.0],
                high_side_direction: [1.0, 0.0, 0.0, 0.0],
            });
            let value = self
                .balance_candidate
                .as_mut()
                .expect("retained Balance candidate");
            value.non_grind_frames = Some(
                refs.try_lean_min_non_grind_frames()
                    .map_err(GrindConstructorError::Cvar)?
                    .wrapping_add(1),
            );
            value.exit_start_time = Some(
                refs.try_curb_exit_start_time()
                    .map_err(GrindConstructorError::Cvar)?,
            );
            self.balance = self.balance_candidate.take();
            self.phases[3] = Published;
        } else {
            self.phases[3] = AllocationNull;
        }
        if allocate(GrindAllocationSite::Engagement) {
            self.engagement = Some(GrindEngagement {
                bindings: self.bindings.clone(),
                tipslide_grab_timer: 0,
            });
            self.phases[4] = Published;
        } else {
            self.phases[4] = AllocationNull;
        }
        if allocate(GrindAllocationSite::Controls) {
            self.controls = Some(GrindControls {
                bindings: self.bindings.clone(),
                lr: 0.0,
                ud: 0.0,
            });
            self.phases[5] = Published;
        } else {
            self.phases[5] = AllocationNull;
        }
        if allocate(GrindAllocationSite::Jumper) {
            self.jumper = Some(GrindJumper::new(self.bindings.clone()));
            self.phases[6] = Published;
        } else {
            self.phases[6] = AllocationNull;
        }
        self.time_in_grind = 0.0;
        self.gravity_relief_timer = 0.0;
        self.flicker_state = 0;
        self.investigation.possible = false;
        self.grind_forbidden = false;
        self.too_much_flickering = false;
        self.flicker_score = 0;
        self.flicker_avoid_frames = 0;
        self.complete = true;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn check_original(value: &GrindManager, owner: &Arc<PhysicalPlayerAssemblyOwner>) {
    assert!(value.complete);
    assert_eq!(value.phases, [GrindChildPhase::Published; 7]);
    assert!(Arc::ptr_eq(&value.bindings.owner.upgrade().unwrap(), owner));
    let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
    assert!(Arc::ptr_eq(&value.bindings.processed, &processed));
    let collision = value.collision.as_ref().unwrap();
    assert_eq!(collision.result.lock().unwrap().count(), 0);
    for index in 0..40 {
        assert_eq!(
            collision.result.lock().unwrap().flag_bits(index),
            Some((0, 0))
        );
    }
    assert!(Arc::ptr_eq(
        value
            .materials
            .as_ref()
            .unwrap()
            .collision
            .as_ref()
            .unwrap(),
        collision
    ));
    assert!(Arc::ptr_eq(
        value
            .investigator
            .as_ref()
            .unwrap()
            .collision
            .as_ref()
            .unwrap(),
        collision
    ));
    let balance = value.balance.as_ref().unwrap();
    assert!(value.balance_candidate.is_none());
    assert_eq!(balance.non_grind_frames, Some(21));
    assert_eq!(
        balance.exit_start_time.map(f32::to_bits),
        Some(2.0_f32.to_bits())
    );
    assert_eq!(value.investigation.grind_type, 0);
    assert!(!value.investigation.possible);
    assert_eq!(value.jumper.as_ref().unwrap().last_grind_type, 3);
}
