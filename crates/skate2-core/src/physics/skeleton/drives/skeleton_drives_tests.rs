//! Source-order discriminators. Synthetic pose/tuning inputs are not a mounted
//! player or a Xenon numeric trace; the separate original-data corpus covers it.
use super::*;
use crate::skateboard_wheels::{HostSkateboardWheelMassNumeric, SkateboardWheelInertiaAxis};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Event {
    Bind(SkeletonDriveType),
    Lookup(u64),
    Load(u64),
    Layout(SkeletonDriveType, usize),
    Quaternion(SkeletonDriveQuaternionSite),
    Mass(SkateboardWheelInertiaAxis),
    Displacement,
    Strength,
}
type Events = Rc<RefCell<Vec<Event>>>;

struct Attributes {
    events: Events,
    fail_layout: Option<(SkeletonDriveType, usize)>,
}
impl SkeletonDriveAttributeRead for Attributes {
    type Pointer = u64;
    type Error = &'static str;
    fn find_and_change_animation_drive(
        &mut self,
        kind: SkeletonDriveType,
    ) -> Result<(), Self::Error> {
        self.events.borrow_mut().push(Event::Bind(kind));
        Ok(())
    }
    fn lookup_global(&mut self, key: u64) -> Result<u64, Self::Error> {
        self.events.borrow_mut().push(Event::Lookup(key));
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push(Event::Load(*key));
        Ok(f32::from_bits(match *key {
            0xe1a7_f2dc_fab0_b8d0 => 0x8000_0000,
            0xf300_ffd5_99e1_709e => 0x7fc1_2345,
            0xf508_d452_b962_f68d => 0x3f80_0000,
            0x15a2_6b61_43e7_df97 => 0xc000_0000,
            _ => panic!("unexpected source key"),
        }))
    }
    fn animation_float(
        &mut self,
        kind: SkeletonDriveType,
        offset: usize,
    ) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push(Event::Layout(kind, offset));
        if self.fail_layout == Some((kind, offset)) {
            return Err("cached read failed");
        }
        let layout = match kind {
            SkeletonDriveType::Local => [7.0, 11.0, 13.0, 17.0],
            SkeletonDriveType::LocalRoot => [19.0, 23.0, 29.0, 31.0],
        };
        Ok(layout[offset / 4])
    }
}
struct Numeric(Events);
impl SkeletonDriveNumeric for Numeric {
    type Error = &'static str;
    fn quaternion_estimate(
        &mut self,
        site: SkeletonDriveQuaternionSite,
        values: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        self.0.borrow_mut().push(Event::Quaternion(site));
        Ok(values.map(|value| value.sqrt().recip()))
    }
}
struct Mass(Events);
impl SkateboardWheelMassNumeric for Mass {
    fn reciprocal_estimate(&mut self, axis: SkateboardWheelInertiaAxis, value: f32) -> f32 {
        self.0.borrow_mut().push(Event::Mass(axis));
        value.recip()
    }
}
struct Cvars {
    events: Events,
    next: usize,
    fail_at: Option<usize>,
}
impl Cvars {
    fn sample(&mut self, event: Event) -> Result<f32, &'static str> {
        self.events.borrow_mut().push(event);
        if self.fail_at == Some(self.next) {
            return Err("current cvar unavailable");
        }
        let value = [2.0, 3.0, 5.0, 7.0][self.next];
        self.next += 1;
        Ok(value)
    }
}
impl SkeletonDriveCvars for Cvars {
    type Error = &'static str;
    fn hard_drive_displacement(&mut self) -> Result<f32, Self::Error> {
        self.sample(Event::Displacement)
    }
    fn hard_drive_strength(&mut self) -> Result<f32, Self::Error> {
        self.sample(Event::Strength)
    }
}

fn data() -> Arc<SkeletonData> {
    let (hierarchy, parameters, pose) = crate::skeleton_data_tests::fixture();
    Arc::new(crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap())
}
fn create(data: Arc<SkeletonData>) -> (SkeletonDrives, Events) {
    let events = Events::default();
    let owner = SkeletonDrives::try_create(
        data,
        &mut Attributes {
            events: events.clone(),
            fail_layout: None,
        },
        &mut Numeric(events.clone()),
        &mut Mass(events.clone()),
        &mut Cvars {
            events: events.clone(),
            next: 0,
            fail_at: None,
        },
    )
    .unwrap();
    (owner, events)
}
fn part(index: usize) -> SkeletonPartIndex {
    SkeletonPartIndex::from_index(index).unwrap()
}

