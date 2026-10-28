//! PC binding of normalized KickTurnSteering Spin into the animated root.
//! TU3 Toolkit_CalcHeadingTorque82D92CF8 resolves this control through the
//! authored SpinVsSpeed degree rate and ManualSpinBlendSpeed. Preview8 treated
//! the normalized input itself as radians/second, discarding that response.
use skate3_core::{player::input_phase::ProcessedPhysicsInput, riding::heading::HeadingSettings};

pub(super) fn advance(
    heading: [f32; 4],
    up: [f32; 4],
    kickturn: bool,
    p: &ProcessedPhysicsInput,
    settings: &HeadingSettings,
    balance: f32,
    previous_rate: &mut f32,
) -> [f32; 4] {
    if (!kickturn && balance == 0.0) || p.spin_input_2672 == 0.0 {
        return heading;
    }
    // Same native manual rate conversion and history blend. This is an explicit
    // PC animated-root repair; it is not a full replacement of Reckoning.
    let target = settings.manual_response.evaluate(p.scalar_2652) * f32::from_bits(0x3c8e_fa35);
    *previous_rate = (1.0 - settings.manual_damping)
        .mul_add(*previous_rate, target * settings.manual_damping);
    let wrong_wheels = (p.flags_2472 & 0x0800_0000 != 0 && balance < 0.0)
        || (p.flags_2472 & 0x0400_0000 != 0 && balance > 0.0);
    let spin = if wrong_wheels {p.spin_input_2672 * settings.manual_wrong_wheel_scalar}
        else {p.spin_input_2672};
    let angle = (p.timestep_2604 * *previous_rate) * spin;
    if balance != 0.0 {
        crate::diagnostics::test_log::event("skate3_manual_turn",format_args!(
            "kickturn={kickturn} balance={balance} spin={spin} angle={angle} rate={}",*previous_rate));
    }
    let (s, c) = skate3_core::trigonometry::sin_cos(angle);
    let dot = (heading[0] * up[0] + heading[1] * up[1]) + heading[2] * up[2];
    let cross = [
        up[1] * heading[2] - up[2] * heading[1],
        up[2] * heading[0] - up[0] * heading[2],
        up[0] * heading[1] - up[1] * heading[0],
    ];
    let mut result = heading;
    for i in 0..3 {
        result[i] = (heading[i] * c + cross[i] * s) + up[i] * (dot * (1.0 - c));
    }
    result
}
