use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;

struct AnimationProbe {
    fail_mirror: bool,
    calls: Vec<&'static str>,
}
impl CameraSubjectAnimationRead for AnimationProbe {
    type Error = &'static str;
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("mirror");
        if self.fail_mirror {
            Err("mirror")
        } else {
            Ok(true)
        }
    }
    fn running_out(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("runout");
        Err("runout")
    }
}

#[test]
fn animation_failure_preserves_published_prefix_and_prior_transform() {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    let global = CameraGlobalCondition::construct();
    let mut subject = CameraSubject::try_construct().unwrap();
    subject.flags.teleported = true;
    subject.flags.running_out = true;
    subject.board_position = SkeletonRootVector::known([17.0; 4]);
    let mut animation = AnimationProbe {
        fail_mirror: true,
        calls: Vec::new(),
    };
    assert!(matches!(
        subject.try_publish_fields(&output, &mut animation, &global),
        Err(CameraSubjectUpdateError::Animation("mirror"))
    ));
    assert_eq!(animation.calls, ["mirror"]);
    assert_eq!(
        subject.flags.teleported,
        output.try_state_output().unwrap().has_just_teleported
    );
    assert!(subject.flags.running_out);
    assert_eq!(subject.board_position, SkeletonRootVector::known([17.0; 4]));
    animation.fail_mirror = false;
    animation.calls.clear();
    assert!(matches!(
        subject.try_publish_fields(&output, &mut animation, &global),
        Err(CameraSubjectUpdateError::Animation("runout"))
    ));
    assert_eq!(animation.calls, ["mirror", "runout"]);
    assert!(subject.flags.mirrored);
    assert!(subject.flags.running_out);
    assert_eq!(subject.board_position, SkeletonRootVector::known([17.0; 4]));
}
