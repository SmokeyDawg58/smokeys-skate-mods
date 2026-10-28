//! StepSelector::GetStepPos82d81278; retained ground/rail arbitration.
use crate::{
    actor_animation_construction::AnimationStepInput,
    grind_data_octree::OctreeBounds,
    grind_data_runtime::GrindDataQueryResult,
    input_cvar_runtime::InputCvarReferences,
    physical_player_footplant::RetainedTrajectory,
    reckoning_numeric::{
        ReckoningNumeric, cross_typed, length_from_squared_and_estimate, try_normalize_safe,
    },
    simulation_assembly_backend::constraint_session::grind_world::{
        PhysicalGrindError, PhysicalGrindServices,
    },
    skeleton_root_transforms::SkeletonRootVector as V,
    walking_step_query::StepTrajectoryResult,
};

const ZERO: V = V::known([0.0; 4]);
const UP: V = V::known([0.0, 1.0, 0.0, 0.0]);

/// The actual future payload; all copied lanes retain unknown W.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StepSelectionResult {
    pub(crate) trajectory_contact: V,
    pub(crate) position: V,
    pub(crate) normal: V,
    pub(crate) surface: u32,
    pub(crate) valid: bool,
}

pub(crate) trait StepSelectionTuning {
    type Error;
    fn added_distance_ratio(&mut self) -> Result<f32, Self::Error>;
    fn height_margin(&mut self) -> Result<f32, Self::Error>;
    fn box_half_size(&mut self) -> Result<f32, Self::Error>;
    fn vertical_sine(&mut self) -> Result<f32, Self::Error>;
    fn minimum_distance(&mut self) -> Result<f32, Self::Error>;
    fn cosine_spread(&mut self) -> Result<f32, Self::Error>;
    fn low_step_height(&mut self) -> Result<f32, Self::Error>;
    fn reduce_speed_ratio(&mut self) -> Result<f32, Self::Error>;
    fn acceptable_distance(&mut self) -> Result<f32, Self::Error>;
    fn shoe_offset(&mut self) -> Result<f32, Self::Error>;
}

/// Numeric children are native trajectory/segment routines, not query answers.
pub(crate) trait StepTrajectoryDistance<N: ReckoningNumeric> {
    type Error;
    fn line(
        &mut self,
        trajectory: &RetainedTrajectory,
        start: V,
        end: V,
        position: &mut V,
        distance: &mut f32,
        numeric: &mut N,
    ) -> Result<(), Self::Error>;
    fn ground_plane(
        &mut self,
        trajectory: &RetainedTrajectory,
        destination: V,
        start: V,
        end: V,
        position: &mut V,
        other: &mut V,
        numeric: &mut N,
    ) -> Result<bool, Self::Error>;
}

pub(crate) struct NativeStepTrajectoryDistance;
impl<N: ReckoningNumeric> StepTrajectoryDistance<N> for NativeStepTrajectoryDistance {
    type Error = N::Error;
    fn line(
        &mut self,
        trajectory: &RetainedTrajectory,
        start: V,
        end: V,
        position: &mut V,
        distance: &mut f32,
        numeric: &mut N,
    ) -> Result<(), Self::Error> {
        crate::walking_trajectory_distance::calc_dist_from_trajectory_to_line(
            trajectory, &start, &end, position, distance, numeric,
        )
    }
    fn ground_plane(
        &mut self,
        trajectory: &RetainedTrajectory,
        destination: V,
        start: V,
        end: V,
        position: &mut V,
        other: &mut V,
        numeric: &mut N,
    ) -> Result<bool, Self::Error> {
        crate::walking_trajectory_distance::calc_dist_from_trajectory_to_ground_plane(
            trajectory,
            &destination,
            &start,
            &end,
            position,
            other,
            numeric,
        )
    }
}

#[derive(Debug)]
pub(crate) enum StepSelectionError<N, T, D> {
    Numeric(N),
    Tuning(T),
    Distance(D),
    Grind(PhysicalGrindError),
    RailCapacity,
    MaximumHeightAssertion,
}
type ResultOf<R, N, T, D> = Result<R, StepSelectionError<N, T, D>>;

