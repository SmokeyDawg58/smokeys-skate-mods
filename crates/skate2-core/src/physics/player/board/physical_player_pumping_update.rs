//! Pumping.Update82de0428 and Calculate82dde2f8; source-reviewed numeric body.
use super::{PhysicalPlayerPumpingOwner, Pumping, PumpingError, attributes::PumpingAttributeRead};
use crate::{
    physical_player_phys_out::construct::board_fill::board_speed_from_squared_and_estimate,
    reckoning_numeric::{self, ReckoningNumeric},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::cmp::Ordering;

pub(crate) trait PumpingDebug {
    type Error;
    fn draw_deck_angle(
        &mut self,
        deck: f32,
        deck_input: f32,
        minimum_crouch: f32,
        ground_input: f32,
    ) -> Result<(), Self::Error>;
    fn draw_pump(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub(crate) enum PumpingUpdateError<A, N, D> {
    Owner(PumpingError),
    Attribute(A),
    Numeric(N),
    Debug(D),
}
type UpdateResult<T, A, N, D> = Result<T, PumpingUpdateError<A, N, D>>;
impl<A: std::fmt::Debug, N: std::fmt::Debug, D: std::fmt::Debug> std::fmt::Display
    for PumpingUpdateError<A, N, D>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "Pumping owner: {error:?}"),
            Self::Attribute(error) => write!(f, "Pumping attribute: {error:?}"),
            Self::Numeric(error) => write!(f, "Pumping numeric: {error:?}"),
            Self::Debug(error) => write!(f, "Pumping debug: {error:?}"),
        }
    }
}

fn scalar<A: PumpingAttributeRead>(attributes: &mut A, key: u64) -> Result<f32, A::Error> {
    let pointer = attributes.lookup(key)?;
    attributes.scalar(&pointer, key)
}

