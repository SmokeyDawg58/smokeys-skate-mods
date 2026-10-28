//! Controlled kernel fixtures; actual attached-owner/corpus coverage is separate.
#![expect(
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "exact source stores and integral Part IDs in the bounded 0..24 fixtures"
)]
use super::*;
use crate::{
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_collision_attributes::{SkeletonCollisionAttributeRead, SkeletonCollisionOffset},
    skeleton_physics_history::{
        HostSkeletonPhysicsHistoryNumeric, SkeletonHistoryInput, SkeletonPhysicsHistory,
    },
};

#[derive(Debug, PartialEq)]
enum Event {
    Dot([f32; 4]),
    Reciprocal(f32),
    Sqrt(f32),
}
#[derive(Default)]
struct Numeric {
    events: Vec<Event>,
    fail_at: Option<usize>,
}
impl Numeric {
    fn event(&mut self, event: Event) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail_at == Some(self.events.len()) {
            Err("numeric")
        } else {
            Ok(())
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.event(Event::Dot(a))?;
        Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event(Event::Reciprocal(value))?;
        Ok(1.0 / value)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event(Event::Sqrt(value))?;
        Ok(1.0 / value.sqrt())
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("unexpected signed angle")
    }
}

#[derive(Default)]
struct Services {
    events: Vec<&'static str>,
    draw_values: Vec<u8>,
    draws: usize,
    normals: u8,
    fail: Option<&'static str>,
}
impl Services {
    fn event(&mut self, event: &'static str) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail == Some(event) {
            Err(event)
        } else {
            Ok(())
        }
    }
}
impl SkeletonPostPhysicsRecordServices for Services {
    type Error = &'static str;
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.event("ground")?;
        Ok(known([0.0, 1.0, 0.0, 0.0]))
    }
    fn use_normals_for_proxy_offset(&mut self) -> Result<u8, Self::Error> {
        self.event("normals")?;
        Ok(self.normals)
    }
    fn scale_hook_to_hips_y(&mut self) -> Result<f32, Self::Error> {
        self.event("scale")?;
        Ok(0.25)
    }
    fn draw_hook_to_hips(&mut self) -> Result<u8, Self::Error> {
        self.event("draw")?;
        let result = self.draw_values.get(self.draws).copied().unwrap_or(0);
        self.draws += 1;
        Ok(result)
    }
}
struct CollisionAttributes;
impl SkeletonCollisionAttributeRead for CollisionAttributes {
    type Error = &'static str;
    type Pointer = ();
    fn cached_offset(&mut self, _: SkeletonCollisionOffset) -> Result<[f32; 4], Self::Error> {
        Ok([0.0; 4])
    }
    fn lookup(&mut self, _: u64) -> Result<(), Self::Error> {
        Ok(())
    }
    fn float(&mut self, (): &()) -> Result<f32, Self::Error> {
        Ok(0.0)
    }
}
struct HistoryInput;
impl SkeletonHistoryInput for HistoryInput {
    type Error = &'static str;
    type Up = ();
    fn acquire_up_vector(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        Ok(1)
    }
    fn read_up_vector(&mut self, (): &()) -> Result<[f32; 4], Self::Error> {
        panic!("XYZ branch")
    }
}
fn matrix(position: [f32; 4]) -> AnimationMatrix {
    let mut result = FRESH_VOLUME_TRANSFORM;
    result[12..16].copy_from_slice(&position);
    result
}
struct Fixture {
    history: SkeletonPhysicsHistory,
    adjusted: [AnimationMatrix; 24],
    world: SkeletonRootMatrix,
    collision: SkeletonCollisionData,
}
impl Fixture {
    fn new() -> Self {
        let mut masses = [0.0; 24];
        masses[1] = 1.0;
        let mut history = SkeletonPhysicsHistory::new(masses);
        history.try_initialize_masses().unwrap();
        history
            .try_update(
                FRESH_VOLUME_TRANSFORM,
                &(),
                &mut |part| Ok::<_, &'static str>(matrix([part as f32 + 2.0, 4.0, 6.0, 8.0])),
                &mut HistoryInput,
                &mut HostSkeletonPhysicsHistoryNumeric,
            )
            .unwrap();
        Self {
            history,
            adjusted: [FRESH_VOLUME_TRANSFORM; 24],
            world: SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
            collision: SkeletonCollisionData::try_new(&mut CollisionAttributes).unwrap(),
        }
    }
    fn input(&self) -> SkeletonPostPhysicsRecordInput<'_> {
        SkeletonPostPhysicsRecordInput {
            physics: self.history.record(),
            adjusted_volumes: &self.adjusted,
            anim_to_world: &self.world,
            proxy_target: Some(SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0])),
            time_delta: 2.0,
            flags_8c8: 0,
            collision: &self.collision,
        }
    }
}

