use super::*;
use crate::skeleton_root_transforms::SkeletonRootVector;

#[derive(Default)]
struct Attributes {
    reads: Vec<&'static str>,
    fail_first: bool,
    fail_last: bool,
}
impl SkateboardQueuedForceAttributes for Attributes {
    type Error = &'static str;
    fn local_y_offset(&mut self) -> Result<f32, Self::Error> {
        self.reads.push("offset");
        if self.fail_first {
            Err("first")
        } else {
            Ok(-0.25)
        }
    }
    fn finish_lookup(&mut self) -> Result<(), Self::Error> {
        self.reads.push("last pointer only");
        if self.fail_last { Err("last") } else { Ok(()) }
    }
}
fn force(x: f32) -> SkateboardFootForce {
    SkateboardFootForce {
        world_space_force: SkeletonRootVector {
            xyz: [x, 2.0, 3.0],
            w: None,
        },
        local_space_position: SkeletonRootVector::known([0.0, -0.0, 0.22, 17.0]),
    }
}

#[test]
fn empty_skips_every_collaborator_and_cap_preserves_first_21_full_records() {
    let mut queue = SkateboardForceQueue::new();
    let mut attributes = Attributes::default();
    queue
        .try_apply(&mut attributes, |_, _| -> Result<(), ()> {
            panic!("empty")
        })
        .unwrap();
    assert!(attributes.reads.is_empty());
    for index in 0_u8..24 {
        queue.add(u32::from(index), force(f32::from(index)));
    }
    assert_eq!(queue.len(), 21);
    for index in 0_u8..21 {
        assert_eq!(
            queue.entries[usize::from(index)],
            Some(SkateboardQueuedForce {
                kind: u32::from(index),
                force: force(f32::from(index))
            })
        );
    }
    let mut seen = Vec::new();
    queue
        .try_apply(&mut attributes, |record, offset| {
            assert_eq!(offset.to_bits(), (-0.25_f32).to_bits());
            seen.push(record);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(
        seen,
        (0_u8..21)
            .map(|index| force(f32::from(index)))
            .collect::<Vec<_>>()
    );
    assert_eq!(attributes.reads, ["offset", "last pointer only"]);
    assert_eq!(queue.len(), 21);
    queue.clear_post_physics();
    queue.add(15, force(99.0));
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.entries[0].unwrap().kind, 15);
}

#[test]
fn failures_preserve_completed_body_prefix_and_queue_without_eager_final_lookup() {
    let mut queue = SkateboardForceQueue::new();
    queue.add(4, force(1.0));
    queue.add(5, force(2.0));
    let mut attributes = Attributes {
        fail_first: true,
        ..Default::default()
    };
    assert!(matches!(
        queue.try_apply(&mut attributes, |_, _| -> Result<(), ()> {
            panic!("initial refusal")
        }),
        Err(SkateboardForceApplyError::Attribute("first"))
    ));
    attributes.fail_first = false;
    attributes.reads.clear();
    let mut effects = Vec::new();
    let result = queue.try_apply(&mut attributes, |record, _| {
        if !effects.is_empty() {
            return Err("second body");
        }
        effects.push(record);
        Ok(())
    });
    assert!(matches!(
        result,
        Err(SkateboardForceApplyError::Body {
            entry: 1,
            source: "second body"
        })
    ));
    assert_eq!(effects, [force(1.0)]);
    assert_eq!(attributes.reads, ["offset"]);
    attributes.fail_last = true;
    effects.clear();
    assert!(matches!(
        queue.try_apply(&mut attributes, |record, _| {
            effects.push(record);
            Ok::<_, ()>(())
        }),
        Err(SkateboardForceApplyError::Attribute("last"))
    ));
    assert_eq!(effects, [force(1.0), force(2.0)]);
    assert_eq!(queue.len(), 2);
}
