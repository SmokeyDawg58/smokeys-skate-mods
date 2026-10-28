//! `Skateboard::UpdateAIPathFollowing` 0x82b35fe8, which the ground states' `UpdatePostPhysics`
//! run when the AI controller's `AIPhysicsInput` asks for an adjust: the deck moves a blended
//! share of the way to the path's position, velocity and heading, each share capped by
//! `physics_ai.default`.
use crate::{
    ai::actor::physics_flags,
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    reckoning_numeric::{
        ReckoningNumeric, length_from_squared_and_estimate, try_normalize_safe, try_signed_angle,
    },
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};
use std::sync::Arc;

pub(crate) const PHYSICS_AI_CLASS: u64 = 0x527c_93f5_5cfc_663d;
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
/// Inline `physics_ai` attributes: the position blend, and the heading cap in degrees.
const POSITION_BLEND: u64 = 0x0cda_debe_b684_d3fc;
const MAX_HEADING_ADJUST_DEGREES: u64 = 0xff6f_2622_d3f9_b765;
/// The layout: maximum velocity adjust, velocity blend, maximum position adjust, heading blend.
const LAYOUT_SIZE: usize = 16;
const MAX_VELOCITY_ADJUST: usize = 0;
const VELOCITY_BLEND: usize = 4;
const MAX_POSITION_ADJUST: usize = 8;
const HEADING_BLEND: usize = 12;
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
const TWO_PI_RECIPROCAL: f32 = f32::from_bits(0x3e22_f983);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
const K: [f32; 4] = [0.0, 0.0, 1.0, 0.0];
const J: [f32; 4] = [0.0, 1.0, 0.0, 0.0];

/// The skateboard the adjust moves.
pub(crate) trait AiPathFollowingBoard {
    type Error;
    /// `Body::GetPosition(6)` 0x82de7b20.
    fn deck_part_transform(&mut self) -> Result<Matrix, Self::Error>;
    /// `Body::SetPosition(.., 6)` 0x82de8090.
    fn set_deck_part_position(&mut self, matrix: &Matrix) -> Result<(), Self::Error>;
    /// `Skateboard::GetDeckVelocity` 0x82b25ff8.
    fn deck_velocity(&mut self) -> Result<[f32; 3], Self::Error>;
    /// The deck body's `mVel`.
    fn set_deck_body_velocity(&mut self, velocity: [f32; 3]) -> Result<(), Self::Error>;
    /// `mReckoning->mGroundTransform`.
    fn ground_transform(&mut self) -> Result<Matrix, Self::Error>;
    /// `SkateboardBody::SetTransform` 0x82b34a90.
    fn set_board_transform(&mut self, matrix: Matrix) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub(crate) enum PhysicsAiAttributeError {
    Attribute(AttributeRuntimeError),
    MissingLayout,
}

#[derive(Debug)]
pub(crate) enum AiPathFollowingError<B, N> {
    Board(B),
    Attributes(PhysicsAiAttributeError),
    Numeric(N),
    /// `mutators.cpp`'s `maxAdjust >= 0` assertion.
    NegativeMaximum { bits: u32 },
}

impl<B, N> From<PhysicsAiAttributeError> for AiPathFollowingError<B, N> {
    fn from(error: PhysicsAiAttributeError) -> Self {
        Self::Attributes(error)
    }
}

impl<B: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for AiPathFollowingError<B, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AI path following: {self:?}")
    }
}

/// The `physics_ai.default` collection the adjust reads.
pub(crate) struct PhysicsAiAttributes {
    instance: AttributeRuntimeInstance,
}

impl PhysicsAiAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self { instance: vault.instance(PHYSICS_AI_CLASS, DEFAULT_COLLECTION) }
    }

    fn layout(&self, offset: usize) -> Result<f32, PhysicsAiAttributeError> {
        let layout = self
            .instance
            .try_cached_layout(LAYOUT_SIZE)
            .map_err(PhysicsAiAttributeError::Attribute)?
            .ok_or(PhysicsAiAttributeError::MissingLayout)?;
        let bytes = layout.bytes()[offset..offset + 4].try_into().expect("physics_ai layout field");
        Ok(f32::from_be_bytes(bytes))
    }

    /// `GetAttributePointer`, reading `DefaultDataArea(4)`'s zero when absent.
    fn scalar(&self, key: u64) -> Result<f32, PhysicsAiAttributeError> {
        let data = self.instance.try_get_data(key, 0).map_err(PhysicsAiAttributeError::Attribute)?;
        let bytes = data.as_ref().map_or(&[0; 4][..], |data| data.bytes());
        let bytes: [u8; 4] = bytes.try_into().map_err(|_| {
            PhysicsAiAttributeError::Attribute(AttributeRuntimeError::PayloadSize { key, expected: 4, actual: bytes.len() })
        })?;
        Ok(f32::from_be_bytes(bytes))
    }
}

