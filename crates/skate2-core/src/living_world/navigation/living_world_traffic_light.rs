//! Original TrafficLight82e54578..82e60230; shared vehicle/crossing clock.
use super::{LightStates, RainMan, TrafficLight};
use crate::attribute_runtime::AttributeRuntimeError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum Colour {
    Red = 0,
    Yellow = 1,
    Green = 2,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LightState {
    pub(crate) colour: Colour,
    pub(crate) total: f32,
    pub(crate) remaining: f32,
    pub(crate) visual_index: u32,
}
#[derive(Debug)]
pub(crate) enum TrafficError {
    Attribute(AttributeRuntimeError),
    Capacity,
    InvalidState,
    InvalidLightIndex(u32),
    AlreadyInitialized,
}
impl LightStates {
    fn push(
        &mut self,
        colour: Colour,
        duration: f32,
        visual_index: u32,
    ) -> Result<(), TrafficError> {
        let slot = self
            .records
            .get_mut(self.count)
            .ok_or(TrafficError::Capacity)?;
        *slot = Some(LightState {
            colour,
            total: duration,
            remaining: duration,
            visual_index,
        });
        self.count += 1;
        Ok(())
    }
    fn state(&mut self, index: i32) -> Result<&mut LightState, TrafficError> {
        self.records
            .get_mut(index as usize)
            .and_then(Option::as_mut)
            .ok_or(TrafficError::InvalidState)
    }
    fn reset(&mut self, index: i32) -> Result<(), TrafficError> {
        let state = self.state(index)?;
        state.remaining = state.total;
        Ok(())
    }
    /// One transition per call, even for a delta larger than several states.
    /// Vehicle cycle wrap discards negative remainder and resets pedestrian phase.
    fn update(
        &mut self,
        index: &mut i32,
        dt: f32,
        force_wrap: bool,
        discard_wrap: bool,
    ) -> Result<bool, TrafficError> {
        if self.count == 0 || *index < 0 {
            return Ok(false);
        }
        let previous = *index;
        let remaining = {
            let state = self.state(previous)?;
            state.remaining -= dt;
            state.remaining
        };
        if !(remaining < 0.0 || force_wrap) {
            return Ok(false);
        }
        *index = if force_wrap {
            0
        } else {
            (*index + 1) % self.count as i32
        };
        let wrapped = *index == 0;
        if force_wrap || (discard_wrap && wrapped) {
            self.state(previous)?.remaining = 0.0;
        }
        let carry = self.state(previous)?.remaining;
        self.state(*index)?.remaining += carry;
        self.reset(previous)?;
        Ok(wrapped)
    }
}
impl TrafficLight {
    /// AddState82e5fea8: green splits the walking interval into walk/brisk-walk.
    pub(crate) fn add_state(&mut self, colour: Colour, duration: f32) -> Result<(), TrafficError> {
        self.light_states.push(colour, duration, colour as u32)?;
        if self.current_state < 0 {
            self.current_state = 0;
        }
        if colour == Colour::Green {
            self.pedestrian_states.push(
                Colour::Green,
                (1.0 - self.brisk_walk_duration_percentage) * duration,
                1,
            )?;
            self.pedestrian_states.push(
                Colour::Yellow,
                duration * self.brisk_walk_duration_percentage,
                1,
            )?;
        } else {
            self.pedestrian_states.push(Colour::Red, duration, 0)?;
        }
        if self.current_pedestrian_state < 0 {
            self.current_pedestrian_state = 0;
        }
        Ok(())
    }
    fn add_sync(&mut self, green: bool, duration: f32) -> Result<(), TrafficError> {
        let vehicle = Some(self.light_states.count as u32);
        let pedestrian = Some(self.pedestrian_states.count as u32);
        if green {
            self.sync_to_green_state = vehicle;
            self.sync_to_green_pedestrian_state = pedestrian;
        } else {
            self.sync_to_red_state = vehicle;
            self.sync_to_red_pedestrian_state = pedestrian;
        }
        self.add_state(Colour::Red, duration)
    }
    pub(crate) fn update(&mut self, dt: f32) -> Result<(), TrafficError> {
        let wrap = self
            .light_states
            .update(&mut self.current_state, dt, false, true)?;
        self.pedestrian_states
            .update(&mut self.current_pedestrian_state, dt, wrap, false)?;
        Ok(())
    }
    pub(crate) fn current(&self, pedestrian: bool) -> Result<&LightState, TrafficError> {
        let (states, index) = if pedestrian {
            (&self.pedestrian_states, self.current_pedestrian_state)
        } else {
            (&self.light_states, self.current_state)
        };
        states
            .records
            .get(index as usize)
            .and_then(Option::as_ref)
            .ok_or(TrafficError::InvalidState)
    }
    pub(crate) fn sync(&mut self, green: bool) -> Result<(), TrafficError> {
        self.light_states.reset(self.current_state)?;
        self.pedestrian_states
            .reset(self.current_pedestrian_state)?;
        if green {
            let index = self.sync_to_green_state.ok_or(TrafficError::InvalidState)? as i32;
            // Raw82e54844/64/70 deliberately uses +138 for BOTH indices;
            // the initialized +13c pedestrian sync index is not read here.
            self.current_state = index;
            self.current_pedestrian_state = index;
        } else {
            self.current_state = self.sync_to_red_state.ok_or(TrafficError::InvalidState)? as i32;
            self.current_pedestrian_state =
                self.sync_to_red_pedestrian_state
                    .ok_or(TrafficError::InvalidState)? as i32;
        }
        Ok(())
    }
}
impl RainMan {
    /// WorldPresentation::DoSimSubmit827d3cd8 captures lights 0/1; Restore
    /// 827da340 writes two shader vectors: 2 in the current vehicle-colour
    /// lane, and twice the pedestrian visual state in W. No interpolation.
    pub(crate) fn presentation_traffic_lights(&self) -> Result<[[f32; 4]; 2], TrafficError> {
        let mut result = [[0.; 4]; 2];
        for (output, light) in result.iter_mut().zip(&self.traffic_lights[..2]) {
            let vehicle = light.current(false)?.visual_index as usize;
            let pedestrian = light.current(true)?.visual_index;
            *output.get_mut(vehicle).ok_or(TrafficError::InvalidState)? = 2.;
            output[3] = 2. * pedestrian as f32;
        }
        Ok(result)
    }

    /// IsLightBiased826f3ac8: a locked light is biased only if it is green;
    /// during synchronization the requested even/odd group is already biased.
    pub(crate) fn is_light_biased(&self, index: u32) -> Result<bool, TrafficError> {
        if self.lock_lights {
            return Ok(self
                .traffic_lights
                .get(index as usize)
                .ok_or(TrafficError::InvalidLightIndex(index))?
                .current(false)?
                .colour
                == Colour::Green);
        }
        Ok(if index & 1 == 0 {
            self.syncing_even_lights_green
        } else {
            self.syncing_odd_lights_green
        })
    }
    /// BiasLightsGreen82702190 preserves the existing timers when the desired
    /// phase is already green; otherwise all four lamps enter their sync state.
    pub(crate) fn bias_lights_green(&mut self, index: u32) -> Result<(), TrafficError> {
        if self.is_light_biased(index)? {
            return Ok(());
        }
        self.lock_lights = false;
        self.syncing_even_lights_green = false;
        self.syncing_odd_lights_green = false;
        if self
            .traffic_lights
            .get(index as usize)
            .ok_or(TrafficError::InvalidLightIndex(index))?
            .current(false)?
            .colour
            == Colour::Green
        {
            self.lock_lights = true;
        } else {
            self.syncing_even_lights_green = index & 1 == 0;
            self.syncing_odd_lights_green = index & 1 != 0;
            for (i, light) in self.traffic_lights.iter_mut().enumerate() {
                light.sync(i as u32 & 1 == index & 1)?;
            }
        }
        Ok(())
    }
    /// Traffic subsequence of Init8271332c..82713514. Source attribute keys
    /// reconstructed from raw insrdi arguments; never retune shipped durations.
    pub(crate) fn initialize_traffic(&mut self) -> Result<(), TrafficError> {
        if self
            .traffic_lights
            .iter()
            .any(|l| l.light_states.count != 0)
        {
            return Err(TrafficError::AlreadyInitialized);
        }
        let read = |key| {
            self.traffic_attributes
                .try_scalar_word(key, 0)
                .map(|w| f32::from_bits(w.unwrap_or(0)))
                .map_err(TrafficError::Attribute)
        };
        let green = read(0x3836_077d_efb6_f008)?;
        let yellow = read(0xcfca_94d6_7e0b_234e)?;
        let sync = read(0x2c0c_e376_6a00_74cd)?;
        let brisk = read(0x5e41_c959_d175_27cc)?;
        for (i, light) in self.traffic_lights.iter_mut().enumerate() {
            light.brisk_walk_duration_percentage = brisk;
            if i & 1 != 0 {
                light.add_sync(true, sync)?;
                light.add_state(Colour::Green, green)?;
                light.add_state(Colour::Yellow, yellow)?;
                light.add_sync(false, sync)?;
                light.add_state(Colour::Red, green + yellow)?;
            } else {
                light.add_sync(false, sync)?;
                light.add_state(Colour::Red, green + yellow)?;
                light.add_sync(true, sync)?;
                light.add_state(Colour::Green, green)?;
                light.add_state(Colour::Yellow, yellow)?;
            }
        }
        Ok(())
    }
    /// Clock subsection of Update8271e644..6cc, after bias processing.
    pub(crate) fn update_traffic(&mut self, dt: f32) -> Result<(), TrafficError> {
        if self.lock_lights {
            return Ok(());
        }
        for light in &mut self.traffic_lights {
            light.update(dt)?;
        }
        let sync_index = if self.syncing_even_lights_green {
            Some(0)
        } else if self.syncing_odd_lights_green {
            Some(1)
        } else {
            None
        };
        if let Some(i) = sync_index {
            if self.traffic_lights[i].current(false)?.colour == Colour::Green {
                self.lock_lights = true;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_zero_waits_until_next_update_and_overrun_advances_only_once() {
        let mut light = TrafficLight::construct();
        light.brisk_walk_duration_percentage = 0.25;
        light.add_state(Colour::Green, 4.).unwrap();
        light.add_state(Colour::Red, 2.).unwrap();
        light.update(4.).unwrap();
        assert_eq!(light.current(false).unwrap().colour, Colour::Green);
        light.update(10.).unwrap();
        assert_eq!(light.current_state, 1);
        assert_eq!(light.current(false).unwrap().remaining, -8.);
        light.update(0.).unwrap();
        assert_eq!(light.current_state, 0);
        assert_eq!(light.current_pedestrian_state, 0);
        assert_eq!(light.current(false).unwrap().remaining, 4.);
    }
    #[test]
    fn green_sync_preserves_source_shared_index_despite_split_pedestrian_states() {
        let mut light = TrafficLight::construct();
        light.brisk_walk_duration_percentage = 0.2;
        light.add_state(Colour::Green, 10.).unwrap();
        light.add_sync(true, 1.).unwrap();
        assert_eq!(light.sync_to_green_state, Some(1));
        assert_eq!(light.sync_to_green_pedestrian_state, Some(2));
        light.sync(true).unwrap();
        assert_eq!(light.current_state, 1);
        assert_eq!(light.current_pedestrian_state, 1);
    }
}
