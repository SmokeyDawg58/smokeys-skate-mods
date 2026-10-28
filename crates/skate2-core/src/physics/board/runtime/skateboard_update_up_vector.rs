//! Retained outer `Skateboard::UpdateUpVector` (0x82b26158).
//! Constructor execution and frame scheduling belong to the actual caller.
//! Attributes are borrowed services, never a second global binding in this state.

use crate::{
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate, try_normalize_safe},
    skateboard_contact_surface::normal_attributes::SkateboardGroundNormalAttributeRead,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::fmt;

#[cfg(test)]
#[path = "skateboard_update_up_vector_tests.rs"]
mod tests;

pub(crate) const SPEED_DAMPING: u64 = 0xab8f_93ba_080e_0cb3;
pub(crate) const CORRECTION_SCALAR: u64 = 0x9f15_310d_bbe9_6b8c;
pub(crate) const SPEED_DIVISOR: u64 = 0x20f1_eca6_3770_8494;
pub(crate) const DELTA_CAP_SCALAR: u64 = 0x30bf_4da4_bd7a_e00a;
const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);
const UP: SkeletonRootVector = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);

pub(crate) trait SkateboardUpdateUpInput {
    type Error;
    type Body;
    type Collision;
    type Processed;
    fn capture_body(&mut self) -> Result<Self::Body, Self::Error>;
    fn num_wheels(&mut self, body: &Self::Body) -> Result<u8, Self::Error>;
    fn capture_collision(&mut self) -> Result<Self::Collision, Self::Error>;
    fn in_contact(
        &mut self,
        collision: &Self::Collision,
        wheel: usize,
    ) -> Result<bool, Self::Error>;
    fn acceleration(
        &mut self,
        collision: &Self::Collision,
        wheel: usize,
    ) -> Result<SkeletonRootVector, Self::Error>;
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error>;
    fn surface_speed(&mut self, processed: &Self::Processed) -> Result<f32, Self::Error>;
    fn average_normal(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error>;
}

/// The inherited global/pointer interface is shared with the ground-normal
/// provider. Implementations retain both actual global class instances and
/// defer layout/value access to these source stages.
pub(crate) trait SkateboardUpdateUpAttributes: SkateboardGroundNormalAttributeRead {
    type PhysicsGlobal;
    type PhysicsLayout;
    type ReckoningLayout;
    fn capture_physics_global(&mut self) -> Result<Self::PhysicsGlobal, Self::Error>;
    fn capture_physics_layout(
        &mut self,
        global: &Self::PhysicsGlobal,
    ) -> Result<Self::PhysicsLayout, Self::Error>;
    fn world_gravity(
        &mut self,
        layout: &Self::PhysicsLayout,
    ) -> Result<SkeletonRootVector, Self::Error>;
    fn capture_reckoning_layout(
        &mut self,
        global: &Self::Global,
    ) -> Result<Self::ReckoningLayout, Self::Error>;
    fn evaluate_graph8(
        &mut self,
        layout: &Self::ReckoningLayout,
        offset: usize,
        input: f32,
    ) -> Result<f32, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SkateboardUpAlreadyInitialized;
impl fmt::Display for SkateboardUpAlreadyInitialized {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("outer up-vector constructor already executed")
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardUpdateUpError<I, A, N> {
    ConstructorRequired,
    Unready(&'static str),
    Input(I),
    Attribute(A),
    Numeric(N),
}
impl<I: fmt::Debug, A: fmt::Debug, N: fmt::Debug> fmt::Display
    for SkateboardUpdateUpError<I, A, N>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConstructorRequired => {
                f.write_str("outer up-vector constructor has not executed")
            }
            Self::Unready(field) => write!(f, "outer up-vector {field} is unready"),
            Self::Input(error) => write!(f, "outer up-vector input: {error:?}"),
            Self::Attribute(error) => write!(f, "outer up-vector attribute: {error:?}"),
            Self::Numeric(error) => write!(f, "outer up-vector numeric: {error:?}"),
        }
    }
}

pub(crate) struct SkateboardUpdateUpState {
    initialized: bool,
    pub(crate) dynamic_up: Option<SkeletonRootVector>,
    pub(crate) dynamic_speed: Option<SkeletonRootVector>,
    pub(crate) contact_acceleration: Option<SkeletonRootVector>,
    pub(crate) average_normal: Option<SkeletonRootVector>,
}
impl SkateboardUpdateUpState {
    pub(crate) const fn uninitialized() -> Self {
        Self {
            initialized: false,
            dynamic_up: None,
            dynamic_speed: None,
            contact_acceleration: None,
            average_normal: None,
        }
    }