#[test]
fn constructor_repeats_actual_fetches_then_frames_layouts_mass_and_live_cvars_in_order() {
    use SkateboardWheelInertiaAxis::{X, Y, Z};
    use SkeletonDriveQuaternionSite::{BoneChild, BoneParent, HookChild, HookParent};
    use SkeletonDriveType::{Local, LocalRoot};
    let (owner, events) = create(data());
    let mut expected = Vec::new();
    for _ in 0..24 {
        expected.extend([Event::Bind(Local), Event::Bind(LocalRoot)]);
        for key in [
            0xe1a7_f2dc_fab0_b8d0,
            0xf300_ffd5_99e1_709e,
            0xf508_d452_b962_f68d,
            0x15a2_6b61_43e7_df97,
        ] {
            expected.extend([Event::Lookup(key), Event::Load(key)]);
        }
    }
    for index in 1..23 {
        let part = part(index);
        expected.extend([
            Event::Quaternion(BoneChild { part, kind: Local }),
            Event::Quaternion(BoneParent { part, kind: Local }),
            Event::Layout(Local, 8),
            Event::Layout(Local, 12),
            Event::Quaternion(BoneChild {
                part,
                kind: LocalRoot,
            }),
            Event::Quaternion(BoneParent {
                part,
                kind: LocalRoot,
            }),
            Event::Layout(LocalRoot, 8),
            Event::Layout(LocalRoot, 12),
            Event::Layout(LocalRoot, 4),
            Event::Layout(LocalRoot, 0),
        ]);
    }
    for hook in 0..2 {
        expected.extend([
            Event::Mass(Y),
            Event::Mass(X),
            Event::Mass(Z),
            Event::Quaternion(HookParent { hook }),
            Event::Quaternion(HookChild { hook }),
        ]);
    }
    expected.extend([
        Event::Displacement,
        Event::Strength,
        Event::Displacement,
        Event::Strength,
    ]);
    assert_eq!(*events.borrow(), expected);
    assert_eq!(owner.ragdoll_strength().to_bits(), 0);
    assert_eq!(owner.ik_active(), &[false; 24]);
    for index in 0..24 {
        assert_eq!(
            owner.bone(part(index)).soft_parameters().map(f32::to_bits),
            [0x8000_0000, 0x7fc1_2345, 0x3f80_0000, 0xc000_0000]
        );
        assert_eq!(
            owner.bone(part(index)).strengths().map(f32::to_bits),
            [0; 2]
        );
    }
}

