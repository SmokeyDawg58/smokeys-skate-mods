//! Child of `skeleton_skateboard_wobble`: retained Update82b4af80.
use super::{SkeletonSkateboardWobble, WobbleGraph};
use crate::attribute_runtime::AttributeRuntimeError;

pub(crate) trait WobbleUpdateDebug {
    type Error;
    fn debug_draw_wobble(&mut self) -> Result<bool, Self::Error>;
    /// Actual enabled branch requires `DebugOverlay` before its debug calls.
    fn draw_wobble(&mut self, adjustment: &WobbleAdjustment) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WobbleAdjustment {
    pub(crate) do_adjust: bool,
    pub(crate) tilt: f32,
    pub(crate) squish: f32,
}
#[derive(Debug)]
pub(crate) enum WobbleUpdateError<D> {
    Attribute(AttributeRuntimeError),
    Missing(&'static str),
    GraphIntervalAssertion,
    Debug(D),
}
impl<D: std::fmt::Debug> std::fmt::Display for WobbleUpdateError<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(e) => write!(f, "Wobble duration: {e:?}"),
            Self::Missing(v) => write!(f, "Wobble missing {v}"),
            Self::GraphIntervalAssertion => f.write_str("Wobble graph interval assertion824e2ccc"),
            Self::Debug(e) => write!(f, "Wobble debug: {e:?}"),
        }
    }
}

/// `PointGraphEval824e2c40`. Ordered interval comparisons and BLE width guard
/// are intentional: NaN value returns last Y; raw BO4/BI25 selects upper Y for NaN width.
#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "native unordered terminal graph branch"
)]
fn evaluate<D>(graph: &WobbleGraph, value: f32) -> Result<f32, WobbleUpdateError<D>> {
    if value < graph.x[0] {
        return Ok(graph.y[0]);
    }
    if !(value < graph.x[7]) {
        return Ok(graph.y[7]);
    }
    let upper = (1..8)
        .find(|&i| value < graph.x[i])
        .ok_or(WobbleUpdateError::GraphIntervalAssertion)?;
    let lower = upper - 1;
    let width = graph.x[upper] - graph.x[lower];
    if !(width > 0.0) {
        return Ok(graph.y[upper]);
    }
    let delta = value - graph.x[lower];
    let slope = (graph.y[upper] - graph.y[lower]) / width;
    Ok(slope.mul_add(delta, graph.y[lower]))
}

impl SkeletonSkateboardWobble {
    /// Returns actual final active state; callers MUST use `out.do_adjust` to
    /// decide whether this call produced an adjustment, including expiry calls.
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "native unordered expiry branch"
    )]
    pub(crate) fn try_update_retained<D: WobbleUpdateDebug>(
        &mut self,
        out: &mut WobbleAdjustment,
        debug: &mut D,
    ) -> Result<bool, WobbleUpdateError<D::Error>> {
        use WobbleUpdateError as E;
        let data = self
            .attributes
            .try_get_data(0x4fc2_2a8e_5b8b_3676, 0)
            .map_err(E::Attribute)?;
        // Native captures activityafc4 before dereferencing durationafc8.
        let active = self.active.ok_or(E::Missing("activity"))?;
        let duration = match data {
            None => 0.0, // actual DefaultDataArea4
            Some(data) => {
                let bytes = data.bytes();
                if bytes.len() < 4 {
                    return Err(E::Attribute(AttributeRuntimeError::PayloadSize {
                        key: 0x4fc2_2a8e_5b8b_3676,
                        expected: 4,
                        actual: bytes.len(),
                    }));
                }
                f32::from_be_bytes(bytes[..4].try_into().expect("four bytes"))
            }
        };
        if !active {
            out.do_adjust = false;
            out.squish = 0.0;
            out.tilt = 0.0;
            return Ok(false);
        }
        let squish = evaluate(
            self.squish.as_ref().ok_or(E::Missing("squish graph"))?,
            self.time.ok_or(E::Missing("time"))?,
        )?;
        out.squish = self.strength.ok_or(E::Missing("strength"))? * squish;
        let tilt = evaluate(
            self.tilt.as_ref().ok_or(E::Missing("tilt graph"))?,
            self.time.ok_or(E::Missing("time"))?,
        )?;
        let strength = self.mirrored_scalar.ok_or(E::Missing("mirror"))?
            * self.strength.ok_or(E::Missing("strength"))?;
        out.do_adjust = true;
        out.tilt = strength * tilt;
        if debug.debug_draw_wobble().map_err(E::Debug)? {
            debug.draw_wobble(out).map_err(E::Debug)?;
        }
        let time = self.time.ok_or(E::Missing("time"))? + f32::from_bits(0x3c88_8889);
        self.time = Some(time);
        if !(time <= duration) {
            self.active = Some(false);
            self.time = Some(0.0);
        }
        Ok(self.active.expect("activity read above"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn graph() -> WobbleGraph {
        WobbleGraph {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 7.0,
            max_y: 7.0,
            x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            y: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        }
    }
    #[test]
    fn graph_nan_value_returns_last_and_nan_width_selects_upper() {
        let mut g = graph();
        assert_eq!(
            evaluate::<()>(&g, f32::NAN).unwrap().to_bits(),
            7.0f32.to_bits()
        );
        g.x[0] = f32::NAN;
        assert_eq!(evaluate::<()>(&g, 0.5).unwrap().to_bits(), 1.0f32.to_bits());
        assert_eq!(
            evaluate::<()>(&graph(), 0.5).unwrap().to_bits(),
            0.5f32.to_bits()
        );
    }
}
