//! Actual constructed owner and retained payload tests. Controlled pose/tuning
//! exercise source equations; they do not claim admission or live animation.
use super::*;
use crate::{
    skateboard_wheels::HostSkateboardWheelMassNumeric,
    skeleton_collision_attributes::{SkeletonCollisionAttributeRead, SkeletonCollisionOffset},
};
use std::{cell::RefCell, convert::Infallible, rc::Rc};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Lookup(u64),
    Float(u64),
    Byte(u64),
    Capture(SkeletonDriveType),
    Layout(SkeletonDriveType, usize),
    Quaternion(SkeletonDriveQuaternionSite),
    Offboard,
    Damping,
}

#[derive(Clone)]
struct Services {
    events: Rc<RefCell<Vec<Event>>>,
    fail: Option<usize>,
    follow: u8,
    local: f32,
    root: f32,
    offboard: f32,
    watch: Option<Arc<RwLock<SimulationDriveFrames>>>,
}
impl Services {
    fn new() -> Self {
        Self {
            events: Rc::default(),
            fail: None,
            follow: 1,
            local: 2.0,
            root: 3.0,
            offboard: 0.5,
            watch: None,
        }
    }
    fn event(&mut self, event: Event) -> Result<(), usize> {
        let mut events = self.events.borrow_mut();
        let index = events.len();
        events.push(event);
        if self.fail == Some(index) {
            Err(index)
        } else {
            Ok(())
        }
    }
}
impl SkeletonDriveAttributeRead for Services {
    type Pointer = u64;
    type Error = usize;
    fn find_and_change_animation_drive(&mut self, _: SkeletonDriveType) -> Result<(), usize> {
        Ok(())
    }
    fn lookup_global(&mut self, key: u64) -> Result<u64, usize> {
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, usize> {
        Ok(if *key == SOFT_KEYS[0] { 8.0 } else { 4.0 })
    }
    fn animation_float(&mut self, _: SkeletonDriveType, _: usize) -> Result<f32, usize> {
        Ok(1.0)
    }
}
impl SkeletonDriveCvars for Services {
    type Error = usize;
    fn hard_drive_displacement(&mut self) -> Result<f32, usize> {
        Ok(1.0)
    }
    fn hard_drive_strength(&mut self) -> Result<f32, usize> {
        Ok(10.0)
    }
}
impl SkeletonDrivesUpdateAttributeRead for Services {
    type Pointer = u64;
    type Layout = SkeletonDriveType;
    type Error = usize;
    fn lookup_global(&mut self, key: u64) -> Result<u64, usize> {
        self.event(Event::Lookup(key))?;
        Ok(key)
    }
    fn float(&mut self, key: &u64) -> Result<f32, usize> {
        self.event(Event::Float(*key))?;
        Ok(if *key == LOCAL_SCALE_KEY {
            self.local
        } else {
            self.root
        })
    }
    fn byte(&mut self, key: &u64) -> Result<u8, usize> {
        self.event(Event::Byte(*key))?;
        Ok(self.follow)
    }
    fn capture_animation_layout(
        &mut self,
        kind: SkeletonDriveType,
    ) -> Result<SkeletonDriveType, usize> {
        self.event(Event::Capture(kind))?;
        Ok(kind)
    }
    fn animation_float(&mut self, kind: &SkeletonDriveType, offset: usize) -> Result<f32, usize> {
        self.event(Event::Layout(*kind, offset))?;
        Ok([2.0, 3.0, 5.0, 7.0][offset / 4])
    }
}
impl SkeletonDrivesUpdateCvars for Services {
    type Error = usize;
    fn offboard_drive_scalar(&mut self) -> Result<f32, usize> {
        self.event(Event::Offboard)?;
        Ok(self.offboard)
    }
    fn soft_damping(&mut self) -> Result<f32, usize> {
        self.event(Event::Damping)?;
        Ok(6.0)
    }
}
impl SkeletonDriveNumeric for Services {
    type Error = usize;
    fn quaternion_estimate(
        &mut self,
        site: SkeletonDriveQuaternionSite,
        values: [f32; 4],
    ) -> Result<[f32; 4], usize> {
        if let Some(frames) = &self.watch {
            // Read succeeds during callbacks: the author must not hold a lock.
            let frames = frames.try_read().expect("no payload lock across callback");
            if matches!(site,SkeletonDriveQuaternionSite::BoneParent{part,kind:SkeletonDriveType::Local} if part.index()==1)
            {
                assert_eq!(frames.position_a.map(f32::to_bits), [0; 4]);
                assert_eq!(frames.position_b.map(f32::to_bits), [99.0_f32.to_bits(); 4]);
            }
        }
        self.event(Event::Quaternion(site))?;
        Ok(values.map(|v| v.sqrt().recip()))
    }
}

struct Collision;
impl SkeletonCollisionAttributeRead for Collision {
    type Pointer = ();
    type Error = Infallible;
    fn cached_offset(&mut self, _: SkeletonCollisionOffset) -> Result<[f32; 4], Infallible> {
        Ok([0.0; 4])
    }
    fn lookup(&mut self, _: u64) -> Result<(), Infallible> {
        Ok(())
    }
    fn float(&mut self, (): &()) -> Result<f32, Infallible> {
        Ok(1.0)
    }
}
fn collision(strength: f32) -> SkeletonCollisionData {
    let mut collision = SkeletonCollisionData::try_new(&mut Collision).unwrap();
    collision.drive_strength = strength;
    collision
}
fn create(missing: Option<usize>) -> SkeletonDrives {
    let (mut hierarchy, parameters, pose) = crate::skeleton_data_tests::fixture();
    if let Some(part) = missing {
        hierarchy.parents[part] = -1;
    }
    let data = Arc::new(crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap());
    SkeletonDrives::try_create(
        data,
        &mut Services::new(),
        &mut HostSkeletonDriveNumeric,
        &mut HostSkateboardWheelMassNumeric,
        &mut Services::new(),
    )
    .unwrap()
}
fn targets() -> [[f32; 16]; 24] {
    std::array::from_fn(|part| {
        let mut m = FRESH_VOLUME_TRANSFORM;
        m[12] = f32::from(u8::try_from(part).unwrap());
        m[15] = 7.0;
        m
    })
}
fn coefficients() -> [SkeletonDriveAttributes; 24] {
    let mut coefficient = SkeletonDriveAttributes::constructor_value();
    coefficient.collision_strength = 0.25;
    coefficient.collision_strength_root = 0.5;
    [coefficient; 24]
}
fn run(
    owner: &mut SkeletonDrives,
    strength: f32,
    mask: u8,
    services: &Services,
) -> Result<(), SkeletonDrivesUpdateError<usize, usize, usize>> {
    owner.try_update(
        &targets(),
        &collision(strength),
        &coefficients(),
        mask,
        &mut services.clone(),
        &mut services.clone(),
        &mut services.clone(),
    )
}
fn globals() -> Vec<Event> {
    vec![
        Event::Lookup(LOCAL_SCALE_KEY),
        Event::Float(LOCAL_SCALE_KEY),
        Event::Lookup(ROOT_SCALE_KEY),
        Event::Float(ROOT_SCALE_KEY),
        Event::Lookup(NORMAL_FOLLOW_KEY),
        Event::Byte(NORMAL_FOLLOW_KEY),
    ]
}
fn frames_words(value: SimulationDriveFrames) -> [[u32; 4]; 4] {
    [
        value.quaternion_a,
        value.position_a,
        value.quaternion_b,
        value.position_b,
    ]
    .map(|v| v.map(f32::to_bits))
}
fn params_words(value: SimulationDriveDynamicsParams) -> [u32; 4] {
    [
        value.spring.to_bits(),
        value.damping.to_bits(),
        value.strength.to_bits(),
        value.raw_type,
    ]
}

#[test]
fn normal_update_mutates_same_payloads_in_part_local_root_order_and_captures_each_layout() {
    let mut owner = create(None);
    let frame_owner = Arc::clone(&owner.bones[1].frames[0]);
    let hook0_angular = owner.hook_dynamics[0].read().unwrap().angular;
    let hook1 = *owner.hook_dynamics[1].read().unwrap();
    let untouched = [
        frames_words(*owner.bones[0].frames[0].read().unwrap()),
        frames_words(*owner.bones[23].frames[0].read().unwrap()),
    ];
    let services = Services::new();
    run(&mut owner, 1.0, 0, &services).unwrap();
    let mut expected = globals();
    for part in 1..23 {
        for kind in [SkeletonDriveType::Local, SkeletonDriveType::LocalRoot] {
            let part = SkeletonPartIndex::from_index(part).unwrap();
            expected.extend([
                Event::Quaternion(SkeletonDriveQuaternionSite::BoneChild { part, kind }),
                Event::Quaternion(SkeletonDriveQuaternionSite::BoneParent { part, kind }),
                Event::Capture(kind),
            ]);
            expected.extend([8, 12, 4, 0].map(|offset| Event::Layout(kind, offset)));
        }
    }
    assert_eq!(*services.events.borrow(), expected);
    assert!(Arc::ptr_eq(&frame_owner, &owner.bones[1].frames[0]));
    for bone in &owner.bones[1..23] {
        assert_eq!(bone.flags, 0);
        assert_eq!(
            bone.strengths.map(f32::to_bits),
            [2.0_f32.to_bits(), 3.0_f32.to_bits()]
        );
        for dynamics in &bone.dynamics {
            let d = dynamics.read().unwrap();
            assert_eq!(
                params_words(d.angular),
                [
                    (7.0 * DISPLACEMENT_SCALE).to_bits(),
                    0,
                    (5.0 * STRENGTH_SCALE).to_bits(),
                    2
                ]
            );
            assert_eq!(
                params_words(d.linear),
                [
                    (3.0 * DISPLACEMENT_SCALE).to_bits(),
                    0,
                    (2.0 * STRENGTH_SCALE).to_bits(),
                    2
                ]
            );
        }
    }
    assert_eq!(
        frame_owner.read().unwrap().position_a.map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(
        frame_owner.read().unwrap().position_b.map(f32::to_bits),
        [(-1.0_f32).to_bits(), 0, 0, 0]
    );
    assert_eq!(
        owner.bones[1].frames[1]
            .read()
            .unwrap()
            .position_b
            .map(f32::to_bits),
        [(-22.0_f32).to_bits(), 0, 0, 0]
    );
    assert_eq!(
        params_words(owner.hook_dynamics[0].read().unwrap().angular),
        params_words(hook0_angular)
    );
    assert_eq!(
        params_words(owner.hook_dynamics[1].read().unwrap().linear),
        params_words(hook1.linear)
    );
    assert_eq!(
        params_words(owner.hook_dynamics[1].read().unwrap().angular),
        params_words(hook1.angular)
    );
    assert_eq!(
        [
            frames_words(*owner.bones[0].frames[0].read().unwrap()),
            frames_words(*owner.bones[23].frames[0].read().unwrap())
        ],
        untouched
    );
}

#[test]
fn soft_mask_uses_original_scales_with_follow_disabled_and_each_drive_reads_damping() {
    let mut owner = create(None);
    let mut services = Services::new();
    services.follow = 0;
    run(&mut owner, 0.5, 0x80, &services).unwrap();
    // Root (.5+.5*.5)*.5*3=1.125; local(.25+.75*.5)*.5*2=.625.
    for (part, bone) in owner.bones.iter().enumerate().take(23).skip(1) {
        if [1, 2, 3, 4, 5, 7, 8, 9].contains(&part) {
            assert_eq!(bone.flags, 3);
            assert_eq!(
                bone.strengths.map(f32::to_bits),
                [0.625_f32.to_bits(), 1.125_f32.to_bits()]
            );
            for (kind, argument) in [(0, 0.625), (1, 1.125)] {
                let d = bone.dynamics[kind].read().unwrap();
                assert_eq!(
                    params_words(d.angular),
                    [
                        (8.0_f32 * argument).to_bits(),
                        6.0_f32.to_bits(),
                        ((4.0 * argument) * STRENGTH_SCALE).to_bits(),
                        1
                    ]
                );
                assert_eq!(params_words(d.linear), [0, 0, 0, 1]);
            }
        } else {
            assert_eq!(bone.flags, 0);
            assert_eq!(bone.strengths.map(f32::to_bits), [0; 2]);
        }
    }
    let events = services.events.borrow();
    assert_eq!(events[6], Event::Offboard);
    assert_eq!(events.iter().filter(|e| **e == Event::Damping).count(), 16);
}

#[test]
fn soft_coefficients_use_fused_affine_and_ordered_offboard_then_global_products() {
    let mut owner = create(None);
    let mut service = Services::new();
    service.local = 1.0;
    service.root = 1.0;
    let mut values = coefficients();
    values[1].collision_strength = f32::from_bits(0x3eaa_aaab);
    values[1].collision_strength_root = values[1].collision_strength;
    let strength = f32::from_bits(0xbf00_0001);
    owner
        .try_update(
            &targets(),
            &collision(strength),
            &values,
            0,
            &mut service.clone(),
            &mut service.clone(),
            &mut service.clone(),
        )
        .unwrap();
    // Independent exact rational oracle: c=11184811/2^25,
    // rounded(1-c)=5592405/2^23; s=-8388609/2^24.
    // Fused (1-c)*s+c = -1398101/2^47. A rounded product then
    // addition would instead cancel to zero. No argument<=0 gate exists.
    assert_eq!(owner.bones[1].strengths.map(f32::to_bits), [0xb22a_aaa8; 2]);
    let rounded_product = (1.0 - values[1].collision_strength) * strength;
    assert_eq!(
        (rounded_product + values[1].collision_strength).to_bits(),
        0
    );
    let mut owner = create(None);
    let mut service = Services::new();
    service.local = 0.5;
    service.root = 0.5;
    service.offboard = 2.0;
    values[1].collision_strength = f32::MAX;
    values[1].collision_strength_root = f32::MAX;
    owner
        .try_update(
            &targets(),
            &collision(0.0),
            &values,
            1,
            &mut service.clone(),
            &mut service.clone(),
            &mut service.clone(),
        )
        .unwrap();
    // (MAX*offboard2)*global.5 overflows; MAX*(2*.5) does not.
    assert_eq!(
        owner.bones[1].strengths.map(f32::to_bits),
        [f32::INFINITY.to_bits(); 2]
    );
    assert!((f32::MAX * (2.0 * 0.5)).is_finite());
}

#[test]
fn unordered_collision_strength_selects_normal_and_offboard_still_samples() {
    let mut owner = create(None);
    let mut services = Services::new();
    services.follow = 0;
    run(&mut owner, f32::NAN, 1, &services).unwrap();
    assert_eq!(services.events.borrow()[6], Event::Offboard);
    assert!(!services.events.borrow().contains(&Event::Damping));
    assert!(
        owner.bones[1..23]
            .iter()
            .all(|b| b.flags == 0 && b.strengths.map(f32::to_bits) == [0; 2])
    );
    assert_eq!(
        owner.bones[1].dynamics[0].read().unwrap().angular.raw_type,
        2
    );
}

#[test]
fn child_frame_commit_precedes_parent_callback_failure_with_no_lock_held() {
    let mut owner = create(None);
    let frames = Arc::clone(&owner.bones[1].frames[0]);
    frames.write().unwrap().position_a = [88.0; 4];
    frames.write().unwrap().position_b = [99.0; 4];
    let previous_dynamics = *owner.bones[1].dynamics[0].read().unwrap();
    let mut services = Services::new();
    services.fail = Some(7);
    services.watch = Some(frames.clone());
    assert!(matches!(
        run(&mut owner, 1.0, 0, &services),
        Err(SkeletonDrivesUpdateError::Numeric(7))
    ));
    assert_eq!(frames.read().unwrap().position_a.map(f32::to_bits), [0; 4]);
    assert_eq!(
        frames.read().unwrap().position_b.map(f32::to_bits),
        [99.0_f32.to_bits(); 4]
    );
    assert_eq!(
        params_words(owner.bones[1].dynamics[0].read().unwrap().angular),
        params_words(previous_dynamics.angular)
    );
    assert_eq!(
        owner.bones[1].strengths.map(f32::to_bits),
        [2.0_f32.to_bits(), 3.0_f32.to_bits()]
    );
    assert_eq!(owner.bones[2].strengths.map(f32::to_bits), [0; 2]);
}

#[test]
fn late_soft_cvar_failure_retains_frames_strengths_but_not_dynamics() {
    let mut owner = create(None);
    let previous = *owner.bones[1].dynamics[0].read().unwrap();
    let mut services = Services::new();
    services.fail = Some(8);
    assert!(matches!(
        run(&mut owner, 0.5, 0, &services),
        Err(SkeletonDrivesUpdateError::Cvar(8))
    ));
    assert_eq!(services.events.borrow()[8], Event::Damping);
    assert_eq!(
        owner.bones[1].frames[0].read().unwrap().position_b[0].to_bits(),
        (-1.0_f32).to_bits()
    );
    assert_eq!(
        params_words(owner.bones[1].dynamics[0].read().unwrap().angular),
        params_words(previous.angular)
    );
    assert_eq!(owner.bones[1].flags, 3);
    assert_eq!(
        owner.bones[1].strengths.map(f32::to_bits),
        [1.25_f32.to_bits(), 2.25_f32.to_bits()]
    );
}

#[test]
fn global_and_offboard_failures_preserve_the_exact_hook_prefix() {
    for failed in 0..7 {
        let mut owner = create(None);
        owner.hook_dynamics[0].write().unwrap().linear = ZERO_PARAMS;
        let mut services = Services::new();
        services.fail = Some(failed);
        let error = run(&mut owner, 1.0, 1, &services).unwrap_err();
        if failed < 6 {
            assert!(matches!(error,SkeletonDrivesUpdateError::Attribute(i) if i==failed));
        } else {
            assert!(matches!(error, SkeletonDrivesUpdateError::Cvar(6)));
        }
        assert_eq!(services.events.borrow().len(), failed + 1);
        let linear = owner.hook_dynamics[0].read().unwrap().linear;
        assert_eq!(linear.raw_type, if failed < 6 { 0 } else { 2 });
        assert_eq!(owner.bones[1].strengths.map(f32::to_bits), [0; 2]);
    }
}

#[test]
fn inactive_rows_still_validate_parent_after_flags_and_previous_rows_remain() {
    let mut owner = create(Some(6));
    assert_eq!(owner.bones[6].active, [false; 2]);
    assert!(matches!(
        run(&mut owner, 0.0, 0, &Services::new()),
        Err(SkeletonDrivesUpdateError::MissingParent { part: 6 })
    ));
    assert_eq!(owner.bones[6].flags, 3);
    assert_eq!(owner.bones[6].strengths.map(f32::to_bits), [0; 2]);
    assert_eq!(
        owner.bones[5].dynamics[1].read().unwrap().angular.raw_type,
        1
    );
    assert_eq!(owner.bones[7].flags, 0);
}

#[test]
fn poisoned_dynamics_refusal_occurs_after_services_and_frame_writes() {
    let mut owner = create(None);
    let dynamics = Arc::clone(&owner.bones[1].dynamics[0]);
    let _ = std::panic::catch_unwind(|| {
        let _held = dynamics.write().unwrap();
        panic!("controlled poison");
    });
    let services = Services::new();
    assert!(matches!(
        run(&mut owner, 1.0, 0, &services),
        Err(SkeletonDrivesUpdateError::Poisoned {
            part: Some(1),
            kind: SkeletonDriveType::Local,
            payload: SkeletonDriveUpdatePayload::Dynamics
        })
    ));
    assert_eq!(services.events.borrow().len(), 13);
    assert_eq!(
        owner.bones[1].frames[0].read().unwrap().position_b[0].to_bits(),
        (-1.0_f32).to_bits()
    );
    assert_eq!(owner.bones[2].strengths.map(f32::to_bits), [0; 2]);
}
