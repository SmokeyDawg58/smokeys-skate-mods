use super::{Arc, OwnedAnimationSqtBuffer};
use skate2_formats::andale::AndaleSqt;

#[test]
fn extraction_and_sync_publish_new_generations_in_the_same_allocation() {
    let output = OwnedAnimationSqtBuffer::allocate_unknown(1);
    let alias = output.clone();
    assert!(Arc::ptr_eq(&output.storage, &alias.storage));
    assert!(output.is_unwritten().unwrap());
    assert!(output.read().is_err());
    output.begin_extract().unwrap();
    let mut pose = AndaleSqt {
        scale: [1.0; 4],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [2.0, 3.0, 4.0, 1.0],
    };
    output.extract(&[pose]).unwrap();
    assert!(alias.read().is_err());
    output.complete_sync().unwrap();
    let (generation, values) = alias.read().unwrap();
    assert_eq!(generation, 1);
    assert_eq!(
        values[0].translation.map(f32::to_bits),
        pose.translation.map(f32::to_bits)
    );
    output.begin_extract().unwrap();
    pose.translation[0] = 9.0;
    output.extract(&[pose]).unwrap();
    output.complete_sync().unwrap();
    let (generation, values) = alias.read().unwrap();
    assert_eq!(generation, 2);
    assert_eq!(values[0].translation[0].to_bits(), 9.0_f32.to_bits());
    assert!(Arc::ptr_eq(&output.storage, &alias.storage));
}
