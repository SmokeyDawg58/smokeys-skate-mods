//! Live same-board dispatch for the two EnableBoardAnimation leaves.
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SkateboardEntrySessionError,
    air_drive::{self, AirDriveError},
};
use crate::{
    input_cvar_runtime::InputCvarReferences, reckoning_numeric::ReckoningNumeric,
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_force_runtime::entry::animation_enable::EnableBoardAnimationError,
};
use std::{cell::RefCell, sync::Arc};

#[derive(Debug)]
pub(crate) enum BoardAnimationRuntimeError<N> {
    Owner(SkateboardEntrySessionError),
    Animation(EnableBoardAnimationError<AirDriveError<N>>),
}
impl<N: std::fmt::Debug> std::fmt::Display for BoardAnimationRuntimeError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "board animation: {self:?}")
    }
}

impl SimulationSkaterAssemblyPool {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_enable_source_plant(
        &mut self,
        backend:&SkateboardSimulationBackend,
        board:&SimulationSkaterAssemblyHandle,
        boneless:bool,
    )->Result<(),BoardAnimationRuntimeError<std::convert::Infallible>> {
        use BoardAnimationRuntimeError as E;
        self.check_backend(backend).map_err(|e|E::Owner(SkateboardEntrySessionError::Pool(e)))?;
        let index=self.member_index(board).map_err(|e|E::Owner(SkateboardEntrySessionError::Pool(e)))?;
        let candidate=self.members[index].content.board_mut()
            .map_err(|e|E::Owner(SkateboardEntrySessionError::Pool(e)))?;
        let runtime=candidate.force_runtime.as_mut()
            .ok_or(E::Owner(SkateboardEntrySessionError::MissingConstruction))?;
        let hook=&runtime.hook_dynamics;
        let capture=||hook.as_ref().map(Arc::clone).ok_or(AirDriveError::Entry(SkateboardEntrySessionError::MissingHook));
        let mut drive=skate3_core::physics::hook_drive::HookDriveState::initial();
        if boneless {crate::skate3_native::physics::boneless::Boneless::configure_drive(&mut drive);}
        else {let mut animated=0;drive.enable_angular_only(&mut animated);}
        // Only the source parameter words are consumed. Temporary source frame
        // initialization is never published to the actual S2 joint frames.
        runtime.entry.try_enable_source_plant(capture,drive.dynamics,!boneless).map_err(E::Animation)
    }

    /// Source `EnableBoardAnimation` 0x82b2a140 or soft 0x82b2a188.
    /// Closures capture the Body only at their native call sites; in the soft
    /// path the animated bit is therefore retained if the first Body is absent.
    pub(crate) fn try_enable_board_animation_for_hold<N: ReckoningNumeric>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        soft: bool,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), BoardAnimationRuntimeError<N::Error>> {
        use BoardAnimationRuntimeError as E;
        self.check_backend(backend)
            .map_err(|e| E::Owner(SkateboardEntrySessionError::Pool(e)))?;
        let index = self
            .member_index(board)
            .map_err(|e| E::Owner(SkateboardEntrySessionError::Pool(e)))?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(|e| E::Owner(SkateboardEntrySessionError::Pool(e)))?;
        let runtime = candidate
            .force_runtime
            .as_mut()
            .ok_or(E::Owner(SkateboardEntrySessionError::MissingConstruction))?;
        let hook = &runtime.hook_dynamics;
        let capture = || {
            hook.as_ref().map(Arc::clone).ok_or(AirDriveError::Entry(
                SkateboardEntrySessionError::MissingHook,
            ))
        };
        if soft {
            let numeric = RefCell::new(numeric);
            runtime
                .entry
                .try_enable_board_animation_soft(
                    1.0,
                    |strength| {
                        let body = capture()?;
                        air_drive::write_board_linear_soft(
                            &body,
                            strength,
                            refs,
                            &mut **numeric.borrow_mut(),
                        )
                    },
                    |strength| {
                        let body = capture()?;
                        air_drive::write_board_angular_soft(
                            &body,
                            strength,
                            refs,
                            &mut **numeric.borrow_mut(),
                        )
                    },
                )
                .map_err(E::Animation)
        } else {
            runtime
                .entry
                .try_enable_board_animation(capture, |strength| {
                    let body = hook.as_ref().ok_or(AirDriveError::Entry(
                        SkateboardEntrySessionError::MissingHook,
                    ))?;
                    air_drive::write_board_angular_soft(body, strength, refs, numeric)
                })
                .map_err(E::Animation)
        }
    }
}
