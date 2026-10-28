//! `SkateboardController` hand-drive enable/disable on the same retained
//! dynamics payloads admitted by `InitHandDrives`.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "live hand-drive frame service is not wired")
)]

use super::{SkateboardController, SkateboardControllerError, SkateboardControllerPhase};

impl<D> SkateboardController<D> {
    /// `DisableHandDrive` 0x82dc7620..0x82dc76d8. The one fresh cvar read
    /// gates the dynamics writes. The selected hand is cleared only after
    /// both angular and linear stores, or immediately when drives are off.
    pub(crate) fn try_disable_hand_drive_with<E>(
        &mut self,
        mut use_hand_drives: impl FnMut() -> Result<bool, E>,
    ) -> Result<(), SkateboardControllerError<E>> {
        use SkateboardControllerError as Error;
        if self.phase != SkateboardControllerPhase::Complete {
            return Err(Error::NotComplete(self.phase));
        }
        if use_hand_drives().map_err(Error::Service)? {
            let hand = self.hand.expect("constructed hand");
            if hand == 2 {
                return Err(Error::HandAlreadyHolding(hand));
            }
            let slot = usize::try_from(hand)
                .ok()
                .filter(|&slot| slot < 2)
                .ok_or(Error::InvalidHandDrive(hand))?;
            let dynamics = self.dynamics[slot]
                .as_ref()
                .expect("constructed hand dynamics");
            {
                let mut dynamics = dynamics
                    .write()
                    .map_err(|_| Error::PoisonedDynamics(slot))?;
                dynamics.angular.spring = 0.0;
                dynamics.angular.raw_type = 2;
                dynamics.angular.damping = 0.0;
                dynamics.angular.strength = 0.0;
            }
            {
                let mut dynamics = dynamics
                    .write()
                    .map_err(|_| Error::PoisonedDynamics(slot))?;
                dynamics.linear.spring = 0.0;
                dynamics.linear.raw_type = 2;
                dynamics.linear.damping = 0.0;
                dynamics.linear.strength = 0.0;
            }
        }
        self.hand = Some(2);
        Ok(())
    }

