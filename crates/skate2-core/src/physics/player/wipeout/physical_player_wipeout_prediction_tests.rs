use super::*;
use crate::simulation_assembly_backend::constraint_session::physical_grounded::wipeout::prediction::try_consume_wipeout_prediction;

#[test]
fn ready_marker_requires_all_lanes_and_checked_time_before_estimate_store() {
    let mut fields = WipeoutGroundFields::constructed();
    try_consume_wipeout_prediction::<()>(&mut fields).unwrap();
    assert_eq!(
        fields.estimated_time_to_collision.to_bits(),
        f32::MAX.to_bits()
    );
    fields.prediction.ready = true;
    assert!(try_consume_wipeout_prediction::<()>(&mut fields).is_err());
    fields.prediction.collision_time = Some([0.25, 0.25, 0.25, f32::NAN]);
    try_consume_wipeout_prediction::<()>(&mut fields).unwrap();
    assert_eq!(
        fields.estimated_time_to_collision.to_bits(),
        f32::MAX.to_bits()
    );
    fields.prediction.collision_time = Some([100.0; 4]);
    assert!(try_consume_wipeout_prediction::<()>(&mut fields).is_err());
    assert_eq!(
        fields.estimated_time_to_collision.to_bits(),
        f32::MAX.to_bits()
    );
    fields.prediction.collision_time = Some([0.25; 4]);
    try_consume_wipeout_prediction::<()>(&mut fields).unwrap();
    assert_eq!(
        fields.estimated_time_to_collision.to_bits(),
        0.25_f32.to_bits()
    );
    fields.prediction.collision_time = Some([-1.0; 4]);
    try_consume_wipeout_prediction::<()>(&mut fields).unwrap();
    assert_eq!(
        fields.estimated_time_to_collision.to_bits(),
        0.25_f32.to_bits()
    );
}
