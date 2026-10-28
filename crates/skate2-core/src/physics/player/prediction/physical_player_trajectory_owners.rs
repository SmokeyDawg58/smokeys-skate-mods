//! Retained trajectory constructor owners preceding either Body in `HiLOD`.
//! Offboard constructor82dc0bc0, caller82e04068; generated physics827b2660.
#![expect(
    dead_code,
    reason = "retained native trajectory fields await launched prediction consumers"
)]
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    grind_data_runtime::GrindDataQueryResult,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_footplant::RetainedTrajectory,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, Mutex, Weak};

#[path = "physical_player_trajectory_adjust_start.rs"]
pub(crate) mod adjust_start;
#[path = "../runtime/physical_player_onboard_launch_prefix.rs"]
pub(crate) mod launch_prefix;
#[path = "physical_player_trajectory_max_time.rs"]
pub(crate) mod max_time;
#[path = "../skeleton/physical_player_skeleton_launch_info.rs"]
pub(crate) mod skeleton_launch_info;

#[path = "physical_player_trajectory_launch_prepare.rs"]
pub(crate) mod launch_prepare;

#[path = "physical_player_trajectory_launch_continue.rs"]
pub(crate) mod launch_continue;

#[path = "../offboard/physical_player_offboard_trajectory.rs"]
pub(crate) mod offboard_runtime;

#[derive(Default)]
pub(crate) struct PhysicalTrajectoryOwners {
    pub(crate) onboard: Option<OnboardTrajectoryOwner>,
    pub(crate) offboard: Option<OffboardTrajectoryOwner>,
}

#[cfg(test)]
pub(crate) fn check_original(owner: &Arc<PhysicalPlayerAssemblyOwner>) {
    let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
    let trajectories = owner.trajectories().lock().unwrap();
    let onboard = trajectories.onboard.as_ref().unwrap();
    let offboard = trajectories.offboard.as_ref().unwrap();
    assert!(onboard.bindings.matches(owner, &processed));
    assert!(offboard.bindings.matches(owner, &processed));
    assert!(!offboard.attributes.is_null());
    assert!(!offboard.physics_attributes.is_null());
    assert!(!onboard.ready && !offboard.ready);
    assert!(onboard.winning_result.is_none() && offboard.winning_result.is_none());
    for query in onboard.queries.iter().chain(&offboard.queries) {
        assert!(!query.ready);
        assert_eq!(
            query.result.trajectory.time.map(f32::to_bits),
            [(-1.0_f32).to_bits(); 4]
        );
        // Zero parent collision result is retained but cannot signal completion.
        assert_eq!(query.result.time.map(f32::to_bits), [0; 4]);
    }
    let orientations = onboard.possible_orientations.as_ref().unwrap();
    assert!(orientations.geometry_type.is_none());
    assert!(orientations.boardslide_direction.is_none());
    assert!(onboard.grind_trajectory_results.is_empty());
    assert_eq!(onboard.grind_test_results.count(), 0);
}

/// Collision payload is initially zero because these parents are Sim allocations
/// (category55, OR100), not because `PredictionResults` initializes these fields.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RetainedPredictionResult {
    pub(crate) position: SkeletonRootVector,
    pub(crate) normal: SkeletonRootVector,
    pub(crate) ground_normal: SkeletonRootVector,
    pub(crate) time: [f32; 4],
    pub(crate) frame: i32,
    pub(crate) surface: u32,
    pub(crate) trajectory: RetainedTrajectory,
    pub(crate) radius: f32,
    pub(crate) max_error_start: f32,
    pub(crate) max_error_end: f32,
    pub(crate) draw_true: bool,
}
impl RetainedPredictionResult {
    const fn in_sim_allocation() -> Self {
        Self {
            position: SkeletonRootVector::known([0.0; 4]),
            normal: SkeletonRootVector::known([0.0; 4]),
            ground_normal: SkeletonRootVector::known([0.0; 4]),
            time: [0.0; 4],
            frame: 0,
            surface: 0,
            trajectory: RetainedTrajectory::constructed(),
            radius: 0.0,
            max_error_start: 0.0,
            max_error_end: 0.0,
            draw_true: false,
        }
    }
}