    /// Actual outer constructor38aa0/38ab0/38ac0/38ad0. In particular SPEED
    /// starts at UP. Native `TeleportReset` does not repeat these writes.
    pub(crate) fn initialize_constructor(&mut self) -> Result<(), SkateboardUpAlreadyInitialized> {
        if self.initialized {
            return Err(SkateboardUpAlreadyInitialized);
        }
        self.dynamic_up = Some(UP);
        self.dynamic_speed = Some(UP);
        self.contact_acceleration = Some(ZERO);
        self.average_normal = Some(ZERO);
        self.initialized = true;
        Ok(())
    }

    #[expect(
        clippy::type_complexity,
        reason = "preserves separate input, attribute and numeric failure identities"
    )]
    pub(crate) fn try_update<
        I: SkateboardUpdateUpInput,
        A: SkateboardUpdateUpAttributes,
        N: ReckoningNumeric,
    >(
        &mut self,
        input: &mut I,
        attributes: &mut A,
        numeric: &mut N,
    ) -> Result<(), SkateboardUpdateUpError<I::Error, A::Error, N::Error>> {
        use SkateboardUpdateUpError as E;
        if !self.initialized {
            return Err(E::ConstructorRequired);
        }
        let body = input.capture_body().map_err(E::Input)?;
        if input.num_wheels(&body).map_err(E::Input)? == 0 {
            // 261c8 is +90 contact acceleration, NOT +80 dynamic speed.
            self.contact_acceleration = Some(ZERO);
            return Ok(());
        }
        let collision = input.capture_collision().map_err(E::Input)?;
        let mut sum = ZERO;
        let global = attributes.capture_physics_global().map_err(E::Attribute)?;
        let first_contact = input.in_contact(&collision, 0).map_err(E::Input)?;
        let physics = attributes
            .capture_physics_layout(&global)
            .map_err(E::Attribute)?;
        let gravity = attributes.world_gravity(&physics).map_err(E::Attribute)?;
        if first_contact {
            let acceleration = input.acceleration(&collision, 0).map_err(E::Input)?;
            sum = subtract(add(sum, acceleration), gravity);
        }
        // Native unrolled source visits ONLY wheels1 and2 after wheel0.
        for wheel in 1..3 {
            if input.in_contact(&collision, wheel).map_err(E::Input)? {
                let acceleration = input.acceleration(&collision, wheel).map_err(E::Input)?;
                sum = subtract(add(sum, acceleration), gravity);
            }
        }
        let target = normalize(sum, numeric).map_err(E::Numeric)?;
        let global = attributes.capture_global().map_err(E::Attribute)?;
        let damping = attributes
            .lookup(&global, SPEED_DAMPING)
            .map_err(E::Attribute)?;
        let damping = attributes.float(&damping).map_err(E::Attribute)?;
        let old_speed = self.dynamic_speed.ok_or(E::Unready("dynamic speed"))?;
        self.dynamic_speed = Some(scale(old_speed, damping)); // 26358

        let global = attributes.capture_global().map_err(E::Attribute)?;
        let old_up = self.dynamic_up.ok_or(E::Unready("dynamic up"))?;
        let correction = subtract(target, old_up);
        let scalar = attributes
            .lookup(&global, CORRECTION_SCALAR)
            .map_err(E::Attribute)?;
        let scalar = attributes.float(&scalar).map_err(E::Attribute)?;
        let processed = input.capture_processed().map_err(E::Input)?;
        let global = attributes.capture_global().map_err(E::Attribute)?;
        let speed = input.surface_speed(&processed).map_err(E::Input)?;
        let absolute_speed = f32::from_bits(speed.to_bits() & 0x7fff_ffff);
        let correction = scale(correction, scalar);
        let divisor = attributes
            .lookup(&global, SPEED_DIVISOR)
            .map_err(E::Attribute)?;
        // Native next-global capture26448 precedes the divisor load2644c.
        let global = attributes.capture_global().map_err(E::Attribute)?;
        let divisor = attributes.float(&divisor).map_err(E::Attribute)?;
        let ratio = absolute_speed / divisor;
        let cap_pointer = attributes
            .lookup(&global, DELTA_CAP_SCALAR)
            .map_err(E::Attribute)?;
        let global = attributes.capture_global().map_err(E::Attribute)?;
        let graph = attributes
            .capture_reckoning_layout(&global)
            .map_err(E::Attribute)?;
        let curve = attributes
            .evaluate_graph8(&graph, 0x460, ratio)
            .map_err(E::Attribute)?;
        let squared = numeric
            .dot3(scratch(correction), scratch(correction))
            .map_err(E::Numeric)?;
        // Deferred pointer D dereference is AFTER graph and dot, BEFORE estimate.
        let cap = attributes.float(&cap_pointer).map_err(E::Attribute)? * curve;
        let estimate = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        let length = length_from_squared_and_estimate(squared, estimate);
        let positive_length = select(f32::from_bits(length.to_bits() ^ 0x8000_0000), 0.0, length);
        let direction = normalize(correction, numeric).map_err(E::Numeric)?;
        let test = cap - positive_length;
        let old_speed = self.dynamic_speed.ok_or(E::Unready("damped speed"))?;
        let amount = select(test, positive_length, cap);
        let new_speed = multiply_add(direction, amount, old_speed);
        self.dynamic_speed = Some(new_speed); // 265b8
        let old_up = self.dynamic_up.ok_or(E::Unready("dynamic up"))?;
        self.dynamic_up = Some(add(old_up, new_speed)); // 265c4
        let normalized =
            normalize(self.dynamic_up.expect("just stored"), numeric).map_err(E::Numeric)?;
        self.dynamic_up = Some(normalized); // 265d8
        let body = input.capture_body().map_err(E::Input)?;
        self.average_normal = Some(input.average_normal(&body).map_err(E::Input)?); //265f8
        Ok(())
    }
}

