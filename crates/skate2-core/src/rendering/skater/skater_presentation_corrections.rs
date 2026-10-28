//! Same retained bind/replay channels for the post-read correction passes.
use super::{
    ActorFaceError, PresentationAnimation,
    expressions::{
        ExpressionsError, PresentationTwistAttributes, TwistAttributeError,
        try_compute_expression_channels,
    },
    move_root::{MoveRootError, matrix_inverse::PresentationMatrixNumeric, try_move_root_channels},
};
use crate::{
    actor_animation_construction::initialization::bind_pose::sqt_output::mutation::{
        SqtChannelEdit, SqtMutationError,
    },
    reckoning_sin_cos::ReckoningSinCosNumeric,
};
use skate2_formats::andale::AndaleSqt;
/// Access policy only; all arithmetic remains in the existing recovered leaves.
pub(crate) trait CorrectionChannels {
    fn bind(&self, index: usize) -> Option<AndaleSqt>;
    fn replay(&self, index: usize) -> Option<AndaleSqt>;
    fn rotation(&mut self, index: usize, value: [f32; 4]) -> Result<(), ActorFaceError>;
    fn translation(&mut self, index: usize, value: [f32; 4]) -> Result<(), ActorFaceError>;
    fn scale(&mut self, index: usize, value: [f32; 4]) -> Result<(), ActorFaceError>;
}
impl CorrectionChannels for SqtChannelEdit<'_> {
    fn bind(&self, i: usize) -> Option<AndaleSqt> {
        self.source(i).ok()
    }
    fn replay(&self, i: usize) -> Option<AndaleSqt> {
        self.destination(i).ok()
    }
    fn rotation(&mut self, i: usize, v: [f32; 4]) -> Result<(), ActorFaceError> {
        self.set_rotation(i, v)
    }
    fn translation(&mut self, i: usize, v: [f32; 4]) -> Result<(), ActorFaceError> {
        self.set_translation(i, v)
    }
    fn scale(&mut self, i: usize, v: [f32; 4]) -> Result<(), ActorFaceError> {
        self.set_scale(i, v)
    }
}
pub(crate) struct SliceChannels<'a> {
    pub(crate) bind: &'a [AndaleSqt],
    pub(crate) replay: &'a mut [AndaleSqt],
}
impl CorrectionChannels for SliceChannels<'_> {
    fn bind(&self, i: usize) -> Option<AndaleSqt> {
        self.bind.get(i).copied()
    }
    fn replay(&self, i: usize) -> Option<AndaleSqt> {
        self.replay.get(i).copied()
    }
    fn rotation(&mut self, i: usize, v: [f32; 4]) -> Result<(), ActorFaceError> {
        if let Some(row) = self.replay.get_mut(i) {
            row.rotation = v;
            Ok(())
        } else {
            Err(ActorFaceError::UnsupportedPose)
        }
    }
    fn translation(&mut self, i: usize, v: [f32; 4]) -> Result<(), ActorFaceError> {
        if let Some(row) = self.replay.get_mut(i) {
            row.translation = v;
            Ok(())
        } else {
            Err(ActorFaceError::UnsupportedPose)
        }
    }
    fn scale(&mut self, i: usize, v: [f32; 4]) -> Result<(), ActorFaceError> {
        if let Some(row) = self.replay.get_mut(i) {
            row.scale = v;
            Ok(())
        } else {
            Err(ActorFaceError::UnsupportedPose)
        }
    }
}
#[derive(Debug)]
pub(crate) enum CorrectionError<N, R> {
    Missing(&'static str),
    Owner(ActorFaceError),
    Move(SqtMutationError<MoveRootError<N>>),
    Expression(SqtMutationError<ExpressionsError<TwistAttributeError, N, R>>),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display for CorrectionError<N, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "presentation correction missing {s}"),
            Self::Owner(e) => write!(f, "presentation correction owner {e:?}"),
            Self::Move(e) => e.fmt(f),
            Self::Expression(e) => e.fmt(f),
        }
    }
}
/// Keep native signed bits until the actual checked channel use. Unknown
/// constructor slots remain out-of-range too; neither projection fails early.
pub(crate) fn native_bone_index(index: Option<i32>) -> usize {
    index.map_or(usize::MAX, |value| {
        usize::try_from(u32::from_ne_bytes(value.to_ne_bytes())).unwrap_or(usize::MAX)
    })
}
impl PresentationAnimation {
    /// `UpdateBodies827deb24/deb2c`: complete the root loop before expressions.
    /// Every admitted count entry participates, regardless of model presence.
    pub(crate) fn try_correct_replay<N: PresentationMatrixNumeric, R: ReckoningSinCosNumeric>(
        &self,
        count: usize,
        attributes: Option<&PresentationTwistAttributes>,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), CorrectionError<N::Error, R::Error>> {
        use CorrectionError as E;
        if count == 0 {
            return Ok(());
        }
        let hips = native_bone_index(self.hips);
        let board = native_bone_index(self.skateboard);
        let mut transforms = self
            .hips_transforms
            .lock()
            .map_err(|_| E::Owner(ActorFaceError::Lock))?;
        for index in 0..count {
            let replay = self
                .replay
                .get(2 * index)
                .ok_or(E::Missing("even replay allocation"))?;
            let transform = transforms
                .get_mut(index)
                .ok_or(E::Missing("hips transform slot"))?;
            replay
                .try_edit_channels(replay, |channels| {
                    try_move_root_channels(channels, hips, board, transform, numeric)
                })
                .map_err(E::Move)?;
        }
        let bind = self
            .bind
            .as_ref()
            .ok_or(E::Missing("authored bind allocation"))?;
        let indices = self.twist_bones.map(native_bone_index);
        let mut attributes = attributes;
        for index in 0..count {
            let replay = self
                .replay
                .get(2 * index)
                .ok_or(E::Missing("even replay allocation"))?;
            replay
                .try_edit_channels(bind, |channels| {
                    try_compute_expression_channels(
                        channels,
                        &indices,
                        &mut attributes,
                        numeric,
                        rounding,
                    )
                })
                .map_err(E::Expression)?;
        }
        Ok(())
    }
}
impl PresentationMatrixNumeric for crate::reckoning_numeric::HostReckoningNumeric {
    fn dot4(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        super::read::decode::PresentationDecodeNumeric::dot4(self, a, b)
    }
}
#[cfg(test)]
#[path = "skater_presentation_corrections_corpus.rs"]
pub(crate) mod corpus;
#[cfg(test)]
#[path = "skater_presentation_corrections_tests.rs"]
mod tests;
