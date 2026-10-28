//! EasyBodySpins on S2's retained BodySpin history, TU3 Update82D8BF58.
//! The transient source view is written back to the same owner, not advanced
//! alongside a second spin history. Ground, resets and non-Easy remain S2.
use super::{
    BodySpin,
    air::{BodySpinAirAttributes, BodySpinAirCvars, BodySpinAirError},
};
use crate::skate3_native::physics::player::tuning::Tuning;
use skate3_core::air::body_spin::{self, BodySpinState};

impl BodySpin {
    fn source_words(&self) -> [u32; 44] {
        let mut words = [0; 44];
        words[..30].copy_from_slice(&self.derivative_history.map(f32::to_bits));
        words[30..42].copy_from_slice(
            &[
                self.input,
                self.input_derivative,
                self.input_derivative_filtered,
                self.last_input_smoothed,
                self.body_spin_target,
                self.body_spin_speed,
                self.body_spin_speed_prop,
                self.body_spin_speed_deriv,
                self.time_in_air,
                self.derivative_time,
                self.clamping_amount,
                self.auto_spin,
            ]
            .map(f32::to_bits),
        );
        words[42] = self.derivative_history_pos as u32;
        words
    }
    fn update_source_air(&mut self, input: f32, auto_spin: f32, tuning: &Tuning) {
        let mut source = BodySpinState::from_words(self.source_words());
        body_spin::update(
            &mut source,
            &tuning.air_assists.spin,
            input,
            auto_spin,
            true,
            1,
        );
        let words = source.words();
        for (value, word) in self.derivative_history.iter_mut().zip(&words[..30]) {
            *value = f32::from_bits(*word);
        }
        let [
            input,
            derivative,
            filtered,
            smoothed,
            target,
            speed,
            proportional,
            spin_derivative,
            time,
            derivative_time,
            clamping,
            auto,
        ] = std::array::from_fn::<_, 12, _>(|i| f32::from_bits(words[30 + i]));
        self.input = input;
        self.input_derivative = derivative;
        self.input_derivative_filtered = filtered;
        self.last_input_smoothed = smoothed;
        self.body_spin_target = target;
        self.body_spin_speed = speed;
        self.body_spin_speed_prop = proportional;
        self.body_spin_speed_deriv = spin_derivative;
        self.time_in_air = time;
        self.derivative_time = derivative_time;
        self.clamping_amount = clamping;
        self.auto_spin = auto;
        self.derivative_history_pos = words[42] as usize;
    }
    pub(crate) fn try_update_selected_air<A: BodySpinAirAttributes, C: BodySpinAirCvars>(
        &mut self,
        input: f32,
        auto_spin: f32,
        a: &mut A,
        c: &mut C,
        tuning: Option<&Tuning>,
    ) -> Result<(), BodySpinAirError<A::Error, C::Error>> {
        if let Some(tuning) = tuning.filter(|t| t.easy_body_spins) {
            self.update_source_air(input, auto_spin, tuning);
            Ok(())
        } else {
            self.try_update_air(input, auto_spin, a, c)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::gameplay::Difficulty;
    #[test]
    fn native_easy_spin_uses_source_curves_and_retained_history() {
        for mode in [
            Difficulty::Easy,
            Difficulty::EasyMotorized,
            Difficulty::Motorized,
        ] {
            let tuning = Tuning::select(mode).unwrap().unwrap();
            assert!(tuning.easy_body_spins);
            assert_eq!(tuning.perfect_body_flips, mode != Difficulty::Motorized);
            assert_eq!(tuning.auto_push_enabled, mode != Difficulty::Motorized);
            let mut actual = BodySpin::new();
            // Actual retained history at air entry, including a nonzero cursor.
            actual.derivative_history_pos = 17;
            actual.body_spin_target = -0.25;
            actual.input = -0.3;
            actual.derivative_history = std::array::from_fn(|i| (i as f32 * 0.17).sin());
            let mut expected = BodySpinState::from_words(actual.source_words());
            for tick in 0..240 {
                let input = if tick < 40 {
                    0.8
                } else if tick < 100 {
                    0.
                } else {
                    -0.6
                };
                let auto = if tick < 140 { 0. } else { -1.5 };
                actual.update_source_air(input, auto, &tuning);
                body_spin::update(
                    &mut expected,
                    &tuning.air_assists.spin,
                    input,
                    auto,
                    true,
                    1,
                );
                assert_eq!(
                    actual.source_words()[..43],
                    expected.words()[..43],
                    "{mode:?} tick={tick}"
                );
            }
            assert_eq!(actual.body_spin_target, -0.25);
        }
        for mode in [Difficulty::Normal, Difficulty::Hardcore] {
            let tuning = Tuning::select(mode).unwrap().unwrap();
            assert!(
                !tuning.easy_body_spins && !tuning.perfect_body_flips && !tuning.auto_push_enabled
            );
        }
        assert!(Tuning::select(Difficulty::Skate2Default).unwrap().is_none());
    }
}