fn scratch(vector: SkeletonRootVector) -> [f32; 4] {
    [
        vector.xyz[0],
        vector.xyz[1],
        vector.xyz[2],
        vector.w.unwrap_or(0.0),
    ]
}
fn add(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] + right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left + right),
    }
}
fn subtract(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] - right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left - right),
    }
}
fn scale(vector: SkeletonRootVector, factor: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: vector.xyz.map(|lane| lane * factor),
        w: vector.w.map(|lane| lane * factor),
    }
}
fn multiply_add(
    vector: SkeletonRootVector,
    factor: f32,
    old: SkeletonRootVector,
) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| vector.xyz[lane].mul_add(factor, old.xyz[lane])),
        w: vector
            .w
            .zip(old.w)
            .map(|(vector, old)| vector.mul_add(factor, old)),
    }
}
fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

/// Reuse the shared `NormalizeSafe` graph; observe which branch it executed.
/// Unknown scratch W never escapes a nonfallback normalization. A full-zero
/// source fallback establishes all four lanes even for unknown input W.
fn normalize<N: ReckoningNumeric>(
    vector: SkeletonRootVector,
    numeric: &mut N,
) -> Result<SkeletonRootVector, N::Error> {
    let mut tracker = Normalization {
        numeric,
        normalized: false,
    };
    let output = try_normalize_safe(scratch(vector), [0.0; 4], &mut tracker)?;
    let mut result = SkeletonRootVector::known(output);
    if tracker.normalized && vector.w.is_none() {
        result.w = None;
    }
    Ok(result)
}
struct Normalization<'a, N> {
    numeric: &'a mut N,
    normalized: bool,
}
impl<N: ReckoningNumeric> ReckoningNumeric for Normalization<'_, N> {
    type Error = N::Error;
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        self.numeric.dot3(left, right)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        let value = self.numeric.reciprocal_sqrt_estimate(value)?;
        self.normalized = true;
        Ok(value)
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.numeric.reciprocal_estimate(value)
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        self.numeric.truncate_i32(value)
    }
}
