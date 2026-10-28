//! SAME `ProcessedPhysIn` state/category/timer fields, not a parallel state enum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ProcessedPhysicsStateFields {
    pub(crate) next: Option<i32>,
    pub(crate) last: Option<i32>,
    pub(crate) physics: Option<i32>,
    pub(crate) current: Option<i32>,
    pub(crate) category: Option<i32>,
    pub(crate) last_category: Option<i32>,
    pub(crate) state_timer: Option<f32>,
    /// Source +93c; Reset is not a claim that Update1 has sampled this frame.
    pub(crate) time_delta: Option<f32>,
    pub(crate) balance: Option<f32>,
    /// Native +9a4, distinct from animation `push_speed+940`.
    pub(crate) push: Option<f32>,
    pub(crate) num_wheels_in_contact: Option<i32>,
    pub(crate) heading_adjust_factor: Option<f32>,
    pub(crate) frames_since_jump: Option<i32>,
    pub(crate) frames_since_jump_correct: Option<i32>,
    pub(crate) state_intended_jump_velocity:
        Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) start_revert: Option<f32>,
    pub(crate) slide: Option<f32>,
    pub(crate) flags_8cc: Option<u32>,
    pub(crate) flags_8d0: Option<u32>,
    pub(crate) flags_8d4: (u32, u32),
    pub(crate) ai_flags: (u32, u32),
    pub(crate) grind_possible: Option<bool>,
    pub(crate) grind_type: Option<crate::physics::GrindType>,
    pub(crate) grind_substate: Option<i32>,
    /// Native +360, from the retained Landing manager's complete vector.
    pub(crate) landing_on_deck_adjust: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    /// Native +8d8, distinct from the output's teleport-finished boolean.
    pub(crate) teleport_exit_state: Option<i32>,
    /// Native +8c0, retained through an unready interactable query.
    pub(crate) interactable_object_id: Option<u32>,
}
impl ProcessedPhysicsStateFields {
    pub(crate) const fn new() -> Self {
        Self {
            next: None,
            last: None,
            physics: None,
            current: None,
            category: None,
            last_category: None,
            state_timer: None,
            time_delta: None,
            balance: None,
            push: None,
            num_wheels_in_contact: None,
            heading_adjust_factor: None,
            frames_since_jump: None,
            frames_since_jump_correct: None,
            state_intended_jump_velocity: None,
            start_revert: None,
            slide: None,
            flags_8cc: None,
            flags_8d0: None,
            flags_8d4: (0, 0),
            ai_flags: (0, 0),
            grind_possible: None,
            grind_type: None,
            grind_substate: None,
            landing_on_deck_adjust: None,
            teleport_exit_state: None,
            interactable_object_id: None,
        }
    }
    /// Reset82b39488/90/98/a0/a8/b0; timer39450 and balance394a4.
    pub(crate) fn reset(&mut self) {
        self.state_timer = Some(0.0);
        // Reset82b3943c loads original constant 0x3c888889.
        self.time_delta = Some(f32::from_bits(0x3c88_8889));
        self.next = Some(0);
        self.last = Some(0);
        self.physics = Some(0);
        self.current = Some(0);
        self.balance = Some(0.0);
        self.push = Some(0.0); // Reset82b394ac
        // Raw Reset393e8..f0 preserves the AI word's lower 25 bits.
        self.ai_flags.0 &= 0x01ff_ffff;
        self.ai_flags.1 |= 0xfe00_0000;
        self.flags_8d0 = Some(0); //39478
        // Reset394e0 preserves lower22 and clears the upper10 flags.
        self.flags_8d4.0 &= 0x003f_ffff;
        self.flags_8d4.1 |= 0xffc0_0000;
        self.slide = Some(0.0); //394b4
        self.grind_possible = Some(false); //394f0, GrindInvestigation+170
        self.grind_type = Some(crate::physics::GrindType::None); //394c0
        self.grind_substate = Some(0); //394c8
        self.landing_on_deck_adjust = Some(
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
        ); //39390
        self.teleport_exit_state = Some(0); //39480
        self.interactable_object_id = Some(0); //39420, r30=0 from391ac
        self.num_wheels_in_contact = Some(0); //394f8
        self.start_revert = Some(0.0); //394fc
        self.heading_adjust_factor = Some(0.0); //394f4
        self.frames_since_jump = Some(100); //39518
        self.frames_since_jump_correct = Some(1000); //39520, r10 from39464
        self.state_intended_jump_velocity = Some(
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
        ); //39388, r28=350
        self.flags_8cc = Some(0); //39558
        self.category = Some(0);
        self.last_category = Some(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics_state_ids::{
        PHYSICS_CATEGORY_OTHER, PHYSICS_STATE_PHYSICS_GROUND, PHYSICS_STATE_SLEEPING,
    };
    #[test]
    fn reset_and_transition_fields_do_not_publish_a_new_category() {
        let mut state = ProcessedPhysicsStateFields::new();
        assert_eq!(state.category, None);
        state.reset();
        state.last = Some(PHYSICS_STATE_SLEEPING);
        state.last_category = Some(PHYSICS_CATEGORY_OTHER);
        state.next = Some(PHYSICS_STATE_PHYSICS_GROUND);
        state.current = Some(PHYSICS_STATE_PHYSICS_GROUND);
        assert_eq!(state.category, Some(0));
        assert_eq!(state.physics, Some(0));
        assert_eq!(state.balance.unwrap().to_bits(), 0);
    }
}
