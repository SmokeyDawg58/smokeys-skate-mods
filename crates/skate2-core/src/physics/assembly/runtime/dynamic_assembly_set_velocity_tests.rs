//! Private diagnostic fault on a supplied actual admitted board session.
//! No body is removed, replaced, or fabricated; only one Part reference is
//! temporarily hidden and restored before assertions or subsequent cleanup.
use crate::{
    dynamic_assembly_session::part_contact_spy::DynamicAssemblyPartContactSpyError,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skateboard_set_velocity::{SkateboardSetVelocityError, try_set_skateboard_velocity},
    skeleton_root_transforms::SkeletonRootVector,
};

/// Call before the corpus's intended all-Part setter and next real solve.
/// This diagnostic leaves the native successful prefix (Parts0/1) in place
/// and the host frame invalidated. The caller must not treat it as a tick.
pub(crate) fn check_late_part_velocity_failure(
    session: &mut SkateboardAssemblySession,
    backend: &mut SkateboardSimulationBackend,
) {
    assert_eq!(session.part_attachment.parts.len(), 7);
    assert!(backend.collision_body_pool().frame_identity().is_some());
    let bodies: [_; 7] = std::array::from_fn(|part| {
        session
            .try_part_contact_spy_observation(backend, part)
            .unwrap()
    });
    let before: [[u32; 3]; 7] = bodies.map(|body| {
        backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(body)
            .unwrap()
            .map(f32::to_bits)
    });
    let requested = SkeletonRootVector {
        xyz: [-12.75, 3.125, -0.0],
        w: None,
    };
    let bits = requested.xyz.map(f32::to_bits);
    assert_ne!(before[0], bits, "Part0 must discriminate a real write");
    assert_ne!(before[1], bits, "Part1 must discriminate a real write");

    let retained = session.part_attachment.parts[2]
        .body
        .take()
        .expect("actual admitted Part2 body");
    let result = try_set_skateboard_velocity(session, backend, &requested);
    // Restore the exact original reference even when the result is unexpected.
    // All assertions below operate on a fully restored actual Part list.
    session.part_attachment.parts[2].body = Some(retained);

    assert!(matches!(
        result,
        Err(SkateboardSetVelocityError::Part {
            index: 2,
            source: DynamicAssemblyPartContactSpyError::MissingPartBody { index: 2 },
        })
    ));
    assert!(backend.collision_body_pool().frame_identity().is_none());
    for (part, body) in bodies.into_iter().enumerate() {
        assert_eq!(
            session
                .try_part_contact_spy_observation(backend, part)
                .unwrap(),
            body,
            "Part {part} retains its actual body occurrence"
        );
        let actual = backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(body)
            .unwrap()
            .map(f32::to_bits);
        assert_eq!(
            actual,
            if part < 2 { bits } else { before[part] },
            "Part {part} source failure prefix"
        );
    }
    println!(
        "actual SetVelocity diagnostic missing Part2: Parts0/1 committed, Parts2..6 unchanged, all body occurrences restored, stale frame retained"
    );
}
