//! Source ordered `CalcLandingQuality`82e2aa88 geometry, live tuning and stores.
use super::super::{
    PhysOutComponent, PhysOutMotionVector, PhysicalPlayerPhysOut, SkeletonRootVector as V,
};
use super::prior_input::PriorInputReadError;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    input_cvar_runtime::{InputCvarError, landing_quality::LandingCvars},
    reckoning_numeric::ReckoningNumeric,
};
#[derive(Debug)]
pub(crate) enum LandingQualityError<E> {
    ForeignOutput,
    MissingPhysics,
    Output(PriorInputReadError),
    Numeric(E),
    Cvar(InputCvarError),
    Attribute(AttributeRuntimeError),
    MissingGlobalAnimation,
    MissingAnimationLayout,
    GraphIntervalAssertion,
}
impl<E: std::fmt::Debug> std::fmt::Display for LandingQualityError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignOutput => f.write_str("landing-quality foreign output"),
            Self::MissingPhysics => f.write_str("landing-quality missing Physics conditioner"),
            Self::Output(e) => write!(f, "landing-quality output: {e:?}"),
            Self::Numeric(e) => write!(f, "landing-quality numeric: {e:?}"),
            Self::Cvar(e) => write!(f, "landing-quality cvar: {e:?}"),
            Self::Attribute(e) => write!(f, "landing-quality attribute: {e:?}"),
            Self::MissingGlobalAnimation => {
                f.write_str("landing-quality requires actual global physics_animation.default")
            }
            Self::MissingAnimationLayout => {
                f.write_str("landing-quality requires retained physics_animation layout")
            }
            Self::GraphIntervalAssertion => {
                f.write_str("landing-quality point graph assertion824e2ccc")
            }
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn landing_guard(&self, component: PhysOutComponent) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "preserve one source callback's lazy input and six output write order"
    )]
    #[expect(
        clippy::if_not_else,
        reason = "retain the original landing branch and store order"
    )]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "native BO4/BI25 selects Sketchy for unordered spin"
    )]
    pub(crate) fn try_calc_landing_quality<N: ReckoningNumeric>(
        &mut self,
        cvars: &LandingCvars<'_>,
        global: Option<&AttributeRuntimeInstance>,
        numeric: &mut N,
    ) -> Result<(), LandingQualityError<N::Error>> {
        use LandingQualityError as E;
        self.landing_guard(PhysOutComponent::Collision)
            .map_err(E::Output)?;
        let normal = self
            .concrete
            .collision
            .as_ref()
            .unwrap()
            .average_ground_normal;
        self.landing_guard(PhysOutComponent::Motion)
            .map_err(E::Output)?;
        let velocity_owner = &self.motion_vectors[PhysOutMotionVector::DeckVelocity as usize];
        self.landing_guard(PhysOutComponent::Motion)
            .map_err(E::Output)?;
        let flipped = self.concrete.motion.as_ref().unwrap().skateboard_flipped;
        self.landing_guard(PhysOutComponent::SkateboardReckoning)
            .map_err(E::Output)?;
        let board_owner = self.concrete.skateboard_reckoning.as_ref().unwrap();
        let velocity = velocity_owner.as_ref().unwrap().value;
        let velocity_normal = numeric.dot_vector(normal, velocity).map_err(E::Numeric)?;
        let board = board_owner.transform.columns()[2];
        let board_normal = numeric.dot_vector(normal, board).map_err(E::Numeric)?;
        let projected_velocity = zip(velocity, scale(normal, velocity_normal), |a, b| a - b);
        let projected_board = zip(board, scale(normal, board_normal), |a, b| a - b);
        let velocity_squared = numeric
            .dot_vector(projected_velocity, projected_velocity)
            .map_err(E::Numeric)?;
        let board_squared = numeric
            .dot_vector(projected_board, projected_board)
            .map_err(E::Numeric)?;
        let cosine = numeric
            .dot_vector(projected_velocity, projected_board)
            .map_err(E::Numeric)?;
        let velocity_seed = numeric
            .reciprocal_sqrt_estimate(velocity_squared)
            .map_err(E::Numeric)?;
        let board_seed = numeric
            .reciprocal_sqrt_estimate(board_squared)
            .map_err(E::Numeric)?;
        let mut cosine = cosine.abs();
        let speed = length(velocity_squared, velocity_seed);
        let board_length = length(board_squared, board_seed);
        let side = cross(projected_board, normal);
        let product = board_length * speed;
        let mut side_speed = numeric
            .dot_vector(side, velocity)
            .map_err(E::Numeric)?
            .abs();
        // Native BO4/BI24 skips the degenerate path on !LT, including NaN.
        let (result, scalar) = if product < f32::from_bits(0x3727_c5ac) {
            cosine = 0.0;
            side_speed = 0.0;
            (0, 0.0)
        } else {
            let velocity_seed = numeric.reciprocal_estimate(speed).map_err(E::Numeric)?;
            let board_seed = numeric
                .reciprocal_estimate(board_length)
                .map_err(E::Numeric)?;
            let velocity = scale(projected_velocity, reciprocal(speed, velocity_seed));
            let board = scale(projected_board, reciprocal(board_length, board_seed));
            let angle = cross(board, velocity).xyz[1];
            let angle = if flipped { -angle } else { angle };
            self.landing_guard(PhysOutComponent::Air)
                .map_err(E::Output)?;
            let spin = -self.concrete.air.as_ref().unwrap().body_spin_speed;
            let lo = cvars.capture(206).map_err(E::Cvar)?;
            let hi = cvars.capture(207).map_err(E::Cvar)?;
            let abs_spin = spin.abs();
            let lo = LandingCvars::read(&lo).map_err(E::Cvar)?;
            let hi = LandingCvars::read(&hi).map_err(E::Cvar)?;
            let width = hi - lo;
            let spin_fraction = (abs_spin - lo) / width;
            let spin_response = graph(global, 0x170, spin_fraction)?;
            let lo = cvars.capture(208).map_err(E::Cvar)?;
            let hi = cvars.capture(209).map_err(E::Cvar)?;
            let lo = LandingCvars::read(&lo).map_err(E::Cvar)?;
            let hi = LandingCvars::read(&hi).map_err(E::Cvar)?;
            let delta = side_speed - lo;
            let side_fraction = delta / (hi - lo);
            let side_response = graph(global, 0x190, side_fraction)?;
            let speed_threshold = cvars.value(210).map_err(E::Cvar)?;
            if speed < speed_threshold {
                (0, 0.0)
            } else {
                let board = if flipped {
                    board.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000))
                } else {
                    board
                };
                let alignment = numeric.dot_vector(board, velocity).map_err(E::Numeric)?;
                let aligned = alignment >= 0.0;
                let spin_threshold = cvars.value(205).map_err(E::Cvar)?;
                let (mut result, response) = if !(abs_spin > spin_threshold) {
                    (1, side_response)
                } else {
                    let spinning_with_angle = spin * angle > 0.0;
                    let result = if aligned != spinning_with_angle { 2 } else { 1 };
                    let response = if spin_response - side_response >= 0.0 {
                        spin_response
                    } else {
                        side_response
                    };
                    (result, response)
                };
                let threshold = cvars.capture(211).map_err(E::Cvar)?;
                let mut scalar = if spin >= 0.0 { response } else { -response };
                let threshold = LandingCvars::read(&threshold).map_err(E::Cvar)?;
                if angle.abs() < threshold {
                    let force = cvars.value(212).map_err(E::Cvar)?;
                    if !(result == 2 && scalar.abs() > force) {
                        scalar = 0.0;
                        result = 0;
                    }
                }
                (result, scalar)
            }
        };
        self.landing_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?;
        self.concrete
            .animation
            .as_mut()
            .unwrap()
            .just_analyzed_sketchyness = true;
        self.landing_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?;
        self.concrete
            .animation
            .as_mut()
            .unwrap()
            .sketchyness_anim_result = result;
        self.landing_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?;
        self.concrete
            .animation
            .as_mut()
            .unwrap()
            .sketchyness_anim_scalar = scalar;
        self.landing_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?;
        self.concrete.animation.as_mut().unwrap().sketchyness_cosine = cosine;
        self.landing_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?;
        self.concrete
            .animation
            .as_mut()
            .unwrap()
            .sketchyness_side_speed = side_speed;
        self.landing_guard(PhysOutComponent::Animation)
            .map_err(E::Output)?;
        self.concrete.animation.as_mut().unwrap().sketchyness = scalar;
        Ok(())
    }
}
fn zip(a: V, b: V, f: impl Fn(f32, f32) -> f32) -> V {
    V {
        xyz: std::array::from_fn(|i| f(a.xyz[i], b.xyz[i])),
        w: a.w.zip(b.w).map(|(a, b)| f(a, b)),
    }
}
fn scale(a: V, b: f32) -> V {
    a.map(|a| a * b)
}
fn cross(a: V, b: V) -> V {
    let perm = |v: V| V {
        xyz: [v.xyz[1], v.xyz[2], v.xyz[0]],
        w: v.w,
    };
    let product = zip(a, perm(b), |a, b| a * b);
    let ap = perm(a);
    perm(V {
        xyz: std::array::from_fn(|i| -ap.xyz[i].mul_add(b.xyz[i], -product.xyz[i])),
        w: ap
            .w
            .zip(b.w)
            .zip(product.w)
            .map(|((a, b), c)| -a.mul_add(b, -c)),
    })
}
fn length(q: f32, seed: f32) -> f32 {
    let refine = |s: f32| {
        let square = s * s;
        let half = s * 0.5;
        let residual = -q.mul_add(square, -1.0);
        half.mul_add(residual, s)
    };
    let estimate = refine(refine(seed));
    let value = q * estimate;
    if q == 0.0 { 0.0 } else { value }
}
fn reciprocal(x: f32, seed: f32) -> f32 {
    let refine = |s: f32| s.mul_add(-s.mul_add(x, -1.0), s);
    refine(refine(seed))
}
fn graph<NE>(
    global: Option<&AttributeRuntimeInstance>,
    offset: usize,
    fraction: f32,
) -> Result<f32, LandingQualityError<NE>> {
    use LandingQualityError as E;
    let global = global.ok_or(E::MissingGlobalAnimation)?;
    let layout = global
        .try_cached_layout(0x220)
        .map_err(E::Attribute)?
        .ok_or(E::MissingAnimationLayout)?;
    let bytes = layout.bytes();
    let read = |at| f32::from_bits(u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()));
    let x: [f32; 4] = std::array::from_fn(|i| read(offset + i * 4));
    let y: [f32; 4] = std::array::from_fn(|i| read(offset + 16 + i * 4));
    // FSEL upper bound only. NaN fraction selects1; negative values stay negative.
    let value = if 1.0 - fraction >= 0.0 { fraction } else { 1.0 };
    if value < x[0] {
        return Ok(y[0]);
    }
    if !matches!(value.partial_cmp(&x[3]), Some(std::cmp::Ordering::Less)) {
        return Ok(y[3]);
    }
    let upper = (1..4)
        .find(|&i| value < x[i])
        .ok_or(E::GraphIntervalAssertion)?;
    let lower = upper - 1;
    let width = x[upper] - x[lower];
    if !matches!(width.partial_cmp(&0.0), Some(std::cmp::Ordering::Greater)) {
        return Ok(y[upper]);
    }
    let delta = value - x[lower];
    let slope = (y[upper] - y[lower]) / width;
    Ok(slope.mul_add(delta, y[lower]))
}
