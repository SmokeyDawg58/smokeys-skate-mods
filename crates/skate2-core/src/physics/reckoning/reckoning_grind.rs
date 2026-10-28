//! `Reckoning::UpdateGrindStates` 0x82ddd2a8 on the retained filters/BodySpin.
use super::{
    Reckoning, ReckoningCvars, ReckoningError, RunResult, Vector, ZERO, blend_fma,
    grounded_input::{GroundInputRead, GroundScalar},
    numeric, scale,
};
use crate::{reckoning_attributes::ReckoningAttributeRead, reckoning_numeric::ReckoningNumeric};

impl<A: ReckoningAttributeRead> Reckoning<A> {
    /// Selected OffBoard caller supplies world-up normal, live Skeleton Z,
    /// smoothing 0.5, and its true body-spin input branch.
    pub(crate) fn try_update_grind_states<
        I: GroundInputRead,
        C: ReckoningCvars,
        N: ReckoningNumeric,
    >(
        &mut self,
        input: &I,
        normal: Vector,
        heading: Vector,
        smoothing: f32,
        in_air: bool,
        cvars: &mut C,
        numeric: &mut N,
    ) -> RunResult<A::Error, C::Error, N::Error> {
        if self.is_flipping {
            return Err(ReckoningError::FlippingAssertion);
        }
        let old_up = self.up;
        self.dynamic_up = old_up;
        self.dynamic_lean = 0.0;
        self.ground_blend = 0.0;
        self.up_speed = Vector::known(ZERO);
        self.update_ground_normal::<C::Error, N>(normal, numeric)?;
        let inverse_smoothing = 1.0 - smoothing;
        self.heading = heading;
        let first = scale(normal, inverse_smoothing);
        let blended = blend_fma(old_up, smoothing, first);
        let next = numeric::try_normalize_safe(blended, old_up, numeric)
            .map_err(ReckoningError::Numeric)?;
        self.up = next;
        self.up_target = next;
        self.maintain_filters();
        self.calculate_transform(numeric)
            .map_err(ReckoningError::Numeric)?;
        self.calculate_tilt(input, cvars, numeric)?;
        let input_spin = if in_air {
            input
                .scalar(GroundScalar::BodySpin)
                .map_err(ReckoningError::Input)?
        } else {
            0.0
        };
        self.body_spin
            .try_update_grounded(input_spin, 0.0, cvars)
            .map_err(ReckoningError::Cvar)?;
        Ok(())
    }
}
