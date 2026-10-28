//! Proposed child of existing post-record tests: reuse its numeric/owner fixture.
use super::*;

#[test]
fn late_flags_failure_retains_error_com_and_velocity_but_never_enters_hook() {
    let fixture = Fixture::new();
    let mut records = SkeletonPostPhysicsRecords::new();
    records.centre_of_mass = [10.0, 20.0, 30.0, 40.0];
    let mut services = Services::default();
    let result = records.try_update_with_flags(
        fixture.input().map_flags(|_| || Err("flags unavailable")),
        &mut services,
        &mut Numeric::default(),
        &mut HostReckoningSinCosNumeric,
    );
    assert_eq!(
        result,
        Err(SkeletonPostPhysicsRecordError::Service("flags unavailable"))
    );
    assert_eq!(
        records.centre_of_mass.map(f32::to_bits),
        [3.0_f32, 4.0, 6.0, 8.0].map(f32::to_bits)
    );
    assert_eq!(
        records.previous_centre_of_mass.map(f32::to_bits),
        [10.0_f32, 20.0, 30.0, 40.0].map(f32::to_bits)
    );
    assert_eq!(
        records.centre_of_mass_velocity.map(f32::to_bits),
        [-3.5_f32, -8.0, -12.0, -16.0].map(f32::to_bits)
    );
    assert_eq!(
        records.errors[23].xyz.map(f32::to_bits),
        [25.0_f32, 4.0, 6.0].map(f32::to_bits)
    );
    assert!(records.max_displacement.is_none());
    assert!(services.events.is_empty());
}

#[test]
fn earlier_numeric_failure_does_not_read_later_processed_flags() {
    let fixture = Fixture::new();
    let mut records = SkeletonPostPhysicsRecords::new();
    let mut services = Services::default();
    let mut numeric = Numeric {
        fail_at: Some(1),
        ..Numeric::default()
    };
    let mut read = false;
    let result = records.try_update_with_flags(
        fixture.input().map_flags(|_| {
            || {
                read = true;
                Ok(0)
            }
        }),
        &mut services,
        &mut numeric,
        &mut HostReckoningSinCosNumeric,
    );
    assert_eq!(
        result,
        Err(SkeletonPostPhysicsRecordError::Numeric("numeric"))
    );
    assert!(!read);
}
