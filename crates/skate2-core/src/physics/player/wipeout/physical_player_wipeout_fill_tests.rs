use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;

fn output(missing: Option<PhysOutComponent>) -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|component| {
            Ok::<_, ()>(if Some(component) == missing {
                PhysOutAllocation::Null
            } else {
                PhysOutAllocation::Host
            })
        })
        .unwrap();
    output
}
#[test]
fn fill_preserves_bits_and_only_latches_requested_teleport() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.over = true;
    fields.time_over = -0.0;
    fields.estimated_time_to_collision = f32::from_bits(0x7fc1_2345);
    fields.time_until_teleport = -3.5;
    fields.auto_teleport_pending = true;
    fields.enough_time_for_reset = true;
    fields.request_teleport = true;
    let mut output = output(None);
    output.try_fill_wipeout_state(&fields).unwrap();
    let skeleton = output.concrete.skeleton.as_ref().unwrap();
    assert!(skeleton.is_wipeout_over && skeleton.is_wipeout_teleport_pending);
    assert_eq!(
        skeleton.time_in_wipeout_over.to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(
        skeleton.time_until_wipeout_teleport.to_bits(),
        (-3.5_f32).to_bits()
    );
    assert_eq!(
        output
            .concrete
            .animation
            .as_ref()
            .unwrap()
            .wipeout_estimated_time_to_land
            .to_bits(),
        0x7fc1_2345
    );
    assert!(output.concrete.state.as_ref().unwrap().can_leave_wipeout);
    fields.request_teleport = false;
    fields.prevent_respawn = true;
    output.try_fill_wipeout_state(&fields).unwrap();
    assert!(
        output
            .concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requested_teleport
    );
    assert!(!output.concrete.state.as_ref().unwrap().can_leave_wipeout);
    fields.prevent_respawn = false;
    fields.ignore_reset_button = true;
    output.try_fill_wipeout_state(&fields).unwrap();
    assert!(!output.concrete.state.as_ref().unwrap().can_leave_wipeout);
}
#[test]
fn missing_animation_preserves_prior_stores_and_skips_later_stores() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.over = true;
    fields.time_over = 2.5;
    fields.enough_time_for_reset = true;
    fields.time_until_teleport = 7.0;
    fields.auto_teleport_pending = true;
    fields.request_teleport = true;
    let mut output = output(Some(PhysOutComponent::Animation));
    assert!(matches!(
        output.try_fill_wipeout_state(&fields),
        Err(WipeoutFillError::Output(_))
    ));
    let skeleton = output.concrete.skeleton.as_ref().unwrap();
    assert!(skeleton.is_wipeout_over);
    assert_eq!(skeleton.time_in_wipeout_over.to_bits(), 2.5_f32.to_bits());
    assert_eq!(
        skeleton.time_until_wipeout_teleport.to_bits(),
        0.0_f32.to_bits()
    );
    assert!(!skeleton.is_wipeout_teleport_pending);
    assert!(output.concrete.state.as_ref().unwrap().can_leave_wipeout);
    assert!(
        !output
            .concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requested_teleport
    );
}
#[test]
fn locked_output_rejects_before_first_publication() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.over = true;
    let mut output = output(None);
    output.lock_after_update1().unwrap();
    assert!(matches!(
        output.try_fill_wipeout_state(&fields),
        Err(WipeoutFillError::Output(_))
    ));
    assert!(!output.concrete.skeleton.as_ref().unwrap().is_wipeout_over);
}
