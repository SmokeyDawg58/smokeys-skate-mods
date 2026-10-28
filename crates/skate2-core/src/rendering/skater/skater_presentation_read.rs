//! `ReadSkater827ddb60`: ordered decoding and edits of the retained replay SQT.
use super::{
    ActorFaceError, ActorGraphIntents, PresentationAnimation,
    offsets::{PresentationOffsetsError, part_range},
};
use crate::{
    actor_animation_construction::initialization::{
        bind_pose::{
            BindPoseError,
            sqt_output::{
                OwnedAnimationSqtBuffer,
                mutation::{SqtChannelEdit, SqtMutationError},
            },
        },
        body_dispatch::tree_commands::TreeCommands,
    },
    animation::AnimationMatrix,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    world::try_simulation_quaternion_candidate,
};
#[path = "skater_presentation_decode.rs"]
pub(crate) mod decode;
use decode::{DecodeError, PresentationDecodeNumeric};
use skate2_formats::andale::AndaleHierarchy;

pub(crate) trait PresentationReadSettings {
    type Error;
    fn no_hand_animation(&mut self) -> Result<bool, Self::Error>;
    fn no_face_animation(&mut self) -> Result<bool, Self::Error>;
}
impl PresentationReadSettings for &InputCvarReferences {
    type Error = InputCvarError;
    fn no_hand_animation(&mut self) -> Result<bool, Self::Error> {
        self.try_no_hand_animation()
    }
    fn no_face_animation(&mut self) -> Result<bool, Self::Error> {
        self.try_no_face_animation()
    }
}
/// Fields come from the actual retained `SkaterPresentation::Data` header.
/// Backing includes native readable adjacent bytes, not synthesized padding.
pub(crate) struct PresentationReadFrame<'a> {
    pub(crate) read_base: &'a [u8],
    pub(crate) compressed_offset: u16,
    pub(crate) local_hips: AnimationMatrix,
    pub(crate) local_skateboard: AnimationMatrix,
}
pub(crate) struct PresentationReadScratch<'a> {
    pub(crate) quaternions: &'a mut [[f32; 4]],
    pub(crate) translations: &'a mut [[f32; 4]],
}
#[derive(Debug)]
pub(crate) enum PresentationReadError<N, C> {
    Missing(&'static str),
    Capacity,
    Pose(ActorFaceError),
    Part(PresentationOffsetsError),
    Decode(DecodeError<N>),
    Numeric(N),
    Settings(C),
    Command(BindPoseError),
    Bone(i32),
}
impl<N: std::fmt::Display, C: std::fmt::Debug> std::fmt::Display for PresentationReadError<N, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "ReadSkater missing {s}"),
            Self::Capacity => f.write_str("ReadSkater scratch capacity"),
            Self::Pose(e) => write!(f, "ReadSkater SQT: {e:?}"),
            Self::Part(e) => e.fmt(f),
            Self::Decode(e) => e.fmt(f),
            Self::Numeric(e) => e.fmt(f),
            Self::Settings(e) => write!(f, "ReadSkater live cvar: {e:?}"),
            Self::Command(e) => write!(f, "ReadSkater SetSQT: {e:?}"),
            Self::Bone(i) => write!(f, "ReadSkater invalid bone {i}"),
        }
    }
}
impl PresentationAnimation {
    /// The caller owns `BeginCommands`/`EndCommands` and subsequent extraction.
    /// Decode failures retain scratch prefixes; edit failures retain SQT stores.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual source/destination allocations, scratch, live services and command owner"
    )]
    pub(crate) fn try_read_skater<N: PresentationDecodeNumeric, C: PresentationReadSettings>(
        &self,
        frame: &PresentationReadFrame<'_>,
        source: &OwnedAnimationSqtBuffer,
        destination: &OwnedAnimationSqtBuffer,
        scratch: &mut PresentationReadScratch<'_>,
        settings: &mut C,
        numeric: &mut N,
        commands: &mut TreeCommands,
    ) -> Result<(), PresentationReadError<N::Error, C::Error>> {
        use PresentationReadError as E;
        let offsets = self.offsets.as_ref().ok_or(E::Missing("InitOffsets"))?;
        let q4 = offsets.matrix_rotations.len();
        let q3 = offsets.quaternion3_count;
        let ts = offsets.translation_count;
        if scratch.quaternions.len() < q4 + q3 || scratch.translations.len() < ts {
            return Err(E::Capacity);
        }
        let mut cursor = usize::from(frame.compressed_offset);
        for out in &mut scratch.quaternions[..q4] {
            *out = decode::quaternion4(frame.read_base, cursor, numeric).map_err(E::Decode)?;
            cursor += 8;
        }
        for out in &mut scratch.quaternions[q4..q4 + q3] {
            *out = decode::quaternion3(frame.read_base, cursor, numeric).map_err(E::Decode)?;
            cursor += 6;
        }
        for out in &mut scratch.translations[..ts] {
            *out = decode::translation(frame.read_base, cursor, 0.5).map_err(E::Decode)?;
            cursor += 6;
        }
        // Native resolves the CURRENT global playback only after all decoding.
        let playback = self
            .managers
            .playback
            .lock()
            .map_err(|_| E::Pose(ActorFaceError::Lock))?
            .entries
            .get(&(ActorGraphIntents::native_name(b"SkaterFull"), 0))
            .cloned()
            .ok_or(E::Missing("current SkaterFull"))?;
        let database = std::sync::Arc::clone(
            &playback
                .lock()
                .map_err(|_| E::Pose(ActorFaceError::Lock))?
                .database,
        );
        let full = database
            .lock()
            .map_err(|_| E::Pose(ActorFaceError::Lock))?
            .parsed
            .clone()
            .ok_or(E::Pose(ActorFaceError::MissingRecord))?;
        destination
            .try_edit_channels(source, |edit| {
                apply_channels(
                    &full.hierarchy,
                    edit,
                    scratch,
                    frame,
                    self.hips,
                    self.skateboard,
                    settings,
                    numeric,
                )
            })
            .map_err(|e| match e {
                SqtMutationError::Owner(e) => E::Pose(e),
                SqtMutationError::Operation(e) => e,
            })?;
        commands.set_external_sqt(destination).map_err(E::Command)
    }
}
#[expect(
    clippy::too_many_arguments,
    reason = "native ordered channel mutation over actual retained frame and owners"
)]
fn apply_channels<N: PresentationDecodeNumeric, C: PresentationReadSettings>(
    hierarchy: &AndaleHierarchy,
    edit: &mut SqtChannelEdit<'_>,
    scratch: &PresentationReadScratch<'_>,
    frame: &PresentationReadFrame<'_>,
    hips: Option<i32>,
    board: Option<i32>,
    settings: &mut C,
    numeric: &mut N,
) -> Result<(), PresentationReadError<N::Error, C::Error>> {
    use PresentationReadError as E;
    let mut q = 0;
    let mut t = 0;
    for name in ["Torso", "Head", "Arms", "Legs", "Skateboard"] {
        for index in part_range(hierarchy, name).map_err(E::Part)? {
            let rotation = *scratch.quaternions.get(q).ok_or(E::Capacity)?;
            edit.set_rotation(index, rotation).map_err(E::Pose)?;
            q += 1;
        }
    }
    for index in part_range(hierarchy, "Skateboard_Trucks").map_err(E::Part)? {
        edit.set_rotation(index, *scratch.quaternions.get(q).ok_or(E::Capacity)?)
            .map_err(E::Pose)?;
        q += 1;
        edit.set_translation(index, *scratch.translations.get(t).ok_or(E::Capacity)?)
            .map_err(E::Pose)?;
        t += 1;
    }
    for name in ["LeftHand", "RightHand"] {
        for index in part_range(hierarchy, name).map_err(E::Part)? {
            if !settings.no_hand_animation().map_err(E::Settings)? {
                edit.set_rotation(index, *scratch.quaternions.get(q).ok_or(E::Capacity)?)
                    .map_err(E::Pose)?;
            }
            q += 1;
        }
    }
    for name in ["Face0", "Face1", "Face2"] {
        for index in part_range(hierarchy, name).map_err(E::Part)? {
            if !settings.no_face_animation().map_err(E::Settings)? {
                let delta = *scratch.translations.get(t).ok_or(E::Capacity)?;
                let original = edit.source(index).map_err(E::Pose)?.rotation;
                let previous = edit.destination(index).map_err(E::Pose)?.translation;
                let rotated = rotate(original, delta);
                edit.set_translation(index, std::array::from_fn(|i| previous[i] + rotated[i]))
                    .map_err(E::Pose)?;
                let original = edit.source(index).map_err(E::Pose)?.rotation;
                let delta = *scratch.quaternions.get(q).ok_or(E::Capacity)?;
                let products = cross_products(original, delta);
                let dot = numeric.dot3(original, delta).map_err(E::Numeric)?;
                let crossed = cross_from_products(original, delta, products);
                let scalar = original[3] * delta[3] - dot;
                let mut composed = std::array::from_fn(|i| {
                    original[i].mul_add(delta[3], delta[i].mul_add(original[3], crossed[i]))
                });
                composed[3] = scalar;
                edit.set_rotation(index, composed).map_err(E::Pose)?;
            }
            q += 1;
            t += 1;
        }
    }
    for (index, matrix) in [(hips, frame.local_hips), (board, frame.local_skateboard)] {
        let signed = index.ok_or(E::Missing("InitAnimation bone"))?;
        let index = usize::try_from(signed).map_err(|_| E::Bone(signed))?;
        edit.set_translation(index, [matrix[12], matrix[13], matrix[14], matrix[15]])
            .map_err(E::Pose)?;
        let quaternion = try_simulation_quaternion_candidate(matrix, |magnitudes| {
            let mut seeds = [0.0; 4];
            for i in 0..4 {
                seeds[i] = numeric.reciprocal_sqrt_estimate(magnitudes[i])?;
            }
            Ok(seeds)
        })
        .map_err(E::Numeric)?;
        edit.set_rotation(index, quaternion).map_err(E::Pose)?;
    }
    Ok(())
}
fn cross_products(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let p = [1, 2, 0, 3];
    std::array::from_fn(|i| a[i] * b[p[i]])
}
fn cross_from_products(a: [f32; 4], b: [f32; 4], initial: [f32; 4]) -> [f32; 4] {
    let p = [1, 2, 0, 3];
    let subtract: [f32; 4] = std::array::from_fn(|i| -a[p[i]].mul_add(b[i], -initial[i]));
    std::array::from_fn(|i| subtract[p[i]])
}
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    cross_from_products(a, b, cross_products(a, b))
}
fn rotate(q: [f32; 4], v: [f32; 4]) -> [f32; 4] {
    let crossed = cross(q, v);
    let inner = std::array::from_fn(|i| q[3].mul_add(v[i], crossed[i]));
    let outer = cross(q, inner);
    std::array::from_fn(|i| 2.0_f32.mul_add(outer[i], v[i]))
}
#[cfg(test)]
#[path = "skater_presentation_read_tests.rs"]
mod tests;
