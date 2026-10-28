//! Actual retained request decisions and scalars82e05438..55e0.
use super::PhysicalPlayerAssemblyOwner;
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{
        InputCvarError,
        wipeout_decision::{WipeoutDecisionCvars, WipeoutEnableTarget},
    },
    physical_player_phys_out::construct::update5_decisions::{
        Update5DecisionOutputError, Update5DecisionOutputSource,
    },
    physical_player_wipeout::{
        WipeoutOwnerError,
        decision::{self, HiLodWipeoutDecisionSource, RunoutDecisionSource, WipeoutDecisionSource},
    },
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
    wipeout::WipeoutType,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum DecisionRuntimeError<N> {
    Poisoned,
    Missing(&'static str),
    Wipeout(WipeoutOwnerError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Numeric(N),
}
impl<N: std::fmt::Debug> std::fmt::Display for DecisionRuntimeError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Update5 decision owner poisoned"),
            Self::Missing(field) => write!(f, "Update5 decision requires {field}"),
            Self::Wipeout(error) => write!(f, "Update5 wipeout owner: {error:?}"),
            Self::Cvar(error) => error.fmt(f),
            Self::Overlay(error) => error.fmt(f),
            Self::Numeric(error) => write!(f, "Update5 runout dot: {error:?}"),
        }
    }
}
struct Source<'a, N> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    overlay: &'a DebugOverlaySlot,
    cvars: WipeoutDecisionCvars<'a>,
    numeric: &'a mut N,
}
/// Actual `IsWipeoutRequested82e00a50`, also called by Update4 at82e01d68.
pub(crate) fn try_is_wipeout_requested<N: ReckoningNumeric>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    overlay: &DebugOverlaySlot,
    numeric: &mut N,
) -> Result<bool, DecisionRuntimeError<N::Error>> {
    decision::try_is_wipeout_requested(&mut Source {
        owner,
        overlay,
        cvars: WipeoutDecisionCvars::new(&owner.inputs().cvars),
        numeric,
    })
}
impl<N: ReckoningNumeric> Source<'_, N> {
    fn processed<T>(
        &self,
        read: impl FnOnce(&SkeletonProcessedContact) -> Result<T, DecisionRuntimeError<N::Error>>,
    ) -> Result<T, DecisionRuntimeError<N::Error>> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| DecisionRuntimeError::Poisoned)?
                .processed(),
        );
        let input = processed
            .lock()
            .map_err(|_| DecisionRuntimeError::Poisoned)?;
        read(&input)
    }
    fn flag_word(&self, mask: u32) -> Result<u32, DecisionRuntimeError<N::Error>> {
        self.processed(|p| {
            let (value, known) = p.known_flags_8c4();
            if known & mask == mask {
                Ok(value)
            } else {
                Err(DecisionRuntimeError::Missing("Processed8c4 bits"))
            }
        })
    }
}
impl<N: ReckoningNumeric> WipeoutDecisionSource for Source<'_, N> {
    type Error = DecisionRuntimeError<N::Error>;
    type EnableTarget = WipeoutEnableTarget;
    fn disable_all(&mut self) -> Result<bool, Self::Error> {
        self.cvars.disable_all().map_err(Self::Error::Cvar)
    }
    fn capture_enable(&mut self, index: usize) -> Result<Self::EnableTarget, Self::Error> {
        self.cvars.capture_enable(index).map_err(Self::Error::Cvar)
    }
    fn read_enable(&mut self, target: &Self::EnableTarget) -> Result<bool, Self::Error> {
        WipeoutDecisionCvars::read_enable(target).map_err(Self::Error::Cvar)
    }
    fn requested(&mut self, index: usize) -> Result<bool, Self::Error> {
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_is_requested(WipeoutType::ALL[index])
            .map_err(Self::Error::Wipeout)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
    fn display_debug(&mut self) -> Result<bool, Self::Error> {
        self.cvars.display_debug().map_err(Self::Error::Cvar)
    }
}
impl<N: ReckoningNumeric> RunoutDecisionSource for Source<'_, N> {
    type Error = DecisionRuntimeError<N::Error>;
    fn request_count(&mut self) -> Result<u32, Self::Error> {
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_request_count()
            .map_err(Self::Error::Wipeout)
    }
    fn requested(&mut self, index: usize) -> Result<bool, Self::Error> {
        WipeoutDecisionSource::requested(self, index)
    }
    fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
        self.flag_word(0x20)
    }
    fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
        self.processed(|p| {
            p.physics_state_fields()
                .flags_8cc
                .ok_or(Self::Error::Missing("Processed8cc"))
        })
    }
    fn upright_dot(&mut self) -> Result<f32, Self::Error> {
        let up = self.processed(|p| {
            p.prior_output()
                .reckoning_up()
                .map(|v| v.value)
                .ok_or(Self::Error::Missing("Processed up220"))
        })?;
        self.numeric
            .dot_vector(up, [0.0, 1.0, 0.0, 0.0])
            .map_err(Self::Error::Numeric)
    }
}
impl<N: ReckoningNumeric> HiLodWipeoutDecisionSource for Source<'_, N> {
    type Error = DecisionRuntimeError<N::Error>;
    fn runout(&mut self) -> Result<bool, Self::Error> {
        decision::try_is_runout_requested(self)
    }
    fn player_request(&mut self) -> Result<bool, Self::Error> {
        decision::try_does_player_request(self)
    }
    fn flags_8d0(&mut self) -> Result<u32, Self::Error> {
        self.processed(|p| {
            p.physics_state_fields()
                .flags_8d0
                .ok_or(Self::Error::Missing("Processed8d0"))
        })
    }
}
impl<N: ReckoningNumeric> Update5DecisionOutputSource for Source<'_, N> {
    type Error = DecisionRuntimeError<N::Error>;
    fn wipeout_requested(&mut self) -> Result<bool, Self::Error> {
        decision::try_is_wipeout_requested(self)
    }
    fn runout_requested(&mut self) -> Result<bool, Self::Error> {
        decision::try_is_runout_requested(self)
    }
    fn heading_adjust_factor(&mut self) -> Result<f32, Self::Error> {
        self.processed(|p| {
            p.physics_state_fields()
                .heading_adjust_factor
                .ok_or(Self::Error::Missing("headingAdjust9b4"))
        })
    }
    fn flags_8c4(&mut self, required_mask: u32) -> Result<u32, Self::Error> {
        self.flag_word(required_mask)
    }
    fn animation_push_speed(&mut self) -> Result<f32, Self::Error> {
        self.processed(|p| {
            p.animation_attribute_fields()
                .push_speed
                .ok_or(Self::Error::Missing("pushSpeed940"))
        })
    }
    fn offboard_look_x(&mut self) -> Result<f32, Self::Error> {
        self.processed(|p| {
            p.animation_attribute_fields()
                .look_x
                .ok_or(Self::Error::Missing("lookX990"))
        })
    }
    fn offboard_look_y(&mut self) -> Result<f32, Self::Error> {
        self.processed(|p| {
            p.animation_attribute_fields()
                .look_y
                .ok_or(Self::Error::Missing("lookY994"))
        })
    }
}
pub(crate) fn try_publish_decisions<N: ReckoningNumeric>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    overlay: &DebugOverlaySlot,
    numeric: &mut N,
) -> Result<(), Update5DecisionOutputError<DecisionRuntimeError<N::Error>>> {
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| Update5DecisionOutputError::Source(DecisionRuntimeError::Poisoned))?;
    output.try_publish_update5_decisions(&mut Source {
        owner,
        overlay,
        cvars: WipeoutDecisionCvars::new(&owner.inputs().cvars),
        numeric,
    })
}
