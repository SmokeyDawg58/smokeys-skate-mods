use super::*;
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    collision_mesh_line::tests::simple_triangle_mesh,
};

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];
type Bounds = Result<([f32; 3], [f32; 3]), CollisionError>;
const BOUNDS: Bounds = Ok(([-20.0; 3], [20.0; 3]));

fn input() -> CollisionLineInput {
    CollisionLineInput {
        start: SkeletonRootVector::known([0.5, 0.5, 1.0, 4.0]),
        end: SkeletonRootVector::known([0.5, 0.5, -1.0, 6.0]),
        fatness: 0.0,
    }
}
fn entry<'a>(
    mesh: &'a skate2_formats::collision::ClusteredMesh,
    bytes: &'a [u8],
    bounds: &'a Bounds,
    slot: usize,
    owner: u8,
) -> CollisionLineCollectionMesh<'a, u8> {
    CollisionLineCollectionMesh {
        occurrence: CollisionMeshOccurrence {
            mesh,
            bytes,
            forward: &IDENTITY,
            inverse: &IDENTITY,
            owner: Some(owner),
            slot,
        },
        local_bounds: bounds,
    }
}

fn sentinel() -> CollisionMeshLineNearest<u8> {
    let mut output = CollisionMeshLineNearest::new();
    output.position = SkeletonRootVector::known([9.0; 4]);
    output.normal = SkeletonRootVector::known([7.0; 4]);
    output.surface_id = 999;
    output.owner = Some(99);
    output.line_parameter = 0.25;
    output.hit_count = 123;
    output
}

#[test]
fn exact_small_threshold_and_unordered_axes_skip_all_collection_reads() {
    for delta in [f32::from_bits(0x3780_0000), f32::NAN, -0.0] {
        let ray = CollisionLineInput {
            start: SkeletonRootVector::known([0.0; 4]),
            end: SkeletonRootVector::known([delta, 0.0, 0.0, f32::INFINITY]),
            fatness: 0.0,
        };
        let entries = std::iter::from_fn(|| -> Option<CollisionLineCollectionMesh<'_, u8>> {
            panic!("short segment must not fetch a slot")
        });
        let mut output = sentinel();
        try_collection_line(entries, ray, &mut output, &mut HostCollisionLineNumeric).unwrap();
        assert_eq!(output, sentinel());
    }
}

#[test]
fn slot_order_keeps_first_tie_and_counts_both_hits_with_full_position_w() {
    let (mesh, bytes) = simple_triangle_mesh(133);
    let entries = [
        entry(&mesh, &bytes, &BOUNDS, 9, 21),
        entry(&mesh, &bytes, &BOUNDS, 2, 22),
    ];
    let mut output = CollisionMeshLineNearest::new();
    try_collection_line(
        entries.into_iter(),
        input(),
        &mut output,
        &mut HostCollisionLineNumeric,
    )
    .unwrap();
    assert_eq!(output.hit_count, 2);
    assert_eq!(output.owner, Some(21));
    assert_eq!(output.surface_id, 133);
    assert_eq!(output.provenance.unwrap().occurrence_slot, 9);
    assert_eq!(output.line_parameter.to_bits(), 0.5_f32.to_bits());
    assert_eq!(
        output.position.xyz.map(f32::to_bits),
        [0x3f00_0000, 0x3f00_0000, 0]
    );
    assert_eq!(output.position.w.unwrap().to_bits(), 5.0_f32.to_bits());
}

#[test]
fn late_bounds_refusal_keeps_leaf_prefix_but_does_not_publish_local_job_result() {
    let (mesh, bytes) = simple_triangle_mesh(17);
    let bad = Err(CollisionError::Truncated {
        required: 48,
        actual: 7,
    });
    let entries = || {
        [
            entry(&mesh, &bytes, &BOUNDS, 0, 10),
            entry(&mesh, &bytes, &bad, 1, 11),
        ]
        .into_iter()
    };
    let mut nearest = CollisionMeshLineNearest::new();
    assert!(matches!(
        try_collection_line(
            entries(),
            input(),
            &mut nearest,
            &mut HostCollisionLineNumeric
        ),
        Err(CollisionLineCollectionError::Bounds { slot: 1, .. })
    ));
    assert_eq!(nearest.hit_count, 1);
    assert_eq!(nearest.owner, Some(10));
    let mut output = sentinel();
    assert!(matches!(
        try_publish_collection_line(
            entries(),
            input(),
            &mut output,
            &mut HostCollisionLineNumeric
        ),
        Err(CollisionLineCollectionError::Bounds { slot: 1, .. })
    ));
    assert_eq!(output, sentinel());
}

#[test]
fn ordered_touching_bounds_reach_mesh_but_separation_skips_its_bad_bytes() {
    let (mesh, _) = simple_triangle_mesh(1);
    let far = Ok(([10.0; 3], [20.0; 3]));
    let mut nearest = CollisionMeshLineNearest::new();
    try_collection_line(
        std::iter::once(entry(&mesh, &[], &far, 0, 0)),
        input(),
        &mut nearest,
        &mut HostCollisionLineNumeric,
    )
    .unwrap();
    assert_eq!(nearest.hit_count, 0);
    let touching = Ok(([0.5, -20.0, -20.0], [20.0; 3]));
    assert!(matches!(
        try_collection_line(
            std::iter::once(entry(&mesh, &[], &touching, 3, 0)),
            input(),
            &mut nearest,
            &mut HostCollisionLineNumeric
        ),
        Err(CollisionLineCollectionError::Mesh { slot: 3, .. })
    ));
}

