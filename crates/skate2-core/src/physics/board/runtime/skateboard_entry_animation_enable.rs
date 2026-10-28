//! The two `Skateboard::EnableBoardAnimation` paths at 0x82b2a140/0x82b2a188.
//! The callbacks are the actual Body soft-drive operations, not substitutes for
//! their cvar reads, numeric estimate, or retained dynamics writes.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "live HoldSkateboard collaborators are not wired")
)]

use super::SkateboardEntryState;
use crate::simulation_drive_pool::SimulationDriveDynamics;
use std::sync::{Arc, RwLock};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EnableBoardAnimationError<E> {
    Body(E),
    LinearPoisoned,
    SoftDrive(E),
}

impl SkateboardEntryState {
    /// `EnableBoardAnimation` 0x82b2a140. Capture Body first, publish the
    /// animated bit, write the hard linear parameters in source order, then run
    /// `HookEnableAngularDriveSoft(1.0)` on the same retained Body.
    pub(crate) fn try_enable_board_animation<E>(
        &mut self,
        mut capture_body: impl FnMut() -> Result<Arc<RwLock<SimulationDriveDynamics>>, E>,
        mut enable_angular_soft: impl FnMut(f32) -> Result<(), E>,
    ) -> Result<(), EnableBoardAnimationError<E>> {
        use EnableBoardAnimationError as Error;
        let body = capture_body().map_err(Error::Body)?;
        self.animated = true;
        {
            let mut dynamics = body.write().map_err(|_| Error::LinearPoisoned)?;
            dynamics.linear.raw_type = 2;
            dynamics.linear.spring = f32::from_bits(0x426f_ffff);
            dynamics.linear.damping = 0.0;
            dynamics.linear.strength = f32::from_bits(0x4560_fffe);
        }
        enable_angular_soft(1.0).map_err(Error::SoftDrive)
    }

    /// Mod-only publication of source S3 plant dynamics on this actual hook.
    /// No cvar substitution, second drive allocation or joint-frame change.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_enable_source_plant<E>(
        &mut self,
        mut capture_body:impl FnMut()->Result<Arc<RwLock<SimulationDriveDynamics>>,E>,
        words:[u32;8],
        angular_first:bool,
    )->Result<(),EnableBoardAnimationError<E>> {
        use EnableBoardAnimationError as Error;
        let body=capture_body().map_err(Error::Body)?;
        self.animated=true;
        let params=|offset:usize|crate::simulation_drive_pool::SimulationDriveDynamicsParams {
            spring:f32::from_bits(words[offset]),damping:f32::from_bits(words[offset+1]),
            strength:f32::from_bits(words[offset+2]),raw_type:words[offset+3],
        };
        let mut dynamics=body.write().map_err(|_|Error::LinearPoisoned)?;
        if angular_first {dynamics.angular=params(4);dynamics.linear=params(0);}
        else {dynamics.linear=params(0);dynamics.angular=params(4);}
        Ok(())
    }

    /// `EnableBoardAnimationSoft` 0x82b2a188. The source's two Body calls each
    /// capture the current Body and use the same caller strength. A failure in
    /// the linear call leaves the already-written animated bit in place.
    pub(crate) fn try_enable_board_animation_soft<E>(
        &mut self,
        strength: f32,
        mut enable_linear_soft: impl FnMut(f32) -> Result<(), E>,
        mut enable_angular_soft: impl FnMut(f32) -> Result<(), E>,
    ) -> Result<(), EnableBoardAnimationError<E>> {
        use EnableBoardAnimationError as Error;
        self.animated = true;
        enable_linear_soft(strength).map_err(Error::SoftDrive)?;
        enable_angular_soft(strength).map_err(Error::SoftDrive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation_drive_pool::SimulationDriveDynamicsParams;
    use std::cell::RefCell;

    #[test]
    fn hard_animation_keeps_linear_prefix_when_angular_soft_fails() {
        let zero = SimulationDriveDynamicsParams {
            spring: 0.0,
            damping: 0.0,
            strength: 0.0,
            raw_type: 0,
        };
        let body = Arc::new(RwLock::new(SimulationDriveDynamics {
            linear: zero,
            angular: zero,
        }));
        let mut entry = SkateboardEntryState::new();
        assert_eq!(
            entry.try_enable_board_animation(
                || Ok(Arc::clone(&body)),
                |value| {
                    assert_eq!(value.to_bits(), 1.0f32.to_bits());
                    Err::<(), _>("angular")
                }
            ),
            Err(EnableBoardAnimationError::SoftDrive("angular"))
        );
        assert!(entry.animated());
        let dynamics = body.read().unwrap();
        assert_eq!(dynamics.linear.raw_type, 2);
        assert_eq!(dynamics.linear.spring.to_bits(), 0x426f_ffff);
        assert_eq!(dynamics.linear.strength.to_bits(), 0x4560_fffe);
        assert_eq!(dynamics.angular, zero);
    }

    #[test]
    fn soft_animation_calls_linear_then_angular_with_same_strength() {
        let mut entry = SkateboardEntryState::new();
        let calls = RefCell::new(Vec::new());
        entry
            .try_enable_board_animation_soft(
                0.5,
                |strength| {
                    calls.borrow_mut().push(("linear", strength.to_bits()));
                    Ok::<_, ()>(())
                },
                |strength| {
                    calls.borrow_mut().push(("angular", strength.to_bits()));
                    Ok::<_, ()>(())
                },
            )
            .unwrap();
        assert!(entry.animated());
        assert_eq!(
            *calls.borrow(),
            [("linear", 0.5f32.to_bits()), ("angular", 0.5f32.to_bits())]
        );
    }
}
