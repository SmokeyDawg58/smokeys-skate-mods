//! Scoped suffix ordering; these services are not a World query owner.
use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skeleton_animation_targets::SkeletonAnimationTargets,
    skeleton_ik_prephysics::{SkeletonIkPrePhysicsAttributes, SkeletonIkPrePhysicsServices},
    skeleton_ik_state::{
        contact::{
            SkeletonIkContactLineQuery, SkeletonIkContactLineResult, SkeletonIkContactProcessed,
            SkeletonIkContactServices,
        },
        drive_frames::SkeletonIkDriveFramesServices,
    },
};
use std::{cell::RefCell, rc::Rc};

struct Services {
    events: Rc<RefCell<Vec<&'static str>>>,
    gate: Result<bool, &'static str>,
    contact_fail: bool,
    seed_contact: bool,
    prephysics: bool,
}
impl SkeletonIkSuffixServices for Services {
    fn enable_contact_offset(&mut self) -> Result<bool, &'static str> {
        self.events.borrow_mut().push("gate");
        self.gate
    }
}
impl SkeletonIkContactServices for Services {
    type Error = &'static str;
    fn draw_ik_contact_debug(&mut self) -> Result<bool, Self::Error> {
        self.events.borrow_mut().push("contact");
        if self.contact_fail {
            Err("contact")
        } else {
            Ok(false)
        }
    }
    fn max_ik_contact_penetration_offset(&mut self) -> Result<f32, Self::Error> {
        panic!("no matching foot")
    }
    fn sideways_offset(&mut self) -> Result<f32, Self::Error> {
        assert!(self.seed_contact);
        Ok(0.0)
    }
    fn sideways_offset_draw(&mut self) -> Result<bool, Self::Error> {
        assert!(self.seed_contact);
        Ok(false)
    }
    fn max_ik_contact_offset(&mut self) -> Result<f32, Self::Error> {
        assert!(self.seed_contact);
        Ok(1.0)
    }
    fn toe_contact_threshold(&mut self) -> Result<f32, Self::Error> {
        panic!("no toe threshold")
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("debug disabled")
    }
    fn try_line_query(
        &mut self,
        _: &SkeletonIkContactLineQuery,
    ) -> Result<SkeletonIkContactLineResult, Self::Error> {
        assert!(self.seed_contact);
        Ok(SkeletonIkContactLineResult::Hit {
            position: SkeletonRootVector::known([0.0, -0.2, 0.0, 0.0]),
        })
    }
}
impl SkeletonIkPrePhysicsServices for Services {
    type Error = &'static str;
    fn draw_pre_feet_ik(&mut self) -> Result<bool, Self::Error> {
        panic!("no active modes")
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("debug disabled")
    }
}
impl SkeletonIkDriveFramesServices for Services {
    fn enable_prephysics(&mut self) -> Result<bool, Self::Error> {
        self.events.borrow_mut().push("drive");
        Ok(self.prephysics)
    }
    fn maximum_leg_length(&mut self) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("leg");
        Ok(1.0)
    }
    fn maximum_arm_length(&mut self) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("arm");
        Err("arm")
    }
}
struct Processed(Rc<RefCell<Vec<&'static str>>>);
impl SkeletonIkContactProcessed for Processed {
    fn flags_8c4(&self) -> u32 {
        self.0.borrow_mut().push("flags");
        0
    }
    fn contact_bone_id(&self) -> Option<usize> {
        panic!("no active modes")
    }
    fn effective_transform(&self) -> &SkeletonRootMatrix {
        panic!("no matching foot")
    }
}
struct Attributes;
struct SeedProcessed {
    bone: Option<usize>,
    effective: SkeletonRootMatrix,
}
impl SkeletonIkContactProcessed for SeedProcessed {
    fn flags_8c4(&self) -> u32 {
        0x0200_0000
    }
    fn contact_bone_id(&self) -> Option<usize> {
        self.bone
    }
    fn effective_transform(&self) -> &SkeletonRootMatrix {
        &self.effective
    }
}
impl SkeletonIkPrePhysicsAttributes for Attributes {
    type Error = &'static str;
    fn minimum_angle_degrees(&mut self) -> Result<f32, Self::Error> {
        panic!("no solver entered")
    }
    fn maximum_angle_degrees(&mut self) -> Result<f32, Self::Error> {
        panic!("no solver entered")
    }
}

#[test]
fn disabled_contact_preserves_nonzero_histories_and_feedback_and_drive_disable_succeeds() {
    let (hierarchy, parameters, pose) = crate::skeleton_data_tests::fixture();
    let data = Arc::new(crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap());
    let mut targets = SkeletonIkTargets::new(data);
    targets.initialize();
    let vault = Arc::new(crate::attribute_runtime::tests::graph(
        0xc8be_3349_f2bd_493e,
        vec![],
    ));
    let mut ik = SkeletonIk::new(&vault, &mut targets);
    ik.state.update_status(0);
    let identity = SkeletonRootMatrix::known([
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);
    ik.data_in.inverse_skateboard = identity;
    ik.data_in.skateboard = identity;
    let events = Rc::new(RefCell::new(vec![]));
    let mut services = Services {
        events: events.clone(),
        gate: Ok(true),
        contact_fail: false,
        seed_contact: true,
        prephysics: false,
    };
    let processed = SeedProcessed {
        bone: targets.bone_ids()[0],
        effective: identity,
    };
    // Produce history through Contact itself: first query commits the hit;
    // the next call applies a negative offset and requests push cancellation.
    for _ in 0..2 {
        ik.try_update_suffix(
            &targets,
            &processed,
            &mut SkeletonAnimationTargets::new(),
            &mut Attributes,
            &mut services,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap();
    }
    assert!(ik.state.contact_offsets()[0] < 0.0);
    assert!(ik.state.skateboard_dont_push());
    assert!(ik.state.tell_anim_dont_push_or_brake());
    let offsets = *ik.state.contact_offsets();
    let targets_before = *ik.state.contact_target_offsets();
    let positions = *ik.state.contact_positions();
    let states = *ik.state.contact_states();
    events.borrow_mut().clear();
    services.gate = Ok(false);
    services.seed_contact = false;
    ik.try_update_suffix(
        &targets,
        &Processed(events.clone()),
        &mut SkeletonAnimationTargets::new(),
        &mut Attributes,
        &mut services,
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
    )
    .unwrap();
    assert_eq!(*events.borrow(), ["gate", "drive"]);
    assert_eq!(
        ik.state.contact_offsets().map(f32::to_bits),
        offsets.map(f32::to_bits)
    );
    assert_eq!(
        ik.state.contact_target_offsets().map(f32::to_bits),
        targets_before.map(f32::to_bits)
    );
    assert_eq!(*ik.state.contact_positions(), positions);
    assert_eq!(*ik.state.contact_states(), states);
    assert!(ik.state.skateboard_dont_push());
    assert!(ik.state.tell_anim_dont_push_or_brake());
}

#[test]
fn suffix_stops_at_gate_contact_or_drive_failure_without_preflighting_rows() {
    for (gate, contact_fail, expected) in [
        (Err("gate"), false, vec!["gate"]),
        (Ok(false), false, vec!["gate", "drive", "leg", "arm"]),
        (Ok(true), true, vec!["gate", "flags", "contact"]),
        (
            Ok(true),
            false,
            vec![
                "gate", "flags", "contact", "contact", "contact", "contact", "drive", "leg", "arm",
            ],
        ),
    ] {
        let (hierarchy, parameters, pose) = crate::skeleton_data_tests::fixture();
        let data =
            Arc::new(crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap());
        let mut targets = SkeletonIkTargets::new(data);
        targets.initialize();
        let vault = Arc::new(crate::attribute_runtime::tests::graph(
            0xc8be_3349_f2bd_493e,
            vec![],
        ));
        let mut ik = SkeletonIk::new(&vault, &mut targets);
        let events = Rc::new(RefCell::new(vec![]));
        let mut services = Services {
            events: events.clone(),
            gate,
            contact_fail,
            seed_contact: false,
            prephysics: true,
        };
        let result = ik.try_update_suffix(
            &targets,
            &Processed(events.clone()),
            &mut SkeletonAnimationTargets::new(),
            &mut Attributes,
            &mut services,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        );
        match (gate, contact_fail, result) {
            (Err(_), _, Err(SkeletonIkSuffixError::EnableContact("gate")))
            | (
                Ok(true),
                true,
                Err(SkeletonIkSuffixError::Contact(
                    crate::skeleton_ik_state::contact::SkeletonIkContactError::Service("contact"),
                )),
            )
            | (
                Ok(_),
                false,
                Err(SkeletonIkSuffixError::DriveFrames(
                    crate::skeleton_ik_state::drive_frames::SkeletonIkDriveFramesError::Service(
                        "arm",
                    ),
                )),
            ) => {}
            other => panic!("unexpected stage: {other:?}"),
        }
        assert_eq!(*events.borrow(), expected);
        assert_eq!(ik.state.contact_states(), &[0; 4]);
    }
}
