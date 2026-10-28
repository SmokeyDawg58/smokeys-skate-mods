//! Child of `skeleton_ik_state`: mandatory `UpdatePostPhysics82b55398`.
use super::{
    SkeletonIkState,
    post_wipeout_foot::{FootIkError, FootIkSource, commit, two_bone},
};
use crate::{
    reckoning_numeric::ReckoningNumeric, reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_ik_targets::SkeletonIkTargets,
};

pub(crate) trait IkPostUpdateSource: FootIkSource {
    fn do_new_post_physics_ik(&mut self) -> Result<bool, Self::Error>;
    fn draw_new_post_physics_ik(&mut self) -> Result<bool, Self::Error>;
}
impl SkeletonIkState {
    /// Always called after conditionally selected foot/hand helpers, even while
    /// teleporting/offboard. Disabled IK still consumes all twelve target flags.
    #[expect(
        clippy::type_complexity,
        reason = "preserve separate source and numeric failures"
    )]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "native BO4/BI25 skips unordered as well as nonpositive weights"
    )]
    pub(crate) fn try_post_wipeout_ik_update<
        S: IkPostUpdateSource,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        targets: &SkeletonIkTargets,
        source: &mut S,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), FootIkError<S::Error, N::Error, R::Error>> {
        use FootIkError as E;
        if source.do_new_post_physics_ik().map_err(E::Source)? {
            for entry in 0..2 {
                // Raw55400=4099010c tests !GT, including unordered.
                if !(self.external_blends[entry] > 0.0) {
                    continue;
                }
                let parent = targets.parents()[entry].ok_or(E::Missing("post IK parent"))?;
                let target = self.parent_to_world[entry].columns()[3];
                let current = source.record(parent).map_err(E::Source)?.columns()[3];
                let delta = target.zip(current, |target, current| target - current);
                if two_bone(targets.data(), parent, delta, source, numeric, rounding)? {
                    let part = targets.parts()[entry].ok_or(E::Missing("post IK primary"))?;
                    commit(source, part, &self.part_to_world[entry])?;
                    commit(source, parent, &self.parent_to_world[entry])?;
                }
                if source.draw_new_post_physics_ik().map_err(E::Source)? {
                    source.require_overlay().map_err(E::Source)?;
                }
            }
        }
        // Three groups, not a per-entry interleaving; source55520..5554c.
        for entry in 0..4 {
            self.external_target_normal_set[entry] = false;
        }
        for entry in 0..4 {
            self.external_target_set[entry] = false;
        }
        for entry in 0..4 {
            self.local_target_set[entry] = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ragdoll::SkeletonPartIndex as Part,
        skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
    };
    struct Source {
        enabled: Result<bool, &'static str>,
        events: Vec<&'static str>,
    }
    impl FootIkSource for Source {
        type Error = &'static str;
        fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
            panic!("not in postupdate")
        }
        fn state_timer(&mut self) -> Result<f32, Self::Error> {
            panic!("not in postupdate")
        }
        fn minimum_deck_up_y(&mut self) -> Result<f32, Self::Error> {
            panic!("not in postupdate")
        }
        fn deck_bounds_margin(&mut self) -> Result<Vector, Self::Error> {
            panic!("not in postupdate")
        }
        fn feet_relative_height(&mut self, _: bool) -> Result<f32, Self::Error> {
            panic!("not in postupdate")
        }
        fn record(&mut self, _: Part) -> Result<Matrix, Self::Error> {
            self.events.push("record");
            Err("record")
        }
        fn set_part(&mut self, _: Part, _: &Matrix) -> Result<(), Self::Error> {
            Err("part")
        }
        fn set_record(&mut self, _: Part, _: &Matrix) -> Result<(), Self::Error> {
            Err("record store")
        }
        fn draw_post_feet_ik(&mut self) -> Result<bool, Self::Error> {
            Err("solver debug")
        }
        fn draw_post_phys_ik(&mut self) -> Result<bool, Self::Error> {
            panic!("not in postupdate")
        }
        fn require_overlay(&mut self) -> Result<(), Self::Error> {
            Err("overlay")
        }
    }
    impl IkPostUpdateSource for Source {
        fn do_new_post_physics_ik(&mut self) -> Result<bool, Self::Error> {
            self.events.push("enable");
            self.enabled
        }
        fn draw_new_post_physics_ik(&mut self) -> Result<bool, Self::Error> {
            self.events.push("draw");
            Ok(false)
        }
    }
    fn state() -> (SkeletonIkState, SkeletonIkTargets) {
        let mut targets = SkeletonIkTargets::before_skeleton_data_init();
        let mut state = SkeletonIkState::new(&mut targets);
        state.external_target_normal_set = [true; 4];
        state.external_target_set = [true; 4];
        state.local_target_set = [true; 4];
        (state, targets)
    }
    #[test]
    fn disabled_consumes_all_twelve_flags_without_reading_weights_or_mappings() {
        let (mut state, targets) = state();
        state.external_blends = [f32::NAN; 4];
        let mut source = Source {
            enabled: Ok(false),
            events: vec![],
        };
        state
            .try_post_wipeout_ik_update(
                &targets,
                &mut source,
                &mut crate::reckoning_numeric::HostReckoningNumeric,
                &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
            )
            .unwrap();
        assert_eq!(state.external_target_normal_set, [false; 4]);
        assert_eq!(state.external_target_set, [false; 4]);
        assert_eq!(state.local_target_set, [false; 4]);
        assert!(state.external_blends.iter().all(|v| v.is_nan()));
        assert_eq!(source.events, ["enable"]);
    }
    #[test]
    fn positive_external_blend_enters_real_parent_requirement_and_preserves_clear_suffix() {
        let (mut state, targets) = state();
        state.external_blends = [f32::NAN, 1.0, 1.0, 1.0];
        let mut source = Source {
            enabled: Ok(true),
            events: vec![],
        };
        let result = state.try_post_wipeout_ik_update(
            &targets,
            &mut source,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        );
        assert!(matches!(
            result,
            Err(FootIkError::Missing("post IK parent"))
        ));
        assert_eq!(state.external_target_set, [true; 4]);
        assert_eq!(state.local_target_set, [true; 4]);
        assert_eq!(source.events, ["enable"]);
    }
    #[test]
    fn only_first_two_external_blends_are_visited() {
        let (mut state, targets) = state();
        state.external_blends = [f32::NAN, -0.0, 1.0, f32::NAN];
        let mut source = Source {
            enabled: Ok(true),
            events: vec![],
        };
        state
            .try_post_wipeout_ik_update(
                &targets,
                &mut source,
                &mut crate::reckoning_numeric::HostReckoningNumeric,
                &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
            )
            .unwrap();
        assert_eq!(state.external_target_normal_set, [false; 4]);
        assert_eq!(state.external_target_set, [false; 4]);
        assert_eq!(state.local_target_set, [false; 4]);
        assert_eq!(source.events, ["enable"]);
    }
}
