//! Missing payloads of the SAME processed owner, source2174..242c.
use super::SkeletonProcessedContact;
use crate::actor_construction::anim_out_phys_in::AnimationAiInput;

pub(crate) struct ProcessedInputControls {
    pub(crate) time_since_last_input: Option<f32>,
    pub(crate) crouch_input: Option<f32>,
    pub(crate) crouch_input_delta: Option<f32>,
    pub(crate) transition: Option<f32>,
    pub(crate) time_on_ground: Option<f32>,
    pub(crate) frames_since_teleport: Option<i32>,
    pub(crate) state_count: Option<i32>,
    pub(crate) update_count: Option<i32>,
    /// Ten native AI vectors; flags remain in `physics_state.ai_flags`.
    pub(crate) ai_vectors: [Option<[f32; 4]>; 10],
}
impl ProcessedInputControls {
    pub(super) fn initialize_constructor_fields(&mut self) {
        self.ai_vectors = [Some([0.0; 4]); 10];
        self.ai_vectors[8] = Some([-1.0; 4]);
    }
    pub(super) const fn new() -> Self {
        Self {
            time_since_last_input: None,
            crouch_input: None,
            crouch_input_delta: None,
            transition: None,
            time_on_ground: None,
            frames_since_teleport: None,
            state_count: None,
            update_count: None,
            ai_vectors: [None; 10],
        }
    }
    pub(super) fn reset(&mut self) {
        self.time_since_last_input = Some(0.0); //39504, processed+9bc
        self.crouch_input = Some(0.0); //39534, processed+9d4
        self.crouch_input_delta = Some(0.0); //3953c, processed+9d8
        self.transition = Some(0.0); //394d4
        self.time_on_ground = Some(0.0); //3950c
        self.frames_since_teleport = Some(0); //39530
        self.state_count = Some(0); //39508
        self.update_count = Some(0); //39510
        // AI payload has a distinct constructor/reset contract; do not invent
        // vector writes to make an inactive AI branch appear populated.
    }
}
impl SkeletonProcessedContact {
    pub(crate) fn input_controls(&self) -> &ProcessedInputControls {
        &self.controls
    }
    pub(crate) fn input_controls_mut(&mut self) -> &mut ProcessedInputControls {
        &mut self.controls
    }
    pub(crate) fn write_control_flag8c8(&mut self, mask: u32, value: bool) -> bool {
        let Some(word) = &mut self.flags_8c8 else {
            return false;
        };
        *word = (*word & !mask) | if value { mask } else { 0 };
        true
    }
    pub(crate) fn write_control_flag8c4(&mut self, mask: u32, value: bool) {
        self.jump_events.process_animation_flag(mask, value);
    }
    pub(crate) fn copy_control_ai(&mut self, source: &AnimationAiInput) {
        for column in 0..4 {
            self.controls.ai_vectors[column] = Some(std::array::from_fn(|lane| {
                source.desired_deck_transform[column * 4 + lane]
            }));
        }
        self.controls.ai_vectors[4] = Some(source.desired_deck_velocity);
        self.controls.ai_vectors[5] = Some(source.trajectory_position);
        self.controls.ai_vectors[6] = Some(source.trajectory_velocity);
        self.controls.ai_vectors[7] = Some(source.trajectory_gravity);
        self.controls.ai_vectors[8] = Some(source.trajectory_time);
        self.controls.ai_vectors[9] = Some(source.trajectory_offset);
        for mask in [
            0x8000_0000,
            0x4000_0000,
            0x2000_0000,
            0x1000_0000,
            0x0800_0000,
            0x0400_0000,
            0x0200_0000,
        ] {
            self.physics_state.ai_flags.0 =
                (self.physics_state.ai_flags.0 & !mask) | (source.flags & mask);
            self.physics_state.ai_flags.1 |= mask;
        }
    }
    pub(crate) fn restore_control_ai_trajectory(&mut self, source: &AnimationAiInput) {
        self.controls.ai_vectors[5..10].copy_from_slice(&[
            Some(source.trajectory_position),
            Some(source.trajectory_velocity),
            Some(source.trajectory_gravity),
            Some(source.trajectory_time),
            Some(source.trajectory_offset),
        ]);
        self.physics_state.ai_flags.0 = (self.physics_state.ai_flags.0 & !0x0800_0000)
            | (source.flags & 0x0800_0000)
            | 0x0400_0000;
        self.physics_state.ai_flags.1 |= 0x0c00_0000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn post_skeleton_scalars_require_reset_and_reset_clears_prior_values() {
        let mut controls = ProcessedInputControls::new();
        assert!(controls.time_since_last_input.is_none());
        assert!(controls.crouch_input.is_none());
        assert!(controls.crouch_input_delta.is_none());
        controls.time_since_last_input = Some(17.0);
        controls.crouch_input = Some(f32::NAN);
        controls.crouch_input_delta = Some(-0.0);
        controls.reset();
        assert_eq!(
            [
                controls.time_since_last_input.unwrap().to_bits(),
                controls.crouch_input.unwrap().to_bits(),
                controls.crouch_input_delta.unwrap().to_bits(),
            ],
            [0; 3]
        );
    }
    #[test]
    fn constructor_initializes_ai_vectors_but_reset_preserves_them() {
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        assert!(processed.controls.ai_vectors.iter().all(Option::is_none));
        processed.try_initialize_constructor_fields().unwrap();
        assert_eq!(
            processed.controls.ai_vectors[8].unwrap().map(f32::to_bits),
            [(-1.0_f32).to_bits(); 4]
        );
        processed.controls.ai_vectors[5] = Some([7.0; 4]);
        processed.reset();
        assert_eq!(
            processed.controls.ai_vectors[5].unwrap().map(f32::to_bits),
            [7.0_f32.to_bits(); 4]
        );
        assert_eq!(processed.physics_state.ai_flags.1, 0xfe00_0000);
    }
    #[test]
    fn history_restore_changes_only_trajectory_and_its_two_flags() {
        let mut processed = SkeletonProcessedContact::new();
        let incoming = AnimationAiInput {
            desired_deck_velocity: [7.0; 4],
            trajectory_position: [9.0; 4],
            flags: 0x8200_0000,
            ..AnimationAiInput::default()
        };
        processed.copy_control_ai(&incoming);
        let history = AnimationAiInput {
            trajectory_position: [3.0; 4],
            flags: 0x0800_0000,
            ..AnimationAiInput::default()
        };
        processed.restore_control_ai_trajectory(&history);
        assert_eq!(
            processed.controls.ai_vectors[4].unwrap().map(f32::to_bits),
            [7.0_f32.to_bits(); 4]
        );
        assert_eq!(
            processed.controls.ai_vectors[5].unwrap().map(f32::to_bits),
            [3.0_f32.to_bits(); 4]
        );
        assert_eq!(processed.physics_state.ai_flags, (0x8e00_0000, 0xfe00_0000));
    }
}