/// Native future stride0xb0 includes the result and its own readiness byte+a0.
/// Construction launches no query and publishes no completed prediction.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RetainedPredictionFuture {
    pub(crate) result: RetainedPredictionResult,
    pub(crate) ready: bool,
}
impl RetainedPredictionFuture {
    pub(crate) const fn in_sim_allocation() -> Self {
        Self {
            result: RetainedPredictionResult::in_sim_allocation(),
            ready: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct OffboardTrajectoryData {
    pub(crate) trajectory: RetainedTrajectory,
    pub(crate) landing_normal: SkeletonRootVector,
    pub(crate) landing_velocity: SkeletonRootVector,
    pub(crate) landing_position: SkeletonRootVector,
    pub(crate) index_shift: i32,
    pub(crate) collision_index: i32,
    pub(crate) collision_found: bool,
}
impl OffboardTrajectoryData {
    const fn in_sim_allocation() -> Self {
        Self {
            trajectory: RetainedTrajectory::constructed(),
            landing_normal: SkeletonRootVector::known([0.0; 4]),
            landing_velocity: SkeletonRootVector::known([0.0; 4]),
            landing_position: SkeletonRootVector::known([0.0; 4]),
            // Native ctor preserves these two cells; the actual parent is zeroed.
            index_shift: 0,
            collision_index: 0,
            collision_found: false,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct OffboardLaunchInfo {
    pub(crate) start_velocity: SkeletonRootVector,
    pub(crate) start_com_position: SkeletonRootVector,
    pub(crate) cone_angle_x: f32,
    pub(crate) cone_angle_z: f32,
    pub(crate) num_trajectories: u32,
}

/// Same source owner and Processed pointer; allocation is retained by the physical
/// parent, so the back-reference must be weak to avoid a host ownership cycle.
pub(crate) struct TrajectoryOwnerBindings {
    owner: Weak<PhysicalPlayerAssemblyOwner>,
    processed: Arc<Mutex<SkeletonProcessedContact>>,
}
impl TrajectoryOwnerBindings {
    pub(crate) fn new(
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: Arc<Mutex<SkeletonProcessedContact>>,
    ) -> Self {
        Self {
            owner: Arc::downgrade(owner),
            processed,
        }
    }
    pub(crate) fn matches(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
    ) -> bool {
        self.owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
            && Arc::ptr_eq(&self.processed, processed)
    }
}

/// PDB size0x14e0. Default geometry is not evidence of a launched/completed query.
#[expect(
    clippy::struct_excessive_bools,
    reason = "retain distinct native query flags"
)]
pub(crate) struct OffboardTrajectoryOwner {
    pub(crate) bindings: TrajectoryOwnerBindings,
    attributes: AttributeRuntimeInstance,
    physics_attributes: AttributeRuntimeInstance,
    pub(crate) queries: [RetainedPredictionFuture; 16],
    pub(crate) winning_result: Option<usize>,
    pub(crate) launch: OffboardLaunchInfo,
    pub(crate) data: [OffboardTrajectoryData; 16],
    pub(crate) winning_data: OffboardTrajectoryData,
    pub(crate) winning_com: RetainedTrajectory,
    pub(crate) raw_trajectory: RetainedTrajectory,
    pub(crate) trajectory_offset: SkeletonRootVector,
    pub(crate) trajectory_blend: f32,
    pub(crate) trajectory_blend_time: f32,
    pub(crate) scores: [f32; 16],
    pub(crate) num_trajectories: i32,
    pub(crate) best_trajectory: i32,
    pub(crate) land_index: i32,
    pub(crate) current_index: i32,
    pub(crate) just_started_query: bool,
    pub(crate) ready: bool,
    pub(crate) ai_trajectory_following: bool,
    pub(crate) has_just_changed: bool,
}
impl OffboardTrajectoryOwner {
    /// `OffboardTrajectorySelector::Reset` 0x82db6590. Query futures, candidate
    /// data, scores, launch info, and the winning result remain untouched.
    pub(crate) fn reset(&mut self) {
        self.ready = false;
        self.ai_trajectory_following = false;
        self.has_just_changed = false;
        self.just_started_query = false;
        self.winning_com = RetainedTrajectory::constructed();
        self.raw_trajectory = RetainedTrajectory::constructed();
        self.current_index = 0;
        self.trajectory_blend = 0.0;
        self.land_index = 1000;
        self.trajectory_blend_time = 0.0;
    }

    pub(crate) fn new(
        bindings: TrajectoryOwnerBindings,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Self {
        // Raw82dc0be4..c00: decompiler miscombines the two 64-bit arguments.
        let attributes = vault.instance(0xcdd9_26b3_5ef0_f6c4, 0xd7ed_bd36_2d7d_2152);
        let physics_attributes = vault.instance(0x0429_3d0f_2adc_fbb4, 0xd7ed_bd36_2d7d_2152);
        Self {
            bindings,
            attributes,
            physics_attributes,
            queries: [RetainedPredictionFuture::in_sim_allocation(); 16],
            winning_result: None,
            launch: OffboardLaunchInfo {
                start_velocity: SkeletonRootVector::known([0.0; 4]),
                start_com_position: SkeletonRootVector::known([0.0; 4]),
                cone_angle_x: 0.0,
                cone_angle_z: 0.0,
                num_trajectories: 1,
            },
            data: [OffboardTrajectoryData::in_sim_allocation(); 16],
            winning_data: OffboardTrajectoryData::in_sim_allocation(),
            winning_com: RetainedTrajectory::constructed(),
            raw_trajectory: RetainedTrajectory::constructed(),
            trajectory_offset: SkeletonRootVector::known([0.0; 4]),
            trajectory_blend: 0.0,
            trajectory_blend_time: 0.0,
            scores: [0.0; 16],
            num_trajectories: 0,
            best_trajectory: 0,
            land_index: 0,
            current_index: 0,
            just_started_query: false,
            ready: false,
            ai_trajectory_following: false,
            has_just_changed: false,
        }
    }
}

/// Native affine default has I/J/K axes and a full zero translation column,
/// including W. Do not use a conventional homogeneous identity with final1.
const AFFINE_DEFAULT: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0; 4],
];

/// Constructor82d83cf8, distinct from the older trace-fed launch input.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RetainedTrajectoryLaunchInfo {
    pub(crate) reckoning_orientation: SkeletonRootMatrix,
    pub(crate) reckoning_orientation_inverse: SkeletonRootMatrix,
    pub(crate) start_velocity: SkeletonRootVector,
    pub(crate) velocity_before_jump: SkeletonRootVector,
    pub(crate) animated_com_position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) animated_board_position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) board_position: SkeletonRootVector,
    pub(crate) com_position: SkeletonRootVector,
    pub(crate) trajectory_start_override: SkeletonRootVector,
    pub(crate) board_position_override: SkeletonRootVector,
    pub(crate) cone_angle_x: f32,
    pub(crate) cone_angle_z: f32,
    pub(crate) time_delta: f32,
    pub(crate) player_jumped: bool,
    pub(crate) use_start_override: bool,
    pub(crate) num_trajectories: u16,
}
impl RetainedTrajectoryLaunchInfo {
    pub(crate) fn constructed() -> Self {
        Self {
            reckoning_orientation: SkeletonRootMatrix::from_columns(
                AFFINE_DEFAULT.map(SkeletonRootVector::known),
            ),
            reckoning_orientation_inverse: SkeletonRootMatrix::from_columns(
                AFFINE_DEFAULT.map(SkeletonRootVector::known),
            ),
            start_velocity: SkeletonRootVector::known([0.0; 4]),
            velocity_before_jump: SkeletonRootVector::known([0.0; 4]),
            animated_com_position: crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4],
            ),
            animated_board_position: crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4],
            ),
            board_position: SkeletonRootVector::known([0.0; 4]),
            com_position: SkeletonRootVector::known([0.0; 4]),
            trajectory_start_override: SkeletonRootVector::known([0.0; 4]),
            board_position_override: SkeletonRootVector::known([0.0; 4]),
            cone_angle_x: 0.0,
            cone_angle_z: 0.0,
            time_delta: 0.0,
            player_jumped: false,
            use_start_override: false,
            num_trajectories: 0,
        }
    }
}

