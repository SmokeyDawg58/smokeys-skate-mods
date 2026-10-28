//! `SkateboardController::UpdateDriveFrames` 0x82dcc408 on the retained slot.
//! Animation part reads precede child and parent DriveFrames setters.
use super::{IDENTITY, SkateboardController, SkateboardControllerPhase};
use crate::{
    skateboard_spawn_math::{skateboard_spawn_compose, skateboard_spawn_rigid_inverse},
    world::try_simulation_quaternion_candidate,
};

#[derive(Debug, PartialEq)]
pub(crate) enum ControllerDriveFramesError<E> {
    NotConstructed(SkateboardControllerPhase),
    Source(E),
    FreeHand,
    InvalidHand(u32),
    MissingFrames(usize),
    PoisonedFrames(usize),
}
impl<E: std::fmt::Debug> std::fmt::Display for ControllerDriveFramesError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SkateboardController UpdateDriveFrames: {self:?}")
    }
}

impl<D> SkateboardController<D> {
    /// Source gate/read order and same mutable payload as the two constructed
    /// controller drives. `GetAnimPartTransform` reads Part 0 before Part 3/7;
    /// native builds the hand frame relative to animated skateboard root.
    /// The two frame setters convert their own matrix to quaternion and copy
    /// translation, so a late estimate failure retains the child write.
    pub(crate) fn try_update_drive_frames_with<E>(
        &mut self,
        mut use_hand_drives: impl FnMut() -> Result<bool, E>,
        mut anim_part_transform: impl FnMut(usize) -> Result<[f32; 16], E>,
        mut reciprocal_sqrt_estimates: impl FnMut([f32; 4]) -> Result<[f32; 4], E>,
    ) -> Result<(), ControllerDriveFramesError<E>> {
        use ControllerDriveFramesError as Error;
        if self.phase != SkateboardControllerPhase::Complete {
            return Err(Error::NotConstructed(self.phase));
        }
        if !use_hand_drives().map_err(Error::Source)? {
            return Ok(());
        }
        let hand = self.hand.expect("constructed controller hand");
        if hand == 2 {
            return Err(Error::FreeHand);
        }
        let slot = usize::try_from(hand)
            .ok()
            .filter(|slot| *slot < 2)
            .ok_or(Error::InvalidHand(hand))?;
        let hand_part = if slot == 0 { 3 } else { 7 };
        let root = anim_part_transform(0).map_err(Error::Source)?;
        let hand_transform = anim_part_transform(hand_part).map_err(Error::Source)?;
        let parent =
            skateboard_spawn_compose(&skateboard_spawn_rigid_inverse(&root), &hand_transform);
        let child_quaternion =
            try_simulation_quaternion_candidate(IDENTITY, &mut reciprocal_sqrt_estimates)
                .map_err(Error::Source)?;
        let frames = self.frames[slot]
            .as_ref()
            .ok_or(Error::MissingFrames(slot))?;
        {
            let mut frames = frames.write().map_err(|_| Error::PoisonedFrames(slot))?;
            frames.quaternion_a = child_quaternion;
            frames.position_a.copy_from_slice(&IDENTITY[12..16]);
        }
        let parent_quaternion =
            try_simulation_quaternion_candidate(parent, &mut reciprocal_sqrt_estimates)
                .map_err(Error::Source)?;
        {
            let mut frames = frames.write().map_err(|_| Error::PoisonedFrames(slot))?;
            frames.quaternion_b = parent_quaternion;
            frames.position_b.copy_from_slice(&parent[12..16]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation_drive_pool::SimulationDriveFrames;
    use std::{
        cell::RefCell,
        sync::{Arc, RwLock},
    };

    fn frames() -> Arc<RwLock<SimulationDriveFrames>> {
        Arc::new(RwLock::new(SimulationDriveFrames {
            quaternion_a: [9.0; 4],
            position_a: [9.0; 4],
            quaternion_b: [9.0; 4],
            position_b: [9.0; 4],
        }))
    }

    #[test]
    fn disabled_gate_reads_no_hand_or_animation_and_keeps_both_slots() {
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        let frame = frames();
        controller.frames[0] = Some(Arc::clone(&frame));
        controller
            .try_update_drive_frames_with(
                || Ok::<_, &'static str>(false),
                |_| panic!("animation is gated"),
                |_| panic!("quaternion is gated"),
            )
            .unwrap();
        assert_eq!(frame.read().unwrap().position_a, [9.0; 4]);
    }

    #[test]
    fn selected_hand_reads_root_then_hand_and_updates_same_retained_payload() {
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        controller.hand = Some(1);
        let first = frames();
        let second = frames();
        controller.frames = [Some(Arc::clone(&first)), Some(Arc::clone(&second))];
        let events = RefCell::new(Vec::new());
        controller
            .try_update_drive_frames_with(
                || {
                    events.borrow_mut().push("cvar");
                    Ok::<_, &'static str>(true)
                },
                |part| {
                    events
                        .borrow_mut()
                        .push(if part == 0 { "root" } else { "right hand" });
                    assert!(part == 0 || part == 7);
                    let mut value = IDENTITY;
                    value[12] = if part == 0 { 10.0 } else { 12.0 };
                    Ok(value)
                },
                |magnitudes| {
                    events.borrow_mut().push("estimate");
                    Ok(magnitudes.map(|value| value.sqrt().recip()))
                },
            )
            .unwrap();
        assert_eq!(
            *events.borrow(),
            ["cvar", "root", "right hand", "estimate", "estimate"]
        );
        assert_eq!(first.read().unwrap().position_b, [9.0; 4]);
        let second = second.read().unwrap();
        assert_eq!(second.position_a, [0.0; 4]);
        assert_eq!(second.position_b[0], 2.0);
    }

    #[test]
    fn rotated_root_makes_hand_frame_root_relative_while_child_stays_identity() {
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        controller.hand = Some(0);
        let frame = frames();
        controller.frames[0] = Some(Arc::clone(&frame));
        // Root axes turn world X toward +Y. The hand stays world aligned and
        // sits two world-Y units from the root. In root space this is +X,
        // with the hand rotated -90 degrees around Z.
        let root = [
            0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 20.0, 0.0, 0.0,
        ];
        let mut hand = IDENTITY;
        hand[12] = 10.0;
        hand[13] = 22.0;
        controller
            .try_update_drive_frames_with(
                || Ok::<_, &'static str>(true),
                |part| match part {
                    0 => Ok(root),
                    3 => Ok(hand),
                    _ => panic!("unexpected animation part {part}"),
                },
                |magnitudes| Ok(magnitudes.map(|value| value.sqrt().recip())),
            )
            .unwrap();
        let frame = frame.read().unwrap();
        let half_turn = std::f32::consts::FRAC_1_SQRT_2;
        for (actual, expected) in frame.quaternion_a.into_iter().zip([0.0, 0.0, 0.0, 1.0]) {
            assert!((actual - expected).abs() < 1e-5);
        }
        assert_eq!(frame.position_a, [0.0; 4]);
        for (actual, expected) in frame
            .quaternion_b
            .into_iter()
            .zip([0.0, 0.0, -half_turn, half_turn])
        {
            assert!((actual - expected).abs() < 1e-5);
        }
        assert!((frame.position_b[0] - 2.0).abs() < 1e-5);
        assert!(frame.position_b[1].abs() < 1e-5);
        assert!(frame.position_b[2].abs() < 1e-5);
    }

    #[test]
    fn parent_estimate_failure_retains_child_setter_prefix() {
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        controller.hand = Some(0);
        let frame = frames();
        controller.frames[0] = Some(Arc::clone(&frame));
        let mut estimates = 0;
        assert_eq!(
            controller.try_update_drive_frames_with(
                || Ok::<_, &'static str>(true),
                |part| {
                    assert!(part == 0 || part == 3);
                    Ok(IDENTITY)
                },
                |magnitudes| {
                    estimates += 1;
                    if estimates == 2 {
                        Err("parent estimate")
                    } else {
                        Ok(magnitudes.map(|value| value.sqrt().recip()))
                    }
                },
            ),
            Err(ControllerDriveFramesError::Source("parent estimate"))
        );
        let frame = frame.read().unwrap();
        assert_eq!(frame.position_a, [0.0; 4]);
        assert_eq!(frame.position_b, [9.0; 4]);
    }
}
