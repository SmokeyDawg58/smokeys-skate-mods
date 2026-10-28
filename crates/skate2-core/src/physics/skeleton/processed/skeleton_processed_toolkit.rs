//! Continuations of the existing processed Contact owner, 0x82b269e4..26de4.
//! Direction and board jump-normal tails have distinct retained readiness.
use super::SkeletonProcessedContact;
use crate::{
    reckoning_numeric::{ReckoningNumeric, normalize_from_squared_and_estimate},
    skateboard_spawn_math::skateboard_spawn_rigid_inverse,
    skeleton_ik_line_frame::normalize_safe,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::fmt;
#[path = "skeleton_processed_toolkit_runtime.rs"]
pub(crate) mod runtime;

/// These are current retained processed sources at native load sites. `zSpeed`
/// is the prior `PhysOut` motion publication, never a fresh body velocity query.
pub(crate) trait SkeletonProcessedToolkitInputRead {
    type Error;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error>;
    fn z_speed(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SkeletonProcessedToolkitError<I, N> {
    MissingPrefix,
    Input(I),
    Numeric(N),
}
impl<I: fmt::Debug, N: fmt::Debug> fmt::Display for SkeletonProcessedToolkitError<I, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "processed toolkit directions: {self:?}")
    }
}
/// Raw fields persist across bookkeeping invalidation. Each `Option` is unknown
/// until its source store executes; prefix readiness never marks this complete.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SkeletonProcessedToolkitState {
    inverse_columns: [Option<SkeletonRootVector>; 4],
    body_x: Option<SkeletonRootVector>,
    body_y: Option<SkeletonRootVector>,
    body_z: Option<SkeletonRootVector>,
    flipped_controls_scalar: Option<f32>,
    body_z_direction: Option<SkeletonRootVector>,
    body_y_direction: Option<SkeletonRootVector>,
    z_axis_velocity: Option<SkeletonRootVector>,
    abs_body_speed: Option<f32>,
    z_axis_velocity_dir: Option<SkeletonRootVector>,
    directions_complete: bool,
    processed_jump_normal: Option<SkeletonRootVector>,
    jump_tail_complete: bool,
}
impl SkeletonProcessedToolkitState {
    pub(crate) const fn new() -> Self {
        Self {
            inverse_columns: [None; 4],
            body_x: None,
            body_y: None,
            body_z: None,
            flipped_controls_scalar: None,
            body_z_direction: None,
            body_y_direction: None,
            z_axis_velocity: None,
            abs_body_speed: None,
            z_axis_velocity_dir: None,
            directions_complete: false,
            processed_jump_normal: None,
            jump_tail_complete: false,
        }
    }
    /// Host readiness only. The existing processed reset/prefix owns native reset
    /// writes; this must not invent zero overwrites of retained numerical fields.
    pub(crate) fn invalidate(&mut self) {
        self.directions_complete = false;
        self.jump_tail_complete = false;
    }
    pub(crate) const fn jump_tail_complete(&self) -> bool {
        self.jump_tail_complete
    }
    pub(crate) const fn processed_jump_normal(&self) -> Option<SkeletonRootVector> {
        self.processed_jump_normal
    }
    pub(crate) const fn directions_complete(&self) -> bool {
        self.directions_complete
    }
    pub(crate) fn inverse_effective(&self) -> Option<SkeletonRootMatrix> {
        Some(SkeletonRootMatrix::from_columns([
            self.inverse_columns[0]?,
            self.inverse_columns[1]?,
            self.inverse_columns[2]?,
            self.inverse_columns[3]?,
        ]))
    }
    pub(crate) const fn body_x(&self) -> Option<SkeletonRootVector> {
        self.body_x
    }
    pub(crate) const fn body_y(&self) -> Option<SkeletonRootVector> {
        self.body_y
    }
    pub(crate) const fn body_z(&self) -> Option<SkeletonRootVector> {
        self.body_z
    }
    pub(crate) const fn flipped_controls_scalar(&self) -> Option<f32> {
        self.flipped_controls_scalar
    }
    pub(crate) const fn body_z_direction(&self) -> Option<SkeletonRootVector> {
        self.body_z_direction
    }
    pub(crate) const fn body_y_direction(&self) -> Option<SkeletonRootVector> {
        self.body_y_direction
    }
    pub(crate) const fn z_axis_velocity(&self) -> Option<SkeletonRootVector> {
        self.z_axis_velocity
    }
    pub(crate) const fn abs_body_speed(&self) -> Option<f32> {
        self.abs_body_speed
    }
    pub(crate) const fn z_axis_velocity_dir(&self) -> Option<SkeletonRootVector> {
        self.z_axis_velocity_dir
    }
}
impl SkeletonProcessedContact {
    pub(crate) const fn toolkit_state(&self) -> &SkeletonProcessedToolkitState {
        &self.toolkit
    }
    /// Continue the already committed physical mass/deck/effective prefix without
    /// rereading the body. Completion stops at 0x82b26cc8, before jump-normal work.
    pub(crate) fn try_continue_toolkit_directions<
        I: SkeletonProcessedToolkitInputRead,
        N: ReckoningNumeric,
    >(
        &mut self,
        input: &mut I,
        numeric: &mut N,
    ) -> Result<(), SkeletonProcessedToolkitError<I::Error, N::Error>> {
        self.toolkit.try_continue_directions(
            self.toolkit_prefix_complete,
            self.deck.as_ref(),
            self.effective.as_ref(),
            input,
            numeric,
        )
    }
}
impl SkeletonProcessedToolkitState {
    fn try_continue_directions<I: SkeletonProcessedToolkitInputRead, N: ReckoningNumeric>(
        &mut self,
        prefix_complete: bool,
        deck: Option<&SkeletonRootMatrix>,
        effective: Option<&SkeletonRootMatrix>,
        input: &mut I,
        numeric: &mut N,
    ) -> Result<(), SkeletonProcessedToolkitError<I::Error, N::Error>> {
        use SkeletonProcessedToolkitError as E;
        self.invalidate();
        if !prefix_complete {
            return Err(E::MissingPrefix);
        }
        let deck = deck.ok_or(E::MissingPrefix)?;
        let effective = effective.ok_or(E::MissingPrefix)?;
        // The rigid inverse consumes only XYZ coefficients. Unobserved input W
        // scratch never becomes body/effective W. Generated inverse basis W is +0;
        // translation W still executes the existing graph (including Inf*0 cases).
        let effective_scratch: [f32; 16] = std::array::from_fn(|i| {
            if i % 4 == 3 {
                0.0
            } else {
                effective.columns()[i / 4].xyz[i % 4]
            }
        });
        let inverse = SkeletonRootMatrix::known(skateboard_spawn_rigid_inverse(&effective_scratch));
        for column in [2, 1, 0, 3] {
            self.inverse_columns[column] = Some(inverse.columns()[column]);
        }
        let body_y = deck.columns()[1];
        self.body_y = Some(body_y);
        let body_x = deck.columns()[0];
        self.body_x = Some(body_x);
        let body_z = deck.columns()[2];
        self.body_z = Some(body_z);
        let flags = input.flags_8c4().map_err(E::Input)?;
        let flipped = if flags & 0x0010_0000 != 0 { -1.0 } else { 1.0 };
        self.flipped_controls_scalar = Some(flipped);
        self.body_x = Some(scale(body_x, flipped));
        // Actual second scalar load reads the just-published retained field.
        let flipped = self.flipped_controls_scalar.expect("source scalar store");
        let body_z = scale(body_z, flipped);
        self.body_z = Some(body_z);
        let mut flat = body_z;
        flat.xyz[1] = 0.0;
        let squared = numeric
            .dot3(scratch(flat), scratch(flat))
            .map_err(E::Numeric)?;
        self.body_z_direction = Some(body_z);
        self.body_z_direction = Some(flat);
        if squared > f32::from_bits(0x3727_c5ac) {
            let squared = numeric
                .dot3(scratch(flat), scratch(flat))
                .map_err(E::Numeric)?;
            let seed = numeric
                .reciprocal_sqrt_estimate(squared)
                .map_err(E::Numeric)?;
            self.body_z_direction = Some(normalize_finish(flat, squared, seed));
        }
        let direction = self.body_z_direction.expect("source flattened store");
        let body_y = self.body_y.expect("source body Y store");
        let projection = numeric
            .dot3(scratch(body_y), scratch(direction))
            .map_err(E::Numeric)?;
        let product = scale(direction, projection);
        let raw = subtract(body_y, product);
        self.body_y_direction = Some(raw);
        let squared = numeric
            .dot3(scratch(raw), scratch(raw))
            .map_err(E::Numeric)?;
        let seed = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        self.body_y_direction = Some(normalize_finish(raw, squared, seed));
        let z_speed = input.z_speed().map_err(E::Input)?;
        let body_z = self.body_z.expect("source flipped Z store");
        self.z_axis_velocity = Some(scale(body_z, z_speed));
        let z_speed = input.z_speed().map_err(E::Input)?;
        // BO4 BI25 at 26c70: GT-not-set includes both zeros and unordered.
        let sign = if z_speed > 0.0 { 1.0 } else { -1.0 };
        let body_z = self.body_z.expect("source body Z reread");
        self.abs_body_speed = Some(z_speed * sign);
        self.z_axis_velocity_dir = Some(scale(body_z, sign));
        self.directions_complete = true;
        Ok(())
    }
}
/// Actual prior ground publication at processed+1d0, not a replacement line hit.
pub(crate) trait SkeletonProcessedToolkitJumpInputRead {
    type Error;
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}
/// Retained Skateboard+c0. Construction and authority belong to the actual
/// board runtime; the toolkit never constructs a substitute initial normal.
pub(crate) trait SkeletonProcessedToolkitJumpBoard {
    type Error;
    fn jump_normal(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn set_jump_normal(&mut self, value: SkeletonRootVector) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolkitJumpFloat {
    MaxCosAngleDelta,
    ProportionalBlend,
}
/// Explicit live target capture/value stages. No runtime literal defaults.
pub(crate) trait SkeletonProcessedToolkitJumpCvars {
    type FloatTarget;
    type BoolTarget;
    type Error;
    fn capture_float(&mut self, field: ToolkitJumpFloat) -> Result<Self::FloatTarget, Self::Error>;
    fn float(&mut self, target: &Self::FloatTarget) -> Result<f32, Self::Error>;
    fn capture_draw_jump(&mut self) -> Result<Self::BoolTarget, Self::Error>;
    fn draw_jump(&mut self, target: &Self::BoolTarget) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SkeletonProcessedToolkitJumpError<I, B, C, N> {
    MissingDirections,
    Input(I),
    Board(B),
    Cvar(C),
    Numeric(N),
}
impl<I: fmt::Debug, B: fmt::Debug, C: fmt::Debug, N: fmt::Debug> fmt::Display
    for SkeletonProcessedToolkitJumpError<I, B, C, N>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "processed toolkit jump normal: {self:?}")
    }
}
type JumpResult<I, B, C, N> = Result<
    (),
    SkeletonProcessedToolkitJumpError<
        <I as SkeletonProcessedToolkitJumpInputRead>::Error,
        <B as SkeletonProcessedToolkitJumpBoard>::Error,
        <C as SkeletonProcessedToolkitJumpCvars>::Error,
        <N as ReckoningNumeric>::Error,
    >,