#[test]
fn constructor_retains_known_zeros_but_no_invented_maximum() {
    let records = SkeletonPostPhysicsRecords::new();
    assert_eq!(*records.errors(), [ZERO; 24]);
    assert_eq!(records.centre_of_mass(), [0.0; 4]);
    assert_eq!(records.previous_centre_of_mass(), [0.0; 4]);
    assert_eq!(records.centre_of_mass_velocity(), [0.0; 4]);
    assert_eq!(records.hook_to_hips(), ZERO);
    assert_eq!(records.max_displacement(), None);
    assert!(!records.proxy_enabled());
}

#[test]
fn error_rows_use_proxy_and_adjusted_world_positions_with_full_and_unknown_w() {
    let mut f = Fixture::new();
    f.adjusted[0] = matrix([1000.0; 4]); // Part0 ignores adjusted volume0.
    f.adjusted[7] = matrix([2.0, 3.0, 4.0, f32::NAN]); // Input P.W is ignored.
    let world = [
        known([2.0, 0.0, 0.0, 3.0]),
        known([0.0, 3.0, 0.0, 5.0]),
        known([0.0, 0.0, 4.0, 7.0]),
        known([1.0, 2.0, 3.0, 11.0]),
    ];
    let mut records = SkeletonPostPhysicsRecords::new();
    records.calculate_errors(
        f.history.record(),
        &f.adjusted,
        &world,
        known([1.0, 2.0, 3.0, 4.0]),
    );
    assert_eq!(records.errors[0], known([1.0, 2.0, 3.0, 4.0]));
    assert_eq!(records.errors[7], known([4.0, -7.0, -13.0, -52.0]));
    let mut unknown = world;
    unknown[0].w = None;
    records.calculate_errors(
        f.history.record(),
        &f.adjusted,
        &unknown,
        SkeletonRootVector {
            xyz: [1.0, 2.0, 3.0],
            w: None,
        },
    );
    assert_eq!(records.errors[0].xyz, [1.0, 2.0, 3.0]);
    assert_eq!(records.errors[0].w, None);
    assert_eq!(records.errors[7].xyz, [4.0, -7.0, -13.0]);
    assert_eq!(records.errors[7].w, None);
    assert_eq!(records.errors[1].w, None); // Zero coefficient cannot erase unknown W.
}

#[test]
fn absent_proxy_target_stops_before_any_record_or_service_store() {
    let f = Fixture::new();
    let mut records = SkeletonPostPhysicsRecords::new();
    let mut input = f.input();
    input.proxy_target = None;
    let mut services = Services::default();
    let mut numeric = Numeric::default();
    assert_eq!(
        records.try_update(
            input,
            &mut services,
            &mut numeric,
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonPostPhysicsRecordError::MissingProxyTarget)
    );
    assert_eq!(records.errors, [ZERO; 24]);
    assert_eq!(records.previous_centre_of_mass, [0.0; 4]);
    assert!(numeric.events.is_empty());
    assert!(services.events.is_empty());
}