/// The processed `AIPhysicsInput` the adjust follows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AiPathFollowingInput {
    pub(crate) desired_deck_transform: [[f32; 4]; 4],
    pub(crate) desired_deck_velocity: [f32; 4],
    pub(crate) flags: u32,
}

type AdjustResult<T, B, N> = Result<T, AiPathFollowingError<B, N>>;

/// `UpdateAIPathFollowing`: the position, the velocity, then the heading, each on its flag.
pub(crate) fn try_update_ai_path_following<B: AiPathFollowingBoard, N: ReckoningNumeric>(
    board: &mut B,
    attributes: &PhysicsAiAttributes,
    input: &AiPathFollowingInput,
    numeric: &mut N,
) -> AdjustResult<(), B::Error, N::Error> {
    if input.flags & physics_flags::DO_DECK_POSITION != 0 {
        let blend = attributes.scalar(POSITION_BLEND)?;
        let max = attributes.layout(MAX_POSITION_ADJUST)?;
        try_adjust_position(board, input.desired_deck_transform[3], blend, max, numeric)?;
    }
    if input.flags & physics_flags::DO_DECK_VELOCITY != 0 {
        let blend = attributes.layout(VELOCITY_BLEND)?;
        let max = attributes.layout(MAX_VELOCITY_ADJUST)?;
        try_adjust_velocity(board, input.desired_deck_velocity, blend, max, numeric)?;
    }
    if input.flags & physics_flags::DO_DECK_ORIENTATION != 0 {
        try_adjust_heading(board, input.desired_deck_transform[2], attributes, numeric)?;
    }
    Ok(())
}

/// `vminfp`.
fn vector_min(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        f32::NAN
    } else if a < b {
        a
    } else {
        b
    }
}

/// The capped step shared by the position and velocity mutators: the blended error's length,
/// at most `max`, along its `Math::NormalizeSafe` 0x824b18e0 direction.
fn try_capped_step<N: ReckoningNumeric>(
    delta: Vector,
    max: f32,
    numeric: &mut N,
) -> Result<(Vector, f32), N::Error> {
    let squared = numeric.dot_vector(delta, delta)?;
    let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
    let direction = try_normalize_safe(delta, Vector::known([0.0; 4]), numeric)?;
    let length = length_from_squared_and_estimate(squared, estimate);
    Ok((direction, vector_min(length, max)))
}

/// `PostPhysics_AdjustPosition` 0x82b2acf8: the deck Part moves toward `target` by the
/// blended error, at most `max`.
fn try_adjust_position<B: AiPathFollowingBoard, N: ReckoningNumeric>(
    board: &mut B,
    target: [f32; 4],
    blend: f32,
    max: f32,
    numeric: &mut N,
) -> AdjustResult<(), B::Error, N::Error> {
    if max < 0.0 {
        return Err(AiPathFollowingError::NegativeMaximum { bits: max.to_bits() });
    }
    let position = board.deck_part_transform().map_err(AiPathFollowingError::Board)?.columns()[3];
    let delta = Vector {
        xyz: std::array::from_fn(|lane| (target[lane] - position.xyz[lane]) * blend),
        w: position.w.map(|w| (target[3] - w) * blend),
    };
    let (direction, step) = try_capped_step(delta, max, numeric).map_err(AiPathFollowingError::Numeric)?;
    let mut requested = board.deck_part_transform().map_err(AiPathFollowingError::Board)?;
    requested.set_column(3, direction.zip(position, |direction, position| direction.mul_add(step, position)));
    board.set_deck_part_position(&requested).map_err(AiPathFollowingError::Board)
}

/// `PostPhysics_AdjustVelocity` 0x82b32b28: the deck body's velocity moves toward `target` by
/// the blended error, at most `max`.
fn try_adjust_velocity<B: AiPathFollowingBoard, N: ReckoningNumeric>(
    board: &mut B,
    target: [f32; 4],
    blend: f32,
    max: f32,
    numeric: &mut N,
) -> AdjustResult<(), B::Error, N::Error> {
    if max < 0.0 {
        return Err(AiPathFollowingError::NegativeMaximum { bits: max.to_bits() });
    }
    let velocity = board.deck_velocity().map_err(AiPathFollowingError::Board)?;
    let delta = Vector { xyz: std::array::from_fn(|lane| (target[lane] - velocity[lane]) * blend), w: None };
    let (direction, step) = try_capped_step(delta, max, numeric).map_err(AiPathFollowingError::Numeric)?;
    let adjusted = std::array::from_fn(|lane| direction.xyz[lane].mul_add(step, velocity[lane]));
    board.set_deck_body_velocity(adjusted).map_err(AiPathFollowingError::Board)
}

