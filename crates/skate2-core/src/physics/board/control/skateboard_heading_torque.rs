//! UNREGISTERED Entry child, complete HeadingTorque82b30cc8..315a8.
use super::SkateboardEntryState;
use crate::{
    reckoning_numeric::{
        ReckoningNumeric, cross_typed, try_acos, try_normalize_safe_with_refinement,
    },
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};
#[derive(Clone, Copy, Debug)]
pub(crate) enum HeadingGlobal {
    Manual,
    Heading,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum HeadingScalar {
    Balance,
    ManualTurn,
    DeckSpeed,
    TimeDelta,
    ForwardSpeed,
    VertTurn,
    HeadingFactor,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum HeadingVector {
    GroundNormal,
    ZAxisVelocity,
    AngularVelocity,
}
pub(crate) trait HeadingTorqueSource {
    type Error;
    type Graph;
    fn scalar(&mut self, field: HeadingScalar) -> Result<f32, Self::Error>;
    fn vector(&mut self, field: HeadingVector) -> Result<Vector, Self::Error>;
    fn flags_8c8(&mut self) -> Result<u32, Self::Error>;
    fn attribute(&mut self, global: HeadingGlobal, key: u64) -> Result<f32, Self::Error>;
    fn capture_graph(
        &mut self,
        global: HeadingGlobal,
        offset: usize,
    ) -> Result<Self::Graph, Self::Error>;
    fn evaluate_graph(&mut self, graph: &Self::Graph, input: f32) -> Result<f32, Self::Error>;
    fn ground_transform(&mut self) -> Result<Matrix, Self::Error>;
    fn draw_vert_turning(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum HeadingTorqueError<S, N> {
    Source(S),
    Numeric(N),
    NegativeAngle { bits: u32 },
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for HeadingTorqueError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "heading torque source: {e:?}"),
            Self::Numeric(e) => write!(f, "heading torque numeric: {e:?}"),
            Self::NegativeAngle { bits } => {
                write!(f, "heading torque negative normalized angle: {bits:08x}")
            }
        }
    }
}
fn select(test: f32, yes: f32, no: f32) -> f32 {
    if test >= 0.0 { yes } else { no }
}
fn refine(squared: f32, seed: f32) -> f32 {
    let residual = -(squared.mul_add(seed * seed, -1.0));
    let first = (seed * 0.5).mul_add(residual, seed);
    let residual = -(squared.mul_add(first * first, -1.0));
    (first * 0.5).mul_add(residual, first)
}
fn linear(matrix: &Matrix, v: Vector) -> Vector {
    // Inverse producer establishes each basis W as zero. Still execute every
    // product/FMA; a zero coefficient does not elide exceptional source values.
    let c = matrix.columns();
    let lane = |i: usize| {
        let x = c[0].xyz[i] * v.xyz[0];
        let xy = c[1].xyz[i].mul_add(v.xyz[1], x);
        c[2].xyz[i].mul_add(v.xyz[2], xy)
    };
    let w = c[0].w.zip(c[1].w).zip(c[2].w).map(|((x, y), z)| {
        let x = x * v.xyz[0];
        let xy = y.mul_add(v.xyz[1], x);
        z.mul_add(v.xyz[2], xy)
    });
    Vector {
        xyz: std::array::from_fn(lane),
        w,
    }
}
fn graph<S: HeadingTorqueSource>(
    s: &mut S,
    g: HeadingGlobal,
    offset: usize,
    input: f32,
) -> Result<f32, S::Error> {
    let captured = s.capture_graph(g, offset)?;
    s.evaluate_graph(&captured, input)
}
impl SkateboardEntryState {
    #[expect(
        clippy::too_many_lines,
        reason = "complete two-arm native heading producer with ordered retained writes"
    )]
    pub(crate) fn try_heading_torque<S: HeadingTorqueSource, N: ReckoningNumeric>(
        &mut self,
        source: &mut S,
        numeric: &mut N,
        output: &mut Vector,
    ) -> Result<(), HeadingTorqueError<S::Error, N::Error>> {
        use HeadingGlobal::{Heading, Manual};
        use HeadingScalar as F;
        use HeadingTorqueError as E;
        use HeadingVector as V;
        let balance = source.scalar(F::Balance).map_err(E::Source)?;
        if balance != 0.0 && source.scalar(F::ManualTurn).map_err(E::Source)? != 0.0 {
            let swap = source
                .attribute(Manual, 0x02b0_8c48_a9f1_3390)
                .map_err(E::Source)?;
            let flags = source.flags_8c8().map_err(E::Source)?;
            let mut turn = source.scalar(F::ManualTurn).map_err(E::Source)?;
            if (flags & 0x1000_0000 != 0 && balance < 0.0)
                || (flags & 0x0800_0000 != 0 && balance > 0.0)
            {
                turn *= swap;
            }
            let blend = source
                .attribute(Manual, 0xe56a_c6f5_81f2_799c)
                .map_err(E::Source)?;
            let captured = source.capture_graph(Manual, 0x10).map_err(E::Source)?;
            let speed = source.scalar(F::DeckSpeed).map_err(E::Source)?;
            let target = source.evaluate_graph(&captured, speed).map_err(E::Source)?;
            let old = self.manual_spin_angle;
            let radians = target * f32::from_bits(0x3c8e_fa35);
            let remaining = 1.0 - blend;
            let contribution = radians * blend;
            let smoothed = remaining.mul_add(old, contribution);
            self.manual_spin_angle = smoothed; //30ebc, before DT/normal reads
            let dt = source.scalar(F::TimeDelta).map_err(E::Source)?;
            let normal = source.vector(V::GroundNormal).map_err(E::Source)?;
            let impulse = dt * smoothed;
            let magnitude = impulse * turn;
            *output = normal.map(|v| v * magnitude);
            return Ok(());
        }
        self.manual_spin_angle = 0.0; //30f00, before transform payload
        let inverse = source.ground_transform().map_err(E::Source)?.inverse();
        // Same rigid transpose and Z/Y/X inverse-position graph as sourcef24..fd0.
        let up_y = inverse.columns()[1].xyz[1];
        let lower = select(-1.0 - up_y, -1.0, up_y);
        let cosine = select(1.0 - lower, lower, 1.0);
        let down = linear(&inverse, Vector::known([0.0, -1.0, 0.0, 0.0]));
        let mut angle =
            f32::from_bits(0x3f22_f983) * try_acos(cosine, numeric).map_err(E::Numeric)?;
        if angle < 0.0 {
            return Err(E::NegativeAngle {
                bits: angle.to_bits(),
            });
        }
        if angle > f32::from_bits(0x3f99_999a) {
            angle = 0.0;
        }
        let mut flat = down;
        flat.xyz[1] = 0.0;
        let direction =
            try_normalize_safe_with_refinement(flat, Vector::known([0.0; 4]), numeric, refine)
                .map_err(E::Numeric)?;
        let tilt = graph(source, Heading, 0x210, angle).map_err(E::Source)?;
        let speed = source.scalar(F::ForwardSpeed).map_err(E::Source)?;
        let velocity = source.vector(V::ZAxisVelocity).map_err(E::Source)?;
        let absolute = speed.abs();
        let mut local = linear(&inverse, velocity);
        let scaled = direction.map(|v| v * tilt);
        local.xyz[1] = 0.0;
        let torque = cross_typed(local, scaled);
        let maximum = source
            .attribute(Heading, 0x97bb_45bb_a1c3_68c7)
            .map_err(E::Source)?;
        let lower = select(-absolute, 0.0, absolute);
        let bounded = select(maximum - lower, lower, maximum);
        // Same attribute looked up again, not the previously captured value.
        let divisor = source
            .attribute(Heading, 0x97bb_45bb_a1c3_68c7)
            .map_err(E::Source)?;
        let ratio = bounded / divisor;
        let speed_factor = graph(source, Heading, 0x1d0, ratio).map_err(E::Source)?;
        let coefficient = source
            .attribute(Heading, 0xdc4d_df76_5f8d_599f)
            .map_err(E::Source)?;
        let turn = source.scalar(F::VertTurn).map_err(E::Source)?; //312fc before old force products
        let old = torque.xyz[1] * coefficient;
        let old = old * speed_factor;
        let turn_coefficient = source
            .attribute(Heading, 0x3a71_6c5b_e8bd_0df0)
            .map_err(E::Source)?;
        let negative_turn = -turn;
        let normal = source.vector(V::GroundNormal).map_err(E::Source)?;
        let angular_velocity = source.vector(V::AngularVelocity).map_err(E::Source)?;
        let angular = numeric
            .dot_vector(angular_velocity, normal)
            .map_err(E::Numeric)?;
        let opposed = angular * negative_turn;
        let positive = select(opposed, opposed, 0.0);
        let feedback = graph(source, Heading, 0x110, positive).map_err(E::Source)?;
        let factor = source.scalar(F::HeadingFactor).map_err(E::Source)?;
        let scaled = factor * feedback;
        let scaled = scaled * turn_coefficient;
        let candidate = scaled * turn;
        // Raw3142c BLT then3143c BGT: unordered falls through to old value.
        #[expect(
            clippy::neg_cmp_op_on_partial_ord,
            reason = "native NOT LT and NOT GT deliberately retain unordered values"
        )]
        let result = if !(old * candidate < 0.0) && !(candidate.abs() > old.abs()) {
            old
        } else {
            candidate
        };
        let normal = source.vector(V::GroundNormal).map_err(E::Source)?;
        let draw = source.draw_vert_turning().map_err(E::Source)?; //3146c BEFORE calculation
        let result = normal.map(|v| v * result);
        if draw {
            source.require_overlay().map_err(E::Source)?;
        }
        *output = result; //31588 AFTER overlay (unlike AntiFlip)
        Ok(())
    }
}

#[cfg(test)]
#[path = "skateboard_heading_torque_tests.rs"]
mod tests;
