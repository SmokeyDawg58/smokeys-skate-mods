use super::*;

#[test]
fn lean_uses_prior_mirror_flag_and_preserves_unordered_choice() {
    let mut subject = CameraSubject::try_construct().unwrap();
    subject.input_turn = -0.25;
    subject.input_kickturn = -0.5;
    subject.animation_input_lean = -0.75;
    assert_eq!(
        animation_input_lean(&subject, false).to_bits(),
        (-0.75_f32).to_bits()
    );
    assert_eq!(
        animation_input_lean(&subject, true).to_bits(),
        0.75_f32.to_bits()
    );
    subject.input_turn = f32::from_bits(0xffc0_0042);
    subject.input_kickturn = 1.0;
    subject.animation_input_lean = -0.25;
    assert_eq!(animation_input_lean(&subject, false).to_bits(), 0xffc0_0042);
    subject.animation_input_lean = 0.25;
    assert_eq!(
        animation_input_lean(&subject, false).to_bits(),
        0.25_f32.to_bits()
    );
}

#[test]
fn equal_magnitude_preserves_turn_and_signed_zero_survives() {
    let mut subject = CameraSubject::try_construct().unwrap();
    subject.input_turn = -0.0;
    subject.input_kickturn = 0.0;
    subject.animation_input_lean = 0.0;
    assert_eq!(
        animation_input_lean(&subject, false).to_bits(),
        (-0.0_f32).to_bits()
    );
    subject.input_turn = 0.5;
    subject.input_kickturn = -0.5;
    subject.animation_input_lean = 0.25;
    assert_eq!(
        animation_input_lean(&subject, false).to_bits(),
        0.5_f32.to_bits()
    );
}
