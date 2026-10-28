use super::*;

fn names() -> (FastString36, FastString36) {
    (
        ActorGraphIntents::native_name(b"DownBoardAdjustMag"),
        ActorGraphIntents::native_name(b"DownBoardAdjustAngle"),
    )
}

#[test]
fn present_inputs_publish_magnitude_before_filtered_angle_and_reversal_state() {
    let (mag, angle) = names();
    let mut motion = ActorGraphIntents::try_new::<()>().unwrap();
    let mut instance = BoardAdjustInstance::new();
    instance.begin();
    publish_values(
        &mut motion,
        &mut instance,
        &mag,
        &angle,
        Some(2.0),
        Some(0.8),
        Filter::None,
        true,
        || Ok(false),
    )
    .unwrap();
    assert_eq!(motion.get(&mag), Some(0.8));
    assert_eq!(motion.get(&angle), Some(2.0));
    publish_values(
        &mut motion,
        &mut instance,
        &mag,
        &angle,
        Some(-2.0),
        Some(0.6),
        Filter::None,
        true,
        || Ok(false),
    )
    .unwrap();
    assert_eq!(motion.get(&mag), Some(0.6));
    assert_eq!(motion.get(&angle), Some(std::f32::consts::PI));
}

#[test]
fn missing_either_source_intent_erases_both_destinations_and_resets_state() {
    let (mag, angle) = names();
    let mut motion = ActorGraphIntents::try_new::<()>().unwrap();
    let mut instance = BoardAdjustInstance::new();
    instance.begin();
    publish_values(
        &mut motion,
        &mut instance,
        &mag,
        &angle,
        Some(2.0),
        Some(0.8),
        Filter::None,
        false,
        || unreachable!(),
    )
    .unwrap();
    publish_values(
        &mut motion,
        &mut instance,
        &mag,
        &angle,
        Some(-2.0),
        None,
        Filter::None,
        true,
        || unreachable!("missing path skips mirror"),
    )
    .unwrap();
    assert_eq!(motion.get(&mag), None);
    assert_eq!(motion.get(&angle), None);
    publish_values(
        &mut motion,
        &mut instance,
        &mag,
        &angle,
        Some(-2.0),
        Some(0.4),
        Filter::None,
        false,
        || unreachable!(),
    )
    .unwrap();
    assert_eq!(motion.get(&angle), Some(-2.0));
}

#[test]
fn mirror_service_failure_retains_the_native_magnitude_write_prefix() {
    let (mag, angle) = names();
    let mut motion = ActorGraphIntents::try_new::<()>().unwrap();
    let mut instance = BoardAdjustInstance::new();
    instance.begin();
    assert!(matches!(
        publish_values(
            &mut motion,
            &mut instance,
            &mag,
            &angle,
            Some(1.0),
            Some(0.7),
            Filter::None,
            true,
            || Err(E::UnknownAnimationFlags),
        ),
        Err(E::UnknownAnimationFlags)
    ));
    assert_eq!(motion.get(&mag), Some(0.7));
    assert_eq!(motion.get(&angle), None);
    publish_values(
        &mut motion,
        &mut instance,
        &mag,
        &angle,
        Some(1.0),
        Some(0.7),
        Filter::None,
        true,
        || Ok(true),
    )
    .unwrap();
    assert_eq!(motion.get(&angle), Some(-1.0));
}
