//! Selected `ReadSkater` command pass over actual restored allocations.
//! The host explicitly supplies admitted source/destination; this does not
//! infer `UpdateBodies` model membership, interpolation or frame mapping.
use super::super::{
    ActorFaceError, ActorGraphIntents, PresentationAnimation,
    offsets::part_range,
    read::{
        PresentationReadError, PresentationReadFrame, PresentationReadScratch,
        PresentationReadSettings, decode::PresentationDecodeNumeric,
    },
    runtime,
};
use crate::actor_animation_construction::{
    AnimationPointerState,
    hands_face::{HandFaceStreamCommands, commands::StreamCommands, runtime::HandFaceRuntimeError},
    initialization::{
        ActorAnimationInitializeSource,
        bind_pose::{BindPoseError, sqt_output::OwnedAnimationSqtBuffer},
    },
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum PresentationReadPassError<N, C, J> {
    Missing(&'static str),
    Animation(ActorFaceError),
    Command(BindPoseError),
    Part(super::super::offsets::PresentationOffsetsError),
    Read(PresentationReadError<N, C>),
    Dispatch(HandFaceRuntimeError<J>),
}
impl<N: std::fmt::Display, C: std::fmt::Debug, J: std::fmt::Debug> std::fmt::Display
    for PresentationReadPassError<N, C, J>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "presentation read pass missing {s}"),
            Self::Animation(e) => write!(f, "presentation animation: {e:?}"),
            Self::Command(e) => write!(f, "presentation command: {e:?}"),
            Self::Part(e) => e.fmt(f),
            Self::Read(e) => e.fmt(f),
            Self::Dispatch(e) => write!(f, "presentation dispatch: {e:?}"),
        }
    }
}
impl PresentationAnimation {
    /// Native selected first-pass ordering from `UpdateBodies827de988`: current
    /// full playback/face metadata, begin set/stream, read, extract, end, sync.
    /// Caller supplies the actual source/base and destination allocations.
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "same-owner replay allocations and explicit runtime services"
    )]
    pub(crate) fn try_decode_and_extract<
        N: PresentationDecodeNumeric,
        C: PresentationReadSettings,
        J: ActorAnimationInitializeSource,
    >(
        &self,
        frame: &PresentationReadFrame<'_>,
        source: &OwnedAnimationSqtBuffer,
        destination: &OwnedAnimationSqtBuffer,
        scratch: &mut PresentationReadScratch<'_>,
        settings: &mut C,
        numeric: &mut N,
        jobs: &mut J,
    ) -> Result<
        super::super::final_pose::DecodedPresentationPass,
        PresentationReadPassError<N::Error, C::Error, J::Error>,
    > {
        use PresentationReadPassError as E;
        let offsets = self.offsets.as_ref().ok_or(E::Missing("InitOffsets"))?;
        if offsets.matrix_rotations.len() + offsets.quaternion3_count >= 120
            || offsets.translation_count >= 120
        {
            return Err(E::Missing("native scratch count assertion"));
        }
        let playback = self
            .managers
            .playback
            .lock()
            .map_err(|_| E::Animation(ActorFaceError::Lock))?
            .entries
            .get(&(ActorGraphIntents::native_name(b"SkaterFull"), 0))
            .cloned()
            .ok_or(E::Missing("current SkaterFull"))?;
        let playback = playback
            .lock()
            .map_err(|_| E::Animation(ActorFaceError::Lock))?;
        let _face = {
            let database = playback
                .database
                .lock()
                .map_err(|_| E::Animation(ActorFaceError::Lock))?;
            let parsed = database
                .parsed
                .as_ref()
                .ok_or(E::Animation(ActorFaceError::MissingRecord))?;
            part_range(&parsed.hierarchy, "Face_Offset0").map_err(E::Part)?
        };
        let commands = Arc::clone(&playback.commands);
        drop(playback);
        runtime::begin_command_set(&commands).map_err(E::Command)?;
        let mut stream = StreamCommands::new(
            Arc::clone(&commands),
            destination.clone(),
            AnimationPointerState::Null,
        )
        .map_err(E::Command)?;
        stream.begin_stream().map_err(E::Command)?;
        self.try_read_skater(
            frame,
            source,
            destination,
            scratch,
            settings,
            numeric,
            stream.commands().map_err(E::Command)?,
        )
        .map_err(E::Read)?;
        stream.extract_sqt().map_err(E::Command)?;
        stream.end_stream().map_err(E::Command)?;
        runtime::end_command_set(&commands, jobs).map_err(E::Dispatch)?;
        runtime::sync_command_set(&commands, jobs).map_err(E::Dispatch)?;
        Ok(
            super::super::final_pose::DecodedPresentationPass::from_read(
                self,
                commands,
                destination.clone(),
            ),
        )
    }
}
/// Existing portable numeric profile extended with separately rounded products
/// and left-associated XYZW reduction. Xenon reduction/estimate parity remains
/// an explicit numerical boundary, as for the existing XYZ profile.
impl PresentationDecodeNumeric for crate::reckoning_numeric::HostReckoningNumeric {
    fn dot4(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        let products = std::array::from_fn::<_, 4, _>(|i| a[i] * b[i]);
        Ok(((products[0] + products[1]) + products[2]) + products[3])
    }
}