/// The inline wrap into a half turn either way: the turn's fraction, less a turn past a half.
fn wrap_angle(angle: f32) -> f32 {
    let turns = angle * TWO_PI_RECIPROCAL;
    let fraction = turns - turns.floor();
    let fraction = fraction - if fraction > 0.5 { 1.0 } else { 0.0 };
    fraction * TWO_PI
}

/// `fsel`'s `test >= 0` choice.
fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

/// One `vmulfp` and two `vmaddfp`s of a row against three rows: `row.z * c + (row.y * b +
/// row.x * a)`.
fn combine(row: [f32; 3], a: [f32; 4], b: [f32; 4], c: [f32; 4], lane: usize) -> f32 {
    row[2].mul_add(c[lane], row[1].mul_add(b[lane], row[0] * a[lane]))
}

/// `PostPhysics_AdjustHeading` 0x82b35b88: the target Z in the ground frame, flattened onto the
/// ground plane, gives the heading error from the ground's Z; a blended share of it, capped by
/// the `physics_ai` heading attribute, turns the board about the ground normal.
fn try_adjust_heading<B: AiPathFollowingBoard, N: ReckoningNumeric>(
    board: &mut B,
    target_z: [f32; 4],
    attributes: &PhysicsAiAttributes,
    numeric: &mut N,
) -> AdjustResult<(), B::Error, N::Error> {
    let max = attributes.scalar(MAX_HEADING_ADJUST_DEGREES)? * DEGREES_TO_RADIANS;
    let ground = board.ground_transform().map_err(AiPathFollowingError::Board)?;
    let rows: [Vector; 3] = std::array::from_fn(|row| ground.columns()[row]);
    let ground_lanes: [[f32; 4]; 3] = std::array::from_fn(|row| {
        let [x, y, z] = rows[row].xyz;
        [x, y, z, rows[row].w.unwrap_or(0.0)]
    });
    let columns: [[f32; 4]; 3] =
        std::array::from_fn(|lane| [ground_lanes[0][lane], ground_lanes[1][lane], ground_lanes[2][lane], 0.0]);
    let target = [target_z[0], target_z[1], target_z[2]];
    let blend = attributes.layout(HEADING_BLEND)?;
    let local = [combine(target, columns[0], columns[1], columns[2], 0), 0.0, combine(target, columns[0], columns[1], columns[2], 2), 0.0];
    let direction = try_normalize_safe(local, [0.0; 4], numeric).map_err(AiPathFollowingError::Numeric)?;
    let angle = try_signed_angle(K, direction, J, numeric).map_err(AiPathFollowingError::Numeric)?;
    let scaled = wrap_angle(angle) * blend;
    let lower = select(-max - scaled, -max, scaled);
    let clamped = select(max - lower, lower, max);
    let (sin, cos) = try_sin_cos(clamped, &mut HostReckoningSinCosNumeric).unwrap_or_else(|never| match never {});
    let rotation = [[cos, 0.0, -sin, cos], [0.0, 1.0, 0.0, 0.0], [sin, 0.0, cos, sin]];
    let deck = board.deck_part_transform().map_err(AiPathFollowingError::Board)?;
    let ground_w_known = rows.iter().all(|row| row.w.is_some());
    let rotated: [Vector; 3] = std::array::from_fn(|row| {
        let basis = deck.columns()[row].xyz;
        let local: [f32; 3] = std::array::from_fn(|lane| combine(basis, columns[0], columns[1], columns[2], lane));
        let turned: [f32; 3] = std::array::from_fn(|lane| combine(local, rotation[0], rotation[1], rotation[2], lane));
        Vector {
            xyz: std::array::from_fn(|lane| combine(turned, ground_lanes[0], ground_lanes[1], ground_lanes[2], lane)),
            w: ground_w_known.then(|| combine(turned, ground_lanes[0], ground_lanes[1], ground_lanes[2], 3)),
        }
    });
    let translation = board.deck_part_transform().map_err(AiPathFollowingError::Board)?.columns()[3];
    board
        .set_board_transform(Matrix::from_columns([rotated[0], rotated[1], rotated[2], translation]))
        .map_err(AiPathFollowingError::Board)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_keeps_half_turns_either_way() {
        let quarter = TWO_PI * 0.25;
        assert!((wrap_angle(quarter) - quarter).abs() < 1e-6);
        assert!((wrap_angle(TWO_PI - quarter) + quarter).abs() < 1e-5);
        assert_eq!(wrap_angle(0.0), 0.0);
    }

    #[test]
    fn heading_clamp_is_symmetric() {
        let max = 2.0 * DEGREES_TO_RADIANS;
        for (scaled, expected) in [(1.0, max), (-1.0, -max), (0.01, 0.01)] {
            let lower = select(-max - scaled, -max, scaled);
            assert_eq!(select(max - lower, lower, max), expected);
        }
    }
}
