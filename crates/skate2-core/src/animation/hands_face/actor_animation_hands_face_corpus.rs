//! Original retained hands/face output checks through the production session.
use crate::{
    actor_construction::{
        ActorConstruction, body_session::ActorBodyAnimationServices,
        hands_face_session::HandsFaceSelection,
    },
    actor_face_playback::blend_pose_stream::BlendPoseTokenPool,
};

struct ControlledLocalSelection;
impl HandsFaceSelection for ControlledLocalSelection {
    type Error = std::convert::Infallible;
    fn is_local_player(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn hands_commands_clamp(&mut self) -> Result<i32, Self::Error> {
        panic!("LOD0 must not read the local-LOD5/nonlocal-LOD2 clamp")
    }
}

pub(crate) fn check(
    actor: &ActorConstruction,
    body: &mut ActorBodyAnimationServices,
    tokens: &mut BlendPoseTokenPool,
) -> [u64; 3] {
    // Same controlled local High-quality selection as the original fixture;
    // the production session requires its caller to supply these policies.
    let outputs = actor
        .try_hands_face_session(0, body, tokens, &mut ControlledLocalSelection)
        .unwrap();
    let mut generations = [0; 3];
    for (index, output) in [outputs.left, outputs.right, outputs.face]
        .iter()
        .enumerate()
    {
        let (generation, values) = output.read().unwrap();
        assert_eq!(values.len(), output.len().unwrap());
        assert!(!values.is_empty());
        assert!(generation > 0);
        generations[index] = generation;
    }
    generations
}
