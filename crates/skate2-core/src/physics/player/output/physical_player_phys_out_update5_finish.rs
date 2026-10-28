//! `ProcessIntents`82e04de0 and `AnimOut.Fill`82b3a1e8, the last Update5 publications.
use super::super::{PhysOutComponent, PhysicalPlayerPhysOut};
use super::prior_input::PriorInputReadError;
use crate::play_animation::FastString30;
pub(crate) trait Update5FinishSource {
    type Error;
    fn pushing_motion_intent_present(&mut self) -> Result<bool, Self::Error>;
    fn animation_flag(&mut self, mask: u32) -> Result<bool, Self::Error>;
    fn animation_grind_name(&mut self) -> Result<FastString30, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        physical_player_phys_out::construct::PhysOutAllocation,
        play_animation::ANDALE_NULL_STRING_NAME_30,
    };
    #[derive(Default)]
    struct Source {
        reads: Vec<u32>,
    }
    impl Update5FinishSource for Source {
        type Error = ();
        fn pushing_motion_intent_present(&mut self) -> Result<bool, ()> {
            self.reads.push(0);
            Ok(true)
        }
        fn animation_flag(&mut self, mask: u32) -> Result<bool, ()> {
            self.reads.push(mask);
            Ok(mask != 0x4000_0000)
        }
        fn animation_grind_name(&mut self) -> Result<FastString30, ()> {
            self.reads.push(1);
            Ok(ANDALE_NULL_STRING_NAME_30)
        }
    }
    #[test]
    fn final_flags_invert_no_input_and_do_not_lock_output() {
        let mut out = PhysicalPlayerPhysOut::new();
        out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        let mut source = Source::default();
        out.try_publish_update5_finish(&mut source).unwrap();
        assert_eq!(
            source.reads,
            [
                0,
                0x8000_0000,
                1,
                0x4000_0000,
                0x2000_0000,
                0x1000_0000,
                0x0800_0000
            ]
        );
        let intents = out.concrete.intents.as_ref().unwrap();
        assert!(
            intents.is_pushing
                && intents.is_player_providing_input
                && intents.is_new_auto_pump
                && intents.is_kickturning
                && intents.is_anticipating
        );
        assert!(
            out.concrete
                .grinds
                .as_ref()
                .unwrap()
                .is_grind_facing_forwards
        );
        assert!(out.check_current_guard().is_ok());
    }
    #[test]
    fn initial_locked_output_still_evaluates_motion_intent_presence() {
        let mut out = PhysicalPlayerPhysOut::new();
        out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        out.lock_after_update1().unwrap();
        let mut source = Source::default();
        assert!(out.try_publish_update5_finish(&mut source).is_err());
        assert_eq!(source.reads, [0]);
    }
}
#[derive(Debug)]
pub(crate) enum Update5FinishError<E> {
    Source(E),
    Output(PriorInputReadError),
}
impl<E: std::fmt::Debug> std::fmt::Display for Update5FinishError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "Update5 finish source: {e:?}"),
            Self::Output(e) => write!(f, "Update5 finish output: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn finish_output_guard(&self, component: PhysOutComponent) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    pub(crate) fn try_publish_update5_finish<S: Update5FinishSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), Update5FinishError<S::Error>> {
        use Update5FinishError as E;
        // Presence of a Motion intent, independent of its float value.
        let pushing = source.pushing_motion_intent_present().map_err(E::Source)?;
        self.finish_output_guard(PhysOutComponent::Intents)
            .map_err(E::Output)?;
        self.concrete.intents.as_mut().unwrap().is_pushing = pushing;
        let facing = source.animation_flag(0x8000_0000).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::Grinds)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete
            .grinds
            .as_mut()
            .unwrap()
            .is_grind_facing_forwards = facing;
        self.finish_output_guard(PhysOutComponent::Grinds)
            .map_err(E::Output)?;
        self.concrete.grinds.as_mut().unwrap().anim_grind_name =
            source.animation_grind_name().map_err(E::Source)?;
        for (mask, sample_first) in [
            (0x4000_0000, true),
            (0x2000_0000, false),
            (0x1000_0000, true),
            (0x0800_0000, false),
        ] {
            let value = if sample_first {
                let guard = self
                    .sample_write_guard()
                    .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
                let value = source.animation_flag(mask).map_err(E::Source)?;
                self.check_write_guard(&guard)
                    .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
                self.require_complete_component(PhysOutComponent::Intents)
                    .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
                value
            } else {
                let value = source.animation_flag(mask).map_err(E::Source)?;
                self.finish_output_guard(PhysOutComponent::Intents)
                    .map_err(E::Output)?;
                value
            };
            let intents = self.concrete.intents.as_mut().unwrap();
            match mask {
                0x4000_0000 => intents.is_player_providing_input = !value,
                0x2000_0000 => intents.is_new_auto_pump = value,
                0x1000_0000 => intents.is_kickturning = value,
                _ => intents.is_anticipating = value,
            }
        }
        Ok(())
    }
}
