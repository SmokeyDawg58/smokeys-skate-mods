use super::{HandFaceStreamCommands, emit_tokens};
use crate::{
    actor_animation_tree::eval::AnimationEvalCommands,
    actor_face_playback::{AnimationPoseRef, tree_database::clip_fetch::ClipFetchPlan},
};

#[derive(Default)]
struct Calls(Vec<String>);
impl AnimationEvalCommands for Calls {
    type Error = std::convert::Infallible;
    fn fetch(&mut self, _: ClipFetchPlan) -> Result<(), Self::Error> {
        unreachable!("tokens use retained fixed frames in this call trace")
    }
    fn fetch_pose(&mut self, _: &AnimationPoseRef) -> Result<(), Self::Error> {
        unreachable!("no fallback lookup in token-only trace")
    }
    fn n_blend(&mut self, values: &[f32]) -> Result<(), Self::Error> {
        self.0.push(format!("nblend:{}", values.len()));
        Ok(())
    }
    fn blend(&mut self, value: f32) -> Result<(), Self::Error> {
        self.0.push(format!("blend:{:08x}", value.to_bits()));
        Ok(())
    }
    fn channel_blend(&mut self, _: f32, _: bool) -> Result<(), Self::Error> {
        unreachable!()
    }
    fn add_sqt(&mut self, _: bool) -> Result<(), Self::Error> {
        unreachable!()
    }
    fn mirror(&mut self, _: u32) -> Result<(), Self::Error> {
        unreachable!()
    }
}
impl HandFaceStreamCommands for Calls {
    fn begin_stream(&mut self) -> Result<(), Self::Error> {
        self.0.push("begin".into());
        Ok(())
    }
    fn pose_frame(&mut self, value: u32) -> Result<(), Self::Error> {
        self.0.push(format!("pose:{value}"));
        Ok(())
    }
    fn extract_sqt(&mut self) -> Result<(), Self::Error> {
        self.0.push("extract".into());
        Ok(())
    }
    fn end_stream(&mut self) -> Result<(), Self::Error> {
        self.0.push("end".into());
        Ok(())
    }
}

#[test]
fn clamp_stops_only_after_strict_pose_count_and_reduced_stack() {
    let tokens = [
        1 << 23,
        2 << 23,
        0x4008_0000,
        0x8020_0000,
        3 << 23,
        0x4008_0000,
        0x8020_0000,
        4 << 23,
    ];
    let mut calls = Calls::default();
    assert_eq!(emit_tokens(&tokens, 2, &mut calls).unwrap(), 3);
    assert_eq!(calls.0.len(), 5);
    assert_eq!(&calls.0[..2], &["pose:1", "pose:2"]);
    assert_eq!(calls.0[3], "pose:3");
}

#[test]
fn clamp_does_not_cut_an_unreduced_operand_stack() {
    let tokens = [
        1 << 23,
        2 << 23,
        3 << 23,
        0x4008_0000,
        0x8020_0000,
        0x4008_0000,
        0x8020_0000,
        4 << 23,
    ];
    let mut calls = Calls::default();
    assert_eq!(emit_tokens(&tokens, 0, &mut calls).unwrap(), 3);
    assert_eq!(calls.0.len(), 5);
    assert_eq!(calls.0[2], "pose:3");
}

#[test]
fn raw_token_type_three_uses_same_command_branch_as_type_two() {
    let mut calls = Calls::default();
    assert_eq!(
        emit_tokens(
            &[1 << 23, 2 << 23, 0x4000_0000, 0xc020_0000],
            10000,
            &mut calls
        )
        .unwrap(),
        2
    );
    assert_eq!(calls.0[2], "blend:00000000");
}

#[test]
fn mismatched_native_blend_dimension_asserts_after_prior_pose_calls() {
    let mut calls = Calls::default();
    assert!(
        emit_tokens(
            &[1 << 23, 2 << 23, 0x4000_0000, 0x8030_0000],
            10000,
            &mut calls
        )
        .is_err()
    );
    assert_eq!(calls.0, ["pose:1", "pose:2"]);
}
