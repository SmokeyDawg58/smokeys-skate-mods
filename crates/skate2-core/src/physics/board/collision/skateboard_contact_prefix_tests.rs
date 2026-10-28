use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;

#[derive(Default)]
struct Debug {
    calls: Vec<(char, usize)>,
    colors: Vec<[u32; 4]>,
    fail_store: Option<usize>,
}
impl SkateboardContactDebug for Debug {
    type Error = &'static str;
    fn start(&mut self, part: usize) -> Result<i32, Self::Error> {
        self.calls.push(('s', part));
        Ok(i32::try_from(part).unwrap())
    }
    fn end(&mut self, part: usize) -> Result<i32, Self::Error> {
        self.calls.push(('e', part));
        Ok(i32::try_from(part).unwrap() + 1)
    }
    fn store_color(&mut self, index: i32, color: [f32; 4]) -> Result<(), Self::Error> {
        let index = usize::try_from(index).unwrap();
        self.calls.push(('c', index));
        if self.fail_store == Some(index) {
            return Err("color");
        }
        self.colors.push(color.map(f32::to_bits));
        Ok(())
    }
}
fn state() -> SkateboardContactSurface {
    let mut owner = SkateboardContactSurface::new();
    owner.collision_state.reset();
    owner
}

#[test]
fn begin_failure_keeps_interleaved_part_stores_and_defers_flags_mask() {
    let mut owner = state();
    owner.collision_state.flags = super::super::state::SkateboardContactFlags::known(u32::MAX);
    let mut debug = Debug {
        fail_store: Some(2),
        ..Debug::default()
    };
    assert_eq!(
        owner.begin_post_selection(&mut debug),
        Err(SkateboardContactPrefixError::Debug("color"))
    );
    assert_eq!(
        debug.calls,
        [
            ('s', 0),
            ('e', 0),
            ('c', 0),
            ('e', 0),
            ('s', 1),
            ('e', 1),
            ('c', 1),
            ('e', 1),
            ('s', 2),
            ('e', 2),
            ('c', 2)
        ]
    );
    assert_eq!(debug.colors, vec![RESET_COLOR.map(f32::to_bits); 2]);
    assert_eq!(owner.collision_state.surfaces.validity(2), Some(1));
    assert_eq!(owner.collision_state.surfaces.validity(3), Some(0));
    assert_eq!(owner.collision_state.flags.read(u32::MAX), Some(u32::MAX));
    assert_eq!(owner.post_prefix.max_speed.map(f32::to_bits), Some(0));
    assert!(!owner.post_prefix.begun);
}

#[test]
fn winner_and_debug_failure_keep_full_unknown_w_prefix_and_first_nan() {
    let mut owner = state();
    owner.begin_post_selection(&mut Debug::default()).unwrap();
    let normal = from_xyz([0.0, f32::NAN, 1.0]);
    let mut debug = Debug {
        fail_store: Some(0),
        ..Debug::default()
    };
    let result = owner.commit_contact::<(), _>(
        0,
        normal,
        from_xyz([1.0, 2.0, 3.0]),
        from_xyz([4.0, 5.0, 6.0]),
        (7 << 12) | (3 << 7) | 0x11,
        &mut debug,
    );
    assert_eq!(result, Err(SkateboardContactPrefixError::Debug("color")));
    assert_eq!(debug.calls, [('e', 0), ('s', 0), ('c', 0)]);
    assert_eq!(owner.collision_state.normal[0].unwrap().w, None);
    assert_eq!(
        owner.collision_state.relative_velocity[0]
            .unwrap()
            .xyz
            .map(f32::to_bits),
        [4.0_f32, 5.0, 6.0].map(f32::to_bits)
    );
    assert_eq!(owner.collision_state.pattern_type[0], Some(7));
    assert_eq!(owner.collision_state.audio_type[0], Some(17));
    assert!(owner.parts[0].in_contact);
    let mut debug = Debug::default();
    owner
        .commit_contact::<(), _>(
            0,
            from_xyz([0.0, 1.0, 0.0]),
            from_xyz([9.0; 3]),
            from_xyz([9.0; 3]),
            9 << 7,
            &mut debug,
        )
        .unwrap();
    assert_eq!(owner.parts[0].winner.unwrap().physics_type, 3);
    assert!(owner.collision_state.normal[0].unwrap().xyz[1].is_nan());
    assert_eq!(owner.post_prefix.best_part, None);
    assert_eq!(debug.colors, vec![CONTACT_COLOR.map(f32::to_bits)]);
}

