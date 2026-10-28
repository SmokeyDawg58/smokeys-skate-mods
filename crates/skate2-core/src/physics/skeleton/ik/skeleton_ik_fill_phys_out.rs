//! Unregistered child of `SkeletonIkState`: native Fill56590 and its two leaves.
use super::{SkeletonIkState, SkeletonRootVector};
use crate::{
    reckoning_numeric::ReckoningNumeric, skeleton_ik_attributes::SkeletonIkAttributeRead,
    skeleton_ik_geometry::SkeletonIkGeometry,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::enum_variant_names,
    reason = "source output names distinguish coordinate frames"
)]
pub(crate) enum IkFillVector {
    LeftDeckVelocity,
    RightDeckVelocity,
    LeftWorldVelocity,
    RightWorldVelocity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IkFillFlag {
    GrabbingLeft,
    GrabbingRight,
    LeftOnDeck,
    RightOnDeck,
    DontPush,
    DontPushOrBrake,
}
pub(crate) trait SkeletonIkFillOutput {
    type Error;
    type Guard;
    fn sample_guard(&mut self) -> Result<Self::Guard, Self::Error>;
    fn vector(
        &mut self,
        guard: Self::Guard,
        field: IkFillVector,
        value: SkeletonRootVector,
    ) -> Result<(), Self::Error>;
    fn flag(
        &mut self,
        guard: Self::Guard,
        field: IkFillFlag,
        value: impl FnOnce() -> bool,
    ) -> Result<(), Self::Error>;
}
pub(crate) trait SkeletonIkFillServices {
    type Error;
    type Global;
    fn capture_skeleton_global(&mut self) -> Result<Self::Global, Self::Error>;
    fn hand_grab_margin(&mut self, global: &Self::Global) -> Result<[f32; 4], Self::Error>;
    fn physics_column(
        &mut self,
        part: usize,
        column: usize,
    ) -> Result<SkeletonRootVector, Self::Error>;
    fn delta_time(&mut self) -> Result<f32, Self::Error>;
    fn physics_velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
    fn draw_contact_info(&mut self) -> Result<bool, Self::Error>;
    /// Each enabled source call independently requires the actual `DebugOverlay`,
    /// then invokes its verified shared BLR target.
    fn debug_overlay_noop(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SkeletonIkFillError<S, A, O, N> {
    Source(S),
    Attribute(A),
    Output(O),
    Numeric(N),
    MissingGeometry(&'static str),
}
impl<S: std::fmt::Debug, A: std::fmt::Debug, O: std::fmt::Debug, N: std::fmt::Debug>
    std::fmt::Display for SkeletonIkFillError<S, A, O, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "IK Fill source: {e:?}"),
            Self::Attribute(e) => write!(f, "IK Fill attribute: {e:?}"),
            Self::Output(e) => write!(f, "IK Fill output: {e:?}"),
            Self::Numeric(e) => write!(f, "IK Fill numeric: {e:?}"),
            Self::MissingGeometry(e) => write!(f, "IK Fill missing geometry: {e}"),
        }
    }
}

