//! Original push edges and final intents82e060d0..61e8.
use super::super::{PhysOutComponent, PhysicalPlayerPhysOut};
use super::prior_input::PriorInputReadError;
pub(crate) trait Update5PushIntentSource {
    type Error;
    type Processed;
    /// One source word capture, with independently known source bits.
    fn processed_flags_8c4(&mut self) -> Result<(u32, u32), Self::Error>;
    /// Same captured `AnimOut` owner: regular stance read first, riding switch next.
    fn regular_and_switch(&mut self) -> Result<(bool, bool), Self::Error>;
    fn hilod_push_history(&mut self) -> Result<(bool, bool), Self::Error>;
    /// Native6168 reads history, then616c captures Processed before the store.
    fn capture_history_and_processed(&mut self) -> Result<(u32, Self::Processed), Self::Error>;
    fn store_push_history(&mut self, flags: u32) -> Result<(), Self::Error>;
    fn processed_flags_8c8(&mut self, processed: &Self::Processed) -> Result<u32, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_player_phys_out::construct::PhysOutAllocation;
    struct Source {
        flags: u32,
        known: u32,
        history: u32,
        stance: bool,
        switch: bool,
        fail_capture: bool,
        events: Vec<&'static str>,
    }
    impl Source {
        fn new() -> Self {
            Self {
                flags: 0,
                known: u32::MAX,
                history: 0xe000_0013,
                stance: true,
                switch: false,
                fail_capture: false,
                events: Vec::new(),
            }
        }
    }
    impl Update5PushIntentSource for Source {
        type Error = &'static str;
        type Processed = u32;
        fn processed_flags_8c4(&mut self) -> Result<(u32, u32), Self::Error> {
            self.events.push("flags");
            Ok((self.flags, self.known))
        }
        fn regular_and_switch(&mut self) -> Result<(bool, bool), Self::Error> {
            self.events.push("stance");
            Ok((self.stance, self.switch))
        }
        fn hilod_push_history(&mut self) -> Result<(bool, bool), Self::Error> {
            self.events.push("old_history");
            Ok((
                self.history & 0x0040_0000 != 0,
                self.history & 0x0080_0000 != 0,
            ))
        }
        fn capture_history_and_processed(&mut self) -> Result<(u32, u32), Self::Error> {
            self.events.push("fresh_history");
            self.events.push("capture_processed");
            if self.fail_capture {
                Err("capture")
            } else {
                Ok((self.history, 1))
            }
        }
        fn store_push_history(&mut self, flags: u32) -> Result<(), Self::Error> {
            self.events.push("store_history");
            self.history = flags;
            Ok(())
        }
        fn processed_flags_8c8(&mut self, captured: &u32) -> Result<u32, Self::Error> {
            self.events.push("coffin");
            Ok(*captured)
        }
    }
    fn output(null: Option<PhysOutComponent>) -> PhysicalPlayerPhysOut {
        let mut out = PhysicalPlayerPhysOut::new();
        out.try_construct_complete(|c| {
            Ok::<_, ()>(if Some(c) == null {
                PhysOutAllocation::Null
            } else {
                PhysOutAllocation::Host
            })
        })
        .unwrap();
        out
    }
    #[test]
    fn all_stances_choose_source_push_side_and_preserve_other_history_bits() {
        for stance in [false, true] {
            for switch in [false, true] {
                for right in [false, true] {
                    let mut out = output(None);
                    let mut source = Source::new();
                    source.stance = stance;
                    source.switch = switch;
                    source.flags = 0x0200_0020 | (u32::from(right) << 26);
                    out.try_publish_update5_push_intents(&mut source).unwrap();
                    let mongo = right == (switch == stance);
                    let anim = out.concrete.animation.as_ref().unwrap();
                    assert_eq!((anim.push_mongo, anim.push_regular), (mongo, !mongo));
                    assert_eq!(source.history & !0x00c0_0000, 0xe000_0013);
                    assert_eq!(
                        (source.history & 0x00c0_0000) >> 22,
                        if mongo { 1 } else { 2 }
                    );
                    assert!(
                        out.concrete
                            .intents
                            .as_ref()
                            .unwrap()
                            .is_generally_doing_coffin
                    );
                    assert!(out.concrete.intents.as_ref().unwrap().is_generally_grabbing);
                }
            }
        }
    }
    #[test]
    fn held_push_does_not_pulse_again_but_release_rearms() {
        let mut out = output(None);
        let mut source = Source::new();
        source.flags = 0x0200_0000;
        out.try_publish_update5_push_intents(&mut source).unwrap();
        assert!(out.concrete.animation.as_ref().unwrap().push_mongo);
        out.try_reset_complete().unwrap();
        out.try_publish_update5_push_intents(&mut source).unwrap();
        assert!(!out.concrete.animation.as_ref().unwrap().push_mongo);
        source.flags = 0;
        source.known = 0x0200_0020;
        source.events.clear();
        out.try_publish_update5_push_intents(&mut source).unwrap();
        assert!(!source.events.contains(&"stance"));
        assert_eq!(source.history & 0x00c0_0000, 0);
        source.flags = 0x0200_0000;
        source.known = u32::MAX;
        out.try_publish_update5_push_intents(&mut source).unwrap();
        assert!(out.concrete.animation.as_ref().unwrap().push_mongo);
    }
    #[test]
    fn late_capture_failure_keeps_pulse_but_not_new_history() {
        let mut out = output(None);
        let mut source = Source::new();
        source.flags = 0x0200_0000;
        source.fail_capture = true;
        assert!(matches!(
            out.try_publish_update5_push_intents(&mut source),
            Err(Update5PushIntentError::Source("capture"))
        ));
        assert!(out.concrete.animation.as_ref().unwrap().push_mongo);
        assert_eq!(source.history, 0xe000_0013);
        assert!(!source.events.contains(&"store_history"));
    }
    #[test]
    fn missing_intents_keeps_history_written_before_coffin_access() {
        let mut out = output(Some(PhysOutComponent::Intents));
        let mut source = Source::new();
        source.flags = 0x0200_0000;
        assert!(out.try_publish_update5_push_intents(&mut source).is_err());
        assert_eq!(source.history & 0x00c0_0000, 0x0040_0000);
        assert_eq!(
            &source.events[3..],
            [
                "fresh_history",
                "capture_processed",
                "store_history",
                "coffin"
            ]
        );
    }
}
#[derive(Debug)]
pub(crate) enum Update5PushIntentError<E> {
    Source(E),
    Output(PriorInputReadError),
    UnknownFlags(u32),
}
impl<E: std::fmt::Debug> std::fmt::Display for Update5PushIntentError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "push intent source: {e:?}"),
            Self::Output(e) => write!(f, "push intent output: {e:?}"),
            Self::UnknownFlags(mask) => {
                write!(f, "push intent missing Processed8c4 bits {mask:08x}")
            }
        }
    }
}
fn bit<E>((value, known): (u32, u32), mask: u32) -> Result<bool, Update5PushIntentError<E>> {
    if known & mask == mask {
        Ok(value & mask != 0)
    } else {
        Err(Update5PushIntentError::UnknownFlags(mask))
    }
}
impl PhysicalPlayerPhysOut {
    fn push_intent_guard(&self, component: PhysOutComponent) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    pub(crate) fn try_publish_update5_push_intents<S: Update5PushIntentSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), Update5PushIntentError<S::Error>> {
        use Update5PushIntentError as E;
        let flags = source.processed_flags_8c4().map_err(E::Source)?;
        let (mongo, regular) = if bit(flags, 0x0200_0000)? {
            let right = bit(flags, 0x0400_0000)?;
            let (stance, switch) = source.regular_and_switch().map_err(E::Source)?;
            let mongo = right == (switch == stance);
            (mongo, !mongo)
        } else {
            (false, false)
        };
        let (was_mongo, was_regular) = source.hilod_push_history().map_err(E::Source)?;
        if !was_mongo && mongo {
            self.push_intent_guard(PhysOutComponent::Animation)
                .map_err(E::Output)?;
            self.concrete.animation.as_mut().unwrap().push_mongo = true;
        } else if !was_regular && regular {
            self.push_intent_guard(PhysOutComponent::Animation)
                .map_err(E::Output)?;
            self.concrete.animation.as_mut().unwrap().push_regular = true;
        }
        let (history, processed) = source.capture_history_and_processed().map_err(E::Source)?;
        let history =
            (history & !0x00c0_0000) | (u32::from(mongo) << 22) | (u32::from(regular) << 23);
        source.store_push_history(history).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        let coffin = source.processed_flags_8c8(&processed).map_err(E::Source)? & 1 != 0;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::Intents)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete
            .intents
            .as_mut()
            .unwrap()
            .is_generally_doing_coffin = coffin;
        let grabbing = bit(source.processed_flags_8c4().map_err(E::Source)?, 0x20)?;
        self.push_intent_guard(PhysOutComponent::Intents)
            .map_err(E::Output)?;
        self.concrete
            .intents
            .as_mut()
            .unwrap()
            .is_generally_grabbing = grabbing;
        Ok(())
    }
}
