use crate::physics_state_ids::{
    PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_LOCOMOTION, PHYSICS_STATE_OFFBOARD_PUSHING,
    PHYSICS_STATE_PHYSICS_GROUND, PHYSICS_STATE_SKITCHING,
};
use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
struct Source {
    state: i32,
    valid: bool,
    surface: Result<u32, &'static str>,
    calls: Vec<&'static str>,
}
impl Source {
    fn new(state: i32) -> Self {
        Self {
            state,
            valid: false,
            surface: Ok(0),
            calls: vec![],
        }
    }
}
impl Update5SurfaceCheckSource for Source {
    type Error = &'static str;
    type Processed = u32;
    fn capture_processed(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("capture");
        Ok(71)
    }
    fn physics_state(&mut self, owner: &u32) -> Result<i32, Self::Error> {
        assert_eq!(*owner, 71);
        self.calls.push("state8e4");
        Ok(self.state)
    }
    fn hips_valid(&mut self, owner: &u32) -> Result<bool, Self::Error> {
        assert_eq!(*owner, 71);
        self.calls.push("hips_valid");
        Ok(self.valid)
    }
    fn hips_surface_id(&mut self, owner: &u32) -> Result<u32, Self::Error> {
        assert_eq!(*owner, 71);
        self.calls.push("hips_id");
        self.surface
    }
}
fn output(null: Option<PhysOutComponent>) -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|component| {
        Ok::<_, ()>(if Some(component) == null {
            PhysOutAllocation::Null
        } else {
            PhysOutAllocation::Host
        })
    })
    .unwrap();
    out
}
#[test]
fn ground_uses_current_collision_type_and_preserves_other_requests() {
    for state in PHYSICS_STATE_PHYSICS_GROUND..=PHYSICS_STATE_SKITCHING {
        for (surface, teleport, wipeout) in [
            (6, true, false),
            (9, false, true),
            (1, false, false),
            (u32::MAX, false, false),
        ] {
            let mut out = output(None);
            out.try_publish_collision_type(|| Ok::<_, ()>(surface))
                .unwrap();
            let mut source = Source::new(state);
            out.try_do_update5_surface_check(&mut source).unwrap();
            let value = out.concrete.state.as_ref().unwrap();
            assert_eq!(
                (
                    value.physics_requested_teleport,
                    value.physics_requests_wipeout
                ),
                (teleport, wipeout)
            );
            assert_eq!(source.calls, ["capture", "state8e4"]);
            out.concrete
                .state
                .as_mut()
                .unwrap()
                .physics_requested_teleport = true;
            out.concrete
                .state
                .as_mut()
                .unwrap()
                .physics_requests_wipeout = true;
            out.try_do_update5_surface_check(&mut source).unwrap();
            let value = out.concrete.state.as_ref().unwrap();
            assert!(value.physics_requested_teleport && value.physics_requests_wipeout);
        }
    }
}
#[test]
fn both_offboard_feet_contribute_in_source_order() {
    for state in [PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_STATE_OFFBOARD_LOCOMOTION] {
        let mut out = output(None);
        let feet = out.concrete.offboard.as_mut().unwrap();
        feet.left_foot_down = true;
        feet.right_foot_down = true;
        feet.left_foot_surface_id = (6 << 7) | 0xa000_0005;
        feet.right_foot_surface_id = (9 << 7) | 0xb000_0002;
        out.try_do_update5_surface_check(&mut Source::new(state))
            .unwrap();
        let value = out.concrete.state.as_ref().unwrap();
        assert!(value.physics_requested_teleport && value.physics_requests_wipeout);
    }
}
#[test]
fn contact_and_footplant_paths_use_actual_output_flags() {
    let mut out = output(None);
    out.concrete
        .collision
        .as_mut()
        .unwrap()
        .skeleton
        .unrideable_contact = true;
    out.try_do_update5_surface_check(&mut Source::new(300))
        .unwrap();
    assert!(
        out.concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requested_teleport
    );
    let air = out.concrete.air.as_mut().unwrap();
    air.is_foot_planting = true;
    air.foot_plant_surface_id = 9 << 7;
    out.try_do_update5_surface_check(&mut Source::new(601))
        .unwrap();
    assert!(
        out.concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requests_wipeout
    );
}
#[test]
fn hips_reuses_entry_owner_and_does_not_read_invalid_surface_id() {
    let mut out = output(None);
    let mut source = Source::new(602);
    source.surface = Err("unwritten surface");
    out.try_do_update5_surface_check(&mut source).unwrap();
    assert_eq!(source.calls, ["capture", "state8e4", "hips_valid"]);
    source.calls.clear();
    source.valid = true;
    assert!(matches!(
        out.try_do_update5_surface_check(&mut source),
        Err(Update5SurfaceCheckError::Source("unwritten surface"))
    ));
    assert!(
        !out.concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requests_wipeout
    );
    source.calls.clear();
    source.surface = Ok(9 << 7);
    out.try_do_update5_surface_check(&mut source).unwrap();
    assert_eq!(
        source.calls,
        ["capture", "state8e4", "hips_valid", "hips_id"]
    );
    assert!(
        out.concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requests_wipeout
    );
}
#[test]
fn unhandled_states_do_not_touch_output_and_ground_requires_collision() {
    for state in [-1, 0, 99, 105, 200, 301, 499, 502, 503, 505, 600, 603] {
        let mut out = PhysicalPlayerPhysOut::new();
        out.try_do_update5_surface_check(&mut Source::new(state))
            .unwrap();
    }
    let mut out = output(Some(PhysOutComponent::Collision));
    assert!(matches!(
        out.try_do_update5_surface_check(&mut Source::new(100)),
        Err(Update5SurfaceCheckError::Output(_))
    ));
    assert!(
        !out.concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requested_teleport
    );
}
