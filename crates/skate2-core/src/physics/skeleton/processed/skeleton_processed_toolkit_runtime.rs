//! Same retained processed sources for the complete `PrepareToolkitInput` call.
use super::{
    SkeletonProcessedContact, SkeletonProcessedToolkitError, SkeletonProcessedToolkitInputRead,
    SkeletonProcessedToolkitJumpBoard, SkeletonProcessedToolkitJumpCvars,
    SkeletonProcessedToolkitJumpError, SkeletonProcessedToolkitJumpInputRead,
};
use crate::{
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::{
        SkeletonProcessedContactBoard, jump_events::SkeletonProcessedJumpEvents,
        prior_output::SkeletonProcessedPriorOutputState,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

pub(crate) trait ToolkitPhysicalBoardRead {
    type Error;
    fn mass(&mut self) -> Result<f32, Self::Error>;
    fn deck_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub(crate) enum ToolkitRetainedInputError {
    UnknownBoardFlip,
    MissingZSpeed,
    MissingGroundNormal,
}
impl std::fmt::Display for ToolkitRetainedInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnknownBoardFlip => "toolkit processed board-flip bit is unknown",
            Self::MissingZSpeed => "toolkit processed prior Z speed is unknown",
            Self::MissingGroundNormal => "toolkit processed prior ground normal is unknown",
        })
    }
}

#[derive(Debug)]
pub(crate) enum ToolkitPhysicalReadError<P> {
    Physical(P),
    Input(ToolkitRetainedInputError),
}
impl<P: std::fmt::Display> std::fmt::Display for ToolkitPhysicalReadError<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Physical(error) => write!(f, "toolkit physical prefix: {error}"),
            Self::Input(error) => error.fmt(f),
        }
    }
}

fn read_flip(flags: &SkeletonProcessedJumpEvents) -> Result<u32, ToolkitRetainedInputError> {
    let (value, known) = flags.known_word();
    if known & 0x0010_0000 == 0 {
        return Err(ToolkitRetainedInputError::UnknownBoardFlip);
    }
    // Only this proven mask is consumed by the source. Unknown neighboring bits
    // are not presented as a completed raw flags word to any other consumer.
    Ok(value & 0x0010_0000)
}

struct PhysicalRead<'a, P> {
    board: &'a mut P,
    flags: &'a SkeletonProcessedJumpEvents,
}
impl<P: ToolkitPhysicalBoardRead> SkeletonProcessedContactBoard for PhysicalRead<'_, P> {
    type Error = ToolkitPhysicalReadError<P::Error>;
    fn mass(&mut self) -> Result<f32, Self::Error> {
        self.board
            .mass()
            .map_err(ToolkitPhysicalReadError::Physical)
    }
    fn deck_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.board
            .deck_transform()
            .map_err(ToolkitPhysicalReadError::Physical)
    }
    fn flags_8c4(&self) -> u32 {
        self.flags.known_word().0
    }
}

struct RetainedRead<'a> {
    prior: &'a SkeletonProcessedPriorOutputState,
    flags: &'a SkeletonProcessedJumpEvents,
}
impl SkeletonProcessedToolkitInputRead for RetainedRead<'_> {
    type Error = ToolkitRetainedInputError;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
        read_flip(self.flags)
    }
    fn z_speed(&mut self) -> Result<f32, Self::Error> {
        self.prior
            .z_speed()
            .map(|v| v.value)
            .ok_or(ToolkitRetainedInputError::MissingZSpeed)
    }
}
impl SkeletonProcessedToolkitJumpInputRead for RetainedRead<'_> {
    type Error = ToolkitRetainedInputError;
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.prior
            .ground_normal()
            .map(|v| v.value)
            .ok_or(ToolkitRetainedInputError::MissingGroundNormal)
    }
}

type LiveJumpResult<B, C, N> = Result<
    (),
    SkeletonProcessedToolkitJumpError<
        ToolkitRetainedInputError,
        <B as SkeletonProcessedToolkitJumpBoard>::Error,
        <C as SkeletonProcessedToolkitJumpCvars>::Error,
        <N as ReckoningNumeric>::Error,
    >,
>;

impl SkeletonProcessedContact {
    /// Mass, deck, fresh deck, THEN retained flip. Earlier stores survive errors.
    pub(crate) fn try_sample_live_toolkit_prefix<P: ToolkitPhysicalBoardRead>(
        &mut self,
        board: &mut P,
    ) -> Result<(), ToolkitPhysicalReadError<P::Error>> {
        let mut source = PhysicalRead {
            board,
            flags: &self.jump_events,
        };
        super::super::try_sample_toolkit_prefix_fields(
            &mut self.physical_mass,
            &mut self.deck,
            &mut self.effective,
            &mut self.toolkit_prefix_complete,
            &mut self.toolkit,
            &mut source,
            |source| read_flip(source.flags).map_err(ToolkitPhysicalReadError::Input),
        )
    }
    pub(crate) fn try_continue_live_toolkit_directions<N: ReckoningNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), SkeletonProcessedToolkitError<ToolkitRetainedInputError, N::Error>> {
        let mut input = RetainedRead {
            prior: &self.prior_output,
            flags: &self.jump_events,
        };
        self.toolkit.try_continue_directions(
            self.toolkit_prefix_complete,
            self.deck.as_ref(),
            self.effective.as_ref(),
            &mut input,
            numeric,
        )
    }
    pub(crate) fn try_continue_live_toolkit_jump_normal<
        B: SkeletonProcessedToolkitJumpBoard,
        C: SkeletonProcessedToolkitJumpCvars,
        N: ReckoningNumeric,
    >(
        &mut self,
        board: &mut B,
        cvars: &mut C,
        numeric: &mut N,
    ) -> LiveJumpResult<B, C, N> {
        let mut input = RetainedRead {
            prior: &self.prior_output,
            flags: &self.jump_events,
        };
        self.toolkit
            .try_continue_jump_normal(&mut input, board, cvars, numeric)
    }
}
