use super::*;
use std::cell::RefCell;

#[test]
fn live_maximum_precedes_flag_read_even_when_no_jump() {
    let calls = RefCell::new(Vec::new());
    let result = ground_jump(
        || {
            calls.borrow_mut().push("maximum");
            Ok(f32::NAN)
        },
        || {
            calls.borrow_mut().push("flags");
            Ok((0, 0x0040_0000))
        },
        |_| panic!("inactive branch must not calculate jump"),
    )
    .unwrap();
    assert_eq!(*calls.borrow(), ["maximum", "flags"]);
    assert!(!result.do_jump);
    assert_eq!(result.com_height.to_bits(), 0);
    assert_eq!(
        result.velocity.known_lanes().unwrap().map(f32::to_bits),
        [0; 4]
    );
}

#[test]
fn maximum_failure_does_not_sample_processed_flags() {
    let result = ground_jump(
        || Err(GroundAnimationUpdateError::Missing("maximum failure")),
        || panic!("native maximum failure precedes flag sampling"),
        |_| panic!("failed maximum must not calculate jump"),
    );
    assert!(matches!(
        result,
        Err(GroundAnimationUpdateError::Missing("maximum failure"))
    ));
}

#[test]
fn unknown_and_enabled_jump_never_return_a_publishable_no_jump_result() {
    assert!(matches!(
        ground_jump(|| Ok(3.0), || Ok((0, 0)), |_| panic!("unknown bit")),
        Err(GroundAnimationUpdateError::Missing(
            "Processed StartJump bit"
        ))
    ));
    assert!(matches!(
        ground_jump(
            || Ok(3.0),
            || Ok((0x0040_0000, 0x0040_0000)),
            |maximum| {
                assert_eq!(maximum.to_bits(), 3.0_f32.to_bits());
                Err(GroundAnimationUpdateError::Missing("active failure"))
            }
        ),
        Err(GroundAnimationUpdateError::Missing("active failure"))
    ));
}
