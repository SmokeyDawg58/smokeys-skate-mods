//! BodySpin.Update82cc4600; retained per-occurrence instance, not Reckoning spin.
use crate::physics_state_ids::PHYSICS_STATE_LANDINGONDECK;
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BodySpinInstance {
    pub(crate) state: i32,
    pub(crate) last_input: f32,
    pub(crate) back_weight: f32,
    pub(crate) back_delta: f32,
    pub(crate) front_weight: f32,
    pub(crate) front_delta: f32,
}
impl BodySpinInstance {
    /// `CreateInstance82c9eb10:` source allocation20/alignment10; NULL remains None.
    pub(crate) fn try_create<E>(
        allocate: impl FnOnce(usize, usize) -> Result<bool, E>,
    ) -> Result<Option<Self>, E> {
        Ok(allocate(0x20, 0x10)?.then(Self::default))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpinChannel {
    Front,
    Back,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpinTuning {
    MaximumStep,
    MaximumDeltaChange,
    Blend,
    AlternateBlend,
}
pub(crate) trait BodySpinSource {
    type Error;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error>;
    fn enable_landing_on_board(&mut self) -> Result<bool, Self::Error>;
    fn physical_state(&mut self) -> Result<i32, Self::Error>;
    fn filtered_state(&mut self) -> Result<u32, Self::Error>;
    fn doing_trick(&mut self) -> Result<bool, Self::Error>;
    fn body_spin_intent(&mut self) -> Result<Option<f32>, Self::Error>;
    fn mirrored(&mut self) -> Result<bool, Self::Error>;
    fn input_graph(&mut self, magnitude: f32) -> Result<f32, Self::Error>;
    fn time_to_prepare_land(&mut self) -> Result<bool, Self::Error>;
    fn grab_busy(&mut self) -> Result<[i32; 2], Self::Error>;
    fn missing_instance(&mut self) -> Self::Error;
    fn tuning(&mut self, field: SpinTuning) -> Result<f32, Self::Error>;
    fn has_channel(&mut self, channel: SpinChannel) -> Result<bool, Self::Error>;
    fn new_channel(&mut self, channel: SpinChannel) -> Result<(), Self::Error>;
    fn end_channel(&mut self, channel: SpinChannel, length: f32) -> Result<(), Self::Error>;
    fn influence(&mut self, channel: SpinChannel, value: f32) -> Result<(), Self::Error>;
}

/// Begin/End vtable entries are verified `BLR825187c0`; no instance mutation.
#[expect(
    clippy::too_many_lines,
    reason = "original ordered state transitions and channel publications"
)]
pub(crate) fn update<S: BodySpinSource>(
    source: &mut S,
    instance: Option<&mut BodySpinInstance>,
) -> Result<(), S::Error> {
    source.resolve_interfaces()?;
    let landing_on_board = source.enable_landing_on_board()? && source.physical_state()? == PHYSICS_STATE_LANDINGONDECK;
    let airborne = source.filtered_state()? == 2 || landing_on_board;
    let wipeout = source.filtered_state()? == 3;
    let ground = source.filtered_state()? == 1;
    let spin_allowed = airborne || source.doing_trick()?;
    let input = source.body_spin_intent()?.unwrap_or(0.0);
    let input = if source.mirrored()? { -input } else { input };
    let weight = source.input_graph(input.abs())?;
    let weight = select(input, weight, -weight);
    let preparing = airborne && source.time_to_prepare_land()?;
    let grab = source.grab_busy()?;
    let grabbed = grab[0] != 0 || grab[1] != 0;
    let instance = instance.ok_or_else(|| source.missing_instance())?;
    let mut state = instance.state;
    match state {
        0 => {
            if wipeout {
                state = 4;
            }
            if spin_allowed {
                state = if grabbed { 4 } else { 1 };
            }
        }
        1 => {
            if !spin_allowed {
                state = 0;
            }
            if grabbed {
                state = 4;
            }
            if input.abs() > 0.3 {
                state = 2;
            }
        }
        2 => {
            if !spin_allowed {
                state = 0;
            }
            if grabbed {
                state = 4;
            }
            if airborne {
                state = 3;
            }
        }
        3 => {
            if grabbed || (preparing && landing_on_board) {
                state = 4;
            }
            if !airborne {
                state = 0;
            }
        }
        4 if !spin_allowed && ground => state = 0,
        _ => {}
    }
    // All four live scalar reads occur even when state0 merely ends channels.
    let max_step = source.tuning(SpinTuning::MaximumStep)?;
    let max_delta = source.tuning(SpinTuning::MaximumDeltaChange)?;
    let blend = source.tuning(SpinTuning::Blend)?;
    let alternate = source.tuning(SpinTuning::AlternateBlend)?;
    match state {
        0 | 4 => {
            source.end_channel(SpinChannel::Front, 0.2)?;
            source.end_channel(SpinChannel::Back, 0.2)?;
        }
        1 => {
            instance.front_weight = 0.0;
            instance.front_delta = 0.0;
            instance.back_weight = 0.0;
            instance.back_delta = 0.0;
        }
        2 | 3 => {
            if state == 2 {
                if input > 0.3 {
                    if !source.has_channel(SpinChannel::Front)? && !preparing {
                        source.new_channel(SpinChannel::Front)?;
                    }
                    source.end_channel(SpinChannel::Back, 0.15)?;
                }
                if input < -0.3 {
                    if !source.has_channel(SpinChannel::Back)? && !preparing {
                        source.new_channel(SpinChannel::Back)?;
                    }
                    source.end_channel(SpinChannel::Front, 0.15)?;
                }
            }
            if source.has_channel(SpinChannel::Front)? {
                let target = unit(weight);
                let rate = if target < instance.front_weight {
                    alternate
                } else {
                    blend
                };
                smooth(
                    &mut instance.front_weight,
                    &mut instance.front_delta,
                    target,
                    rate,
                    max_step,
                    max_delta,
                );
                source.influence(SpinChannel::Front, instance.front_weight)?;
            }
            if source.has_channel(SpinChannel::Back)? {
                let lower = select(-1.0 - weight, -1.0, weight);
                let negative = select(-lower, lower, 0.0);
                let target = -negative;
                // Native asymmetry: Back uses alternate when target is GREATER.
                let rate = if target > instance.back_weight {
                    alternate
                } else {
                    blend
                };
                smooth(
                    &mut instance.back_weight,
                    &mut instance.back_delta,
                    target,
                    rate,
                    max_step,
                    max_delta,
                );
                source.influence(SpinChannel::Back, instance.back_weight)?;
            }
        }
        _ => {}
    }
    instance.state = state;
    instance.last_input = input;
    Ok(())
}
fn select(test: f32, nonnegative: f32, other: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { other }
}
fn unit(value: f32) -> f32 {
    let low = select(-value, 0.0, value);
    select(1.0 - low, low, 1.0)
}
fn limited(value: f32, old: f32, maximum: f32) -> f32 {
    let low = old - maximum;
    let high = old + maximum;
    let value = select(low - value, low, value);
    select(high - value, value, high)
}
fn smooth(value: &mut f32, delta: &mut f32, target: f32, rate: f32, max_step: f32, max_delta: f32) {
    let old = *value;
    let desired = (1.0 - rate).mul_add(old, rate * target);
    *delta = limited(desired - old, *delta, max_delta);
    *value = unit(limited(*delta + old, old, max_step));
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "exact authored branch and retained scalar expectations"
)]
mod tests {
    use super::*;
    struct Source {
        events: Vec<String>,
        input: Option<f32>,
        state: u32,
        channels: [bool; 2],
        fail_end: bool,
    }
    impl Source {
        fn new() -> Self {
            Self {
                events: Vec::new(),
                input: None,
                state: 1,
                channels: [false; 2],
                fail_end: false,
            }
        }
        fn log(&mut self, text: impl Into<String>) {
            self.events.push(text.into());
        }
    }
    fn index(c: SpinChannel) -> usize {
        usize::from(c != SpinChannel::Front)
    }
    impl BodySpinSource for Source {
        type Error = &'static str;
        fn resolve_interfaces(&mut self) -> Result<(), Self::Error> {
            self.log("interfaces");
            Ok(())
        }
        fn enable_landing_on_board(&mut self) -> Result<bool, Self::Error> {
            self.log("cvar");
            Ok(false)
        }
        fn physical_state(&mut self) -> Result<i32, Self::Error> {
            panic!("disabled cvar must skip state")
        }
        fn filtered_state(&mut self) -> Result<u32, Self::Error> {
            self.log("filtered");
            Ok(self.state)
        }
        fn doing_trick(&mut self) -> Result<bool, Self::Error> {
            self.log("doing");
            Ok(false)
        }
        fn body_spin_intent(&mut self) -> Result<Option<f32>, Self::Error> {
            self.log("intent");
            Ok(self.input)
        }
        fn mirrored(&mut self) -> Result<bool, Self::Error> {
            self.log("mirror");
            Ok(false)
        }
        fn input_graph(&mut self, value: f32) -> Result<f32, Self::Error> {
            self.log("graph");
            Ok(value)
        }
        fn time_to_prepare_land(&mut self) -> Result<bool, Self::Error> {
            self.log("prepare");
            Ok(false)
        }
        fn grab_busy(&mut self) -> Result<[i32; 2], Self::Error> {
            self.log("grab");
            Ok([0; 2])
        }
        fn missing_instance(&mut self) -> Self::Error {
            "missing instance"
        }
        fn tuning(&mut self, field: SpinTuning) -> Result<f32, Self::Error> {
            self.log(format!("tuning:{field:?}"));
            Ok(1.0)
        }
        fn has_channel(&mut self, c: SpinChannel) -> Result<bool, Self::Error> {
            self.log(format!("has:{c:?}"));
            Ok(self.channels[index(c)])
        }
        fn new_channel(&mut self, c: SpinChannel) -> Result<(), Self::Error> {
            self.log(format!("new:{c:?}"));
            self.channels[index(c)] = true;
            Ok(())
        }
        fn end_channel(&mut self, c: SpinChannel, length: f32) -> Result<(), Self::Error> {
            self.log(format!("end:{c:?}:{length}"));
            if self.fail_end && c == SpinChannel::Back {
                Err("back end")
            } else {
                Ok(())
            }
        }
        fn influence(&mut self, c: SpinChannel, _: f32) -> Result<(), Self::Error> {
            self.log(format!("influence:{c:?}"));
            Ok(())
        }
    }
    #[test]
    fn neutral_runs_common_path_and_both_channel_ends() {
        let mut source = Source::new();
        let mut data = BodySpinInstance::default();
        update(&mut source, Some(&mut data)).unwrap();
        assert_eq!(
            source.events,
            [
                "interfaces",
                "cvar",
                "filtered",
                "filtered",
                "filtered",
                "doing",
                "intent",
                "mirror",
                "graph",
                "grab",
                "tuning:MaximumStep",
                "tuning:MaximumDeltaChange",
                "tuning:Blend",
                "tuning:AlternateBlend",
                "end:Front:0.2",
                "end:Back:0.2"
            ]
        );
        assert_eq!(data, BodySpinInstance::default());
        assert_eq!(
            BodySpinInstance::try_create::<()>(|bytes, align| {
                assert_eq!((bytes, align), (0x20, 0x10));
                Ok(false)
            })
            .unwrap(),
            None
        );
    }
    #[test]
    fn second_channel_failure_preserves_old_instance_tail() {
        let mut source = Source::new();
        source.fail_end = true;
        let mut data = BodySpinInstance {
            state: 4,
            last_input: 7.0,
            ..Default::default()
        };
        assert_eq!(update(&mut source, Some(&mut data)), Err("back end"));
        assert_eq!(data.state, 4);
        assert_eq!(data.last_input, 7.0);
        assert!(source.events.iter().any(|s| s == "end:Front:0.2"));
    }
    #[test]
    fn state_one_threshold_overrides_exit_and_updates_channel_weight() {
        let mut source = Source::new();
        source.input = Some(0.5);
        let mut data = BodySpinInstance {
            state: 1,
            ..Default::default()
        };
        update(&mut source, Some(&mut data)).unwrap();
        assert_eq!(data.state, 2);
        assert_eq!(data.front_weight, 0.5);
        assert!(source.events.iter().any(|s| s == "new:Front"));
        assert!(source.events.iter().any(|s| s == "end:Back:0.15"));
        assert_eq!(unit(f32::NAN), 1.0);
        assert_eq!(unit(-0.0).to_bits(), 0);
    }
}
