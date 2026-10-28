//! Wipeout.Fill82dd46b0..4764: request6 and prior `FootPlant` output publication.
use super::super::{PhysOutComponent, PhysicalPlayerPhysOut};
use super::prior_input::PriorInputReadError;

pub(crate) trait WipeoutFillSource {
    type Error;
    fn heavy_landing_requested(&mut self) -> Result<bool, Self::Error>;
    fn heavy_landing_speed(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum WipeoutFillError<E> {
    Output(PriorInputReadError),
    Source(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for WipeoutFillError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Output(error) => write!(f, "Wipeout.Fill output: {error:?}"),
            Self::Source(error) => write!(f, "Wipeout.Fill source: {error:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn wipeout_fill_guard(&self, component: PhysOutComponent) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    pub(crate) fn try_fill_wipeout<S: WipeoutFillSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), WipeoutFillError<S::Error>> {
        use WipeoutFillError as E;
        // No output access at all when the actual request flag is false.
        if !source.heavy_landing_requested().map_err(E::Source)? {
            return Ok(());
        }
        self.wipeout_fill_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        let speed = source.heavy_landing_speed().map_err(E::Source)?;
        self.concrete
            .state
            .as_mut()
            .unwrap()
            .special_wipeout_info
            .landing_speed = speed;
        self.wipeout_fill_guard(PhysOutComponent::Air)
            .map_err(E::Output)?;
        let state = if self.concrete.air.as_ref().unwrap().is_left_foot_planting {
            1
        } else {
            // Native GetAir82dd4748 performs a fresh guard only on this branch.
            self.wipeout_fill_guard(PhysOutComponent::Air)
                .map_err(E::Output)?;
            if self.concrete.air.as_ref().unwrap().is_right_foot_planting {
                2
            } else {
                0
            }
        };
        self.concrete
            .state
            .as_mut()
            .unwrap()
            .special_wipeout_info
            .state = state;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_player_phys_out::construct::PhysOutAllocation;
    struct Source {
        requested: bool,
        calls: Vec<&'static str>,
    }
    impl WipeoutFillSource for Source {
        type Error = ();
        fn heavy_landing_requested(&mut self) -> Result<bool, ()> {
            self.calls.push("request");
            Ok(self.requested)
        }
        fn heavy_landing_speed(&mut self) -> Result<f32, ()> {
            self.calls.push("speed");
            Ok(f32::from_bits(0x8000_0000))
        }
    }
    fn output() -> PhysicalPlayerPhysOut {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        output
    }
    #[test]
    fn absent_request_skips_even_unconstructed_output_and_speed_read() {
        let mut source = Source {
            requested: false,
            calls: Vec::new(),
        };
        PhysicalPlayerPhysOut::new()
            .try_fill_wipeout(&mut source)
            .unwrap();
        assert_eq!(source.calls, ["request"]);
    }
    #[test]
    fn footplant_priority_and_raw_landing_speed_are_preserved() {
        for (left, right, expected) in [
            (false, false, 0),
            (true, false, 1),
            (false, true, 2),
            (true, true, 1),
        ] {
            let mut output = output();
            let air = output.concrete.air.as_mut().unwrap();
            air.is_left_foot_planting = left;
            air.is_right_foot_planting = right;
            let mut source = Source {
                requested: true,
                calls: Vec::new(),
            };
            output.try_fill_wipeout(&mut source).unwrap();
            let special = &output.concrete.state.as_ref().unwrap().special_wipeout_info;
            assert_eq!(special.state, expected);
            assert_eq!(special.landing_speed.to_bits(), 0x8000_0000);
            assert_eq!(source.calls, ["request", "speed"]);
        }
    }
    #[test]
    fn missing_air_preserves_landing_speed_prefix_before_special_state() {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|component| {
                Ok::<_, ()>(if component == PhysOutComponent::Air {
                    PhysOutAllocation::Null
                } else {
                    PhysOutAllocation::Host
                })
            })
            .unwrap();
        let prior_state = output
            .concrete
            .state
            .as_ref()
            .unwrap()
            .special_wipeout_info
            .state;
        let mut source = Source {
            requested: true,
            calls: Vec::new(),
        };
        assert!(output.try_fill_wipeout(&mut source).is_err());
        let special = &output.concrete.state.as_ref().unwrap().special_wipeout_info;
        assert_eq!(special.landing_speed.to_bits(), 0x8000_0000);
        assert_eq!(special.state, prior_state);
    }
}
