//! `WriteSkater827d8ab8` over the actual captured Actor pose allocations.
//! Caller retains scratch records on failure; publication remains an explicit
//! ordered service (Q block first, T block second, alignment two).
use crate::{
    actor_construction::presentation_pose_source::{
        CapturedPresentationPose, PresentationPoseSourceError,
    },
    actor_face_playback::{ActorFaceError, presentation::offsets::PresentationOffsets},
    reckoning_numeric::ReckoningNumeric,
};
#[path = "skater_presentation_compression.rs"]
mod compression;
#[path = "skater_presentation_compression_q4.rs"]
mod matrix_compression;
use compression::{CompressionSource, CompressionVectors, QuaternionTranslationDestination};

pub(crate) struct PresentationWriteScratch<'a> {
    pub(crate) matrix_rotations: &'a mut [[u16; 4]],
    pub(crate) rotations: &'a mut [[u16; 3]],
    pub(crate) translations: &'a mut [[u16; 3]],
}
/// This is the native call boundary, not an implementation of `DataProvider`.
/// A second submission failure must not undo the first submission.
pub(crate) trait PresentationDataSubmission {
    type Error;
    fn submit_data(&mut self, bytes: &[u8], alignment: usize) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum PresentationWriteError<N, P> {
    Source(PresentationPoseSourceError),
    Pose(ActorFaceError),
    Capacity,
    MissingMatrix(usize),
    Matrix(matrix_compression::MatrixQ4CompressionError<N>),
    Compression(compression::CompressionError<N>),
    Publication(P),
}
impl<N: std::fmt::Display, P: std::fmt::Display> std::fmt::Display
    for PresentationWriteError<N, P>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "presentation source: {e}"),
            Self::Pose(e) => write!(f, "presentation SQT source: {e:?}"),
            Self::Capacity => f.write_str("presentation scratch/count admission failed"),
            Self::MissingMatrix(i) => write!(f, "presentation missing local matrix {i}"),
            Self::Matrix(e) => write!(f, "presentation body rotation: {e}"),
            Self::Compression(e) => write!(f, "presentation pose compression: {e}"),
            Self::Publication(e) => write!(f, "presentation submission: {e}"),
        }
    }
}
impl CapturedPresentationPose<'_> {
    /// Native getter calls have already captured allocations in local/face/right/
    /// left order. This admitted route requires initialized synchronized buffers;
    /// it does not convert an unknown non-null pose into native null-hand zeros.
    #[expect(
        clippy::too_many_lines,
        reason = "native staged compression and submission preserve partial writes"
    )]
    pub(crate) fn try_write<N: ReckoningNumeric, P: PresentationDataSubmission>(
        &self,
        offsets: &PresentationOffsets,
        scratch: &mut PresentationWriteScratch<'_>,
        numeric: &mut N,
        provider: &mut P,
    ) -> Result<(), PresentationWriteError<N::Error, P::Error>> {
        use PresentationWriteError as E;
        let face_count = offsets.face.iter().map(std::ops::Range::len).sum::<usize>();
        if scratch.matrix_rotations.len() < offsets.matrix_rotations.len()
            || scratch.rotations.len() < offsets.quaternion3_count
            || scratch.translations.len() < offsets.translation_count
            || offsets
                .left_hand_count
                .checked_add(offsets.right_hand_count)
                .and_then(|n| n.checked_add(face_count))
                != Some(offsets.quaternion3_count)
            || offsets.matrix_translations.len().checked_add(face_count)
                != Some(offsets.translation_count)
        {
            return Err(E::Capacity);
        }
        let vectors = CompressionVectors {
            ratio: [f32::from_bits(0x46ff_fe00); 4],
            zero: [0.0; 4],
            minus_one: [-1.0; 4],
        };
        {
            let owner = self
                .local
                .lock()
                .map_err(|_| E::Source(PresentationPoseSourceError::Lock))?;
            let local = owner.local_pose.as_ref().ok_or(E::Source(
                PresentationPoseSourceError::Missing("local matrix allocation"),
            ))?;
            if std::env::var_os("SKATE2_TRACE_ADJUSTED_POSE").is_some() {
                for (bone, matrix) in local.iter().enumerate().filter(|(_, matrix)| matrix.iter().any(|value| !value.is_finite())) {
                    eprintln!("presentation nonfinite local bone={bone} matrix={matrix:?}");
                }
            }
            matrix_compression::try_compress_matrix_q4(
                local,
                &offsets.matrix_rotations,
                scratch.matrix_rotations,
                vectors.ratio,
                numeric,
            )
            .map_err(E::Matrix)?;
            for (ordinal, &index) in offsets.matrix_translations.iter().enumerate() {
                let matrix = local.get(index).ok_or(E::MissingMatrix(index))?;
                // Native loads translation then explicitly overwrites W with one.
                let translation = [matrix[12], matrix[13], matrix[14], 1.0];
                compression::try_compress_translation(
                    translation,
                    0.5,
                    vectors.ratio,
                    &mut scratch.translations[ordinal],
                    numeric,
                )
                .map_err(E::Compression)?;
            }
        }
        let mut q_index = 0;
        for (hand, count) in [
            (&self.left, offsets.left_hand_count),
            (&self.right, offsets.right_hand_count),
        ] {
            let (_, values) = hand.read().map_err(E::Pose)?;
            if std::env::var_os("SKATE2_TRACE_ADJUSTED_POSE").is_some() {
                for (bone, sqt) in values.iter().enumerate().filter(|(_, sqt)| sqt.rotation.iter().chain(&sqt.translation).any(|value| !value.is_finite())) {
                    eprintln!("presentation nonfinite hand q_base={q_index} bone={bone} sqt={sqt:?}");
                }
            }
            let count_i32 = i32::try_from(count).map_err(|_| E::Capacity)?;
            compression::try_compress_q3(
                CompressionSource::Sqts(&values),
                count_i32,
                &mut scratch.rotations[q_index..q_index + count],
                vectors,
                numeric,
            )
            .map_err(E::Compression)?;
            q_index += count;
        }
        let mut t_index = offsets.matrix_translations.len();
        if self
            .actor
            .try_presentation_face_initialized()
            .map_err(E::Source)?
        {
            let (_, values) = self.face.read().map_err(E::Pose)?;
            if std::env::var_os("SKATE2_TRACE_ADJUSTED_POSE").is_some() {
                for (bone, sqt) in values.iter().enumerate().filter(|(_, sqt)| sqt.rotation.iter().chain(&sqt.translation).any(|value| !value.is_finite())) {
                    eprintln!("presentation nonfinite face bone={bone} sqt={sqt:?}");
                }
            }
            for range in &offsets.face {
                let values = values
                    .get(range.clone())
                    .ok_or(E::Pose(ActorFaceError::UnsupportedPose))?;
                let count = range.len();
                compression::try_compress_quaternion_translation(
                    CompressionSource::Sqts(values),
                    i32::try_from(count).map_err(|_| E::Capacity)?,
                    &mut QuaternionTranslationDestination {
                        quaternions: &mut scratch.rotations[q_index..q_index + count],
                        translations: &mut scratch.translations[t_index..t_index + count],
                    },
                    0.5,
                    vectors,
                    numeric,
                )
                .map_err(E::Compression)?;
                q_index += count;
                t_index += count;
            }
        } else {
            for _ in 0..face_count {
                scratch.rotations[q_index] = [0; 3];
                scratch.translations[t_index] = [0; 3];
                q_index += 1;
                t_index += 1;
            }
        }
        let quaternions = scratch.matrix_rotations[..offsets.matrix_rotations.len()]
            .iter()
            .flatten()
            .chain(
                scratch.rotations[..offsets.quaternion3_count]
                    .iter()
                    .flatten(),
            )
            .flat_map(|value| value.to_be_bytes())
            .collect::<Vec<_>>();
        provider
            .submit_data(&quaternions, 2)
            .map_err(E::Publication)?;
        let translations = scratch.translations[..offsets.translation_count]
            .iter()
            .flatten()
            .flat_map(|value| value.to_be_bytes())
            .collect::<Vec<_>>();
        provider
            .submit_data(&translations, 2)
            .map_err(E::Publication)
    }
}

#[cfg(test)]
#[path = "skater_presentation_write_corpus.rs"]
pub(crate) mod corpus;
