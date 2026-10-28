//! Real retained post-physics downward COM -> active `LandingOnSkateboardAdjust`
//! -> retained offsets -> same mapped deck/all4IK -> animation record and roots
//! input. Runs after the second existing controlled animation/root preparation,
//! before its post-physics records replace the first frame's downward velocity.
//! Existing roots are the explicit component input; this is not `GeneralUpdate`
//! timing recovery. Geometry has an independent f64 oracle; downstream source
//! arithmetic is checked bitwise using the geometrically verified offset.

use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    animation::AnimationMatrix,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    skeleton_animation_record::SkeletonAnimationRecordInput,
    skeleton_current_animation::{SkeletonAdjustmentInput, SkeletonCurrentAnimationCvars},
    skeleton_landing_on_board::SkeletonLandingOnBoardInput,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

const TARGET_BONES: [usize; 4] = [33, 32, 35, 34];
const TARGET_PARTS: [usize; 4] = [15, 19, 3, 7];

struct Cvars {
    calls: Vec<&'static str>,
    frames: usize,
    fail_frame: Option<usize>,
}
impl Cvars {
    fn new(fail_frame: Option<usize>) -> Self {
        Self {
            calls: Vec::new(),
            frames: 0,
            fail_frame,
        }
    }
}
impl SkeletonAnimationRecordInput for Cvars {
    type Error = &'static str;
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("COM");
        Ok(1)
    }
}
impl SkeletonCurrentAnimationCvars for Cvars {
    fn landing_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("spring debug disabled")
    }
    fn landing_use_physics_com(&mut self) -> Result<u8, Self::Error> {
        panic!("no spring transition")
    }
    fn landing_first_frame_hack(&mut self) -> Result<f32, Self::Error> {
        panic!("no spring transition")
    }
    fn landing_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        panic!("no spring transition")
    }
    fn debug_landing_adjust(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("landing debug");
        Ok(0)
    }
    fn offboard_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        panic!("ground category must not enter Offboard")
    }
    fn landing_on_board_show_adjust(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("show");
        Ok(0)
    }
    fn landing_on_board_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("show cvar is false")
    }
    fn landing_on_board_negate_y(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("negate Y");
        Ok(1)
    }
    fn landing_on_board_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.frames += 1;
        self.calls.push(if self.frames == 1 {
            "full frames"
        } else {
            "Y frames"
        });
        if self.fail_frame == Some(self.frames) {
            Err("frames")
        } else {
            Ok(if self.frames == 1 { 3.0 } else { 5.0 })
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "same-owner active producer, both cvar failure prefixes and independent downstream oracle"
)]
pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    board: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) {
    assert!(Arc::ptr_eq(
        world.physical_body_member(skeleton).unwrap().owner(),
        owner
    ));
    assert!(Arc::ptr_eq(
        world.physical_body_member(board).unwrap().owner(),
        owner
    ));
    let normal = world.skeleton(skeleton).unwrap();
    let retained_velocity = normal.post_physics_records().centre_of_mass_velocity();
    assert!(
        retained_velocity[1] < 0.0,
        "actual first solved COM must supply Landing's downward gate"
    );
    let current = normal.current_animation();
    let old_centre = current.record().centre_of_mass();
    let old_com_to_deck = current.record().com_to_deck();
    let old_deck_zy = current.last_deck_zy();
    let old_poses = *current.record().poses();
    let old_full_count = current.offset().full_count;
    let old_vertical_count = current.offset().vertical_count;
    let old_vertical_flag = current.offset().vertical_just_set;
    assert_eq!(current.record().reset_scalar().to_bits(), 1.0_f32.to_bits());
    let data = Arc::clone(&normal.state().binding.body().data);
    let animation_to_board = *normal.root_transforms().animation_to_board().columns();
    let world_to_animation = normal.root_transforms().world_to_animation_lanes();
    let masses = *normal.physics_history().part_masses();
    let total = masses.into_iter().fold(0.0_f32, |sum, mass| sum + mass);
    let weights = masses.map(|mass| mass * (1.0 / total));
    assert_eq!(
        weights.map(f32::to_bits),
        normal
            .physics_history()
            .fractional_part_masses()
            .map(f32::to_bits)
    );

    let pose = &owner.inputs().pose_matrices;
    let raw: [AnimationMatrix; 24] = std::array::from_fn(|part| {
        compose(&pose[data.bone_ids()[part]], &data.bone_to_volume()[part])
    });
    let raw_ik = std::array::from_fn::<_, 4, _>(|target| {
        compose(
            &pose[TARGET_BONES[target]],
            &data.bone_to_volume()[TARGET_PARTS[target]],
        )
    });
    // Controlled desired local deck rotates by a real, nondegenerate amount.
    // This defines input; the oracle below derives the actual round-tripped
    // desired matrix from the same WorldToAnim input that production reads.
    let rotation = [
        0.8, 0.6, 0.0, 0.0, -0.6, 0.8, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let mut desired_local = compose(&rotation, &raw[0]);
    desired_local[13] = raw[0][13] + 0.75;
    let deck = compose(&inverse(&world_to_animation), &desired_local);
    let (time_to_land, blend) = original_blend_sample(owner);
    let input = SkeletonAdjustmentInput {
        category: 0,
        state: 0,
        filtered_state: 1,
        flags_8c4: 0,
        flags_8c8: 0,
        flags_8cc: 0x200,
        balance: 0.0,
        // Deliberately opposite sign: the active producer must use retained COM
        // from the real prior records update, not this processed-input field.
        centre_of_mass_velocity: [0.0, 123.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0, 0.0],
        landing_on_board: SkeletonLandingOnBoardInput {
            flags_8d0: 0,
            deck_transform: deck,
            landing_on_deck_adjust: [0.2, 0.0, 0.3, 0.0],
            time_to_land,
        },
    };
    let geometric_offset =
        expected_offset(&raw[0], &world_to_animation, &input.landing_on_board, blend);
    let mut passed = 99.0_f32;
    for failure in [1, 2] {
        let mut cvars = Cvars::new(Some(failure));
        assert!(
            world
                .try_process_skeleton_current_animation(
                    skeleton,
                    pose,
                    &input,
                    &mut passed,
                    &mut cvars
                )
                .is_err()
        );
        assert_eq!(
            cvars.calls,
            if failure == 1 {
                vec!["show", "negate Y", "full frames"]
            } else {
                vec!["show", "negate Y", "full frames", "Y frames"]
            }
        );
        let normal = world.skeleton(skeleton).unwrap();
        let current = normal.current_animation();
        for (index, expected) in geometric_offset.iter().enumerate() {
            if index == 13 {
                assert_eq!(current.offset().transform[index].to_bits(), 0);
            } else {
                close(
                    current.offset().transform[index],
                    *expected,
                    "failed setter full matrix",
                );
            }
        }
        assert!(current.offset().full_just_set);
        assert_eq!(
            current.offset().full_count.to_bits(),
            if failure == 1 { old_full_count } else { 3.0 }.to_bits()
        );
        assert_eq!(
            current.offset().vertical_count.to_bits(),
            old_vertical_count.to_bits()
        );
        assert_eq!(current.offset().vertical_just_set, old_vertical_flag);
        assert_eq!(
            normal
                .animation_targets()
                .completed_volumes()
                .unwrap()
                .map(|m| m.map(f32::to_bits)),
            raw.map(|m| m.map(f32::to_bits))
        );
        assert_eq!(
            current.ik().targets().map(|m| m.map(f32::to_bits)),
            raw_ik.map(|m| m.map(f32::to_bits))
        );
        assert_eq!(
            current.record().poses().map(|m| m.map(f32::to_bits)),
            old_poses.map(|m| m.map(f32::to_bits))
        );
        assert_eq!(
            current.record().centre_of_mass().map(f32::to_bits),
            old_centre.map(f32::to_bits)
        );
        assert!(current.root_input(0.0).is_none());
        assert_eq!(passed.to_bits(), 99.0_f32.to_bits());
        assert_eq!(
            normal
                .post_physics_records()
                .centre_of_mass_velocity()
                .map(f32::to_bits),
            retained_velocity.map(f32::to_bits)
        );
    }
    let mut cvars = Cvars::new(None);
    world
        .try_process_skeleton_current_animation(skeleton, pose, &input, &mut passed, &mut cvars)
        .unwrap();
    assert_eq!(
        cvars.calls,
        [
            "show",
            "negate Y",
            "full frames",
            "Y frames",
            "landing debug",
            "COM"
        ]
    );
    let normal = world.skeleton(skeleton).unwrap();
    let current = normal.current_animation();
    let offset = &current.offset().transform;
    for (actual, expected) in offset.iter().zip(geometric_offset) {
        close(*actual, expected, "active landing offset");
    }
    assert_eq!(current.offset().full_count.to_bits(), 3.0_f32.to_bits());
    assert_eq!(current.offset().vertical_count.to_bits(), 5.0_f32.to_bits());
    assert!(!current.offset().full_just_set && !current.offset().vertical_just_set);
    assert_ne!(
        offset[13].to_bits(),
        0,
        "actual producer must retain nonzero negated Y"
    );

    // Stage isolation: geometry validates the producer above. Independent
    // source-form consumers now use that same verified retained offset once.
    let mut volumes = raw;
    volumes[0] = compose(offset, &raw[0]);
    let ik = raw_ik.map(|matrix| compose(offset, &matrix));
    assert_eq!(
        normal
            .animation_targets()
            .completed_volumes()
            .unwrap()
            .map(|m| m.map(f32::to_bits)),
        volumes.map(|m| m.map(f32::to_bits))
    );
    assert_eq!(
        current.ik().targets().map(|m| m.map(f32::to_bits)),
        ik.map(|m| m.map(f32::to_bits))
    );
    assert_eq!(
        current.record().poses().map(|m| m.map(f32::to_bits)),
        volumes.map(|m| m.map(f32::to_bits))
    );
    for target in 0..4 {
        assert_ne!(
            ik[target].map(f32::to_bits),
            raw_ik[target].map(f32::to_bits)
        );
    }
    assert_eq!(
        current.raw_skateboard().map(f32::to_bits),
        raw[0].map(f32::to_bits)
    );
    let mut centre = [0.0; 4];
    for (part, matrix) in volumes.iter().enumerate() {
        for lane in 0..4 {
            centre[lane] = matrix[12 + lane].mul_add(weights[part], centre[lane]);
        }
    }
    assert_eq!(
        current.record().centre_of_mass().map(f32::to_bits),
        centre.map(f32::to_bits)
    );
    assert_eq!(
        current.record().centre_of_mass_delta().map(f32::to_bits),
        std::array::from_fn::<_, 4, _>(|lane| centre[lane] - old_centre[lane]).map(f32::to_bits)
    );
    let com = point(&animation_to_board, centre);
    let deck = point(
        &animation_to_board,
        std::array::from_fn(|lane| volumes[0][12 + lane]),
    );
    let com_to_deck = subtract(com, deck);
    assert_vector(current.record().com_to_deck(), com_to_deck);
    assert_vector(
        current.record().com_to_deck_delta(),
        subtract(com_to_deck, old_com_to_deck),
    );
    assert_eq!(passed.to_bits(), (volumes[0][9] - old_deck_zy).to_bits());
    assert_eq!(current.last_deck_zy().to_bits(), volumes[0][9].to_bits());
    assert_eq!(
        current
            .root_input(0.0)
            .unwrap()
            .skate_animation
            .map(f32::to_bits),
        volumes[0].map(f32::to_bits)
    );
    assert_eq!(
        current.second_last_com_velocity().to_bits(),
        123.0_f32.to_bits()
    );
    assert_eq!(
        normal
            .post_physics_records()
            .centre_of_mass_velocity()
            .map(f32::to_bits),
        retained_velocity.map(f32::to_bits)
    );
}

fn original_blend_sample(owner: &PhysicalPlayerAssemblyOwner) -> (f32, f32) {
    let instance = owner
        .inputs()
        .vault
        .instance(0x74bc_9c93_affe_6aac, 0xd7ed_bd36_2d7d_2152);
    let data = instance
        .try_get_data(0x6aea_4c4e_fbba_747b, 0)
        .unwrap()
        .expect("original landing PointNegGraphData8");
    let bytes = data.bytes();
    assert_eq!(bytes.len(), 0x50);
    let sample = |offset| f32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
    let x: [f32; 8] = std::array::from_fn(|index| sample(0x10 + 4 * index));
    let y: [f32; 8] = std::array::from_fn(|index| sample(0x30 + 4 * index));
    // Independently choose an interior authored segment with a real partial
    // blend; do not substitute a constant or call production PointGraphEval.
    for upper in 1..8 {
        let time = x[upper - 1].midpoint(x[upper]);
        if !(time >= x[0] && time < x[7] && time > x[upper - 1] && time < x[upper]) {
            continue;
        }
        if (1..upper).any(|index| time < x[index]) {
            continue;
        }
        let slope = (y[upper] - y[upper - 1]) / (x[upper] - x[upper - 1]);
        let blend = slope.mul_add(time - x[upper - 1], y[upper - 1]);
        if blend > 0.0 && blend < 1.0 {
            return (time, blend);
        }
    }
    panic!("original landing graph has no interior partial blend: x={x:?}, y={y:?}");
}

fn expected_offset(
    mapped: &AnimationMatrix,
    world_to_animation: &AnimationMatrix,
    input: &SkeletonLandingOnBoardInput,
    blend: f32,
) -> [f64; 16] {
    let mut desired = compose(world_to_animation, &input.deck_transform);
    let mut adjustment: [f32; 4] = std::array::from_fn(|lane| {
        let x = world_to_animation[lane] * input.landing_on_deck_adjust[0];
        let y = world_to_animation[4 + lane].mul_add(input.landing_on_deck_adjust[1], x);
        world_to_animation[8 + lane].mul_add(input.landing_on_deck_adjust[2], y)
    });
    adjustment[1] = if desired[13] > mapped[13] {
        desired[13] - mapped[13]
    } else {
        0.0
    };
    for lane in 0..4 {
        desired[12 + lane] = mapped[12 + lane] + adjustment[lane];
    }
    let relative: [[f64; 3]; 3] = std::array::from_fn(|column| {
        std::array::from_fn(|lane| {
            (0..3)
                .map(|axis| {
                    f64::from(desired[axis * 4 + lane]) * f64::from(mapped[axis * 4 + column])
                })
                .sum()
        })
    });
    let raw_axis = [
        relative[1][2] - relative[2][1],
        relative[2][0] - relative[0][2],
        relative[0][1] - relative[1][0],
    ];
    let magnitude = raw_axis.iter().map(|lane| lane * lane).sum::<f64>().sqrt();
    let angle = magnitude.atan2(relative[0][0] + relative[1][1] + relative[2][2] - 1.0);
    assert!(
        angle > 0.1 && angle < 3.0,
        "controlled real pose selects nondegenerate SLerp"
    );
    let axis = raw_axis.map(|lane| lane / magnitude);
    let theta = angle * f64::from(blend);
    let cosine = theta.cos();
    let sine = theta.sin();
    let mut interpolated = [0.0_f64; 16];
    for column in 0..3 {
        let original = [
            f64::from(mapped[column * 4]),
            f64::from(mapped[column * 4 + 1]),
            f64::from(mapped[column * 4 + 2]),
        ];
        let cross = [
            axis[1] * original[2] - axis[2] * original[1],
            axis[2] * original[0] - axis[0] * original[2],
            axis[0] * original[1] - axis[1] * original[0],
        ];
        let parallel: f64 = axis.into_iter().zip(original).map(|(a, b)| a * b).sum();
        for lane in 0..3 {
            interpolated[column * 4 + lane] = original[lane] * cosine
                + cross[lane] * sine
                + axis[lane] * parallel * (1.0 - cosine);
        }
        interpolated[column * 4 + 3] = interpolated[column * 4]; // source Rodrigues basis W=X
    }
    for lane in 0..4 {
        interpolated[12 + lane] =
            f64::from(mapped[12 + lane].mul_add(1.0 - blend, desired[12 + lane] * blend));
    }
    let inverse = inverse(mapped);
    let mut offset: [f64; 16] = std::array::from_fn(|index| {
        let column = index / 4;
        let lane = index % 4;
        let rotation: f64 = (0..3)
            .map(|axis| interpolated[axis * 4 + lane] * f64::from(inverse[column * 4 + axis]))
            .sum();
        rotation
            + if column == 3 {
                interpolated[12 + lane]
            } else {
                0.0
            }
    });
    offset[13] = -offset[13]; // selected current NegateY cvar, independent of full count.
    offset
}

fn compose(outer: &AnimationMatrix, inner: &AnimationMatrix) -> AnimationMatrix {
    std::array::from_fn(|index| {
        let column = index / 4;
        let lane = index % 4;
        let base = column * 4;
        let x = if column == 3 {
            inner[base].mul_add(outer[lane], outer[12 + lane])
        } else {
            inner[base] * outer[lane]
        };
        let y = inner[base + 1].mul_add(outer[4 + lane], x);
        inner[base + 2].mul_add(outer[8 + lane], y)
    })
}
fn inverse(matrix: &AnimationMatrix) -> AnimationMatrix {
    let mut result = [0.0; 16];
    for column in 0..3 {
        for lane in 0..3 {
            result[column * 4 + lane] = matrix[lane * 4 + column];
        }
    }
    let negative: [f32; 3] = std::array::from_fn(|lane| 0.0 - matrix[12 + lane]);
    for lane in 0..4 {
        let z = negative[2] * result[8 + lane];
        let y = negative[1].mul_add(result[4 + lane], z);
        result[12 + lane] = negative[0].mul_add(result[lane], y);
    }
    result
}
fn point(columns: &[SkeletonRootVector; 4], value: [f32; 4]) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| {
            columns[2].xyz[lane].mul_add(
                value[2],
                columns[1].xyz[lane].mul_add(
                    value[1],
                    columns[0].xyz[lane].mul_add(value[0], columns[3].xyz[lane]),
                ),
            )
        }),
        w: columns[0]
            .w
            .zip(columns[1].w)
            .zip(columns[2].w)
            .zip(columns[3].w)
            .map(|(((x, y), z), p)| {
                z.mul_add(value[2], y.mul_add(value[1], x.mul_add(value[0], p)))
            }),
    }
}
fn subtract(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| a.xyz[lane] - b.xyz[lane]),
        w: a.w.zip(b.w).map(|(a, b)| a - b),
    }
}
fn assert_vector(actual: SkeletonRootVector, expected: SkeletonRootVector) {
    assert_eq!(actual.xyz.map(f32::to_bits), expected.xyz.map(f32::to_bits));
    assert_eq!(actual.w.map(f32::to_bits), expected.w.map(f32::to_bits));
}
fn close(actual: f32, expected: f64, label: &str) {
    assert!(
        (f64::from(actual) - expected).abs() <= 0.002 * (1.0 + expected.abs()),
        "{label}: {actual} vs {expected}"
    );
}