fn horizontal(mut v: V) -> V {
    v.xyz[1] = 0.0;
    v
}
fn length<N: ReckoningNumeric>(v: V, numeric: &mut N) -> Result<f32, N::Error> {
    let square = numeric.dot_vector(v, v)?;
    let seed = numeric.reciprocal_sqrt_estimate(square)?;
    Ok(length_from_squared_and_estimate(square, seed))
}

struct Selection {
    best_slope: f32,
    strong: bool,
    geometry_height: f32,
    range: f32,
    slope_threshold: f32,
    forbid_reduction: bool,
}
impl Selection {
    /// Shared source graph duplicated in the line and ground-plane branches.
    fn consider<N: ReckoningNumeric, T: StepSelectionTuning, D>(
        &mut self,
        point: V,
        base: V,
        rail_horizontal: V,
        forward: V,
        distance: f32,
        numeric: &mut N,
        tuning: &mut T,
    ) -> ResultOf<(bool, bool), N::Error, T::Error, D> {
        use StepSelectionError as E;
        let delta = point.zip(base, |a, b| a - b);
        let direction = try_normalize_safe(horizontal(delta), ZERO, numeric).map_err(E::Numeric)?;
        let alignment = numeric
            .dot_vector(rail_horizontal, forward)
            .map_err(E::Numeric)?
            .abs();
        let cosine = tuning.cosine_spread().map_err(E::Tuning)?;
        let aligned = alignment > cosine
            || point.xyz[1] - base.xyz[1] > tuning.low_step_height().map_err(E::Tuning)?;
        let slope = delta.xyz[1] / distance;
        let dot = numeric.dot_vector(forward, direction).map_err(E::Numeric)?;
        let advancing = dot > cosine && slope > self.best_slope && distance < self.range;
        let strong = advancing
            && slope > self.slope_threshold
            && aligned
            && self.geometry_height < point.xyz[1];
        let weak = !self.strong
            && self.geometry_height < point.xyz[1]
            && advancing
            && distance > tuning.reduce_speed_ratio().map_err(E::Tuning)? * self.range
            && !self.forbid_reduction;
        let backup = advancing
            && aligned
            && self.geometry_height < point.xyz[1]
            && distance > tuning.reduce_speed_ratio().map_err(E::Tuning)? * self.range;
        if strong || weak {
            self.best_slope = slope;
            self.strong |= strong;
        }
        Ok((strong || weak, backup))
    }
}

fn publish_rail<N: ReckoningNumeric>(
    out: &mut StepSelectionResult,
    position: V,
    start: V,
    end: V,
    numeric: &mut N,
) -> Result<(), N::Error> {
    out.position = position;
    let delta = end.zip(start, |a, b| a - b);
    out.normal = try_normalize_safe(cross_typed(cross_typed(delta, UP), delta), ZERO, numeric)?;
    out.surface = 0;
    out.valid = true;
    out.trajectory_contact = out.position;
    Ok(())
}

/// Raw81844/81b44/81f24 BO4 BI24 branch to acceptance when LT is clear.
/// Unlike an ordered >= comparison, an unordered height also accepts here.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn within_collision_bounds(
    height: f32,
    distance: f32,
    collision_y: f32,
    collision_distance: f32,
) -> bool {
    !(height < collision_y) || distance < collision_distance
}

/// GetStepPos82d81df4..81e1c: commit the closer obstacle before testing whether
/// it invalidates the current selected point. raw BO4 branches reject !GT/!LT, including unordered values.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn update_obstacle_bounds(
    height: f32,
    distance: f32,
    collision_y: &mut f32,
    collision_distance: &mut f32,
    selected_height: f32,
    best_distance: f32,
) -> bool {
    if height > *collision_y && distance < *collision_distance {
        *collision_y = height;
        *collision_distance = distance;
        height > selected_height && distance > best_distance
    } else {
        false
    }
}

#[allow(clippy::neg_cmp_op_on_partial_ord)] // Preserve native unordered branches.
pub(crate) fn get_step_position<
    N: ReckoningNumeric,
    T: StepSelectionTuning,
    D: StepTrajectoryDistance<N>,