#[test]
fn dt_seed_failure_keeps_all_errors_and_previous_but_not_new_com_or_velocity() {
    let f = Fixture::new();
    let mut records = SkeletonPostPhysicsRecords::new();
    records.centre_of_mass = [10.0, 20.0, 30.0, 40.0];
    records.centre_of_mass_velocity = [-1.0; 4];
    let mut services = Services::default();
    let mut numeric = Numeric {
        fail_at: Some(1),
        ..Numeric::default()
    };
    assert_eq!(
        records.try_update(
            f.input(),
            &mut services,
            &mut numeric,
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonPostPhysicsRecordError::Numeric("numeric"))
    );
    assert_eq!(records.errors[23], known([25.0, 4.0, 6.0, 8.0]));
    assert_eq!(records.previous_centre_of_mass, [10.0, 20.0, 30.0, 40.0]);
    assert_eq!(records.centre_of_mass, records.previous_centre_of_mass);
    assert_eq!(records.centre_of_mass_velocity, [-1.0; 4]);
    assert_eq!(records.max_displacement, None);
    assert!(services.events.is_empty());
}

#[test]
fn flag_restamps_previous_after_full4_velocity_and_before_hook_debug_failure() {
    let f = Fixture::new();
    let mut records = SkeletonPostPhysicsRecords::new();
    records.centre_of_mass = [1.0, 2.0, 3.0, 4.0];
    let mut input = f.input();
    input.flags_8c8 = 0x1000;
    let mut services = Services {
        draw_values: vec![1],
        ..Services::default()
    };
    assert_eq!(
        records.try_update(
            input,
            &mut services,
            &mut Numeric::default(),
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonPostPhysicsRecordError::Debug(
            SkeletonHookDebugStage::NormalBeforeDisplacements
        ))
    );
    assert_eq!(records.centre_of_mass, [3.0, 4.0, 6.0, 8.0]);
    assert_eq!(records.centre_of_mass_velocity, [1.0, 1.0, 1.5, 2.0]);
    assert_eq!(records.previous_centre_of_mass, records.centre_of_mass);
    assert!(records.max_displacement.unwrap() > 25.0);
    assert_eq!(records.hook_to_hips, ZERO);
    assert_eq!(services.events, ["draw"]);
}

#[test]
fn first_max_reduction_preserves_source_callbacks_and_part17_tie() {
    let errors = std::array::from_fn(|part| known([1.0, 0.0, 0.0, part as f32]));
    let mut numeric = Numeric::default();
    assert_eq!(maximum_length(&errors, &mut numeric), Ok(1.0));
    let tags: Vec<_> = numeric
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Dot(value) => Some(value[3] as usize),
            _ => None,
        })
        .collect();
    assert_eq!(
        tags,
        [23, 17, 18, 16, 21, 22, 20, 1, 17, 17, 17, 17, 17, 17, 17]
    );
    assert_eq!(numeric.events.last(), Some(&Event::Sqrt(1.0)));
}

#[test]
fn second_reduction_uses_part23_tie_and_delayed_part16_selection() {
    let mut values = HOOK_PARTS.map(|part| known([1.0, 0.0, 0.0, part as f32]));
    let mut numeric = Numeric::default();
    assert_eq!(
        select_adjusted_hook(&values, &mut numeric).unwrap(),
        values[0]
    );
    let tags: Vec<_> = numeric
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Dot(value) => Some(value[3] as usize),
            _ => None,
        })
        .collect();
    assert_eq!(
        tags,
        [17, 18, 16, 21, 23, 23, 23, 22, 20, 1, 23, 23, 23, 23]
    );
    values[3].xyz[0] = 3.0;
    let mut numeric = Numeric::default();
    assert_eq!(
        select_adjusted_hook(&values, &mut numeric).unwrap(),
        values[3]
    );
    assert!(matches!(numeric.events[9], Event::Dot(v) if v[3] == 1.0));
    assert!(matches!(numeric.events[10], Event::Dot(v) if v[3] == 16.0));
}

#[test]
fn unordered_initial_comparisons_keep_their_distinct_false_branch_vectors() {
    let mut errors = [ZERO; 24];
    errors[23] = known([f32::NAN, 0.0, 0.0, 23.0]);
    errors[17] = known([1.0, 0.0, 0.0, 17.0]);
    assert_eq!(maximum_length(&errors, &mut Numeric::default()), Ok(1.0));
    let values = HOOK_PARTS.map(|part| errors[part]);
    let winner = select_adjusted_hook(&values, &mut Numeric::default()).unwrap();
    assert!(winner.xyz[0].is_nan());
    assert_eq!(winner.w, Some(23.0));
}

