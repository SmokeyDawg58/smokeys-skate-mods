//! `CreateAttribute1927` lifecycle gates and Create82ca1568 publication order.
use crate::{generated_motion_load::construct::CreateAttribute, play_animation::FastString30};

#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Begin,
    Update,
    End,
}
pub(crate) trait CreateAttributeSource {
    type Error;
    fn resolve_skater_motion(&mut self) -> Result<(), Self::Error>;
    fn append_motion(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error>;
    /// An absent `IAnimatable` is allowed. A foreign cached/list occurrence is not.
    fn resolve_animatable(&mut self) -> Result<bool, Self::Error>;
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error>;
}
/// `InstanceCreate82bb3da8` publishes null; there is no behavior-owned instance.
pub(crate) fn run<S: CreateAttributeSource>(
    source: &mut S,
    payload: &CreateAttribute,
    phase: Phase,
) -> Result<(), S::Error> {
    let phase = match phase {
        Phase::Begin => &payload.begin,
        Phase::Update => &payload.update,
        Phase::End => &payload.end,
    };
    if !phase.active {
        return Ok(());
    }
    source.resolve_skater_motion()?;
    source.append_motion(payload.name, phase.value)?;
    if payload.settable && source.resolve_animatable()? {
        source.settable(payload.name, phase.value)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated_motion_load::construct::AttributePhaseValue;
    #[derive(Default)]
    struct Source {
        log: Vec<&'static str>,
        motion: Vec<f32>,
        animation: Vec<f32>,
        present: bool,
        fail: bool,
    }
    impl CreateAttributeSource for Source {
        type Error = &'static str;
        fn resolve_skater_motion(&mut self) -> Result<(), Self::Error> {
            self.log.push("motion owner");
            Ok(())
        }
        fn append_motion(&mut self, _: FastString30, value: f32) -> Result<(), Self::Error> {
            self.log.push("append");
            self.motion.push(value);
            Ok(())
        }
        fn resolve_animatable(&mut self) -> Result<bool, Self::Error> {
            self.log.push("animation owner");
            if self.fail {
                Err("animation")
            } else {
                Ok(self.present)
            }
        }
        fn settable(&mut self, _: FastString30, value: f32) -> Result<(), Self::Error> {
            self.log.push("settable");
            self.animation.push(value);
            Ok(())
        }
    }
    fn payload() -> CreateAttribute {
        CreateAttribute {
            name: FastString30::encode("Turn").unwrap(),
            begin: AttributePhaseValue {
                active: true,
                value: 1.0,
            },
            update: AttributePhaseValue {
                active: true,
                value: 2.0,
            },
            end: AttributePhaseValue {
                active: true,
                value: -0.0,
            },
            settable: true,
        }
    }
    #[test]
    fn disabled_phase_resolves_nothing() {
        let mut p = payload();
        p.begin.active = false;
        let mut s = Source::default();
        run(&mut s, &p, Phase::Begin).unwrap();
        assert!(s.log.is_empty());
    }
    #[test]
    fn phases_publish_in_order_end_is_an_attribute_write() {
        let mut s = Source {
            present: true,
            ..Source::default()
        };
        let p = payload();
        for phase in [Phase::Begin, Phase::Update, Phase::End] {
            run(&mut s, &p, phase).unwrap();
        }
        assert_eq!(
            s.motion.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            [1.0f32.to_bits(), 2.0f32.to_bits(), 0x8000_0000]
        );
        assert_eq!(s.animation.len(), 3);
        assert_eq!(
            &s.log[..4],
            ["motion owner", "append", "animation owner", "settable"]
        );
    }
    #[test]
    fn missing_animation_is_optional_and_failure_keeps_motion_append() {
        let p = payload();
        let mut s = Source::default();
        run(&mut s, &p, Phase::Update).unwrap();
        assert_eq!(s.motion, [2.0]);
        assert!(s.animation.is_empty());
        s.fail = true;
        assert_eq!(run(&mut s, &p, Phase::End), Err("animation"));
        assert_eq!(s.motion.len(), 2);
        assert_eq!(s.motion[1].to_bits(), 0x8000_0000);
    }
    #[test]
    fn unset_does_not_query_animation_and_motion_preserves_nan() {
        let mut p = payload();
        p.settable = false;
        p.update.value = f32::from_bits(0x7fc0_1234);
        let mut s = Source {
            fail: true,
            ..Source::default()
        };
        run(&mut s, &p, Phase::Update).unwrap();
        assert_eq!(s.log, ["motion owner", "append"]);
        assert_eq!(s.motion[0].to_bits(), 0x7fc0_1234);
    }
}
