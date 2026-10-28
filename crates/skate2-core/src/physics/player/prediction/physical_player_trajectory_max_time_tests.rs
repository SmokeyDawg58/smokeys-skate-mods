#![expect(clippy::float_cmp, reason = "exact source cap selection")]
use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;

#[test]
fn cap_and_unordered_cap_follow_native_fsel() {
    let overlay = DebugOverlaySlot::default();
    let time = calculate(
        10.0,
        1.0,
        0.0,
        &mut HostReckoningNumeric,
        || Ok(false),
        &overlay,
    )
    .unwrap();
    assert!(time > 0.45 && time < 0.46);
    assert_eq!(
        calculate(
            0.1,
            1.0,
            0.0,
            &mut HostReckoningNumeric,
            || Ok(false),
            &overlay
        )
        .unwrap(),
        0.1
    );
    assert_eq!(
        calculate(
            f32::NAN,
            1.0,
            0.0,
            &mut HostReckoningNumeric,
            || Ok(false),
            &overlay
        )
        .unwrap()
        .to_bits(),
        time.to_bits()
    );
    assert_eq!(
        calculate(
            -0.0,
            1.0,
            0.0,
            &mut HostReckoningNumeric,
            || Ok(false),
            &overlay
        )
        .unwrap()
        .to_bits(),
        (-0.0_f32).to_bits()
    );
}

#[test]
fn invalid_discriminant_stops_before_numeric_or_cvar_reads() {
    for parameter in [0.0, -1.0, f32::NAN] {
        let mut numeric = RefuseEstimate;
        assert!(matches!(
            calculate(
                1.0,
                parameter,
                0.0,
                &mut numeric,
                || panic!("invalid discriminant cannot read cvar"),
                &DebugOverlaySlot::default()
            ),
            Err(MaximumTimeError::NonPositiveDiscriminant)
        ));
    }
}

struct RefuseEstimate;
impl ReckoningNumeric for RefuseEstimate {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("no dot")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        Err("estimate")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no reciprocal")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no conversion")
    }
}

#[test]
fn estimate_failure_precedes_live_cvar_and_overlay() {
    assert!(matches!(
        calculate(
            1.0,
            1.0,
            0.0,
            &mut RefuseEstimate,
            || panic!("estimate failure precedes cvar"),
            &DebugOverlaySlot::default()
        ),
        Err(MaximumTimeError::Numeric("estimate"))
    ));
}

#[test]
fn debug_cvar_is_live_and_only_enabled_value_requires_overlay() {
    let refs = InputCvarReferences::new_static_initialized();
    let overlay = DebugOverlaySlot::default();
    assert!(matches!(
        calculate(
            1.0,
            1.0,
            0.0,
            &mut HostReckoningNumeric,
            || refs.try_trajectory_debug_draw(),
            &overlay
        ),
        Err(MaximumTimeError::Cvar(_))
    ));
    let mut manager = crate::input_cvar_runtime::InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    calculate(
        1.0,
        1.0,
        0.0,
        &mut HostReckoningNumeric,
        || refs.try_trajectory_debug_draw(),
        &overlay,
    )
    .unwrap();
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkbDebugDraw=1\r\n"))
        .unwrap();
    assert!(matches!(
        calculate(
            1.0,
            1.0,
            0.0,
            &mut HostReckoningNumeric,
            || refs.try_trajectory_debug_draw(),
            &overlay
        ),
        Err(MaximumTimeError::Overlay(_))
    ));
}