#[test]
fn single_flags_field_is_zero_but_local_prepared_dynamics_stays_angular_only() {
    use SkeletonDriveType::{Local, LocalRoot};
    let (owner, _) = create(data());
    let bone = owner.bone(part(1));
    assert_eq!(bone.flags(), 0);
    assert_eq!(bone.active(), &[true; 2]);
    assert_eq!(bone.parent_parts(), &[Some(part(2)), Some(part(23))]);
    let local = *bone.dynamics(Local).read().unwrap();
    let root = *bone.dynamics(LocalRoot).read().unwrap();
    assert_eq!(
        local.linear,
        SimulationDriveDynamicsParams {
            raw_type: 2,
            spring: 0.0,
            damping: 0.0,
            strength: 0.0
        }
    );
    assert_eq!(
        local.angular.spring.to_bits(),
        (17.0 * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert_eq!(
        local.angular.strength.to_bits(),
        (13.0 * f32::from_bits(0x4560_fffe)).to_bits()
    );
    assert_eq!(
        root.angular.spring.to_bits(),
        (31.0 * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert_eq!(
        root.angular.strength.to_bits(),
        (29.0 * f32::from_bits(0x4560_fffe)).to_bits()
    );
    assert_eq!(
        root.linear.spring.to_bits(),
        (23.0 * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert_eq!(
        root.linear.strength.to_bits(),
        (19.0 * f32::from_bits(0x4560_fffe)).to_bits()
    );
    for index in [0, 23] {
        let untouched = owner.bone(part(index));
        assert_eq!(untouched.part(), None);
        assert_eq!(untouched.parent_parts(), &[None; 2]);
        assert_eq!(untouched.active(), &[false; 2]);
        assert_eq!(*untouched.dynamics(Local).read().unwrap(), fresh_dynamics());
    }
    let emissions: Vec<_> = owner
        .eligible_bone_drives()
        .map(|entry| (entry.part.index(), entry.parent.index(), entry.kind))
        .collect();
    let expected: Vec<_> = (1..23)
        .flat_map(|index| [(index, index + 1, Local), (index, 23, LocalRoot)])
        .collect();
    assert_eq!(emissions, expected);
}

#[test]
fn bone_frames_use_world_axis_anchor_not_child_rotation_or_volume_mapping() {
    let mut parent = FRESH_VOLUME_TRANSFORM;
    parent[12..16].copy_from_slice(&[1.0, 2.0, 3.0, 17.0]);
    let child = [
        0.0, 1.0, 0.0, 7.0, -1.0, 0.0, 0.0, 9.0, 0.0, 0.0, 1.0, 11.0, 4.0, 5.0, 6.0, 13.0,
    ];
    let events = Events::default();
    let frames = try_bone_frames(
        part(1),
        SkeletonDriveType::Local,
        &parent,
        &child,
        &mut Numeric(events.clone()),
    )
    .unwrap();
    assert_eq!(frames.position_a.map(f32::to_bits), [0; 4]);
    assert_eq!(
        frames.position_b.map(f32::to_bits),
        [3.0_f32, 3.0, 3.0, 0.0].map(f32::to_bits)
    );
    assert_eq!(
        frames.quaternion_b.map(f32::to_bits),
        [0.0_f32, 0.0, 0.0, 1.0].map(f32::to_bits)
    );
    assert!(frames.quaternion_a[0].abs() < 0.000_001 && frames.quaternion_a[1].abs() < 0.000_001);
    assert!((frames.quaternion_a[2].abs() - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.000_001);
    assert!((frames.quaternion_a[3].abs() - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.000_001);
    assert!(
        frames.quaternion_a[2] * frames.quaternion_a[3] < 0.0,
        "inverse90-degree child angular frame"
    );
    assert_eq!(
        *events.borrow(),
        [
            Event::Quaternion(SkeletonDriveQuaternionSite::BoneChild {
                part: part(1),
                kind: SkeletonDriveType::Local
            }),
            Event::Quaternion(SkeletonDriveQuaternionSite::BoneParent {
                part: part(1),
                kind: SkeletonDriveType::Local
            }),
        ]
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "complete source payload and ownership oracle"
)]
fn real_two_box_hook_template_moves_once_and_geometry_releases_before_bone_payloads() {
    let source = data();
    let (mut owner, _) = create(Arc::clone(&source));
    assert!(Arc::ptr_eq(owner.data(), &source));
    let geometry = Arc::downgrade(owner.hook_definition().unwrap());
    let bone_payload = Arc::downgrade(owner.bone(part(1)).frames(SkeletonDriveType::Local));
    let session = owner.take_hook_session().unwrap();
    assert!(owner.take_hook_session().is_none());
    assert!(Arc::ptr_eq(
        session.template().collision(),
        owner.hook_definition().unwrap()
    ));
    assert_eq!(session.template().joints().len(), 0);
    assert_eq!(session.template().drives().len(), 0);
    // Independent native Part stores9020/24/28/2c, not an expected value
    // borrowed from the producer's Volume initialization constant.
    let initial_part_words = [
        0x3f80_0000,
        0,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0,
        0,
    ];
    assert_eq!(
        session
            .part_transforms()
            .map(|matrix| matrix.expect("known fixture Part cache"))
            .map(|matrix| matrix.map(f32::to_bits))
            .collect::<Vec<_>>(),
        vec![initial_part_words; 2]
    );
    assert!(!session.is_attached());
    let definition = owner.hook_definition().unwrap();
    assert!(!Arc::ptr_eq(
        definition.parts()[0].inertia(),
        definition.parts()[1].inertia()
    ));
    for (index, hook) in definition.parts().iter().enumerate() {
        assert_eq!(hook.volume().vtable_offset, VolumeType::Box as u32);
        assert_eq!(hook.volume().shape_words, [0x3dcc_cccd; 3]);
        assert_eq!(hook.volume().flags, 0);
        assert_eq!(
            hook.volume().transform.map(f32::to_bits),
            FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
        );
        assert_eq!(hook.volume().radius.to_bits(), 0);
        assert_eq!((hook.volume().group_id, hook.volume().surface_id), (0, 0));
        assert!(!hook.is_static_body());
        assert!(hook.inverse_body_transform().is_none());
        assert_eq!(hook.inertia().inverse_mass.to_bits(), 0x3c23_d70a);
        assert_eq!(
            hook.inverse_tensor_w().to_bits(),
            hook.inertia().inverse_tensor[0].to_bits()
        );
        assert!(
            hook.inertia()
                .inverse_tensor
                .into_iter()
                .all(|value| (value - 1.5).abs() < 0.000_001)
        );
        assert_eq!(
            [
                hook.material().static_friction,
                hook.material().dynamic_friction,
                hook.material().restitution
            ]
            .map(f32::to_bits),
            [0x3f00_0000, 0x3e99_999a, 0x3ecc_cccd]
        );
        assert!(Arc::ptr_eq(
            session.template().part_payloads()[index]
                .inertia
                .as_immutable()
                .unwrap(),
            hook.inertia()
        ));
        let frames = owner.hook_frames(index).read().unwrap();
        assert_eq!(frames.position_a.map(f32::to_bits), [0; 4]);
        assert_eq!(frames.position_b.map(f32::to_bits), [0; 4]);
        assert_eq!(
            frames.quaternion_a.map(f32::to_bits),
            [0, 0, 0, 0x3f80_0000]
        );
        assert_eq!(
            frames.quaternion_b.map(f32::to_bits),
            [0, 0, 0, 0x3f80_0000]
        );
    }
    let hard0 = *owner.hook_dynamics(0).read().unwrap();
    assert_eq!(hard0.linear.spring.to_bits(), 0x4415_ffff);
    assert_eq!(hard0.linear.strength.to_bits(), 0x470c_9fff);
    assert_eq!(hard0.angular, hard0.linear);
    let hard1 = *owner.hook_dynamics(1).read().unwrap();
    assert_eq!(
        hard1.linear.spring.to_bits(),
        (2.0 * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert_eq!(
        hard1.linear.strength.to_bits(),
        (3.0 * f32::from_bits(0x4560_fffe)).to_bits()
    );
    assert_eq!(
        hard1.angular.spring.to_bits(),
        (5.0 * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert_eq!(
        hard1.angular.strength.to_bits(),
        (7.0 * f32::from_bits(0x4560_fffe)).to_bits()
    );
    drop(session);
    assert!(geometry.upgrade().is_some());
    owner.release_hook_definition();
    assert!(geometry.upgrade().is_none());
    assert!(bone_payload.upgrade().is_some());
    owner.mark_bone_released(part(1), SkeletonDriveType::Local);
    assert_eq!(owner.bone(part(1)).active(), &[false, true]);
    assert_eq!(
        owner.bone(part(1)).parent_parts(),
        &[Some(part(2)), Some(part(23))]
    );
    drop(owner);
    assert!(bone_payload.upgrade().is_none());
}

#[test]
fn source_prepares_parented_root_slot_but_admission_selection_still_excludes_it() {
    let (mut hierarchy, parameters, pose) = crate::skeleton_data_tests::fixture();
    hierarchy.parents[0] = 23;
    hierarchy.parents[22] = -1; // Keep native22-joint capacity; no invented tail.
    let source = Arc::new(crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap());
    let (owner, _) = create(source);
    assert_eq!(owner.bone(part(0)).active(), &[true; 2]);
    assert_eq!(owner.bone(part(22)).active(), &[false; 2]);
    let emitted: Vec<_> = owner
        .eligible_bone_drives()
        .map(|entry| entry.part.index())
        .collect();
    assert_eq!(emitted.len(), 42);
    assert!(!emitted.iter().any(|index| [0, 22, 23].contains(index)));
}

#[test]
fn refused_cached_load_or_second_cvar_pair_stops_the_actual_read_prefix() {
    let events = Events::default();
    let result = SkeletonDrives::try_create(
        data(),
        &mut Attributes {
            events: events.clone(),
            fail_layout: Some((SkeletonDriveType::LocalRoot, 4)),
        },
        &mut Numeric(events.clone()),
        &mut HostSkateboardWheelMassNumeric,
        &mut Cvars {
            events: events.clone(),
            next: 0,
            fail_at: None,
        },
    );
    assert!(matches!(
        result,
        Err(SkeletonDrivesError::Attribute("cached read failed"))
    ));
    assert_eq!(
        events.borrow().last(),
        Some(&Event::Layout(SkeletonDriveType::LocalRoot, 4))
    );
    assert!(
        !events
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::Displacement | Event::Strength))
    );
    let events = Events::default();
    let result = SkeletonDrives::try_create(
        data(),
        &mut Attributes {
            events: events.clone(),
            fail_layout: None,
        },
        &mut Numeric(events.clone()),
        &mut HostSkateboardWheelMassNumeric,
        &mut Cvars {
            events: events.clone(),
            next: 0,
            fail_at: Some(2),
        },
    );
    assert!(matches!(
        result,
        Err(SkeletonDrivesError::Cvar("current cvar unavailable"))
    ));
    let samples: Vec<_> = events
        .borrow()
        .iter()
        .filter(|event| matches!(event, Event::Displacement | Event::Strength))
        .copied()
        .collect();
    assert_eq!(
        samples,
        [Event::Displacement, Event::Strength, Event::Displacement]
    );
}
