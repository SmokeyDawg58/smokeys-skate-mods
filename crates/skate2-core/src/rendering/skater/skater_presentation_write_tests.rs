use super::*;
use crate::{
    actor_face_playback::presentation::{offsets::PresentationOffsets, write::*},
    reckoning_numeric::HostReckoningNumeric,
};
use skate2_formats::andale::AndaleSqt;
struct NoSubmission;
impl PresentationDataSubmission for NoSubmission {
    type Error = std::convert::Infallible;
    fn submit_data(&mut self, _: &[u8], _: usize) -> Result<(), Self::Error> {
        panic!("an incomplete compression cannot submit")
    }
}
fn offsets() -> PresentationOffsets {
    PresentationOffsets {
        matrix_rotations: vec![0],
        matrix_translations: vec![],
        left_hand_count: 1,
        right_hand_count: 1,
        face: [0..1, 1..2, 2..3],
        quaternion3_count: 5,
        translation_count: 3,
        pose_bytes: 56,
    }
}
fn hand() -> OwnedAnimationSqtBuffer {
    let result = OwnedAnimationSqtBuffer::allocate_unknown(1);
    result.begin_extract().unwrap();
    result
        .extract(&[AndaleSqt {
            scale: [1.0; 4],
            rotation: [0.25, -0.5, 0.125, 0.75],
            translation: [0.0; 4],
        }])
        .unwrap();
    result.complete_sync().unwrap();
    result
}
fn source(actor: &ActorConstruction) -> CapturedPresentationPose<'_> {
    let mut local = AnimOutPhysInOwner::new_sim_allocation();
    local.try_construct(1).unwrap();
    local.local_pose.as_mut().unwrap()[0] = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
    CapturedPresentationPose {
        actor,
        local: Arc::new(Mutex::new(local)),
        face: OwnedAnimationSqtBuffer::allocate_unknown(3),
        right: hand(),
        left: hand(),
    }
}
#[test]
fn later_body_source_failure_retains_first_rotation_without_reading_hands() {
    let (actor, _route) = super::super::tests::actor_with_listener();
    let mut source = source(&actor);
    source.left = OwnedAnimationSqtBuffer::allocate_unknown(1);
    let mut offsets = offsets();
    offsets.matrix_rotations.push(7);
    let mut matrices = [[0xabcd; 4]; 2];
    let mut rotations = [[0xabcd; 3]; 5];
    let mut translations = [[0xabcd; 3]; 3];
    let error = source
        .try_write(
            &offsets,
            &mut PresentationWriteScratch {
                matrix_rotations: &mut matrices,
                rotations: &mut rotations,
                translations: &mut translations,
            },
            &mut HostReckoningNumeric,
            &mut NoSubmission,
        )
        .unwrap_err();
    assert!(matches!(error, PresentationWriteError::Matrix(_)));
    assert_ne!(matrices[0], [0xabcd; 4]);
    assert_eq!(matrices[1], [0xabcd; 4]);
    assert_eq!(rotations, [[0xabcd; 3]; 5]);
}
#[test]
fn right_hand_failure_preserves_left_and_face_query_occurs_after_both_hands() {
    let (actor, _route) = super::super::tests::actor_with_listener();
    for missing_right in [true, false] {
        let mut source = source(&actor);
        if missing_right {
            source.right = OwnedAnimationSqtBuffer::allocate_unknown(1);
        }
        let mut matrices = [[0xabcd; 4]; 1];
        let mut rotations = [[0xabcd; 3]; 5];
        let mut translations = [[0xabcd; 3]; 3];
        let error = source
            .try_write(
                &offsets(),
                &mut PresentationWriteScratch {
                    matrix_rotations: &mut matrices,
                    rotations: &mut rotations,
                    translations: &mut translations,
                },
                &mut HostReckoningNumeric,
                &mut NoSubmission,
            )
            .unwrap_err();
        assert_eq!(
            rotations[0],
            [8191, u16::from_ne_bytes((-16383_i16).to_ne_bytes()), 4095]
        );
        if missing_right {
            assert!(matches!(error, PresentationWriteError::Pose(_)));
            assert_eq!(rotations[1], [0xabcd; 3]);
        } else {
            assert!(matches!(error, PresentationWriteError::Source(_)));
            assert_eq!(rotations[1], rotations[0]);
        }
        assert_eq!(&rotations[2..], &[[0xabcd; 3]; 3]);
        assert_eq!(translations, [[0xabcd; 3]; 3]);
    }
}