    /// Source 0x82dcc618..0x82dcc71c. The first cvar read gates the entire
    /// method. A second read gates the two dynamics writes, while the frames
    /// callback still runs if that second read is false. `UpdateDriveFrames`
    /// owns its own third cvar read and animated-part transforms.
    pub(crate) fn try_enable_hand_drive_with<E>(
        &mut self,
        mut use_hand_drives: impl FnMut() -> Result<bool, E>,
        mut hand_drive_strength: impl FnMut() -> Result<f32, E>,
        mut hand_drive_displacement: impl FnMut() -> Result<f32, E>,
        mut update_drive_frames: impl FnMut(&mut Self, u32) -> Result<(), E>,
    ) -> Result<(), SkateboardControllerError<E>> {
        use SkateboardControllerError as Error;
        if self.phase != SkateboardControllerPhase::Complete {
            return Err(Error::NotComplete(self.phase));
        }
        if !use_hand_drives().map_err(Error::Service)? {
            return Ok(());
        }
        let hand = self.hand.expect("constructed hand");
        if hand == 2 {
            return Err(Error::HandAlreadyHolding(hand));
        }
        let slot = usize::try_from(hand)
            .ok()
            .filter(|&slot| slot < 2)
            .ok_or(Error::InvalidHandDrive(hand))?;
        if use_hand_drives().map_err(Error::Service)? {
            let angular_strength =
                hand_drive_strength().map_err(Error::Service)? * f32::from_bits(0x4560_fffe);
            let angular_spring =
                hand_drive_displacement().map_err(Error::Service)? * f32::from_bits(0x426f_ffff);
            {
                let mut dynamics = self.dynamics[slot]
                    .as_ref()
                    .expect("constructed hand dynamics")
                    .write()
                    .map_err(|_| Error::PoisonedDynamics(slot))?;
                dynamics.angular.spring = angular_spring;
                dynamics.angular.raw_type = 2;
                dynamics.angular.damping = 0.0;
                dynamics.angular.strength = angular_strength;
            }
            let linear_strength =
                hand_drive_strength().map_err(Error::Service)? * f32::from_bits(0x4560_fffe);
            let linear_spring =
                hand_drive_displacement().map_err(Error::Service)? * f32::from_bits(0x426f_ffff);
            {
                let mut dynamics = self.dynamics[slot]
                    .as_ref()
                    .expect("constructed hand dynamics")
                    .write()
                    .map_err(|_| Error::PoisonedDynamics(slot))?;
                dynamics.linear.spring = linear_spring;
                dynamics.linear.raw_type = 2;
                dynamics.linear.damping = 0.0;
                dynamics.linear.strength = linear_strength;
            }
        }
        update_drive_frames(self, hand).map_err(Error::Service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation_drive_pool::{SimulationDriveDynamics, SimulationDriveDynamicsParams};
    use std::{
        cell::RefCell,
        sync::{Arc, RwLock},
    };

    #[test]
    fn first_disabled_read_skips_hand_and_frame_services() {
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        controller.hand = Some(2);
        let mut reads = 0;
        controller
            .try_enable_hand_drive_with(
                || {
                    reads += 1;
                    Ok::<_, ()>(false)
                },
                || panic!("strength is gated"),
                || panic!("displacement is gated"),
                |_, _| panic!("frame update is gated"),
            )
            .unwrap();
        assert_eq!(reads, 1);
    }

    #[test]
    fn fresh_cvar_reads_and_selected_dynamics_order_precede_frames() {
        let zero = SimulationDriveDynamicsParams {
            spring: 0.0,
            damping: 0.0,
            strength: 0.0,
            raw_type: 0,
        };
        let dynamics = Arc::new(RwLock::new(SimulationDriveDynamics {
            linear: zero,
            angular: zero,
        }));
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        controller.hand = Some(1);
        controller.dynamics[1] = Some(Arc::clone(&dynamics));
        let reads = RefCell::new(Vec::new());
        controller
            .try_enable_hand_drive_with(
                || {
                    reads.borrow_mut().push("use");
                    Ok::<_, ()>(true)
                },
                || {
                    reads.borrow_mut().push("strength");
                    Ok::<_, ()>(2.0)
                },
                || {
                    reads.borrow_mut().push("displacement");
                    Ok::<_, ()>(3.0)
                },
                |_, hand| {
                    assert_eq!(hand, 1);
                    reads.borrow_mut().push("frames");
                    Ok::<_, ()>(())
                },
            )
            .unwrap();
        assert_eq!(
            *reads.borrow(),
            [
                "use",
                "use",
                "strength",
                "displacement",
                "strength",
                "displacement",
                "frames"
            ]
        );
        let held = dynamics.read().unwrap();
        assert_eq!(
            held.angular.spring.to_bits(),
            (3.0 * f32::from_bits(0x426f_ffff)).to_bits()
        );
        assert_eq!(
            held.linear.strength.to_bits(),
            (2.0 * f32::from_bits(0x4560_fffe)).to_bits()
        );
        assert_eq!(held.angular.raw_type, 2);
        assert_eq!(held.linear.raw_type, 2);
    }

    #[test]
    fn second_disabled_read_still_updates_frames_without_dynamics_writes() {
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        controller.hand = Some(0);
        let mut reads = 0;
        let mut frames = false;
        controller
            .try_enable_hand_drive_with(
                || {
                    reads += 1;
                    Ok::<_, ()>(reads == 1)
                },
                || panic!("strength is gated"),
                || panic!("displacement is gated"),
                |_, hand| {
                    assert_eq!(hand, 0);
                    frames = true;
                    Ok::<_, ()>(())
                },
            )
            .unwrap();
        assert_eq!(reads, 2);
        assert!(frames);
    }

    #[test]
    fn disable_true_clears_selected_dynamics_before_hand_sentinel() {
        let params = SimulationDriveDynamicsParams {
            spring: 3.0,
            damping: 4.0,
            strength: 5.0,
            raw_type: 7,
        };
        let selected = Arc::new(RwLock::new(SimulationDriveDynamics {
            angular: params,
            linear: params,
        }));
        let untouched = Arc::new(RwLock::new(SimulationDriveDynamics {
            angular: params,
            linear: params,
        }));
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        controller.hand = Some(1);
        controller.dynamics = [Some(Arc::clone(&untouched)), Some(Arc::clone(&selected))];
        let mut reads = 0;
        controller
            .try_disable_hand_drive_with(|| {
                reads += 1;
                Ok::<_, ()>(true)
            })
            .unwrap();
        assert_eq!(reads, 1);
        assert_eq!(controller.hand, Some(2));
        let selected = selected.read().unwrap();
        for drive in [selected.angular, selected.linear] {
            assert_eq!(drive.spring.to_bits(), 0.0_f32.to_bits());
            assert_eq!(drive.raw_type, 2);
            assert_eq!(drive.damping.to_bits(), 0.0_f32.to_bits());
            assert_eq!(drive.strength.to_bits(), 0.0_f32.to_bits());
        }
        let untouched = untouched.read().unwrap();
        assert_eq!(untouched.angular.spring.to_bits(), 3.0_f32.to_bits());
        assert_eq!(untouched.linear.raw_type, 7);
    }

    #[test]
    fn disable_false_clears_hand_without_inspecting_dynamics() {
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        controller.hand = Some(1);
        controller
            .try_disable_hand_drive_with(|| Ok::<_, ()>(false))
            .unwrap();
        assert_eq!(controller.hand, Some(2));
    }

    #[test]
    fn disable_cvar_failure_and_active_sentinel_preserve_hand() {
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        controller.hand = Some(0);
        assert_eq!(
            controller.try_disable_hand_drive_with(|| Err::<bool, _>("cvar")),
            Err(SkateboardControllerError::Service("cvar"))
        );
        assert_eq!(controller.hand, Some(0));
        controller.hand = Some(2);
        assert_eq!(
            controller.try_disable_hand_drive_with(|| Ok::<_, &str>(true)),
            Err(SkateboardControllerError::HandAlreadyHolding(2))
        );
        assert_eq!(controller.hand, Some(2));
    }
}
