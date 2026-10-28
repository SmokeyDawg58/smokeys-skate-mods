//! PhysState_KnownAir::CalculateBodySpinSpeed82d8bbd0.
use super::{KnownAir, known_air_trajectory::KnownAirTrajectoryError as E};
use crate::{
    input_cvar_runtime::{InputCvarReferences, known_air::KnownAirCvar as C},
    physical_player_phys_out::construct::skeleton_fill_suffix::plane_angle,
    reckoning::grounded_input::{GroundInputRead, GroundScalar},
    reckoning_numeric::{ReckoningNumeric, try_normalize_angle},
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
};
const DEG: f32 = f32::from_bits(0x4265_2ee1);
const RAD: f32 = f32::from_bits(0x3c8e_fa35);
const PI: f32 = f32::from_bits(0x4049_0fdb);
fn fsel(test: f32, yes: f32, no: f32) -> f32 {
    if test >= 0.0 { yes } else { no }
}
impl KnownAir {
    pub(crate) fn scalar<N>(&self, key: u64) -> Result<f32, E<N>> {
        #[cfg(feature = "skate3-runtime")]
        if key == 0xa61c_64a0_d9cf_4982 {
            if let Some(owner) = self.base.physical.owner.upgrade() {
                if let Some(tuning) = &*owner.skate3_tuning.lock().map_err(|_| E::Missing("difficulty lock"))? {
                    return Ok(tuning.automatic_spin_speed);
                }
            }
        }
        Ok(self
            .attributes
            .as_ref()
            .ok_or(E::Missing("KnownAir attributes"))?
            .try_scalar_word(key, 0)
            .map_err(E::Attribute)?
            .map_or(0.0, f32::from_bits))
    }
    pub(crate) fn try_body_spin<N: ReckoningNumeric>(
        &self,
        p: &SkeletonProcessedContact,
        mut heading: V,
        up: V,
        current_speed: f32,
        refs: &InputCvarReferences,
        n: &mut N,
    ) -> Result<f32, E<N::Error>> {
        let scale = self.scalar(0xc84e_b5b5_a3d7_dd4a)?;
        // 82d8bc24 reads mBodySpin (+960), the animation's negated BodySpin, not the
        // reckoning's mPhysBodySpin (+9f8).
        let body_spin = p
            .animation_attribute_fields()
            .body_spin
            .ok_or(E::Missing("Processed body spin+960"))?;
        let manual = (body_spin * scale) * RAD;
        let result = if p.flag8c8(0x4000_0000).map_err(E::Input)? {
            manual
        } else {
            0.0
        };
        if manual != 0.0
            || !self
                .landing_heading_valid
                .ok_or(E::Missing("landing heading validity"))?
        {
            return Ok(result);
        }
        let fakie = p.flag8c4(0x0010_0000).map_err(E::Input)?;
        if fakie != self.start_flipped.ok_or(E::Missing("entry flipped"))? {
            heading = heading.map(|x| -x);
        }
        let mut angle = 0.0;
        if n.dot_vector(up, self.landing_normal.ok_or(E::Missing("landing normal"))?)
            .map_err(E::Numeric)?
            > 0.001
        {
            angle = plane_angle(
                heading,
                self.landing_heading.ok_or(E::Missing("landing heading"))?,
                up,
                n,
            )
            .map_err(E::Numeric)?;
            if fakie {
                angle += PI;
            }
        }
        let mut normalized = try_normalize_angle(angle, n).map_err(E::Numeric)?;
        let y = p
            .prior_output()
            .reckoning_up()
            .ok_or(E::Missing("Processed up220"))?
            .value
            .xyz[1];
        let data = self
            .attributes
            .as_ref()
            .ok_or(E::Missing("KnownAir attributes"))?
            .try_get_data(0xa563_44e7_f12b_6730, 0)
            .map_err(E::Attribute)?;
        let bytes = data.as_ref().map(|d| d.bytes()).unwrap_or(&[0; 80]);
        if bytes.len() < 80 {
            return Err(E::Missing("KnownAir alignment graph80"));
        }
        let word =
            |offset: usize| f32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let graph = crate::physics::PointGraph::<8> {
            x: std::array::from_fn(|i| word(16 + 4 * i)),
            y: std::array::from_fn(|i| word(48 + 4 * i)),
        };
        if (normalized * DEG).abs() > graph.evaluate(y) {
            let pi = try_normalize_angle(PI, n).map_err(E::Numeric)?;
            normalized = try_normalize_angle(pi + normalized, n).map_err(E::Numeric)?;
        }
        let remaining = self.coll_time.ok_or(E::Missing("collision time"))?
            - self.time_in_state.ok_or(E::Missing("state time"))?;
        let denominator = fsel(remaining - 0.032, remaining, 0.032);
        let inverse = 1.0 / denominator;
        let mut speed = (inverse * normalized) * 1.2;
        let maximum = self.scalar(0xa61c_64a0_d9cf_4982)?;
        let current = if current_speed.abs() < 0.001 {
            0.0
        } else {
            current_speed
        };
        if current * speed < 0.0 {
            let alternate = try_normalize_angle(angle + PI, n).map_err(E::Numeric)?;
            let limit = refs
                .try_known_air_float(C::FakieAlignAngle)
                .map_err(E::Cvar)?
                * RAD;
            if alternate.abs() < limit && (inverse * alternate) * current > 0.0 {
                speed = inverse * alternate;
            }
        }
        speed = fsel(-maximum - speed, -maximum, speed);
        speed = fsel(maximum - speed, speed, maximum);
        if speed.abs() < self.scalar(0xc11e_befd_1417_7944)? {
            speed *= 0.5;
        }
        if angle.abs() < self.scalar(0xc11e_befd_1417_7944)? {
            speed = (angle / p.scalar(GroundScalar::TimeDelta).map_err(E::Input)?) * 0.5;
        }
        if speed * current < 0.0 {
            speed = refs
                .try_known_air_float(C::AgainstSpinScalar)
                .map_err(E::Cvar)?
                * speed;
        } else if current.abs() > speed.abs() {
            speed *= 0.75;
        }
        Ok(speed)
    }
}
