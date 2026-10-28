use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;

#[test]
fn full_fill_preserves_source_bits_and_read_order() {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    let expected = [
        PumpingFillField::Absorption,
        PumpingFillField::GroundAbsorption,
        PumpingFillField::MinimumCrouch,
        PumpingFillField::DeckAbsorption,
        PumpingFillField::Acceleration,
    ];
    let bits = [
        0x8000_0000,
        0x7fc1_2345,
        0x3f80_0000,
        0xbf00_0000,
        0x4080_0000,
    ];
    let mut index = 0;
    out.try_fill_pumping(|field| {
        assert_eq!(field, expected[index]);
        let value = f32::from_bits(bits[index]);
        index += 1;
        Ok::<_, ()>(value)
    })
    .unwrap();
    assert_eq!(index, expected.len());
    let skeleton = out.concrete.skeleton.as_ref().unwrap();
    assert_eq!(skeleton.pumping_compression.to_bits(), 0);
    let actual = [
        skeleton.absorption_compression,
        skeleton.ground_normal_compression,
        skeleton.ground_normal_min_crouch,
        skeleton.deck_angle_compression,
        out.concrete.ground.as_ref().unwrap().pumping_acceleration,
    ];
    assert_eq!(actual.map(f32::to_bits), bits);
}
