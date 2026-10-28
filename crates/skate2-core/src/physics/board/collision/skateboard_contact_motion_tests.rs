use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Capture,
    Distance(usize),
    Threshold,
    Normal(usize),
    Velocity(usize),
    GripEnabled,
    GripCosine,
    GripVector,
    Dot,
}

struct Services {
    events: Vec<Event>,
    fail: Option<Event>,
    distances: [f32; 4],
    threshold: f32,
    velocities: [SkeletonRootVector; 7],
    grip: bool,
    cosine: f32,
    dot: f32,
}
impl Default for Services {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            fail: None,
            distances: [0.0; 4],
            threshold: 0.1,
            velocities: [SkeletonRootVector {
                xyz: [3.0, 5.0, 7.0],
                w: None,
            }; 7],
            grip: false,
            cosine: 0.5,
            dot: 0.25,
        }
    }
}
impl Services {
    fn visit(&mut self, event: Event) -> Result<(), Event> {
        self.events.push(event);
        if self.fail == Some(event) {
            Err(event)
        } else {
            Ok(())
        }
    }
}
impl ReckoningNumeric for Services {
    type Error = Event;
    fn dot3(&mut self, normal: [f32; 4], grip: [f32; 4]) -> Result<f32, Event> {
        self.visit(Event::Dot)?;
        assert_eq!(normal.map(f32::to_bits), [0, 0x3f80_0000, 0, 0]);
        assert_eq!(
            grip.map(f32::to_bits),
            [0x4000_0000, 0x4040_0000, 0x4080_0000, 0]
        );
        Ok(self.dot)
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Event> {
        panic!("motion has no estimate")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Event> {
        panic!("motion uses one scalar division")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Event> {
        panic!("motion has no conversion")
    }
}
impl SkateboardContactMotionServices for Services {
    type LineTarget = ();
    fn capture_line_threshold(&mut self) -> Result<(), Event> {
        self.visit(Event::Capture)
    }
    fn wheel_distance(&mut self, wheel: usize) -> Result<f32, Event> {
        self.visit(Event::Distance(wheel))?;
        Ok(self.distances[wheel])
    }
    fn line_threshold(&mut self, (): &()) -> Result<f32, Event> {
        self.visit(Event::Threshold)?;
        Ok(self.threshold)
    }
    fn wheel_normal(&mut self, wheel: usize) -> Result<SkeletonRootVector, Event> {
        self.visit(Event::Normal(wheel))?;
        Ok(SkeletonRootVector {
            xyz: [1.0, 0.0, 0.0],
            w: None,
        })
    }
    fn body_velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Event> {
        self.visit(Event::Velocity(part))?;
        Ok(self.velocities[part])
    }
    fn grip_enabled(&mut self) -> Result<bool, Event> {
        self.visit(Event::GripEnabled)?;
        Ok(self.grip)
    }
    fn grip_cosine(&mut self) -> Result<f32, Event> {
        self.visit(Event::GripCosine)?;
        Ok(self.cosine)
    }
    fn grip_vector(&mut self) -> Result<SkeletonRootVector, Event> {
        self.visit(Event::GripVector)?;
        Ok(SkeletonRootVector {
            xyz: [2.0, 3.0, 4.0],
            w: None,
        })
    }
}
fn state() -> SkateboardContactSurface {
    let mut state = SkateboardContactSurface::new();
    state.collision_state.reset();
    state.post_prefix.opposing_min = Some(0.0);
    state.post_prefix.opposing_max = Some(1.0);
    state.collision_state.velocity = [Some(SkeletonRootVector::known([1.0, 1.0, 1.0, 0.0])); 7];
    state
}
fn assert_vector(actual: Option<SkeletonRootVector>, xyz: [f32; 3], w: Option<f32>) {
    let actual = actual.unwrap();
    assert_eq!(actual.xyz.map(f32::to_bits), xyz.map(f32::to_bits));
    assert_eq!(actual.w.map(f32::to_bits), w.map(f32::to_bits));
}

#[test]
fn line_fallback_samples_in_order_and_does_not_create_contact() {
    let mut state = state();
    for wheel in 0..4 {
        state.collision_state.surfaces.set_validity(wheel, 1);
    }
    state.collision_state.in_contact[3] = Some(true);
    let mut services = Services {
        distances: [0.05, 0.1, f32::NAN, 0.0],
        ..Services::default()
    };
    state.try_finish_line_fallback(&mut services).unwrap();
    assert_eq!(
        services.events,
        [
            Event::Capture,
            Event::Distance(0),
            Event::Threshold,
            Event::Normal(0),
            Event::Capture,
            Event::Distance(1),
            Event::Threshold,
            Event::Capture,
            Event::Distance(2),
            Event::Threshold
        ]
    );
    assert_vector(state.collision_state.normal[0], [1.0, 0.0, 0.0], None);
    for wheel in [1, 2] {
        assert_eq!(state.collision_state.surfaces.validity(wheel), Some(0));
        assert_vector(
            state.collision_state.normal[wheel],
            [0.0, 1.0, 0.0],
            Some(0.0),
        );
    }
    assert_eq!(
        state.collision_state.in_contact,
        [
            Some(false),
            Some(false),
            Some(false),
            Some(true),
            Some(false),
            Some(false),
            Some(false)
        ]
    );
    assert_eq!(state.collision_state.surfaces.validity(0), Some(1));
    assert_eq!(state.collision_state.surfaces.validity(3), Some(1));
}

#[test]
fn threshold_refusal_occurs_after_distance_without_normal_or_validity_store() {
    let mut state = state();
    state.collision_state.surfaces.set_validity(0, 1);
    let mut services = Services {
        fail: Some(Event::Threshold),
        ..Services::default()
    };
    assert_eq!(
        state.try_finish_line_fallback(&mut services),
        Err(SkateboardContactMotionError::Service(Event::Threshold))
    );
    assert_eq!(
        services.events,
        [Event::Capture, Event::Distance(0), Event::Threshold]
    );
    assert_eq!(state.collision_state.surfaces.validity(0), Some(1));
    assert_vector(state.collision_state.normal[0], [0.0, 1.0, 0.0], Some(0.0));
}

#[test]
fn seven_motion_updates_use_current_minus_retained_and_keep_unknown_w() {
    let mut state = state();
    let mut services = Services::default();
    state.try_update_contact_motion(0.5, &mut services).unwrap();
    for part in 0..7 {
        assert_vector(
            state.collision_state.acceleration[part],
            [4.0, 8.0, 12.0],
            None,
        );
        assert_vector(state.collision_state.velocity[part], [3.0, 5.0, 7.0], None);
        assert_eq!(
            &services.events[part * 2..part * 2 + 2],
            [Event::Velocity(part), Event::GripEnabled]
        );
    }
    assert_eq!(services.events.len(), 14);
}

#[test]
fn late_body_and_grip_refusals_preserve_their_distinct_store_prefixes() {
    for failure in [Event::Velocity(2), Event::GripCosine] {
        let mut state = state();
        state.collision_state.in_contact[2] = Some(true);
        state.collision_state.surfaces.set_validity(2, 1);
        let mut services = Services {
            fail: Some(failure),
            grip: true,
            ..Services::default()
        };
        assert_eq!(
            state.try_update_contact_motion(0.5, &mut services),
            Err(SkateboardContactMotionError::Service(failure))
        );
        for part in 0..2 {
            assert_vector(
                state.collision_state.acceleration[part],
                [4.0, 8.0, 12.0],
                None,
            );
        }
        let committed = failure == Event::GripCosine;
        assert_vector(
            state.collision_state.velocity[2],
            if committed { [3.0, 5.0, 7.0] } else { [1.0; 3] },
            if committed { None } else { Some(0.0) },
        );
        assert_vector(
            state.collision_state.acceleration[2],
            if committed {
                [4.0, 8.0, 12.0]
            } else {
                [0.0; 3]
            },
            if committed { None } else { Some(0.0) },
        );
        assert_vector(state.collision_state.velocity[3], [1.0; 3], Some(0.0));
        assert_eq!(services.events.last(), Some(&failure));
        assert!(!services.events.contains(&Event::Velocity(3)));
        assert_eq!(
            state.collision_state.opposing_delta.unwrap().to_bits(),
            1.0_f32.to_bits()
        );
    }
}

#[test]
fn opposing_fsel_preserves_nan_but_both_zero_signs_become_positive_zero() {
    for (maximum, expected) in [
        (0x8000_0000, 0),
        (0, 0),
        (0xbf80_0000, 0),
        (0x3f80_0000, 0x3f80_0000),
        (0x7fc1_2345, 0x7fc1_2345),
    ] {
        let mut state = state();
        state.post_prefix.opposing_max = Some(f32::from_bits(maximum));
        let mut services = Services {
            fail: Some(Event::Velocity(0)),
            ..Services::default()
        };
        assert!(state.try_update_contact_motion(0.5, &mut services).is_err());
        assert_eq!(
            state.collision_state.opposing_delta.unwrap().to_bits(),
            expected
        );
    }
}

#[test]
fn grip_uses_aliased_validity_and_ordered_cosine_greater_than_dot() {
    for (word, cosine, dot, expected_grip) in [
        (0x0100_0000, 0.5, 0.25, true),
        (8, 0.5, 0.25, false),
        (0x0100_0000, 0.5, 0.5, false),
        (0x0100_0000, f32::NAN, 0.25, false),
        (0x0100_0000, 0.5, f32::NAN, false),
    ] {
        let mut state = state();
        state.collision_state.in_contact[4] = Some(true);
        state.collision_state.surfaces.set_physics_type(0, word);
        state.collision_state.surfaces.set_physics_type(4, 8);
        let mut services = Services {
            grip: true,
            cosine,
            dot,
            ..Services::default()
        };
        state.try_update_contact_motion(0.5, &mut services).unwrap();
        assert_eq!(
            state.collision_state.flags.read(0x8000_0000),
            Some(0x8000_0000)
        );
        assert_eq!(
            state.collision_state.flags.read(0x0800_0000),
            Some(if expected_grip { 0x0800_0000 } else { 0 })
        );
        assert_eq!(services.events.contains(&Event::Dot), word != 8);
        if word != 8 {
            let start = services
                .events
                .iter()
                .position(|event| *event == Event::GripCosine)
                .unwrap();
            assert_eq!(
                &services.events[start..start + 3],
                [Event::GripCosine, Event::GripVector, Event::Dot]
            );
        }
    }
}
