//! Skater::CalcAction82c8ae10: classify each existing locomotion stage.
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, sine::try_sine},
};

#[derive(Clone, Copy, Debug)]
pub(crate) enum ActionThreshold {
    LandTwoFeet,
    InclineDistance,
    StairsDistance,
    InclineMaxDown,
    InclineMaxUp,
    JumpStairsDistance,
    LandOneFoot,
}
trait Tuning {
    type Error;
    /// Capture all four targets before reading angle, stairs, incline, jump.
    fn initial(&mut self) -> Result<[f32; 4], Self::Error>;
    fn read(&mut self, threshold: ActionThreshold) -> Result<f32, Self::Error>;
}
impl Tuning for &InputCvarReferences {
    type Error = InputCvarError;
    fn initial(&mut self) -> Result<[f32; 4], Self::Error> {
        self.try_loco_action_initial_thresholds()
    }
    fn read(&mut self, threshold: ActionThreshold) -> Result<f32, Self::Error> {
        self.try_loco_action_threshold(threshold)
    }
}
#[derive(Clone, Copy)]
struct Input {
    state: i32,
    previous_y: f32,
    previous_length: f32,
    next_y: f32,
    next_length: f32,
    previous_normal: f32,
    next_normal: f32,
    vertical_speed: f32,
}

/// The native formal order is next-normal then previous-normal; this API names
/// and orders them as previous/next. Forward speed is genuinely unused native.
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_calc_locomotion_action(
    output: &mut i32,
    state: i32,
    previous_y: f32,
    previous_length: f32,
    next_y: f32,
    next_length: f32,
    previous_normal_projection: f32,
    next_normal_projection: f32,
    vertical_speed: f32,
    _forward_speed: f32,
    cvars: &InputCvarReferences,
) -> Result<i32, InputCvarError> {
    calc_action(
        output,
        Input {
            state,
            previous_y,
            previous_length,
            next_y,
            next_length,
            previous_normal: previous_normal_projection,
            next_normal: next_normal_projection,
            vertical_speed,
        },
        &mut &*cvars,
    )
}

fn publish(output: &mut i32, action: i32) -> i32 {
    *output = action;
    action
}