#[test]
fn zero_maximum_still_calls_estimate_before_selecting_positive_zero() {
    let mut numeric = Numeric::default();
    assert_eq!(
        maximum_length(&[ZERO; 24], &mut numeric).unwrap().to_bits(),
        0
    );
    assert_eq!(numeric.events.last(), Some(&Event::Sqrt(0.0)));
    let mut fail = Numeric {
        fail_at: Some(16),
        ..Numeric::default()
    };
    assert_eq!(maximum_length(&[ZERO; 24], &mut fail), Err("numeric"));
}

#[test]
fn proxy_scales_only_y_then_commits_hook_before_debug_stop() {
    let f = Fixture::new();
    let mut records = SkeletonPostPhysicsRecords::new();
    records.proxy_enabled = true;
    let mut services = Services {
        draw_values: vec![1],
        ..Services::default()
    };
    assert_eq!(
        records.try_update(
            f.input(),
            &mut services,
            &mut Numeric::default(),
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonPostPhysicsRecordError::Debug(
            SkeletonHookDebugStage::ProxyAfterHookStore
        ))
    );
    assert_eq!(records.hook_to_hips, known([1.0, 0.5, 3.0, 4.0]));
    assert_eq!(services.events, ["ground", "normals", "scale", "draw"]);
}

#[test]
fn proxy_normals_failure_retains_maximum_and_com_without_scale_or_hook_store() {
    let f = Fixture::new();
    let mut records = SkeletonPostPhysicsRecords::new();
    records.proxy_enabled = true;
    let mut services = Services {
        fail: Some("normals"),
        ..Services::default()
    };
    assert_eq!(
        records.try_update(
            f.input(),
            &mut services,
            &mut Numeric::default(),
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonPostPhysicsRecordError::Service("normals"))
    );
    assert_eq!(records.centre_of_mass, [3.0, 4.0, 6.0, 8.0]);
    assert!(records.max_displacement.is_some());
    assert_eq!(records.hook_to_hips, ZERO);
    assert_eq!(services.events, ["ground", "normals"]);
}

#[test]
fn normal_full_suffix_applies_no_contact_zero_factor_and_samples_draw_twice() {
    let f = Fixture::new();
    let mut records = SkeletonPostPhysicsRecords::new();
    let mut services = Services::default();
    records
        .try_update(
            f.input(),
            &mut services,
            &mut Numeric::default(),
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    // With no retained normals the native helper still rotates, then multiplies
    // all lanes by zero. It does not return its initial plane projection.
    assert_eq!(records.hook_to_hips, ZERO);
    assert_eq!(services.events, ["draw", "ground", "draw"]);
    let retained = records.hook_to_hips;
    records.errors[23].xyz[0] = 100.0;
    let mut services = Services {
        draw_values: vec![0, 1],
        ..Services::default()
    };
    assert_eq!(
        records.try_update_hook(
            &f.collision,
            &mut services,
            &mut Numeric::default(),
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonPostPhysicsRecordError::Debug(
            SkeletonHookDebugStage::NormalBeforeHookStore
        ))
    );
    assert_eq!(records.hook_to_hips, retained);
    assert!(records.max_displacement.unwrap() > 100.0);
}

#[test]
fn first_displacement_numeric_failure_keeps_maximum_without_publishing_scratch() {
    let f = Fixture::new();
    let mut records = SkeletonPostPhysicsRecords::new();
    let mut services = Services::default();
    // reciprocal, fifteen first-pass dots, rsqrt, then helper projection dot.
    let mut numeric = Numeric {
        fail_at: Some(18),
        ..Numeric::default()
    };
    assert_eq!(
        records.try_update(
            f.input(),
            &mut services,
            &mut numeric,
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonPostPhysicsRecordError::NormalDisplacement {
            part: 23,
            error: SkeletonNormalDisplacementError::Numeric("numeric")
        })
    );
    assert!(records.max_displacement.is_some());
    assert_eq!(records.hook_to_hips, ZERO);
    assert_eq!(services.events, ["draw", "ground"]);
}

#[path = "skeleton_postphysics_late_flags_tests.rs"]
pub(crate) mod late_flags;