>(
    input: &AnimationStepInput,
    candidate: &StepTrajectoryResult,
    out: &mut StepSelectionResult,
    grind: &PhysicalGrindServices,
    cvars: &InputCvarReferences,
    numeric: &mut N,
    tuning: &mut T,
    distance_child: &mut D,
) -> ResultOf<bool, N::Error, T::Error, D::Error> {
    use StepSelectionError as E;
    out.valid = false;
    let base = input.reference;
    let destination = base
        .zip(input.reference_offset, |a, b| a + b)
        .zip(input.parameter_vector, |a, b| a + b);
    let delta = horizontal(destination.zip(base, |a, b| a - b));
    let squared = numeric.dot_vector(delta, delta).map_err(E::Numeric)?;
    let seed = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(E::Numeric)?;
    // Target read follows the seed but precedes length refinement.
    let added = tuning.added_distance_ratio().map_err(E::Tuning)?;
    let range = length_from_squared_and_estimate(squared, seed) * added;
    let forward = try_normalize_safe(delta, ZERO, numeric).map_err(E::Numeric)?;
    let mut collision_y = -f32::MAX;
    let mut collision_distance = f32::MAX;
    if candidate.collision.time[0] >= 0.0 {
        collision_y = candidate.collision.position.xyz[1];
        collision_distance = length(
            horizontal(candidate.collision.position.zip(base, |a, b| a - b)),
            numeric,
        )
        .map_err(E::Numeric)?;
    }
    let mut state = Selection {
        best_slope: -f32::MAX,
        strong: false,
        geometry_height: -f32::MAX,
        range,
        slope_threshold: ((input.parameter + input.reference_offset.xyz[1]) / range) * 0.5,
        forbid_reduction: input.flags & 0x10 != 0,
    };
    if candidate.valid {
        out.position = candidate.position;
        let margin = tuning.height_margin().map_err(E::Tuning)?;
        out.normal = candidate.normal;
        state.geometry_height = margin + candidate.position.xyz[1];
        out.valid = true;
        out.surface = candidate.surface;
    }
    let half = tuning.box_half_size().map_err(E::Tuning)?;
    let bounds = OctreeBounds {
        minimum: [
            destination.xyz[0] - half,
            destination.xyz[1] - half,
            destination.xyz[2] - half,
            destination.xyz[0] - half,
        ],
        maximum: [
            destination.xyz[0] + half,
            destination.xyz[1] + half,
            destination.xyz[2] + half,
            destination.xyz[0] + half,
        ],
    };
    let mut rails = GrindDataQueryResult::new();
    grind
        .try_box_test(bounds, &mut rails, false, cvars)
        .map_err(E::Grind)?;
    if rails.count() > 40 {
        return Err(E::RailCapacity);
    }
    let mut backup = [false; 40];
    let mut points = [ZERO; 40];
    let mut selected: Option<usize> = None;
    let mut best_distance = -f32::MAX;
    let mut reconsider = false;
    for (index, rail) in rails.results().enumerate() {
        let start = V::known(rail.start);
        let end = V::known(rail.end);
        let direction =
            try_normalize_safe(end.zip(start, |a, b| a - b), ZERO, numeric).map_err(E::Numeric)?;
        let vertical = tuning.vertical_sine().map_err(E::Tuning)?;
        let dot = numeric.dot_vector(direction, UP).map_err(E::Numeric)?;
        if !(dot.abs() < vertical) {
            continue;
        }
        let mut point = ZERO;
        let mut unused_distance = 0.0;
        distance_child
            .line(
                &candidate.trajectory,
                start,
                end,
                &mut point,
                &mut unused_distance,
                numeric,
            )
            .map_err(E::Distance)?;
        let mut chosen = false;
        let rail_horizontal =
            try_normalize_safe(horizontal(direction), ZERO, numeric).map_err(E::Numeric)?;
        let mut planar_distance =
            length(horizontal(point.zip(base, |a, b| a - b)), numeric).map_err(E::Numeric)?;
        let mut near_destination = false;
        if point.xyz[1] < input.max_y
            && planar_distance > tuning.minimum_distance().map_err(E::Tuning)?
            && within_collision_bounds(
                point.xyz[1],
                planar_distance,
                collision_y,
                collision_distance,
            )
        {
            let (accept, save) = state.consider(
                point,
                base,
                rail_horizontal,
                forward,
                planar_distance,
                numeric,
                tuning,
            )?;
            backup[index] = save;
            if accept {
                chosen = true;
                let dist =
                    length(point.zip(destination, |a, b| a - b), numeric).map_err(E::Numeric)?;
                near_destination = dist < tuning.acceptable_distance().map_err(E::Tuning)?;
            }
        }
        let mut ground_point = ZERO;
        let mut unused_plane = ZERO;
        if !near_destination
            && distance_child
                .ground_plane(
                    &candidate.trajectory,
                    destination,
                    start,
                    end,
                    &mut ground_point,
                    &mut unused_plane,
                    numeric,
                )
                .map_err(E::Distance)?
        {
            // Even a rejected ground candidate replaces the current Y/distance
            // comparison values, while the selected point remains unchanged.
            planar_distance = length(horizontal(ground_point.zip(base, |a, b| a - b)), numeric)
                .map_err(E::Numeric)?;
            if ground_point.xyz[1] < input.max_y
                && planar_distance > tuning.minimum_distance().map_err(E::Tuning)?
                && within_collision_bounds(
                    ground_point.xyz[1],
                    planar_distance,
                    collision_y,
                    collision_distance,
                )
            {
                let (accept, save) = state.consider(
                    ground_point,
                    base,
                    rail_horizontal,
                    forward,
                    planar_distance,
                    numeric,
                    tuning,
                )?;
                backup[index] = save;
                if accept {
                    chosen = true;
                    point = ground_point;
                } else {
                    backup[index] = false;
                }
            }
        } else {
            ground_point = point;
        }
        if backup[index] {
            points[index] = point;
        }
        if chosen {
            publish_rail(out, point, start, end, numeric).map_err(E::Numeric)?;
            best_distance = planar_distance;
            if let Some(previous) = selected {
                backup[previous] = false;
            }
            selected = Some(index);
        } else if update_obstacle_bounds(
            ground_point.xyz[1],
            planar_distance,
            &mut collision_y,
            &mut collision_distance,
            out.position.xyz[1],
            best_distance,
        ) {
            reconsider = true;
            if let Some(previous) = selected {
                backup[previous] = false;
            }
        }
    }
    if reconsider {
        for (index, rail) in rails.results().enumerate() {
            if !backup[index] {
                continue;
            }
            let point = points[index];
            if point.xyz[1] > input.max_y {
                return Err(E::MaximumHeightAssertion);
            }
            let delta = point.zip(base, |a, b| a - b);
            let distance = length(horizontal(delta), numeric).map_err(E::Numeric)?;
            if within_collision_bounds(point.xyz[1], distance, collision_y, collision_distance)
                && delta.xyz[1] / distance > state.best_slope
            {
                state.best_slope = delta.xyz[1] / distance;
                publish_rail(
                    out,
                    point,
                    V::known(rail.start),
                    V::known(rail.end),
                    numeric,
                )
                .map_err(E::Numeric)?;
            }
        }
    }
    if out.valid {
        out.position.xyz[1] += tuning.shoe_offset().map_err(E::Tuning)?;
    }
    Ok(out.valid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        debug_overlay::DebugOverlaySlot, input_cvar_runtime::InputCvarManager,
        reckoning_numeric::HostReckoningNumeric, walking_step_tuning::WalkingTuning,
    };
    #[test]
    fn unordered_obstacle_is_rejected_before_bound_publication() {
        for (height, distance) in [(f32::NAN, 2.0), (2.0, f32::NAN), (f32::NAN, f32::NAN)] {
            let mut y = 1.0;
            let mut d = 3.0;
            assert!(!update_obstacle_bounds(
                height, distance, &mut y, &mut d, 1.0, 1.0
            ));
            assert_eq!((y, d), (1.0, 3.0));
        }
        let mut y = 1.0;
        let mut d = 3.0;
        assert!(!update_obstacle_bounds(2.0, 2.0, &mut y, &mut d, 4.0, 1.0));
        assert_eq!((y, d), (2.0, 2.0));
        assert!(within_collision_bounds(f32::NAN, 4.0, 1.0, 3.0));
        assert!(within_collision_bounds(1.0, 4.0, f32::NAN, 3.0));
        assert!(!within_collision_bounds(0.0, f32::NAN, 1.0, 3.0));
    }
    struct ExceptionalTuning {
        low: f32,
        reduce: f32,
        fail_margin: bool,
    }
    impl StepSelectionTuning for ExceptionalTuning {
        type Error = ();
        fn added_distance_ratio(&mut self) -> Result<f32, ()> {
            Ok(1.1)
        }
        fn height_margin(&mut self) -> Result<f32, ()> {
            if self.fail_margin { Err(()) } else { Ok(0.05) }
        }
        fn box_half_size(&mut self) -> Result<f32, ()> {
            Ok(2.0)
        }
        fn vertical_sine(&mut self) -> Result<f32, ()> {
            Ok(0.8)
        }
        fn minimum_distance(&mut self) -> Result<f32, ()> {
            Ok(0.05)
        }
        fn cosine_spread(&mut self) -> Result<f32, ()> {
            Ok(0.98)
        }
        fn low_step_height(&mut self) -> Result<f32, ()> {
            Ok(self.low)
        }
        fn reduce_speed_ratio(&mut self) -> Result<f32, ()> {
            Ok(self.reduce)
        }
        fn acceptable_distance(&mut self) -> Result<f32, ()> {
            Ok(0.15)
        }
        fn shoe_offset(&mut self) -> Result<f32, ()> {
            Ok(0.015)
        }
    }
    #[test]
    fn native_rejection_branches_require_ordered_selection_comparisons() {
        // Each row changes a different compared source operand. Weak acceptance
        // does not require alignment; strong/backup do. These are distinct gates.
        for (best, geometry, threshold, low, reduce, accepted, strong, backup) in [
            (-1.0, -1.0, 0.0, 0.0, 0.7, true, true, true),
            (f32::NAN, -1.0, 0.0, 0.0, 0.7, false, false, false),
            (-1.0, f32::NAN, 0.0, 0.0, 0.7, false, false, false),
            (-1.0, -1.0, f32::NAN, 0.0, 0.7, true, false, true),
            (-1.0, -1.0, 0.0, f32::NAN, 0.7, true, false, false),
            (-1.0, -1.0, 10.0, 0.0, f32::NAN, false, false, false),
        ] {
            let mut selection = Selection {
                best_slope: best,
                strong: false,
                geometry_height: geometry,
                range: 1.0,
                slope_threshold: threshold,
                forbid_reduction: false,
            };
            let mut tuning = ExceptionalTuning {
                low,
                reduce,
                fail_margin: false,
            };
            let actual = selection
                .consider::<_, _, ()>(
                    V::known([0.8, 0.1, 0.0, 0.0]),
                    ZERO,
                    V::known([0.0, 0.0, 1.0, 0.0]),
                    V::known([1.0, 0.0, 0.0, 0.0]),
                    0.8,
                    &mut HostReckoningNumeric,
                    &mut tuning,
                )
                .unwrap();
            assert_eq!(actual, (accepted, backup));
            assert_eq!(selection.strong, strong);
        }
    }
    #[test]
    fn geometry_height_cvar_failure_commits_only_position() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut candidate = StepTrajectoryResult::new();
        candidate.valid = true;
        candidate.position = V::known([3.0, 4.0, 5.0, 6.0]);
        candidate.normal = UP;
        candidate.surface = 13;
        let mut out = result();
        let before = out;
        let mut tuning = ExceptionalTuning {
            low: 0.15,
            reduce: 0.7,
            fail_margin: true,
        };
        assert!(matches!(
            get_step_position(
                &input(),
                &candidate,
                &mut out,
                &PhysicalGrindServices::new(),
                &refs,
                &mut HostReckoningNumeric,
                &mut tuning,
                &mut NativeStepTrajectoryDistance
            ),
            Err(StepSelectionError::Tuning(()))
        ));
        assert_eq!(out.position, candidate.position);
        assert_eq!(out.normal, before.normal);
        assert_eq!(out.surface, before.surface);
        assert!(!out.valid);
    }
    fn input() -> AnimationStepInput {
        AnimationStepInput {
            reference: V::known([-1.0, -1.0, 0.0, 0.0]),
            reference_offset: V::known([0.0; 4]),
            parameter_vector: V::known([4.0, 0.0, 0.0, 0.0]),
            speed_at_end: V::known([3.0, 0.0, 0.0, 0.0]),
            parameter: 0.0,
            spread_angle: 0.0,
            turn_intent: 0.0,
            max_y: 2.0,
            flags: 0,
        }
    }
    fn result() -> StepSelectionResult {
        StepSelectionResult {
            trajectory_contact: V::known([7.0; 4]),
            position: ZERO,
            normal: ZERO,
            surface: 99,
            valid: false,
        }
    }
    #[test]
    fn geometry_success_still_requires_real_grind_owner() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let overlay = DebugOverlaySlot::default();
        let mut tuning = WalkingTuning {
            cvars: &refs,
            overlay: &overlay,
        };
        let mut candidate = StepTrajectoryResult::new();
        candidate.valid = true;
        candidate.position = V {
            xyz: [3.0, 0.0, 0.0],
            w: None,
        };
        candidate.normal = UP;
        candidate.surface = 13;
        let mut out = result();
        assert!(matches!(
            get_step_position(
                &input(),
                &candidate,
                &mut out,
                &PhysicalGrindServices::new(),
                &refs,
                &mut HostReckoningNumeric,
                &mut tuning,
                &mut NativeStepTrajectoryDistance
            ),
            Err(StepSelectionError::Grind(_))
        ));
        assert!(out.valid);
        assert_eq!(out.position, candidate.position);
        assert_eq!(out.surface, 13);
        let grind = PhysicalGrindServices::new();
        grind.try_initialize_static().unwrap();
        assert!(
            get_step_position(
                &input(),
                &candidate,
                &mut out,
                &grind,
                &refs,
                &mut HostReckoningNumeric,
                &mut tuning,
                &mut NativeStepTrajectoryDistance
            )
            .unwrap()
        );
        assert_eq!(out.position.xyz[1], 0.015);
        assert_eq!(out.position.w, None);
        assert_eq!(out.trajectory_contact, V::known([7.0; 4]));
    }
    #[test]
    fn actual_registered_rail_supplies_step_without_ground_candidate() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let grind = PhysicalGrindServices::new();
        grind.try_initialize_static().unwrap();
        grind
            .static_data()
            .lock()
            .unwrap()
            .try_add(
                crate::grind_data_runtime::tests::source(123, 1),
                &mut crate::grind_data_runtime::HostGrindDataNumeric,
            )
            .unwrap();
        let overlay = DebugOverlaySlot::default();
        let mut tuning = WalkingTuning {
            cvars: &refs,
            overlay: &overlay,
        };
        let mut candidate = StepTrajectoryResult::new();
        candidate.trajectory = RetainedTrajectory {
            position: input().reference,
            velocity: V::known([3.0, 0.0, 0.0, 0.0]),
            gravity: V::known([0.0, -9.8, 0.0, 0.0]),
            time: [2.0; 4],
        };
        let mut out = result();
        assert!(
            get_step_position(
                &input(),
                &candidate,
                &mut out,
                &grind,
                &refs,
                &mut HostReckoningNumeric,
                &mut tuning,
                &mut NativeStepTrajectoryDistance
            )
            .unwrap()
        );
        assert_eq!(out.surface, 0);
        assert_eq!(out.position.xyz[1], 0.015);
        assert!(out.position.xyz[0] >= 1.0 && out.position.xyz[0] <= 2.0);
        assert_eq!(out.trajectory_contact.xyz[1], 0.0);
    }
}
