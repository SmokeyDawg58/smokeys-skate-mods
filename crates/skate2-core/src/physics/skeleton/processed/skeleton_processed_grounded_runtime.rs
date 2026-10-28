//! Same Processed owner for `GroundSkateboard` stores and subsequent General/IK reads.
use super::SkeletonProcessedContact;
use crate::{
    dynamic_assembly_session::skeleton_normal::general_owner::live_input::{
        GroundedInputError, GroundedInputRead, GroundedOutputWrite,
    },
    skeleton_ik_state::contact::SkeletonIkContactRead,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::cell::RefCell;
pub(crate) struct GroundedProcessed<'a>(RefCell<&'a mut SkeletonProcessedContact>);
impl<'a> GroundedProcessed<'a> {
    /// UpdateOffBoard 0x82b5c7f8..810 publishes the orientation matrix
    /// before setting its validity bit on the same Processed owner.
    pub(crate) fn publish_offboard_orientation(
        &self,
        orientation: SkeletonRootMatrix,
    ) -> Result<(), GroundedInputError> {
        let _prior_flags = self.0.borrow().known_flags_8c4();
        self.write_orientation(orientation)?;
        self.set_animation_orientation_flag();
        Ok(())
    }
    pub(crate) fn set_animation_orientation_flag(&self) {
        self.0
            .borrow_mut()
            .jump_events
            .process_animation_bits(0x0008_0000, 0x0008_0000);
    }
    pub(crate) fn animation_orientation(&self) -> Result<SkeletonRootMatrix, GroundedInputError> {
        need(
            self.0.borrow().skateboard_orientation,
            "animated orientation",
        )
    }
    pub(crate) fn animation_orientation_flag(&self) -> Result<bool, GroundedInputError> {
        let (value, known) = self.0.borrow().known_flags_8c4();
        if known & 0x0008_0000 == 0 {
            return Err(GroundedInputError::Missing("animated flag"));
        }
        Ok(value & 0x0008_0000 != 0)
    }

    pub(crate) fn new(value: &'a mut SkeletonProcessedContact) -> Self {
        Self(RefCell::new(value))
    }
}
fn need<T>(value: Option<T>, name: &'static str) -> Result<T, GroundedInputError> {
    value.ok_or(GroundedInputError::Missing(name))
}
impl GroundedInputRead for GroundedProcessed<'_> {
    fn time_delta(&self) -> Result<f32, GroundedInputError> {
        need(self.0.borrow().physics_state.time_delta, "DT93c")
    }
    fn gravity(&self) -> Result<f32, GroundedInputError> {
        need(self.0.borrow().gravity, "gravity964").map(|v| v.value)
    }
    fn current_state(&self) -> Result<i32, GroundedInputError> {
        need(self.0.borrow().physics_state.current, "current state8e8")
    }
    fn flags_8c8(&self, mask: u32) -> Result<u32, GroundedInputError> {
        need(self.0.borrow().flags_8c8, "8c8").map(|v| v & mask)
    }
    fn flags_8d0(&self, mask: u32) -> Result<u32, GroundedInputError> {
        need(self.0.borrow().physics_state.flags_8d0, "8d0").map(|v| v & mask)
    }
}
impl GroundedOutputWrite for GroundedProcessed<'_> {
    fn write_orientation(&self, value: SkeletonRootMatrix) -> Result<(), GroundedInputError> {
        self.0.borrow_mut().skateboard_orientation = Some(value);
        Ok(())
    }
    fn clear_orientation_flag(&self) -> Result<(), GroundedInputError> {
        self.0
            .borrow_mut()
            .jump_events
            .process_animation_bits(0x0008_0000, 0);
        Ok(())
    }
}
impl SkeletonIkContactRead for GroundedProcessed<'_> {
    fn try_flags_8c4(&self, mask: u32) -> Result<u32, GroundedInputError> {
        let (value, known) = self.0.borrow().known_flags_8c4();
        if known & mask != mask {
            return Err(GroundedInputError::Missing("selected8c4"));
        }
        Ok(value & mask)
    }
    fn try_contact_bone_id(&self) -> Result<Option<usize>, GroundedInputError> {
        Ok(self.0.borrow().contact_bone_id)
    }
    fn try_effective_transform(&self) -> Result<SkeletonRootMatrix, GroundedInputError> {
        need(self.0.borrow().effective, "effective transform80")
    }
}
impl SkeletonProcessedContact {
    pub(crate) const fn skateboard_orientation(&self) -> Option<&SkeletonRootMatrix> {
        self.skateboard_orientation.as_ref()
    }
}

