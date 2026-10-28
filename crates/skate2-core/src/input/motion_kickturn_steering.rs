//! `KickTurnSteering`: Create `0x82ca11b8`, Begin `0x82c95428`,
//! Update `0x82cc5860`, and the shared BLR End `0x825187c0`.
//!
//! The source owns callback order. The Actor adapter supplies the retained
//! animation, Motion interfaces, `anim_motion_kickturn` graphs and live cvars.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct KickTurnSteeringInstance {
    /// Native instance +0x08, captured from the current base animation.
    pub(crate) animation_length: f32,
    /// Native instance +0x0c. The first Update does not advance it.
    pub(crate) elapsed: f32,
    /// Native instance +0x10.
    pub(crate) first_update: bool,
    /// Native instance +0x14.
    pub(crate) balance: f32,
    /// Native instance +0x18, retained when the current intent is zero.
    pub(crate) last_input: f32,
}

impl KickTurnSteeringInstance {
    /// The native allocation is 0x1c bytes with alignment 0x10. Failure leaves
    /// the intrusive instance null; there is no behavior-specific fallback.
    pub(crate) fn try_create<E>(
        mut allocate: impl FnMut(usize, usize) -> Result<bool, E>,
    ) -> Result<Option<Self>, E> {
        Ok(allocate(0x1c, 0x10)?.then_some(Self {
            animation_length: 0.0,
            elapsed: 0.0,
            first_update: true,
            balance: 0.0,
            last_input: 0.0,
        }))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KickTurnGraph {
    /// `anim_motion_kickturn` layout +0xf0, x +0x100, y +0x120.
    Balance,
    /// `anim_motion_kickturn` layout +0xa0, x +0xb0, y +0xd0.
    Spin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KickTurnAttribute {
    Balance,
    Spin,
}

pub(crate) trait KickTurnSteeringSource {
    type Error;

    /// Verify IAnimatable, IMotionGraph, ISkaterMotionGraph, in that order.
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error>;
    fn missing_instance(&mut self) -> Self::Error;
    /// `SkaterAnim::SetAttributes` via IAnimatable vtable +0x4c.
    fn set_animation_attributes(&mut self) -> Result<(), Self::Error>;
    /// `GetBaseAnim` +0x2c, then the base animation's `GetLength` +0x14.
    fn base_animation_length(&mut self) -> Result<f32, Self::Error>;
    /// Native asserts when length is not ordered greater than +0.0.
    fn invalid_animation_length(&mut self, length: f32) -> Self::Error;
    /// The already-held generated controller's `GetDeltaTime` value.
    fn delta_time(&mut self) -> Result<f32, Self::Error>;
    /// `PointGraphEval::Evaluate` with eight authored points.
    fn graph(&mut self, graph: KickTurnGraph, x: f32) -> Result<f32, Self::Error>;
    /// Live `kKickTurnSmoothing`, initialized to 0.5.
    fn smoothing(&mut self) -> Result<f32, Self::Error>;
    /// `IMotionGraph::GetIntents()->HasIntent(KickTurn)`; absent means zero.
    fn kickturn_intent(&mut self) -> Result<Option<f32>, Self::Error>;
    /// Live `kUseFineKickturn`, initialized to false.
    fn use_fine_kickturn(&mut self) -> Result<bool, Self::Error>;
    /// `ISkaterMotionGraph::AddMgAttribute`; appends without replacing.
    fn add_motion_attribute(
        &mut self,
        attribute: KickTurnAttribute,
        value: f32,
    ) -> Result<(), Self::Error>;
}

/// PPC `fsel`: use the first arm for an ordered nonnegative test. Keeping the
/// two source selects separate preserves signed zero and unordered branches.
fn select(test: f32, yes: f32, no: f32) -> f32 {
    if test >= 0.0 { yes } else { no }
}

fn clamp_phase(ratio: f32) -> f32 {
    let lower = select(-ratio, 0.0, ratio);
    select(1.0 - lower, lower, 1.0)
}

fn clamp_balance(value: f32, maximum: f32) -> f32 {
    let lower = select(-value, 0.0, value);
    select(maximum - lower, lower, maximum)
}

/// Begin writes instance fields before its one live graph evaluation. A graph
/// failure leaves the already-written fields in their native partial state.
pub(crate) fn begin<S: KickTurnSteeringSource>(
    source: &mut S,
    instance: Option<&mut KickTurnSteeringInstance>,
) -> Result<(), S::Error> {
    let data = instance.ok_or_else(|| source.missing_instance())?;
    data.first_update = true;
    data.animation_length = 0.0;
    data.elapsed = 0.0;
    data.balance = source.graph(KickTurnGraph::Balance, 0.0)?;
    data.last_input = 0.0;
    Ok(())
}

/// Loaded Right/Left occurrences provide `(max_height, spin_scale)=(0.3,0.5)`;
/// this kernel also accepts the class defaults `(0.5,0.5)` from the constructor.
#[expect(
    clippy::assign_op_pattern,
    reason = "native fadds uses controller dt as the left operand"
)]
pub(crate) fn update<S: KickTurnSteeringSource>(
    source: &mut S,
    instance: Option<&mut KickTurnSteeringInstance>,
    max_height: f32,
    spin_scale: f32,
) -> Result<(), S::Error> {
    source.resolve_interfaces()?;
    let data = instance.ok_or_else(|| source.missing_instance())?;

    if data.first_update {
        source.set_animation_attributes()?;
        let length = source.base_animation_length()?;
        data.animation_length = length;
        if !(length > 0.0) {
            return Err(source.invalid_animation_length(length));
        }
    } else {
        data.elapsed = source.delta_time()? + data.elapsed;
    }

    let phase = clamp_phase(data.elapsed / data.animation_length);
    let balance_curve = source.graph(KickTurnGraph::Balance, phase)?;
    let spin_curve = source.graph(KickTurnGraph::Spin, phase)?;
    let target = max_height * balance_curve;
    let smoothing = source.smoothing()?;
    let old_term = (1.0 - smoothing) * data.balance;
    let blended = target.mul_add(smoothing, old_term);
    data.balance = clamp_balance(blended, max_height);

    let input = source.kickturn_intent()?.unwrap_or(0.0);
    let input = if input == 0.0 {
        data.last_input
    } else {
        data.last_input = input;
        input
    };
    let shaped = if source.use_fine_kickturn()? {
        (input * input) * input
    } else {
        select(input, 1.0, -1.0)
    };
    source.add_motion_attribute(KickTurnAttribute::Balance, -data.balance)?;
    source.add_motion_attribute(KickTurnAttribute::Spin, (spin_scale * shaped) * spin_curve)?;
    data.first_update = false;
    Ok(())
}

/// End is the shared native BLR and does not alter the instance.
pub(crate) fn end(_instance: Option<&mut KickTurnSteeringInstance>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Probe {
        calls: Vec<&'static str>,
        graph_inputs: Vec<(KickTurnGraph, f32)>,
        intent: Option<f32>,
        fine: bool,
        length: f32,
        fail_spin_write: bool,
        writes: Vec<(KickTurnAttribute, f32)>,
    }
    impl KickTurnSteeringSource for Probe {
        type Error = &'static str;
        fn resolve_interfaces(&mut self) -> Result<(), Self::Error> {
            self.calls.push("resolve");
            Ok(())
        }
        fn missing_instance(&mut self) -> Self::Error {
            "missing instance"
        }
        fn set_animation_attributes(&mut self) -> Result<(), Self::Error> {
            self.calls.push("set attributes");
            Ok(())
        }
        fn base_animation_length(&mut self) -> Result<f32, Self::Error> {
            self.calls.push("length");
            Ok(self.length)
        }
        fn invalid_animation_length(&mut self, _: f32) -> Self::Error {
            "invalid length"
        }
        fn delta_time(&mut self) -> Result<f32, Self::Error> {
            self.calls.push("dt");
            Ok(0.25)
        }
        fn graph(&mut self, graph: KickTurnGraph, x: f32) -> Result<f32, Self::Error> {
            self.graph_inputs.push((graph, x));
            match graph {
                KickTurnGraph::Balance => {
                    self.calls.push("balance graph");
                    Ok(0.8)
                }
                KickTurnGraph::Spin => {
                    self.calls.push("spin graph");
                    Ok(0.6)
                }
            }
        }
        fn smoothing(&mut self) -> Result<f32, Self::Error> {
            self.calls.push("smoothing");
            Ok(0.5)
        }
        fn kickturn_intent(&mut self) -> Result<Option<f32>, Self::Error> {
            self.calls.push("intent");
            Ok(self.intent)
        }
        fn use_fine_kickturn(&mut self) -> Result<bool, Self::Error> {
            self.calls.push("fine");
            Ok(self.fine)
        }
        fn add_motion_attribute(
            &mut self,
            attribute: KickTurnAttribute,
            value: f32,
        ) -> Result<(), Self::Error> {
            self.calls.push(match attribute {
                KickTurnAttribute::Balance => "write balance",
                KickTurnAttribute::Spin => "write spin",
            });
            if attribute == KickTurnAttribute::Spin && self.fail_spin_write {
                return Err("write spin");
            }
            self.writes.push((attribute, value));
            Ok(())
        }
    }

    #[test]
    fn first_and_subsequent_updates_preserve_order_and_retained_intent() {
        let mut data = KickTurnSteeringInstance::try_create(|size, align| {
            assert_eq!((size, align), (0x1c, 0x10));
            Ok::<_, ()>(true)
        })
        .unwrap()
        .unwrap();
        let mut source = Probe {
            length: 2.0,
            intent: Some(-0.5),
            ..Probe::default()
        };
        begin(&mut source, Some(&mut data)).unwrap();
        assert_eq!(source.graph_inputs, [(KickTurnGraph::Balance, 0.0)]);
        source.calls.clear();
        source.graph_inputs.clear();
        update(&mut source, Some(&mut data), 0.3, 0.5).unwrap();
        assert_eq!(
            source.graph_inputs,
            [(KickTurnGraph::Balance, 0.0), (KickTurnGraph::Spin, 0.0)]
        );
        assert_eq!(
            source.calls,
            [
                "resolve",
                "set attributes",
                "length",
                "balance graph",
                "spin graph",
                "smoothing",
                "intent",
                "fine",
                "write balance",
                "write spin"
            ]
        );
        assert_eq!(
            (data.animation_length, data.elapsed, data.last_input),
            (2.0, 0.0, -0.5)
        );
        assert_eq!(source.writes[0], (KickTurnAttribute::Balance, -0.3));
        assert_eq!(source.writes[1], (KickTurnAttribute::Spin, -0.3));
        source.calls.clear();
        source.graph_inputs.clear();
        source.intent = None;
        update(&mut source, Some(&mut data), 0.3, 0.5).unwrap();
        assert_eq!(
            source.graph_inputs,
            [
                (KickTurnGraph::Balance, 0.125),
                (KickTurnGraph::Spin, 0.125)
            ]
        );
        assert_eq!(data.elapsed, 0.25);
        assert_eq!(data.last_input, -0.5);
        assert_eq!(
            source.calls,
            [
                "resolve",
                "dt",
                "balance graph",
                "spin graph",
                "smoothing",
                "intent",
                "fine",
                "write balance",
                "write spin"
            ]
        );
    }

    #[test]
    fn first_update_flag_survives_output_failure_and_bad_length_stores_length() {
        let mut data = KickTurnSteeringInstance::try_create(|_, _| Ok::<_, ()>(true))
            .unwrap()
            .unwrap();
        let mut source = Probe {
            length: f32::NAN,
            ..Probe::default()
        };
        assert_eq!(
            update(&mut source, Some(&mut data), 0.3, 0.5),
            Err("invalid length")
        );
        assert!(data.animation_length.is_nan());
        assert!(data.first_update);
        source.length = 2.0;
        source.fail_spin_write = true;
        assert_eq!(
            update(&mut source, Some(&mut data), 0.3, 0.5),
            Err("write spin")
        );
        assert!(data.first_update);
        assert_eq!(source.writes.len(), 1);
    }

    #[test]
    fn native_selects_keep_unordered_and_zero_branches() {
        assert_eq!(clamp_phase(-1.0), 0.0);
        assert_eq!(clamp_phase(2.0), 1.0);
        assert_eq!(clamp_phase(f32::NAN), 1.0);
        assert_eq!(clamp_balance(f32::NAN, 0.3), 0.3);
        assert_eq!(select(0.0, 1.0, -1.0), 1.0);
        assert_eq!(select(-0.0, 1.0, -1.0), 1.0);
    }
}