#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn calc_action<T: Tuning>(output: &mut i32, input: Input, tuning: &mut T) -> Result<i32, T::Error> {
    use ActionThreshold as C;
    let next_length = input.next_length.abs();
    let previous_length = input.previous_length.abs();
    let epsilon = f32::from_bits(0x38d1_b717);
    // Both native BLE branches test GT-clear, including unordered.
    let next_slope = if next_length > epsilon {
        input.next_y / next_length
    } else {
        0.0
    };
    let previous_slope = if previous_length > epsilon {
        input.previous_y / previous_length
    } else {
        0.0
    };
    let slope = if input.state == 0 {
        previous_slope
    } else {
        next_slope
    };
    let normal = if input.state == 0 {
        input.previous_normal
    } else {
        input.next_normal
    };
    let previous = *output;
    let [mut angle, mut stairs, mut incline, mut jump] = tuning.initial()?;
    // Native FMUL is double followed by FRSP only for the 1.15 multiplier.
    match previous {
        3 => jump *= f32::from_bits(0x3f59_999a),
        4 | 5 | 7 => {
            jump = (f64::from(jump) * f64::from_bits(0x3ff2_6666_6666_6666)) as f32;
            stairs *= f32::from_bits(0x3f66_6666);
        }
        8 => stairs *= f32::from_bits(0x3f66_6666),
        9 => {
            jump = (f64::from(jump) * f64::from_bits(0x3ff2_6666_6666_6666)) as f32;
            angle *= f32::from_bits(0x3f59_999a);
            incline *= f32::from_bits(0x3f59_999a);
        }
        10 => {
            angle *= f32::from_bits(0x3f59_999a);
            incline *= f32::from_bits(0x3f59_999a);
        }
        _ => {}
    }
    let sine = try_sine(
        angle * f32::from_bits(0x3c8e_fa35),
        &mut HostReckoningSinCosNumeric,
    )
    .unwrap_or_else(|never| match never {});
    let land_two = tuning.read(C::LandTwoFeet)?;
    if input.vertical_speed < -land_two && input.state == 0 {
        return Ok(publish(output, 4));
    }
    // Each read precedes even the state-based shortcut in the original.
    let can_incline = previous_length < tuning.read(C::InclineDistance)? || input.state != 0;
    let can_stairs = previous_length < tuning.read(C::StairsDistance)? || input.state != 0;
    let can_descend = input.next_y > -jump || input.state == 0;
    if normal > sine
        && slope > -tuning.read(C::InclineMaxDown)?
        && can_descend
        && slope < -incline
        && can_incline
    {
        return Ok(publish(output, 9));
    }
    if slope < -stairs && can_stairs && can_descend {
        return Ok(publish(output, 7));
    }
    if normal < -sine
        && slope < tuning.read(C::InclineMaxUp)?
        && slope > incline
        && can_incline
        && (previous != 4 || input.state != 0)
    {
        return Ok(publish(output, 10));
    }
    if slope > stairs
        && can_stairs
        && (previous != 1 || previous_length > tuning.read(C::JumpStairsDistance)?)
        && (previous != 4 || input.state != 0)
    {
        return Ok(publish(output, 8));
    }
    if input.vertical_speed < -land_two {
        return Ok(publish(output, 4));
    }
    // BGE means LT-clear; unordered does not enter the one-foot branch.
    if input.vertical_speed < -tuning.read(C::LandOneFoot)? && (input.state == 0 || previous == -1)
    {
        *output = -1;
        return Ok(5);
    }
    // Native BGT skips AutoJump only for ordered greater-than.
    if !(input.next_y > -jump) {
        return Ok(publish(output, 3));
    }
    if previous == 4 {
        *output = if input.state == 2 { -1 } else { 4 };
        return Ok(6);
    }
    *output = 0;
    Ok(if previous == -1 || previous == 1 {
        0
    } else {
        previous
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Values {
        calls: Vec<ActionThreshold>,
        fail: Option<ActionThreshold>,
    }
    impl Tuning for Values {
        type Error = ();
        fn initial(&mut self) -> Result<[f32; 4], ()> {
            Ok([4.5, 0.15, 0.07, 0.7])
        }
        fn read(&mut self, c: ActionThreshold) -> Result<f32, ()> {
            self.calls.push(c);
            if self
                .fail
                .is_some_and(|v| std::mem::discriminant(&v) == std::mem::discriminant(&c))
            {
                return Err(());
            }
            Ok(match c {
                ActionThreshold::LandTwoFeet => 4.5,
                ActionThreshold::InclineDistance => 2.3,
                ActionThreshold::StairsDistance => 1.7,
                ActionThreshold::InclineMaxDown => 0.3,
                ActionThreshold::InclineMaxUp => 0.5,
                ActionThreshold::JumpStairsDistance => 1.2,
                ActionThreshold::LandOneFoot => 2.85,
            })
        }
    }
    fn input() -> Input {
        Input {
            state: 0,
            previous_y: 0.0,
            previous_length: 1.0,
            next_y: 0.0,
            next_length: 1.0,
            previous_normal: 0.0,
            next_normal: 0.0,
            vertical_speed: 0.0,
        }
    }
    fn tuning() -> Values {
        Values {
            calls: Vec::new(),
            fail: None,
        }
    }
    #[test]
    fn hard_contact_landing_skips_all_later_tuning() {
        let mut output = -1;
        let mut t = tuning();
        t.fail = Some(ActionThreshold::InclineDistance);
        assert_eq!(
            calc_action(
                &mut output,
                Input {
                    vertical_speed: -5.0,
                    ..input()
                },
                &mut t
            ),
            Ok(4)
        );
        assert_eq!(output, 4);
        assert_eq!(t.calls.len(), 1);
    }
    #[test]
    fn land_out_returns_different_action_and_preserves_store_until_air() {
        let mut output = 4;
        assert_eq!(calc_action(&mut output, input(), &mut tuning()), Ok(6));
        assert_eq!(output, 4);
        assert_eq!(
            calc_action(
                &mut output,
                Input {
                    state: 2,
                    ..input()
                },
                &mut tuning()
            ),
            Ok(6)
        );
        assert_eq!(output, -1);
    }
    #[test]
    fn lazy_failure_keeps_prior_action_and_nan_drop_takes_native_autojump() {
        let mut output = 10;
        let mut t = tuning();
        t.fail = Some(ActionThreshold::InclineMaxDown);
        assert_eq!(
            calc_action(
                &mut output,
                Input {
                    previous_normal: 1.0,
                    ..input()
                },
                &mut t
            ),
            Err(())
        );
        assert_eq!(output, 10);
        assert_eq!(
            calc_action(
                &mut output,
                Input {
                    next_y: f32::NAN,
                    ..input()
                },
                &mut tuning()
            ),
            Ok(3)
        );
        assert_eq!(output, 3);
    }
    #[test]
    fn previous_action_returns_once_while_retained_store_becomes_run() {
        let mut output = 7;
        assert_eq!(calc_action(&mut output, input(), &mut tuning()), Ok(7));
        assert_eq!(output, 0);
        assert_eq!(calc_action(&mut output, input(), &mut tuning()), Ok(0));
    }
}