#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "native NOTGT includes unordered values"
)]
fn hand_inside(point: SkeletonRootVector, extent: [f32; 4]) -> bool {
    for axis in [1, 0, 2] {
        if !(extent[axis] > point.xyz[axis]) {
            return false;
        }
        let lower = f32::from_bits(extent[axis].to_bits() ^ 0x8000_0000);
        if !(point.xyz[axis] > lower) {
            return false;
        }
    }
    true
}
fn foot_inside(point: SkeletonRootVector, extent: [f32; 4]) -> bool {
    point.xyz[0].abs() < extent[0]
        && point.xyz[1].abs() < extent[1]
        && point.xyz[2].abs() < extent[2]
}
fn inverse(dt: f32, mut seed: f32) -> f32 {
    for _ in 0..2 {
        let residual = -seed.mul_add(dt, -1.0);
        seed = seed.mul_add(residual, seed);
    }
    seed
}
fn transpose_rows(
    i: SkeletonRootVector,
    j: SkeletonRootVector,
    k: SkeletonRootVector,
) -> [SkeletonRootVector; 3] {
    std::array::from_fn(|axis| {
        SkeletonRootVector::known([i.xyz[axis], j.xyz[axis], k.xyz[axis], 0.0])
    })
}
fn inverse_translation(
    rows: &[SkeletonRootVector; 3],
    position: SkeletonRootVector,
) -> SkeletonRootVector {
    let negative = position.map(|v| 0.0 - v); //native zero subtract, not sign-bit XOR
    let z = rows[2].map(|v| negative.xyz[2] * v);
    let y = rows[1].zip(z, |v, p| negative.xyz[1].mul_add(v, p));
    rows[0].zip(y, |v, p| negative.xyz[0].mul_add(v, p))
}
fn transform_position(
    rows: &[SkeletonRootVector; 3],
    translation: SkeletonRootVector,
    point: SkeletonRootVector,
) -> SkeletonRootVector {
    let x = rows[0].zip(translation, |v, p| v.mul_add(point.xyz[0], p));
    let y = rows[1].zip(x, |v, p| v.mul_add(point.xyz[1], p));
    rows[2].zip(y, |v, p| v.mul_add(point.xyz[2], p))
}