/// The source fsel pair chooses +0 for nonpositive finite values, and +1 for
/// unordered values. This is deliberately not `f32::clamp/min/max`.
fn unit_interval(value: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

impl Pumping {
    #[expect(
        clippy::too_many_arguments,
        reason = "native input vectors and separate retained services"
    )]
    fn try_calculate<A: PumpingAttributeRead, N: ReckoningNumeric, D: PumpingDebug>(
        &mut self,
        position: SkeletonRootVector,
        normal: SkeletonRootVector,
        deck_angle: f32,
        dt: f32,
        attributes: &mut A,
        numeric: &mut N,
        debug: &mut D,
    ) -> UpdateResult<f32, A::Error, N::Error, D::Error> {
        use PumpingUpdateError as E;
        let displacement = position.zip(self.last_position, |a, b| a - b);
        let estimate = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
        let residual = -estimate.mul_add(dt, -1.0);
        let inverse = estimate.mul_add(residual, estimate);
        let residual = -inverse.mul_add(dt, -1.0);
        let inverse = inverse.mul_add(residual, inverse);
        let velocity = displacement.map(|lane| inverse * lane);
        let squared = numeric.dot_vector(velocity, velocity).map_err(E::Numeric)?;
        let estimate = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        let speed = board_speed_from_squared_and_estimate(squared, estimate);
        let speed_factor = attributes
            .keyed_graph(0x411e_900c_4dbf_7e9f, speed)
            .map_err(E::Attribute)?;

        let displacement = position.zip(self.last_position, |a, b| a - b);
        let zero = SkeletonRootVector::known([0.0; 4]);
        // This first NormalizeSafe result is overwritten by the second call,
        // but its numeric service and possible failure remain in source order.
        let _unused_direction = reckoning_numeric::try_normalize_safe(displacement, zero, numeric)
            .map_err(E::Numeric)?;
        let axis = reckoning_numeric::cross_typed(displacement, normal);
        let normal_change = reckoning_numeric::cross_typed(self.last_normal, normal);
        let estimate = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
        let residual = -estimate.mul_add(dt, -1.0);
        let inverse = estimate.mul_add(residual, estimate);
        let axis =
            reckoning_numeric::try_normalize_safe(axis, zero, numeric).map_err(E::Numeric)?;
        let residual = -inverse.mul_add(dt, -1.0);
        let inverse = inverse.mul_add(residual, inverse);
        let angular = normal_change.map(|lane| inverse * lane);
        let angular = numeric.dot_vector(angular, axis).map_err(E::Numeric)?;
        let smooth = scalar(attributes, 0x91cf_6ee0_4908_570e).map_err(E::Attribute)?;
        let current = angular * smooth;
        self.angular_speed = (1.0 - smooth).mul_add(self.angular_speed, current);
        self.absorption = -(speed * self.angular_speed);

        let ground_y = unit_interval(self.last_normal.xyz[1]);
        let angle = reckoning_numeric::try_acos(ground_y, numeric).map_err(E::Numeric)?;
        let angle_scale = f32::from_bits(0x3f22_f983);
        let ground_input = unit_interval(angle_scale * angle);
        let key = 0xf835_62fb_39c3_031c;
        let ground_scalar = attributes.lookup(key).map_err(E::Attribute)?;
        let graph = attributes
            .layout_graph(0x40, ground_input)
            .map_err(E::Attribute)?;
        let ground_scalar = attributes
            .scalar(&ground_scalar, key)
            .map_err(E::Attribute)?;
        self.ground_normal_absorption = graph * ground_scalar;
        self.ground_normal_minimum_crouch = attributes
            .layout_graph(0, ground_input)
            .map_err(E::Attribute)?;
        let deck_input = deck_angle * angle_scale;
        self.deck_angle_absorption = attributes
            .layout_graph(0x80, deck_input)
            .map_err(E::Attribute)?;
        debug
            .draw_deck_angle(
                self.deck_angle_absorption,
                deck_input,
                self.ground_normal_minimum_crouch,
                ground_input,
            )
            .map_err(E::Debug)?;
        let deck_scalar = scalar(attributes, 0x36a2_4a73_7014_3936).map_err(E::Attribute)?;
        let deck = self.deck_angle_absorption * deck_scalar;
        let ground_squared = self.ground_normal_absorption * self.ground_normal_absorption;
        let deck_squared = deck * deck;
        if deck_squared > ground_squared {
            self.ground_normal_absorption = deck;
        }
        Ok(self.angular_speed * speed_factor)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "native input vectors and separate retained services"
    )]
    pub(super) fn try_update<A: PumpingAttributeRead, N: ReckoningNumeric, D: PumpingDebug>(
        &mut self,
        position: SkeletonRootVector,
        normal: SkeletonRootVector,
        com_to_deck: SkeletonRootVector,
        deck_angle: f32,
        dt: f32,
        attributes: &mut A,
        numeric: &mut N,
        debug: &mut D,
    ) -> UpdateResult<(), A::Error, N::Error, D::Error> {
        use PumpingUpdateError as E;
        self.acceleration = 0.0;
        let height = numeric
            .dot_vector(com_to_deck, normal)
            .map_err(E::Numeric)?;
        if self.record_valid {
            let smooth = scalar(attributes, 0x8734_02e3_c7f1_a2ca).map_err(E::Attribute)?;
            let threshold = scalar(attributes, 0xa898_f4cd_23f5_d9b0).map_err(E::Attribute)?;
            let maximum = scalar(attributes, 0x4bae_22d0_e9c5_851c).map_err(E::Attribute)?;
            let delta = height - self.last_height;
            let delta = if delta >= 0.0 { delta } else { 0.0 };
            let old = (1.0 - smooth) * self.smooth_raw_height_delta;
            self.smooth_raw_height_delta = delta.mul_add(smooth, old);
            let mut delta =
                if self.smooth_raw_height_delta.partial_cmp(&threshold) == Some(Ordering::Less) {
                    0.0
                } else {
                    self.smooth_raw_height_delta
                };
            self.pump_time = if delta > 0.0 {
                self.pump_time + dt
            } else {
                0.0
            };
            if delta.abs() > maximum {
                let sign = if delta >= 0.0 {
                    if delta > 0.0 { 1.0 } else { 0.0 }
                } else {
                    -1.0
                };
                delta = sign * maximum;
            }
            let time_factor = attributes
                .keyed_graph(0xed5d_4acc_75c0_c47d, self.pump_time)
                .map_err(E::Attribute)?;
            let amount = time_factor * delta;
            self.pump_amount =
                self.try_calculate(position, normal, deck_angle, dt, attributes, numeric, debug)?;
            let key = if self.pump_amount * amount > 0.0 {
                0x1ad7_b592_6a86_dc10
            } else {
                0x4747_cb3d_449b_8780
            };
            let multiplier = scalar(attributes, key).map_err(E::Attribute)?;
            self.acceleration = (self.pump_amount * multiplier) * amount;
            let positive = scalar(attributes, 0xca49_d85f_a629_f7e7).map_err(E::Attribute)?;
            let upper = dt * positive;
            let negative = scalar(attributes, 0xced7_9378_3567_47bd).map_err(E::Attribute)?;
            let lower = -(negative * dt);
            let bounded = if lower - self.acceleration >= 0.0 {
                lower
            } else {
                self.acceleration
            };
            self.acceleration = if upper - bounded >= 0.0 {
                bounded
            } else {
                upper
            };
        }
        self.last_position = position;
        self.last_normal = normal;
        self.record_valid = true;
        self.last_height = height;
        debug.draw_pump().map_err(E::Debug)?;
        Ok(())
    }
}

impl PhysicalPlayerPumpingOwner {
    #[expect(
        clippy::too_many_arguments,
        reason = "native input vectors and separate retained services"
    )]
    pub(crate) fn try_update<A: PumpingAttributeRead, N: ReckoningNumeric, D: PumpingDebug>(
        &mut self,
        position: SkeletonRootVector,
        normal: SkeletonRootVector,
        com_to_deck: SkeletonRootVector,
        deck_angle: f32,
        dt: f32,
        attributes: &mut A,
        numeric: &mut N,
        debug: &mut D,
    ) -> UpdateResult<(), A::Error, N::Error, D::Error> {
        self.value
            .as_mut()
            .ok_or(PumpingUpdateError::Owner(PumpingError::NotConstructed))?
            .try_update(
                position,
                normal,
                com_to_deck,
                deck_angle,
                dt,
                attributes,
                numeric,
                debug,
            )
    }
}

#[cfg(test)]
#[path = "physical_player_pumping_update_tests.rs"]
mod tests;
