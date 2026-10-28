//! Vehicle.UpdateAttributes82d25968. Driver probabilities and stopping distances
//! use the authored Vault data, not host tuning. The caller supplies the original
//! rw::math draw occurrence and LWBasicEntity.GetLength result.
use super::{VehicleMotion, driving::VehicleRoute};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    native_skater_random::rw_math::{bernoulli_choice, vector_discrete_choice},
};

pub(crate) const DRIVERS_CLASS: u64 = 0x02aa_3f53_8d6c_188a;
const SKITCH_HELPER_PROBABILITY: u64 = 0xd0ba_2e02_de46_a57a;
const VEHICLE_DISTANCE_FACTOR: u64 = 0x73a1_989b_b90c_93ba;
const ENTITY_DISTANCE_FACTOR: u64 = 0x3031_3ea6_f282_c874;

#[derive(Debug)]
pub(crate) enum VehicleAttributeError<E> {
    Attribute(AttributeRuntimeError),
    Random(E),
}
impl<E> From<AttributeRuntimeError> for VehicleAttributeError<E> {
    fn from(error: AttributeRuntimeError) -> Self {
        Self::Attribute(error)
    }
}
impl VehicleMotion {
    pub(crate) fn update_attributes<E>(
        &mut self,
        length: f32,
        characteristics: &AttributeRuntimeInstance,
        driver: &AttributeRuntimeInstance,
        random_word: &mut impl FnMut() -> Result<u32, E>,
    ) -> Result<(), VehicleAttributeError<E>> {
        use VehicleAttributeError::Random;
        // Original order is layout+8, +4, +0: normal, below, above speed limit.
        let probabilities = [layout(driver, 8)?, layout(driver, 4)?, layout(driver, 0)?];
        match vector_discrete_choice(&probabilities, random_word().map_err(Random)?) {
            Some(0) => self.speed_limit_delta = 0.,
            Some(1) => self.speed_limit_delta = -layout(characteristics, 0x18)?,
            Some(2) => self.speed_limit_delta = layout(characteristics, 0x20)?,
            _ => {} // Native -1 leaves the existing value untouched.
        }
        let probability = scalar(driver, SKITCH_HELPER_PROBABILITY)?;
        self.skitching_speed_limit_delta = Some(
            if bernoulli_choice(probability, random_word().map_err(Random)?) {
                layout(characteristics, 0x1c)?
            } else {
                0.
            },
        );
        self.minimum_intersection_distance = length * layout(characteristics, 0x2c)?;
        self.minimum_vehicle_distance = length * scalar(characteristics, VEHICLE_DISTANCE_FACTOR)?;
        self.minimum_entity_distance = length * scalar(characteristics, ENTITY_DISTANCE_FACTOR)?;
        self.minimum_parking_distance = length * layout(characteristics, 0x24)?;
        let minimum = layout(characteristics, 0x34)?;
        let span = layout(characteristics, 0x38)? - minimum;
        let draw = (random_word().map_err(Random)? % 101) as f32;
        // fmuls82d25d44 then fmadds82d25d48; endpoints0 and100 both possible.
        self.default_acceleration = (draw * span).mul_add(f32::from_bits(0x3c23_d70a), minimum);
        self.minimum_lane_change_distance_factor = layout(characteristics, 0x28)?;
        Ok(())
    }
}
pub(super) fn layout(
    instance: &AttributeRuntimeInstance,
    at: usize,
) -> Result<f32, AttributeRuntimeError> {
    Ok(instance.try_cached_layout_field(at, 4)?.map_or(0., |data| {
        f32::from_be_bytes(data.bytes().try_into().expect("four-byte field"))
    }))
}
pub(super) fn scalar(
    instance: &AttributeRuntimeInstance,
    key: u64,
) -> Result<f32, AttributeRuntimeError> {
    Ok(f32::from_bits(
        instance.try_scalar_word(key, 0)?.unwrap_or(0),
    ))
}

impl VehicleRoute {
    /// MakeNewBehaviourDecision82d19718: six source-global Bernoulli draws,
    /// including probability0/1; never substitute the Manager RNG here.
    pub(crate) fn make_new_behaviour_decision<E>(
        &mut self,
        motion: &mut VehicleMotion,
        driver: &AttributeRuntimeInstance,
        random_word: &mut impl FnMut() -> Result<u32, E>,
    ) -> Result<bool, VehicleAttributeError<E>> {
        use VehicleAttributeError::Random;
        self.avoid_pileup = bernoulli_choice(layout(driver, 0x14)?, random_word().map_err(Random)?);
        self.avoid_npcs = bernoulli_choice(
            scalar(driver, 0xc423_b7e0_449f_8d0e)?,
            random_word().map_err(Random)?,
        );
        self.avoid_skaters = bernoulli_choice(layout(driver, 0xc)?, random_word().map_err(Random)?);
        motion.willing_to_run_red_light =
            bernoulli_choice(layout(driver, 0x10)?, random_word().map_err(Random)?);
        self.honk = bernoulli_choice(layout(driver, 0x24)?, random_word().map_err(Random)?);
        self.park_when_skitched = bernoulli_choice(
            scalar(driver, 0x9908_3122_a1b7_116a)?,
            random_word().map_err(Random)?,
        );
        Ok(true)
    }
}