impl SkeletonIkState {
    #[expect(
        clippy::type_complexity,
        reason = "preserve independent source, attribute, output and numeric failures"
    )]
    pub(crate) fn try_fill_phys_out<
        S: SkeletonIkFillServices,
        A: SkeletonIkAttributeRead,
        O: SkeletonIkFillOutput,
        N: ReckoningNumeric,
    >(
        &mut self,
        geometry: &SkeletonIkGeometry,
        attributes: &mut A,
        services: &mut S,
        output: &mut O,
        numeric: &mut N,
    ) -> Result<(), SkeletonIkFillError<S::Error, A::Error, O::Error, N::Error>> {
        use SkeletonIkFillError as E;
        // HandGrabInfo82b4a270: both private matrices precede Global validation.
        // Native pointer capture a2a0 precedes loads a2a4/ac; the null branch is
        // a2b0. This immutable-global adapter has no observable callback between
        // that capture and these private loads; capture here includes validation.
        let left = self.part_to_deck[2].columns()[3];
        let right = self.part_to_deck[3].columns()[3];
        let global = services.capture_skeleton_global().map_err(E::Source)?;
        let width = geometry
            .box_half_width()
            .ok_or(E::MissingGeometry("half width"))?;
        let length = geometry
            .total_half_length()
            .ok_or(E::MissingGeometry("total half length"))?;
        let margin = services.hand_grab_margin(&global).map_err(E::Source)?;
        let dimensions = [width, 0.0, length, 0.0];
        let extent = std::array::from_fn(|lane| dimensions[lane] + margin[lane]);
        let grabbing_left = hand_inside(left, extent);
        let grabbing_right = hand_inside(right, extent);
        let guard = output.sample_guard().map_err(E::Output)?;
        output
            .flag(guard, IkFillFlag::GrabbingLeft, || grabbing_left)
            .map_err(E::Output)?;
        let guard = output.sample_guard().map_err(E::Output)?;
        output
            .flag(guard, IkFillFlag::GrabbingRight, || grabbing_right)
            .map_err(E::Output)?;
        self.try_fill_feet(geometry, attributes, services, output, numeric)?;
        // Each final flag is read only after its own guard has passed natively.
        let guard = output.sample_guard().map_err(E::Output)?;
        output
            .flag(guard, IkFillFlag::DontPush, || self.skateboard_dont_push)
            .map_err(E::Output)?;
        let guard = output.sample_guard().map_err(E::Output)?;
        output
            .flag(guard, IkFillFlag::DontPushOrBrake, || {
                self.tell_anim_dont_push_or_brake
            })
            .map_err(E::Output)
    }

    #[expect(
        clippy::type_complexity,
        reason = "preserve independent source, attribute, output and numeric failures"
    )]
    fn try_fill_feet<
        S: SkeletonIkFillServices,
        A: SkeletonIkAttributeRead,
        O: SkeletonIkFillOutput,
        N: ReckoningNumeric,
    >(
        &mut self,
        geometry: &SkeletonIkGeometry,
        attributes: &mut A,
        services: &mut S,
        output: &mut O,
        numeric: &mut N,
    ) -> Result<(), SkeletonIkFillError<S::Error, A::Error, O::Error, N::Error>> {
        use SkeletonIkFillError as E;
        let width = geometry
            .box_half_width()
            .ok_or(E::MissingGeometry("half width"))?;
        let length = geometry
            .total_half_length()
            .ok_or(E::MissingGeometry("total half length"))?;
        let margin = attributes
            .vector(0x77dd_00c1_8946_c6cf)
            .map_err(E::Attribute)?;
        let position = services.physics_column(0, 3).map_err(E::Source)?;
        let j = services.physics_column(0, 1).map_err(E::Source)?;
        let k = services.physics_column(0, 2).map_err(E::Source)?;
        let i = services.physics_column(0, 0).map_err(E::Source)?;
        let dt = services.delta_time().map_err(E::Source)?;
        let right = services.physics_column(19, 3).map_err(E::Source)?;
        let left = services.physics_column(15, 3).map_err(E::Source)?;
        let extent = std::array::from_fn(|lane| [width, 0.0, length, 0.0][lane] + margin[lane]);
        let rows = transpose_rows(i, j, k);
        let old_left = self.last_left_foot_position_deck;
        let left_world_velocity = services.physics_velocity(15).map_err(E::Source)?;
        let right_world_velocity = services.physics_velocity(19).map_err(E::Source)?;
        let seed_left = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
        let translation = inverse_translation(&rows, position);
        let left = transform_position(&rows, translation, left);
        let right = transform_position(&rows, translation, right);
        let inverse_left = inverse(dt, seed_left);
        let left_delta = left.zip(old_left, |a, b| a - b);
        let old_right = self.last_right_foot_position_deck;
        let right_delta = right.zip(old_right, |a, b| a - b);
        // Source3d94/98. Later numeric/output errors retain these SAME histories.
        self.last_left_foot_position_deck = left;
        self.last_right_foot_position_deck = right;
        let left_velocity = left_delta.map(|v| inverse_left * v);
        let seed_right = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
        let guard = output.sample_guard().map_err(E::Output)?; //3db8 before second refinements
        let inverse_right = inverse(dt, seed_right);
        let right_velocity = right_delta.map(|v| inverse_right * v);
        output
            .vector(guard, IkFillVector::LeftDeckVelocity, left_velocity)
            .map_err(E::Output)?;
        let guard = output.sample_guard().map_err(E::Output)?;
        output
            .vector(guard, IkFillVector::RightDeckVelocity, right_velocity)
            .map_err(E::Output)?;
        let guard = output.sample_guard().map_err(E::Output)?;
        output
            .vector(guard, IkFillVector::LeftWorldVelocity, left_world_velocity)
            .map_err(E::Output)?;
        let guard = output.sample_guard().map_err(E::Output)?;
        output
            .vector(
                guard,
                IkFillVector::RightWorldVelocity,
                right_world_velocity,
            )
            .map_err(E::Output)?;
        let on_left = foot_inside(left, extent);
        let on_right = foot_inside(right, extent);
        let guard = output.sample_guard().map_err(E::Output)?;
        output
            .flag(guard, IkFillFlag::LeftOnDeck, || on_left)
            .map_err(E::Output)?;
        let guard = output.sample_guard().map_err(E::Output)?;
        output
            .flag(guard, IkFillFlag::RightOnDeck, || on_right)
            .map_err(E::Output)?;
        if services.draw_contact_info().map_err(E::Source)? {
            for _ in 0..4 {
                services.debug_overlay_noop().map_err(E::Source)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "skeleton_ik_fill_phys_out_tests.rs"]
mod tests;