#[test]
fn pre_filter_speed_uses_old_velocity_and_preserves_unknown_w() {
    let mut owner = state();
    owner.begin_post_selection(&mut Debug::default()).unwrap();
    owner.collision_state.velocity[6] = Some(from_xyz([0.0, -3.0, 0.0]));
    owner
        .pre_contact::<_, ()>(
            6,
            from_xyz([0.0, 1.0, 0.0]),
            from_xyz([0.0, 1.0, 0.0]),
            &mut HostReckoningNumeric,
        )
        .unwrap();
    assert_eq!(
        owner.post_prefix.max_speed.map(f32::to_bits),
        Some(3.0_f32.to_bits())
    );
    let velocity = owner.post_prefix.max_velocity.unwrap();
    assert_eq!(
        velocity.xyz.map(f32::to_bits),
        [-0.0_f32, -3.0, -0.0].map(f32::to_bits)
    );
    assert_eq!(velocity.w, None);
    assert!(!owner.parts[6].in_contact); // effects precede admission
    owner
        .pre_contact::<_, ()>(
            6,
            from_xyz([0.0, f32::NAN, 0.0]),
            from_xyz([0.0, 1.0, 0.0]),
            &mut HostReckoningNumeric,
        )
        .unwrap();
    assert_eq!(
        owner.post_prefix.max_speed.map(f32::to_bits),
        Some(3.0_f32.to_bits())
    );
    assert!(owner.post_prefix.opposing_max.unwrap().is_nan());
}

struct Services {
    calls: Vec<&'static str>,
    numeric: HostReckoningNumeric,
    fail_value: bool,
}
impl ReckoningNumeric for Services {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls.push("dot");
        self.numeric.dot3(a, b).map_err(|_| "numeric")
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push("rsqrt");
        self.numeric
            .reciprocal_sqrt_estimate(value)
            .map_err(|_| "numeric")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("unused")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("unused")
    }
}
impl SkateboardPostContactServices for Services {
    type DeckEndTarget = ();
    fn owner(
        &mut self,
        _: &SimulationIslandBodyHandle,
    ) -> Result<SimulationContactEntityOwner, Self::Error> {
        panic!("not used by geometry leaf")
    }
    fn other_group(&mut self, _: &SimulationIslandBodyHandle) -> Result<u32, Self::Error> {
        panic!("not used by geometry leaf")
    }
    fn deck_body_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.calls.push("body");
        Ok(SkeletonRootMatrix::from_columns([
            from_xyz([1.0, 0.0, 0.0]),
            from_xyz([0.0, 1.0, 0.0]),
            from_xyz([0.0, 0.0, 1.0]),
            from_xyz([0.0; 3]),
        ]))
    }
    fn capture_deck_end_target(&mut self) -> Result<(), Self::Error> {
        self.calls.push("capture");
        Ok(())
    }
    fn deck_end_value(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.calls.push("value");
        if self.fail_value {
            Err("value")
        } else {
            Ok(0.9)
        }
    }
}

#[test]
fn group_geometry_reads_cvar_after_normalization_and_final_dot_before_value() {
    let mut owner = state();
    owner.begin_post_selection(&mut Debug::default()).unwrap();
    let mut services = Services {
        calls: vec![],
        numeric: HostReckoningNumeric,
        fail_value: true,
    };
    assert_eq!(
        owner.contact_group::<_, ()>(6, 4, from_xyz([0.0, 2.0, 3.0]), &mut services),
        Err(SkateboardContactPrefixError::Service("value"))
    );
    assert_eq!(
        services.calls,
        ["body", "dot", "dot", "rsqrt", "capture", "dot", "value"]
    );
    assert_eq!(owner.collision_state.flags.read(0x4000_0000), Some(0));
    services.fail_value = false;
    owner
        .contact_group::<_, ()>(6, 4, from_xyz([0.0, 2.0, 3.0]), &mut services)
        .unwrap();
    assert_eq!(
        owner.collision_state.flags.read(0x4000_0000),
        Some(0x4000_0000)
    );
    owner
        .contact_group::<_, ()>(5, 15, from_xyz([0.0; 3]), &mut services)
        .unwrap();
    assert_eq!(
        owner.collision_state.surfaces.physics_type(0),
        Some(0x0100_0100)
    );
    assert_eq!(
        owner.collision_state.flags.read(0x2000_0000),
        Some(0x2000_0000)
    );
}