#[test]
fn identity_inverse_still_rounds_query_midpoint_and_extent_before_culling() {
    let (mesh, _) = simple_triangle_mesh(1);
    let bounds = Ok(([16_777_218.0, -1.0, -1.0], [16_777_220.0, 1.0, 1.0]));
    let ray = CollisionLineInput {
        start: SkeletonRootVector::known([16_777_216.0, 0.0, 0.0, 0.0]),
        end: SkeletonRootVector::known([16_777_218.0, 0.0, 0.0, 0.0]),
        fatness: 0.0,
    };
    // Source rounded midpoint=16777216 and half=1, so recomposed max
    // rounds down to16777216. An identity-copy shortcut would touch the box
    // at16777218 and attempt the deliberately absent KD bytes.
    let mut output = CollisionMeshLineNearest::new();
    try_collection_line(
        std::iter::once(entry(&mesh, &[], &bounds, 0, 0)),
        ray,
        &mut output,
        &mut HostCollisionLineNumeric,
    )
    .unwrap();
    assert_eq!(output.hit_count, 0);
}

#[test]
fn normal_miss_publishes_all_local_defaults_over_previous_batch_result() {
    let mut output = sentinel();
    try_publish_collection_line(
        std::iter::empty(),
        input(),
        &mut output,
        &mut HostCollisionLineNumeric,
    )
    .unwrap();
    assert_eq!(output, CollisionMeshLineNearest::new());
}

#[test]
fn another_positive_axis_admits_a_segment_with_an_unordered_axis() {
    let (mesh, bytes) = simple_triangle_mesh(1);
    let mut ray = input();
    ray.start.xyz[0] = f32::NAN;
    let bad = Err(CollisionError::Truncated {
        required: 48,
        actual: 0,
    });
    assert!(matches!(
        try_collection_line(
            std::iter::once(entry(&mesh, &bytes, &bad, 4, 0)),
            ray,
            &mut CollisionMeshLineNearest::new(),
            &mut HostCollisionLineNumeric,
        ),
        Err(CollisionLineCollectionError::Bounds { slot: 4, .. })
    ));
}

#[test]
fn repeated_and_changed_inverse_boxes_match_uncached_collection_results() {
    let (mesh, bytes) = simple_triangle_mesh(37);
    let inverse = crate::world::inverse_collision_affine_reset(IDENTITY);
    let mut shifted = inverse;
    shifted[12] = 2.0;
    let mut negative_zero = inverse;
    negative_zero[1] = -0.0;
    for transforms in [
        [inverse, inverse, inverse, inverse],
        [inverse, shifted, shifted, inverse],
        [negative_zero, negative_zero, inverse, inverse],
    ] {
        for radius in [0.0, 0.25] {
            for x in [-1.0, 0.25, 1.0, 3.0] {
                let entries = || {
                    transforms.iter().enumerate().map(|(i, transform)| {
                        let mut e = entry(&mesh, &bytes, &BOUNDS, i, i as u8);
                        e.occurrence.inverse = transform;
                        e
                    })
                };
                let mut ray = input();
                ray.fatness = radius;
                ray.start.xyz[0] = x;
                ray.end.xyz[0] = x;
                let mut actual = CollisionMeshLineNearest::new();
                let mut expected = CollisionMeshLineNearest::new();
                try_collection_line(entries(), ray, &mut actual, &mut HostCollisionLineNumeric)
                    .unwrap();
                try_collection_line_with_transform(
                    entries(),
                    ray,
                    &mut expected,
                    &mut HostCollisionLineNumeric,
                    transform_collision_aabb,
                )
                .unwrap();
                assert_eq!(actual, expected);
            }
        }
    }
}

#[test]
fn transform_reuse_preserves_late_error_and_prior_hit_publication() {
    let (mesh, bytes) = simple_triangle_mesh(37);
    let bad = Err(CollisionError::Truncated {
        required: 48,
        actual: 7,
    });
    let entries = || {
        [
            entry(&mesh, &bytes, &BOUNDS, 0, 7),
            entry(&mesh, &bytes, &BOUNDS, 1, 8),
            entry(&mesh, &bytes, &bad, 2, 9),
        ]
        .into_iter()
    };
    let mut actual = CollisionMeshLineNearest::new();
    let mut expected = CollisionMeshLineNearest::new();
    let a = try_collection_line(
        entries(),
        input(),
        &mut actual,
        &mut HostCollisionLineNumeric,
    );
    let b = try_collection_line_with_transform(
        entries(),
        input(),
        &mut expected,
        &mut HostCollisionLineNumeric,
        transform_collision_aabb,
    );
    assert!(matches!(
        a,
        Err(CollisionLineCollectionError::Bounds { slot: 2, .. })
    ));
    assert!(matches!(
        b,
        Err(CollisionLineCollectionError::Bounds { slot: 2, .. })
    ));
    assert_eq!(actual, expected);
    assert_eq!(actual.hit_count, 2);
    assert_eq!(actual.owner, Some(7));
}
