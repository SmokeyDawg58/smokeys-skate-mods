//! Pathogen::NavRig::GetBestNavRay82e51b10, retaining output-pointer behavior.
use crate::{pedestrian_navigation::rig::{NavRig,RAY_ANGLES_DEGREES},
    reckoning_numeric::{ReckoningNumeric,cross_full4}};

impl NavRig {
    /// True means requested direction is usable; false selects ray_id fallback.
    /// max_turn_exceeded is advisory and does not reject that fallback ray.
    pub(crate) fn best_nav_ray<N: ReckoningNumeric>(&self, ray_id:&mut usize,
        requested:[f32;4], minimum_distance:f32, maximum_turn:f32,
        max_turn_exceeded:&mut bool, numeric:&mut N) -> Result<bool,String>
        where N::Error:std::fmt::Debug {
        *max_turn_exceeded = false;
        let dot = numeric.dot3(self.directions[0],requested)
            .map_err(|e|format!("best navigation ray dot: {e:?}"))?;
        // Native calls CRT DOUBLE acos then frsp, not Math::ACos's vector
        // polynomial and not a normalized/clamped dot-product substitute.
        let mut angle = (dot as f64).acos() as f32;
        let zero_direction = !requested[..3].iter().any(|v|v.abs()>f32::from_bits(0x3400_0000));
        if angle < f32::from_bits(0x3d56_7750) || zero_direction {
            // Raw ble rejects <= but allows unordered values through.
            if !(self.distances.distance(0)? <= minimum_distance) {
                return Ok(true); // original does NOT overwrite caller ray ID
            }
        }
        let (distance,closest,_) = self.distances.heading_distance(
            self.position,&self.directions,requested,numeric)?;
        if distance > minimum_distance { *ray_id=0; return Ok(true); }
        if !(self.distances.distance(closest)? <= minimum_distance) {
            *max_turn_exceeded=RAY_ANGLES_DEGREES[closest].abs()>maximum_turn;
            *ray_id=closest; return Ok(false);
        }
        if cross_full4(self.directions[0],requested)[1]<0. { angle *= -1.; }
        let mut last_minus=closest;
        let mut next_minus=(closest+6)%7;
        let mut next_plus=(closest+1)%7;
        loop {
            // Source compares stored DEGREE table values directly against the
            // signed acos radians. Preserve this observed mixed-unit graph.
            let minus_difference=(RAY_ANGLES_DEGREES[next_minus]-angle).abs();
            let plus_difference=(RAY_ANGLES_DEGREES[next_plus]-angle).abs();
            let minus=!(minus_difference>=plus_difference);
            let candidate=if minus { next_minus } else { next_plus };
            if self.distances.distance(candidate)?>minimum_distance {
                *ray_id=candidate;
                *max_turn_exceeded=RAY_ANGLES_DEGREES[candidate].abs()>maximum_turn;
                return Ok(false);
            }
            if minus { last_minus=candidate; next_minus=(candidate+6)%7; }
            else { next_plus=(candidate+1)%7; }
            if last_minus==next_plus { *ray_id=0; return Ok(false); }
        }
    }
}