impl GroundedProcessed<'_> {
    pub(crate) fn wipeout_state_timer(&self) -> Result<f32, GroundedInputError> {
        need(self.0.borrow().physics_state.state_timer, "state timer974")
    }
    pub(crate) fn wipeout_centre_velocity(
        &self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, GroundedInputError> {
        need(
            self.0.borrow().prior_output().centre_of_mass_velocity(),
            "COM velocity260",
        )
        .map(|v| v.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        skeleton_general_update::{SkeletonGeneralUpdate, SkeletonGroundedSuffixError},
        skeleton_root_transforms::{SkeletonRootBoard, SkeletonRootTransforms, SkeletonRootVector},
    };
    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    struct Board {
        fail: bool,
        calls: usize,
    }
    impl SkeletonRootBoard for Board {
        type Error = ();
        fn transform(&mut self) -> Result<[f32; 16], Self::Error> {
            panic!("live path must use typed Part")
        }
        fn transform_typed(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
            self.calls += 1;
            if self.fail {
                Err(())
            } else {
                Ok(SkeletonRootMatrix::from_columns(
                    SkeletonRootMatrix::known(IDENTITY).columns().map(|mut v| {
                        v.w = None;
                        v
                    }),
                ))
            }
        }
        fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
            panic!("suffix must not sample velocity")
        }
    }
    #[test]
    fn actual_processed_publication_precedes_failing_fresh_part_and_keeps_neighbors() {
        let mut processed = SkeletonProcessedContact::new();
        assert!(processed.skateboard_orientation().is_none());
        processed.reset();
        assert_eq!(
            processed
                .skateboard_orientation()
                .unwrap()
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            IDENTITY.map(f32::to_bits)
        );
        processed.skateboard_orientation = Some(SkeletonRootMatrix::known([7.0; 16]));
        processed
            .jump_events
            .process_animation_bits(u32::MAX, u32::MAX);
        let mut general = SkeletonGeneralUpdate::new();
        let roots = SkeletonRootTransforms::new();
        let mut board = Board {
            fail: true,
            calls: 0,
        };
        {
            let input = GroundedProcessed::new(&mut processed);
            assert!(matches!(
                general.try_grounded_suffix_write(&roots, &IDENTITY, &input, &mut board),
                Err(SkeletonGroundedSuffixError::Board(()))
            ));
        }
        assert_eq!(board.calls, 1);
        assert_eq!(
            processed
                .skateboard_orientation()
                .unwrap()
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            IDENTITY.map(f32::to_bits)
        );
        assert_eq!(processed.known_flags_8c4(), (!0x0008_0000, u32::MAX));
    }
    #[test]
    fn actual_typed_part_lanes_and_contact_missing_reads_remain_lazy() {
        let mut processed = SkeletonProcessedContact::new();
        let input = GroundedProcessed::new(&mut processed);
        let mut general = SkeletonGeneralUpdate::new();
        let roots = SkeletonRootTransforms::new();
        let mut board = Board {
            fail: false,
            calls: 0,
        };
        general
            .try_grounded_suffix_write(&roots, &IDENTITY, &input, &mut board)
            .unwrap();
        assert!(
            general
                .physics_skateboard()
                .columns()
                .iter()
                .all(|v| v.w.is_none())
        );
        assert!(matches!(
            input.try_effective_transform(),
            Err(GroundedInputError::Missing("effective transform80"))
        ));
        assert!(input.try_flags_8c4(0x0008_0000).is_ok());
        assert!(input.try_flags_8c4(0x4200_0000).is_err());
    }
}
