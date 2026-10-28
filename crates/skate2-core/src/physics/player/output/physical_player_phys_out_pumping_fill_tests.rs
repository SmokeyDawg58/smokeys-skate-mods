use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}
#[test]
fn absent_pumping_read_occurs_after_initial_zero_publication() {
    let mut out = output();
    out.concrete.skeleton.as_mut().unwrap().pumping_compression = 3.0;
    assert!(matches!(
        out.try_fill_pumping(|_| Err::<f32, _>("missing owner")),
        Err(PumpingFillError::Source("missing owner"))
    ));
    assert_eq!(
        out.concrete
            .skeleton
            .as_ref()
            .unwrap()
            .pumping_compression
            .to_bits(),
        0
    );
}
#[test]
fn late_failure_retains_prior_compressions_and_omits_ground_write() {
    let mut out = output();
    let mut calls = Vec::new();
    assert!(matches!(
        out.try_fill_pumping(|field| {
            calls.push(field);
            if field == PumpingFillField::DeckAbsorption {
                Err("deck")
            } else {
                Ok(-0.0)
            }
        }),
        Err(PumpingFillError::Source("deck"))
    ));
    let skeleton = out.concrete.skeleton.as_ref().unwrap();
    assert_eq!(skeleton.absorption_compression.to_bits(), 0x8000_0000);
    assert_eq!(skeleton.ground_normal_compression.to_bits(), 0x8000_0000);
    assert_eq!(skeleton.ground_normal_min_crouch.to_bits(), 0x8000_0000);
    assert_eq!(
        calls,
        [
            PumpingFillField::Absorption,
            PumpingFillField::GroundAbsorption,
            PumpingFillField::MinimumCrouch,
            PumpingFillField::DeckAbsorption
        ]
    );
}
#[test]
fn locked_entry_does_not_read_pumping() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    assert!(matches!(
        out.try_fill_pumping(|_| -> Result<f32, ()> { panic!("source not reached") }),
        Err(PumpingFillError::Access(_))
    ));
}