/// `PhysicsSystems` allocation41@82ff0d60 has OR0, unlike Sim. There is no
/// constructor call after the allocation82dc4e3c: every payload is unwritten.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnwrittenGrindOrientations {
    pub(crate) geometry_type: Option<i32>,
    pub(crate) is_garbage: Option<bool>,
    pub(crate) boardslide_direction: Option<[f32; 4]>,
    pub(crate) tipslide_direction: Option<[f32; 4]>,
    pub(crate) backslash_direction: Option<[f32; 4]>,
    pub(crate) high_side_direction: Option<[f32; 4]>,
}
impl UnwrittenGrindOrientations {
    const fn allocated() -> Self {
        Self {
            geometry_type: None,
            is_garbage: None,
            boardslide_direction: None,
            tipslide_direction: None,
            backslash_direction: None,
            high_side_direction: None,
        }
    }
}

/// Typed zeroed parent payload; published spline provenance is absent until an
/// actual original-world query supplies a primitive. All four vector lanes stay.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RetainedGrindPrimitive {
    pub(crate) start: [f32; 4],
    pub(crate) end: [f32; 4],
    pub(crate) source: Option<crate::grind::GrindPrimitive>,
    pub(crate) flag_word: u32,
}
impl RetainedGrindPrimitive {
    pub(crate) const fn in_sim_allocation() -> Self {
        Self {
            start: [0.0; 4],
            end: [0.0; 4],
            source: None,
            flag_word: 0,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct RetainedGrindTrajectoryResult {
    pub(crate) closest_point: [f32; 4],
    pub(crate) collision_point: [f32; 4],
    pub(crate) grind_direction: [f32; 4],
    pub(crate) trajectory_direction_in_plane: [f32; 4],
    pub(crate) distance_to_grind: f32,
    pub(crate) penetration_time: f32,
    pub(crate) landing_angle: f32,
    pub(crate) penetration_index: i32,
    pub(crate) grind_index: i32,
    pub(crate) valid: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct OnboardTrajectoryData {
    pub(crate) velocity: SkeletonRootVector,
    pub(crate) landing_normal: SkeletonRootVector,
    pub(crate) landing_velocity: SkeletonRootVector,
    pub(crate) landing_position: SkeletonRootVector,
}
impl OnboardTrajectoryData {
    const fn constructed() -> Self {
        Self {
            velocity: SkeletonRootVector::known([0.0; 4]),
            landing_normal: SkeletonRootVector::known([0.0; 4]),
            landing_velocity: SkeletonRootVector::known([0.0; 4]),
            landing_position: SkeletonRootVector::known([0.0; 4]),
        }
    }
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "distinct native trajectory and scoring flags"
)]
pub(crate) struct OnboardTrajectoryOwner {
    pub(crate) queries: [RetainedPredictionFuture; 7],
    pub(crate) winning_result: Option<usize>,
    pub(crate) winning_buffer: RetainedPredictionResult,
    pub(crate) launch: RetainedTrajectoryLaunchInfo,
    pub(crate) data: [OnboardTrajectoryData; 7],
    pub(crate) winning_data: OnboardTrajectoryData,
    pub(crate) winning_com: RetainedTrajectory,
    pub(crate) initial_offset: SkeletonRootVector,
    pub(crate) com_position_local: SkeletonRootVector,
    pub(crate) ai_com_position_local: [f32; 4],
    pub(crate) ai_trajectory_error: [f32; 4],
    pub(crate) board_position_local: SkeletonRootVector,
    pub(crate) start_board_position: SkeletonRootVector,
    pub(crate) start_trajectory_position: SkeletonRootVector,
    pub(crate) suggested_landing_normal: SkeletonRootVector,
    pub(crate) most_vertical_grind_normal: [f32; 4],
    pub(crate) grind_primitive: RetainedGrindPrimitive,
    // Native fixed_list has capacity40 and starts empty. Future insertion must
    // enforce that capacity; Vec capacity alone is not its overflow policy.
    pub(crate) grind_trajectory_results: Vec<RetainedGrindTrajectoryResult>,
    /// `mGrindTestResults` +1ba0, filled by `GrindData::TrajectoryBoxTest`.
    pub(crate) grind_test_results: GrindDataQueryResult,
    pub(crate) grind_test_results_valid: bool,
    pub(crate) num_grind_results: i32,
    pub(crate) bindings: TrajectoryOwnerBindings,
    pub(crate) possible_orientations: Option<Box<UnwrittenGrindOrientations>>,
    pub(crate) scores: [f32; 7],
    pub(crate) wallride_scores: [f32; 7],
    pub(crate) highest_wallride_position: f32,
    pub(crate) pass: i16,
    pub(crate) num_trajectories: i16,
    pub(crate) best_trajectory: i16,
    pub(crate) is_wallride: [bool; 7],
    pub(crate) on_vert: bool,
    pub(crate) grind_bonus_awarded: bool,
    pub(crate) targeting_grind: bool,
    pub(crate) suggested_landing_normal_valid: bool,
    pub(crate) most_vertical_grind_normal_valid: bool,
    pub(crate) just_started_query: bool,
    pub(crate) ready: bool,
    pub(crate) ai_trajectory_following: bool,
    pub(crate) has_just_changed: bool,
    pub(crate) targeting_wallride: bool,
}
impl OnboardTrajectoryOwner {
    /// TrajectorySelector::Reset82db5f78 on the retained onboard selector.
    /// Later query/result and targeting fields survive this selective reset.
    pub(crate) fn reset_after_teleport(&mut self) {
        let zero = SkeletonRootVector::known([0.0; 4]);
        self.ready = false;
        self.most_vertical_grind_normal = [0.0, 1.0, 0.0, 0.0];
        self.ai_trajectory_following = false;
        self.suggested_landing_normal_valid = false;
        self.most_vertical_grind_normal_valid = false;
        self.has_just_changed = false;
        self.pass = 0;
        self.ai_trajectory_error = [0.0; 4];
        self.winning_com.position = zero;
        self.winning_com.velocity = zero;
        self.winning_com.gravity = zero;
        self.winning_com.time = [-1.0; 4];
    }

    /// Constructor82dc4af8. The explicit nested allocation outcome preserves
    /// native NULL without replacing it with an empty initialized orientation.
    pub(crate) fn new(bindings: TrajectoryOwnerBindings, orientations_allocated: bool) -> Self {
        Self {
            queries: [RetainedPredictionFuture::in_sim_allocation(); 7],
            winning_result: None,
            winning_buffer: RetainedPredictionResult::in_sim_allocation(),
            launch: RetainedTrajectoryLaunchInfo::constructed(),
            data: [OnboardTrajectoryData::constructed(); 7],
            winning_data: OnboardTrajectoryData::constructed(),
            winning_com: RetainedTrajectory::constructed(),
            initial_offset: SkeletonRootVector::known([0.0; 4]),
            com_position_local: SkeletonRootVector::known([0.0; 4]),
            ai_com_position_local: [0.0; 4],
            ai_trajectory_error: [0.0; 4],
            board_position_local: SkeletonRootVector::known([0.0; 4]),
            start_board_position: SkeletonRootVector::known([0.0; 4]),
            start_trajectory_position: SkeletonRootVector::known([0.0; 4]),
            suggested_landing_normal: SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
            most_vertical_grind_normal: [0.0, 1.0, 0.0, 0.0],
            grind_primitive: RetainedGrindPrimitive::in_sim_allocation(),
            grind_trajectory_results: Vec::with_capacity(40),
            grind_test_results: GrindDataQueryResult::new(),
            grind_test_results_valid: false,
            num_grind_results: 0,
            bindings,
            possible_orientations: orientations_allocated
                .then(|| Box::new(UnwrittenGrindOrientations::allocated())),
            scores: [0.0; 7],
            wallride_scores: [0.0; 7],
            highest_wallride_position: 0.0,
            pass: 0,
            num_trajectories: 0,
            best_trajectory: 0,
            is_wallride: [false; 7],
            on_vert: false,
            grind_bonus_awarded: false,
            targeting_grind: false,
            suggested_landing_normal_valid: false,
            most_vertical_grind_normal_valid: false,
            just_started_query: false,
            ready: false,
            ai_trajectory_following: false,
            has_just_changed: false,
            targeting_wallride: false,
        }
    }
}

#[path = "physical_player_selector_numeric.rs"]
pub(crate) mod selector_numeric;

#[path = "physical_player_selector_scoring.rs"]
pub(crate) mod selector_scoring;

#[path = "physical_player_selector_grind.rs"]
pub(crate) mod selector_grind;

#[path = "physical_player_selector_update.rs"]
pub(crate) mod selector_update;

#[path = "physical_player_selector_finish.rs"]
pub(crate) mod selector_finish;

#[cfg(test)]
mod reset_tests {
    use super::*;
    use crate::skeleton_processed_contact::SkeletonProcessedContact;
    use std::sync::{Arc, Mutex, Weak};

    fn offboard_selector() -> OffboardTrajectoryOwner {
        let vault = Arc::new(crate::attribute_runtime::tests::graph(
            0xcdd9_26b3_5ef0_f6c4,
            vec![],
        ));
        OffboardTrajectoryOwner::new(
            TrajectoryOwnerBindings {
                owner: Weak::new(),
                processed: Arc::new(Mutex::new(SkeletonProcessedContact::new())),
            },
            &vault,
        )
    }

    #[test]
    fn offboard_reset_clears_exact_selector_fields() {
        let mut owner = offboard_selector();
        owner.ready = true;
        owner.ai_trajectory_following = true;
        owner.has_just_changed = true;
        owner.just_started_query = true;
        owner.winning_com.position = SkeletonRootVector::known([1.0; 4]);
        owner.winning_com.velocity = SkeletonRootVector::known([2.0; 4]);
        owner.winning_com.gravity = SkeletonRootVector::known([3.0; 4]);
        owner.winning_com.time = [4.0; 4];
        owner.raw_trajectory.position = SkeletonRootVector::known([5.0; 4]);
        owner.raw_trajectory.velocity = SkeletonRootVector::known([6.0; 4]);
        owner.raw_trajectory.gravity = SkeletonRootVector::known([7.0; 4]);
        owner.raw_trajectory.time = [8.0; 4];
        owner.current_index = 3;
        owner.land_index = 4;
        owner.trajectory_blend = 0.25;
        owner.trajectory_blend_time = 0.5;

        owner.reset();

        assert!(!owner.ready);
        assert!(!owner.ai_trajectory_following);
        assert!(!owner.has_just_changed);
        assert!(!owner.just_started_query);
        assert_eq!(owner.winning_com, RetainedTrajectory::constructed());
        assert_eq!(owner.raw_trajectory, RetainedTrajectory::constructed());
        assert_eq!(owner.current_index, 0);
        assert_eq!(owner.land_index, 1000);
        assert_eq!(owner.trajectory_blend.to_bits(), 0);
        assert_eq!(owner.trajectory_blend_time.to_bits(), 0);
    }

    #[test]
    fn offboard_reset_preserves_query_and_candidate_state() {
        let mut owner = offboard_selector();
        owner.queries[0].ready = true;
        owner.queries[0].result.frame = 7;
        owner.winning_result = Some(3);
        owner.launch.start_velocity = SkeletonRootVector::known([2.0; 4]);
        owner.launch.num_trajectories = 5;
        owner.data[0].collision_found = true;
        owner.winning_data.index_shift = 6;
        owner.trajectory_offset = SkeletonRootVector::known([4.0; 4]);
        owner.scores[2] = 8.0;
        owner.num_trajectories = 9;
        owner.best_trajectory = 10;

        owner.reset();

        assert!(owner.queries[0].ready);
        assert_eq!(owner.queries[0].result.frame, 7);
        assert_eq!(owner.winning_result, Some(3));
        assert_eq!(
            owner.launch.start_velocity,
            SkeletonRootVector::known([2.0; 4])
        );
        assert_eq!(owner.launch.num_trajectories, 5);
        assert!(owner.data[0].collision_found);
        assert_eq!(owner.winning_data.index_shift, 6);
        assert_eq!(owner.trajectory_offset, SkeletonRootVector::known([4.0; 4]));
        assert_eq!(owner.scores[2], 8.0);
        assert_eq!(owner.num_trajectories, 9);
        assert_eq!(owner.best_trajectory, 10);
    }

    #[test]
    fn reset_systems_keeps_query_and_targeting_state() {
        let mut owner = OnboardTrajectoryOwner::new(
            TrajectoryOwnerBindings {
                owner: Weak::new(),
                processed: Arc::new(Mutex::new(SkeletonProcessedContact::new())),
            },
            false,
        );
        owner.ready = true;
        owner.ai_trajectory_following = true;
        owner.has_just_changed = true;
        owner.pass = 7;
        owner.most_vertical_grind_normal = [9.0; 4];
        owner.ai_trajectory_error = [8.0; 4];
        owner.winning_com.position = SkeletonRootVector::known([7.0; 4]);
        owner.winning_com.time = [2.0; 4];
        owner.num_trajectories = 3;
        owner.best_trajectory = 2;
        owner.just_started_query = true;
        owner.targeting_grind = true;
        owner.queries[0].ready = true;

        owner.reset_after_teleport();

        assert!(!owner.ready);
        assert!(!owner.ai_trajectory_following);
        assert!(!owner.has_just_changed);
        assert_eq!(owner.pass, 0);
        assert_eq!(owner.most_vertical_grind_normal, [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(owner.ai_trajectory_error, [0.0; 4]);
        assert_eq!(
            owner.winning_com.position,
            SkeletonRootVector::known([0.0; 4])
        );
        assert_eq!(owner.winning_com.time, [-1.0; 4]);
        assert_eq!(owner.num_trajectories, 3);
        assert_eq!(owner.best_trajectory, 2);
        assert!(owner.just_started_query);
        assert!(owner.targeting_grind);
        assert!(owner.queries[0].ready);
    }
}