>;
impl SkeletonProcessedContact {
    /// 0x82b26ccc..26de4. Board normal commits before the debug gate; processed
    /// normal commits only after the final actual board reread succeeds.
    pub(crate) fn try_continue_toolkit_jump_normal<
        I: SkeletonProcessedToolkitJumpInputRead,
        B: SkeletonProcessedToolkitJumpBoard,
        C: SkeletonProcessedToolkitJumpCvars,
        N: ReckoningNumeric,
    >(
        &mut self,
        input: &mut I,
        board: &mut B,
        cvars: &mut C,
        numeric: &mut N,
    ) -> JumpResult<I, B, C, N> {
        self.toolkit
            .try_continue_jump_normal(input, board, cvars, numeric)
    }
}
impl SkeletonProcessedToolkitState {
    fn try_continue_jump_normal<
        I: SkeletonProcessedToolkitJumpInputRead,
        B: SkeletonProcessedToolkitJumpBoard,
        C: SkeletonProcessedToolkitJumpCvars,
        N: ReckoningNumeric,
    >(
        &mut self,
        input: &mut I,
        board: &mut B,
        cvars: &mut C,
        numeric: &mut N,
    ) -> JumpResult<I, B, C, N> {
        use SkeletonProcessedToolkitJumpError as E;
        self.jump_tail_complete = false;
        if !self.directions_complete {
            return Err(E::MissingDirections);
        }
        let max_target = cvars
            .capture_float(ToolkitJumpFloat::MaxCosAngleDelta)
            .map_err(E::Cvar)?;
        let ground = input.ground_normal().map_err(E::Input)?;
        let old = board.jump_normal().map_err(E::Board)?;
        let dot = numeric
            .dot3(scratch(old), scratch(ground))
            .map_err(E::Numeric)?;
        let maximum = cvars.float(&max_target).map_err(E::Cvar)?;
        let candidate = if dot > maximum {
            ground
        } else {
            // Ordered GT alone skips blending; unordered takes this branch.
            let blend_target = cvars
                .capture_float(ToolkitJumpFloat::ProportionalBlend)
                .map_err(E::Cvar)?;
            let blend = cvars.float(&blend_target).map_err(E::Cvar)?;
            let remaining = 1.0 - blend;
            let current = scale(ground, blend);
            let combined = SkeletonRootVector {
                xyz: std::array::from_fn(|lane| {
                    old.xyz[lane].mul_add(remaining, current.xyz[lane])
                }),
                w: old
                    .w
                    .zip(current.w)
                    .map(|(old, current)| old.mul_add(remaining, current)),
            };
            normalize_safe(combined, numeric).map_err(E::Numeric)?
        };
        board.set_jump_normal(candidate).map_err(E::Board)?;
        let draw_target = cvars.capture_draw_jump().map_err(E::Cvar)?;
        if cvars.draw_jump(&draw_target).map_err(E::Cvar)? {
            cvars.require_overlay().map_err(E::Cvar)?;
        }
        let final_normal = board.jump_normal().map_err(E::Board)?;
        self.processed_jump_normal = Some(final_normal);
        self.jump_tail_complete = true;
        Ok(())
    }
}
fn scratch(v: SkeletonRootVector) -> [f32; 4] {
    [v.xyz[0], v.xyz[1], v.xyz[2], v.w.unwrap_or(0.0)]
}
fn scale(v: SkeletonRootVector, s: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: v.xyz.map(|x| x * s),
        w: v.w.map(|x| x * s),
    }
}
fn subtract(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| a.xyz[i] - b.xyz[i]),
        w: a.w.zip(b.w).map(|(a, b)| a - b),
    }
}
fn normalize_finish(v: SkeletonRootVector, squared: f32, seed: f32) -> SkeletonRootVector {
    let normalized = normalize_from_squared_and_estimate(scratch(v), squared, seed);
    SkeletonRootVector {
        xyz: [normalized[0], normalized[1], normalized[2]],
        w: v.w.map(|_| normalized[3]),
    }
}
#[cfg(test)]
#[path = "skeleton_processed_toolkit_tests.rs"]
mod tests;
