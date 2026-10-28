//! Original paired-owner current animation, called inside the existing root
//! corpus over four actual solved frames. Pose edits and cvars are controlled;
//! actual retained offsets include rotation and decay across a failed COM call;
//! other inactive adjustment gates are recovered, not a live Actor/pose selection.
//! Phase2 activates Offboard animation then passes its result to the existing
//! controlled grounded-Reckoning/root arithmetic fixture. This is component
//! integration, not native Offboard `GeneralUpdate` or state scheduling.
use crate::physics_state_ids::{PHYSICS_CATEGORY_OFFBOARD, PHYSICS_STATE_OFFBOARD};
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    animation::AnimationMatrix,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    skeleton_animation_record::SkeletonAnimationRecordInput,
    skeleton_current_animation::{
        SkeletonAdjustmentInput, SkeletonCurrentAnimationAttributeRead,
        SkeletonCurrentAnimationAttributes, SkeletonCurrentAnimationCvars,
    },
    skeleton_root_transforms::{SkeletonRootInput, SkeletonRootMatrix},
    skeleton_skateboard_offset::SkeletonSkateboardOffsetCvars,
};
use std::sync::Arc;

const TARGET_BONES: [usize; 4] = [33, 32, 35, 34];
const TARGET_PARTS: [usize; 4] = [15, 19, 3, 7];
// Controlled full offset: quarter turn around X, with separate nonzero XYZ.
const FULL_OFFSET: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.125, -0.0625, 0.03125, 0.0,
];

struct OffsetCvars {
    calls: usize,
    value: Result<f32, &'static str>,
}
impl SkeletonSkateboardOffsetCvars for OffsetCvars {
    type Error = &'static str;
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.calls += 1;
        self.value
    }
}

struct Cvars {
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
    xyz: u8,
    offboard_reads: usize,
}
impl Cvars {
    fn new(xyz: u8, fail: Option<&'static str>) -> Self {
        Self {
            calls: Vec::new(),
            fail,
            xyz,
            offboard_reads: 0,
        }
    }
    fn sample(&mut self, name: &'static str, value: u8) -> Result<u8, &'static str> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(value)
        }
    }
}
impl SkeletonAnimationRecordInput for Cvars {
    type Error = &'static str;
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        self.sample("animation COM", self.xyz)
    }
}
impl SkeletonCurrentAnimationCvars for Cvars {
    fn landing_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("inactive spring fixture")
    }
    fn landing_use_physics_com(&mut self) -> Result<u8, Self::Error> {
        panic!("inactive spring fixture")
    }
    fn landing_first_frame_hack(&mut self) -> Result<f32, Self::Error> {
        panic!("inactive spring fixture")
    }
    fn landing_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        panic!("inactive spring fixture")
    }
    fn landing_on_board_show_adjust(&mut self) -> Result<u8, Self::Error> {
        panic!("landing on board is inactive in this offset fixture")
    }
    fn landing_on_board_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("landing on board is inactive in this offset fixture")
    }
    fn landing_on_board_negate_y(&mut self) -> Result<u8, Self::Error> {
        panic!("landing on board is inactive in this offset fixture")
    }
    fn landing_on_board_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        panic!("landing on board is inactive in this offset fixture")
    }
    fn debug_landing_adjust(&mut self) -> Result<u8, Self::Error> {
        self.sample("landing debug", 0)
    }
    fn offboard_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.offboard_reads += 1;
        let (name, value) = match self.offboard_reads {
            1 => ("offboard full", 3.0),
            2 => ("offboard Y", 5.0),
            _ => panic!("only two source offboard cvar reads per attempt"),
        };
        self.calls.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(value)
        }
    }
}

pub(crate) struct CurrentAnimationCorpus {
    owner: Arc<PhysicalPlayerAssemblyOwner>,
    updates: u8,
    previous_centre: [f32; 4],
    previous_com_to_deck: [f32; 4],
    previous_deck_zy: f32,
    expected_offset: AnimationMatrix,
    offset_count: f32,
    vertical_count: f32,
}
impl CurrentAnimationCorpus {
    pub(crate) fn new(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Self {
        let mut attributes = SkeletonCurrentAnimationAttributes::bind(&owner.inputs().vault);
        assert_eq!(
            attributes.landing_max_velocity().unwrap().to_bits(),
            8.0_f32.to_bits()
        );
        Self {
            owner: Arc::clone(owner),
            updates: 0,
            previous_centre: [0.0; 4],
            previous_com_to_deck: [0.0; 4],
            previous_deck_zy: 0.0,
            expected_offset: FULL_OFFSET,
            offset_count: 4.0,
            vertical_count: 0.0,
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "same-owner original mapping, independent COM oracle and failure prefixes"
    )]
    pub(crate) fn check(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        current_pose: &[AnimationMatrix],
        time_delta: f32,
    ) -> SkeletonRootInput {
        assert!(Arc::ptr_eq(
            world.physical_body_member(skeleton).unwrap().owner(),
            &self.owner
        ));
        if self.updates == 0 {
            let mut offset_cvars = OffsetCvars {
                calls: 0,
                value: Err("offset frames"),
            };
            assert!(
                world
                    .try_set_skeleton_skateboard_offset(board, &FULL_OFFSET, &mut offset_cvars)
                    .is_err()
            );
            assert_eq!(offset_cvars.calls, 0);
            assert!(
                world
                    .try_set_skeleton_skateboard_offset(skeleton, &FULL_OFFSET, &mut offset_cvars)
                    .is_err()
            );
            assert_eq!(offset_cvars.calls, 1);
            let retained = world
                .skeleton(skeleton)
                .unwrap()
                .current_animation()
                .offset();
            assert_eq!(
                retained.transform.map(f32::to_bits),
                FULL_OFFSET.map(f32::to_bits)
            );
            assert!(retained.full_just_set);
            assert_eq!(retained.full_count.to_bits(), 0);
            offset_cvars.value = Ok(4.0);
            world
                .try_set_skeleton_skateboard_offset(skeleton, &FULL_OFFSET, &mut offset_cvars)
                .unwrap();
            assert_eq!(offset_cvars.calls, 2);
        }
        let normal = world.skeleton(skeleton).unwrap();
        let data = Arc::clone(&normal.state().binding.body().data);
        let animation = normal.current_animation();
        assert!(Arc::ptr_eq(animation.ik().data(), &data));
        assert_eq!(*animation.ik().target_bone_ids(), TARGET_BONES.map(Some));
        assert_eq!(
            TARGET_PARTS.map(|part| data.bone_ids()[part]),
            [24, 20, 16, 12]
        );
        assert_eq!(
            animation.record().centre_of_mass().map(f32::to_bits),
            self.previous_centre.map(f32::to_bits)
        );
        let animation_to_board = known_matrix(normal.root_transforms().animation_to_board());
        if self.updates != 0 {
            assert_ne!(
                animation_to_board[12..16],
                [0.0; 4],
                "second animation record must use retained root output"
            );
        }

        // Independent source mass sum and fractional products, using the SAME
        // initialized mass owner already consumed by live physics history.
        let history = normal.physics_history();
        let total = history
            .part_masses()
            .iter()
            .fold(0.0_f32, |sum, mass| mass + sum);
        assert_eq!(total.to_bits(), history.total_mass().to_bits());
        assert!(total > 0.0);
        let reciprocal = 1.0 / total;
        let weights = history.part_masses().map(|mass| mass * reciprocal);
        assert_eq!(
            weights.map(f32::to_bits),
            history.fractional_part_masses().map(f32::to_bits)
        );

        // The original hierarchy indices select the supplied RAW rig pose.
        // Controlled translation and deck tilt force observable current-pose
        // changes and deckZY history while retaining actual authored offsets.
        let tick = f32::from(self.updates + 1);
        let mut pose = current_pose.to_vec();
        for matrix in &mut pose {
            matrix[12] += 0.125 * tick;
        }
        pose[data.bone_ids()[0]][9] += 0.03125 * tick;
        for bone in TARGET_BONES {
            pose[bone][13] += 0.0625 * tick;
        }
        if self.updates == 2 {
            // Pose-derived activation deliberately separates both real hands
            // from their reparented targets; mirror selects LEFT, never right.
            let hand_change = [
                0.8, 0.0, -0.6, 0.0, 0.0, 1.0, 0.0, 0.0, 0.6, 0.0, 0.8, 0.0, 0.25, 0.5, -0.125, 0.0,
            ];
            let target_change = [
                1.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.6, 0.0, 0.0, -0.6, 0.8, 0.0, -0.125, 0.25, 0.375,
                0.0,
            ];
            pose[16] = compose(&hand_change, &pose[16]);
            pose[35] = compose(&target_change, &pose[35]);
            pose[12][12] += 2.0;
            pose[34][14] -= 3.0;
        }
        let raw_volumes: [AnimationMatrix; 24] = std::array::from_fn(|part| {
            compose(&pose[data.bone_ids()[part]], &data.bone_to_volume()[part])
        });
        let raw_ik: [AnimationMatrix; 4] = std::array::from_fn(|target| {
            compose(
                &pose[TARGET_BONES[target]],
                &data.bone_to_volume()[TARGET_PARTS[target]],
            )
        });
        let mut input = SkeletonAdjustmentInput {
            category: 0,
            state: 0,
            filtered_state: 1,
            flags_8c4: 0,
            flags_8c8: 0,
            // First frame takes inactive GrindAir; second explicitly skips
            // LandingOnSkateboardAdjust through the original two-bit gate.
            flags_8cc: if self.updates == 0 { 0 } else { 0x4_0200 },
            balance: 0.0,
            centre_of_mass_velocity: [2.0, tick + 0.25, 4.0, 99.0],
            up: [0.0, 1.0, 0.0, 0.0],
            landing_on_board: crate::skeleton_landing_on_board::SkeletonLandingOnBoardInput {
                flags_8d0: 0,
                deck_transform: crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM,
                landing_on_deck_adjust: [0.0; 4],
                time_to_land: 0.0,
            },
        };
        if self.updates == 2 {
            input.category = PHYSICS_CATEGORY_OFFBOARD;
            input.state = PHYSICS_STATE_OFFBOARD;
            input.filtered_state = 6;
            input.flags_8cc |= 8;
        }
        let xyz = self.updates;
        let mut passed = f32::from_bits(0x42f6_0000);
        let sentinel = passed.to_bits();
        let old_hips = *animation.hips();
        let old_record = *animation.record().poses();
        let old_filtered = animation.last_filtered_state();
        let old_velocity = animation.second_last_com_velocity();

        let mut forbidden = Cvars::new(xyz, None);
        assert!(
            world
                .try_process_skeleton_current_animation(
                    board,
                    &pose,
                    &input,
                    &mut passed,
                    &mut forbidden
                )
                .is_err()
        );
        assert!(forbidden.calls.is_empty());
        assert_eq!(passed.to_bits(), sentinel);
        assert_eq!(
            world
                .skeleton(skeleton)
                .unwrap()
                .current_animation()
                .hips()
                .map(f32::to_bits),
            old_hips.map(f32::to_bits)
        );

        if self.updates == 1 {
            // Select landing; flip-board ensures it skips even if retained Y<0.
            // A later debug-cvar failure retains raw/mapped/IK and old history.
            input.flags_8cc = 0x200;
            input.landing_on_board.flags_8d0 = 0x10;
            let mut selected_landing = Cvars::new(xyz, Some("landing debug"));
            assert!(
                world
                    .try_process_skeleton_current_animation(
                        skeleton,
                        &pose,
                        &input,
                        &mut passed,
                        &mut selected_landing
                    )
                    .is_err()
            );
            assert_eq!(selected_landing.calls, ["landing debug"]);
            let current = world.skeleton(skeleton).unwrap().current_animation();
            assert_eq!(
                current.raw_skateboard().map(f32::to_bits),
                raw_volumes[0].map(f32::to_bits)
            );
            assert_eq!(
                current
                    .ik()
                    .targets()
                    .map(|matrix| matrix.map(f32::to_bits)),
                raw_ik.map(|matrix| matrix.map(f32::to_bits))
            );
            assert_eq!(current.last_filtered_state(), old_filtered);
            assert_eq!(
                current.second_last_com_velocity().to_bits(),
                old_velocity.to_bits()
            );
            assert!(current.root_input(time_delta).is_none());
            assert_eq!(passed.to_bits(), sentinel);
            input.flags_8cc = 0x4_0200;
            input.landing_on_board.flags_8d0 = 0;

            let mut landing_failure = Cvars::new(xyz, Some("landing debug"));
            assert!(
                world
                    .try_process_skeleton_current_animation(
                        skeleton,
                        &pose,
                        &input,
                        &mut passed,
                        &mut landing_failure
                    )
                    .is_err()
            );
            assert_eq!(landing_failure.calls, ["landing debug"]);
            let current = world.skeleton(skeleton).unwrap().current_animation();
            assert_eq!(current.last_filtered_state(), old_filtered);
            assert_eq!(
                current.second_last_com_velocity().to_bits(),
                old_velocity.to_bits()
            );
            assert_eq!(current.hips().map(f32::to_bits), old_hips.map(f32::to_bits));
            assert_offset(
                current,
                &self.expected_offset,
                self.offset_count,
                self.vertical_count,
                false,
            );

            // COM cvar is later: all24 record poses and landing history have
            // committed, and offset decay has advanced even though neither COM
            // nor adjusted root output has changed.
            decay_full_offset(&mut self.expected_offset, &mut self.offset_count);
            let failed_volumes = adjusted_volumes(&raw_volumes, &self.expected_offset);
            let failed_ik = raw_ik.map(|matrix| compose(&self.expected_offset, &matrix));
            let mut com_failure = Cvars::new(xyz, Some("animation COM"));
            assert!(
                world
                    .try_process_skeleton_current_animation(
                        skeleton,
                        &pose,
                        &input,
                        &mut passed,
                        &mut com_failure
                    )
                    .is_err()
            );
            assert_eq!(com_failure.calls, ["landing debug", "animation COM"]);
            let current = world.skeleton(skeleton).unwrap().current_animation();
            assert_eq!(
                current
                    .record()
                    .poses()
                    .map(|matrix| matrix.map(f32::to_bits)),
                failed_volumes.map(|matrix| matrix.map(f32::to_bits))
            );
            assert_eq!(
                current
                    .ik()
                    .targets()
                    .map(|matrix| matrix.map(f32::to_bits)),
                failed_ik.map(|matrix| matrix.map(f32::to_bits))
            );
            assert_offset(
                current,
                &self.expected_offset,
                self.offset_count,
                self.vertical_count,
                false,
            );
            assert_eq!(
                current.raw_skateboard().map(f32::to_bits),
                raw_volumes[0].map(f32::to_bits)
            );
            assert_eq!(
                current.record().centre_of_mass().map(f32::to_bits),
                self.previous_centre.map(f32::to_bits)
            );
            assert_eq!(current.last_filtered_state(), 1);
            assert_eq!(
                current.second_last_com_velocity().to_bits(),
                (tick + 0.25).to_bits()
            );
            assert_eq!(current.hips().map(f32::to_bits), old_hips.map(f32::to_bits));
            assert!(current.root_input(time_delta).is_none());
            assert_eq!(passed.to_bits(), sentinel);
        }

        if self.updates == 2 {
            let mapping = compose(&pose[16], &rigid_inverse(&pose[35]));
            let wrong_hand = compose(&pose[12], &rigid_inverse(&pose[34]));
            assert_ne!(mapping.map(f32::to_bits), wrong_hand.map(f32::to_bits));
            assert_ne!(
                mapping.map(f32::to_bits),
                self.expected_offset.map(f32::to_bits)
            );
            for (failure, expected_calls, full_count) in [
                ("offboard full", vec!["offboard full"], self.offset_count),
                ("offboard Y", vec!["offboard full", "offboard Y"], 3.0),
            ] {
                let mut cvars = Cvars::new(xyz, Some(failure));
                assert!(
                    world
                        .try_process_skeleton_current_animation(
                            skeleton,
                            &pose,
                            &input,
                            &mut passed,
                            &mut cvars
                        )
                        .is_err()
                );
                assert_eq!(cvars.calls, expected_calls);
                let current = world.skeleton(skeleton).unwrap().current_animation();
                assert_eq!(
                    current.offset().transform.map(f32::to_bits),
                    mapping.map(f32::to_bits)
                );
                assert!(current.offset().full_just_set);
                assert_eq!(current.offset().full_count.to_bits(), full_count.to_bits());
                assert_eq!(
                    current.offset().vertical_count.to_bits(),
                    self.vertical_count.to_bits()
                );
                assert!(!current.offset().vertical_just_set);
                assert_eq!(
                    current
                        .ik()
                        .targets()
                        .map(|matrix| matrix.map(f32::to_bits)),
                    raw_ik.map(|matrix| matrix.map(f32::to_bits))
                );
                assert_eq!(
                    current.raw_skateboard().map(f32::to_bits),
                    raw_volumes[0].map(f32::to_bits)
                );
                assert_eq!(
                    current.record().centre_of_mass().map(f32::to_bits),
                    self.previous_centre.map(f32::to_bits)
                );
                assert_eq!(
                    current
                        .record()
                        .poses()
                        .map(|matrix| matrix.map(f32::to_bits)),
                    old_record.map(|matrix| matrix.map(f32::to_bits))
                );
                assert_eq!(current.hips().map(f32::to_bits), old_hips.map(f32::to_bits));
                assert_eq!(current.last_filtered_state(), old_filtered);
                assert_eq!(
                    current.second_last_com_velocity().to_bits(),
                    old_velocity.to_bits()
                );
                assert_eq!(passed.to_bits(), sentinel);
                assert!(current.root_input(time_delta).is_none());
            }
            self.expected_offset = mapping;
            self.offset_count = 3.0;
            self.vertical_count = 5.0;
        } else if self.updates != 0 {
            let old_y = self.expected_offset[13];
            decay_full_offset(&mut self.expected_offset, &mut self.offset_count);
            assert_eq!(self.expected_offset[13].to_bits(), old_y.to_bits());
            if self.vertical_count > 0.0 {
                let ratio = (self.vertical_count - 1.0) / self.vertical_count;
                self.expected_offset[13] *= ratio * ratio;
                self.vertical_count -= 1.0;
                assert_ne!(self.expected_offset[13].to_bits(), old_y.to_bits());
            }
        }
        let volumes = adjusted_volumes(&raw_volumes, &self.expected_offset);
        let ik = raw_ik.map(|matrix| compose(&self.expected_offset, &matrix));
        let mut cvars = Cvars::new(xyz, None);
        world
            .try_process_skeleton_current_animation(
                skeleton,
                &pose,
                &input,
                &mut passed,
                &mut cvars,
            )
            .unwrap();
        let expected_calls = if self.updates == 2 {
            vec![
                "offboard full",
                "offboard Y",
                "landing debug",
                "animation COM",
            ]
        } else {
            vec!["landing debug", "animation COM"]
        };
        assert_eq!(cvars.calls, expected_calls);
        let normal = world.skeleton(skeleton).unwrap();
        let current = normal.current_animation();
        assert_offset(
            current,
            &self.expected_offset,
            self.offset_count,
            self.vertical_count,
            false,
        );
        assert_ne!(
            volumes[0].map(f32::to_bits),
            raw_volumes[0].map(f32::to_bits)
        );
        for target in 0..4 {
            assert_ne!(
                ik[target].map(f32::to_bits),
                raw_ik[target].map(f32::to_bits)
            );
        }
        assert_eq!(
            normal
                .animation_targets()
                .completed_volumes()
                .unwrap()
                .map(|matrix| matrix.map(f32::to_bits)),
            volumes.map(|matrix| matrix.map(f32::to_bits))
        );
        assert_eq!(
            current
                .ik()
                .targets()
                .map(|matrix| matrix.map(f32::to_bits)),
            ik.map(|matrix| matrix.map(f32::to_bits))
        );
        assert_eq!(
            current
                .record()
                .poses()
                .map(|matrix| matrix.map(f32::to_bits)),
            volumes.map(|matrix| matrix.map(f32::to_bits))
        );
        let mut centre = [0.0_f32; 4];
        for part in 0..24 {
            for (lane, value) in centre.iter_mut().enumerate() {
                *value = volumes[part][12 + lane].mul_add(weights[part], *value);
            }
        }
        if xyz == 0 {
            centre = [volumes[0][12], centre[1], volumes[0][14], volumes[0][15]];
        }
        let centre_delta: [f32; 4] =
            std::array::from_fn(|lane| centre[lane] - self.previous_centre[lane]);
        let world_centre = point(&animation_to_board, centre);
        let world_deck = point(
            &animation_to_board,
            std::array::from_fn(|lane| volumes[0][12 + lane]),
        );
        let com_to_deck: [f32; 4] =
            std::array::from_fn(|lane| world_centre[lane] - world_deck[lane]);
        let scalar = if self.updates == 0 { 0.0 } else { 1.0 };
        let com_delta: [f32; 4] = std::array::from_fn(|lane| {
            (com_to_deck[lane] - self.previous_com_to_deck[lane]) * scalar
        });
        assert_eq!(
            current.record().centre_of_mass().map(f32::to_bits),
            centre.map(f32::to_bits)
        );
        assert_eq!(
            current.record().centre_of_mass_delta().map(f32::to_bits),
            centre_delta.map(f32::to_bits)
        );
        assert_vector(current.record().com_to_deck(), com_to_deck);
        assert_vector(current.record().com_to_deck_delta(), com_delta);
        assert_eq!(current.record().reset_scalar().to_bits(), 1.0_f32.to_bits());
        assert_eq!(
            current.hips().map(f32::to_bits),
            volumes[23].map(f32::to_bits)
        );
        assert_eq!(
            current.raw_skateboard().map(f32::to_bits),
            raw_volumes[0].map(f32::to_bits)
        );
        assert_eq!(
            passed.to_bits(),
            (volumes[0][9] - self.previous_deck_zy).to_bits()
        );
        assert_eq!(current.last_deck_zy().to_bits(), volumes[0][9].to_bits());
        assert_ne!(passed.to_bits(), 0);
        assert_eq!(current.last_filtered_state(), input.filtered_state);
        assert_eq!(
            current.second_last_com_velocity().to_bits(),
            (tick + 0.25).to_bits()
        );
        let root = current.root_input(time_delta).unwrap();
        assert_eq!(
            root.skate_animation.map(f32::to_bits),
            volumes[0].map(f32::to_bits)
        );
        self.previous_centre = centre;
        self.previous_com_to_deck = com_to_deck;
        self.previous_deck_zy = volumes[0][9];
        self.updates += 1;
        root
    }
}

fn assert_offset(
    current: &crate::skeleton_current_animation::SkeletonCurrentAnimation,
    matrix: &AnimationMatrix,
    count: f32,
    vertical_count: f32,
    just_set: bool,
) {
    let offset = current.offset();
    assert_eq!(offset.transform.map(f32::to_bits), matrix.map(f32::to_bits));
    assert_eq!(offset.full_count.to_bits(), count.to_bits());
    assert_eq!(offset.full_just_set, just_set);
    assert_eq!(offset.vertical_count.to_bits(), vertical_count.to_bits());
    assert!(!offset.vertical_just_set);
}

/// Source rigid transpose inverse: +0-P, Z product, Y FMA, X FMA.
fn rigid_inverse(matrix: &AnimationMatrix) -> AnimationMatrix {
    let mut inverse = [0.0; 16];
    for axis in 0..3 {
        for lane in 0..3 {
            inverse[axis * 4 + lane] = matrix[lane * 4 + axis];
        }
    }
    let negative: [f32; 3] = std::array::from_fn(|lane| 0.0 - matrix[12 + lane]);
    for lane in 0..4 {
        let z = negative[2] * inverse[8 + lane];
        let yz = negative[1].mul_add(inverse[4 + lane], z);
        inverse[12 + lane] = negative[0].mul_add(inverse[lane], yz);
    }
    inverse
}

fn adjusted_volumes(
    raw: &[AnimationMatrix; 24],
    offset: &AnimationMatrix,
) -> [AnimationMatrix; 24] {
    let mut result = *raw;
    result[0] = compose(offset, &raw[0]);
    result
}

/// Independent scalar transcription of the SOURCE equations, not a call to
/// either the production offset kernel or its normalization helper. Uses the
/// declared host dot/estimate policy, so exact comparisons are host graph
/// oracles, not captured Xenon arithmetic bits.
fn decay_full_offset(matrix: &mut AnimationMatrix, count: &mut f32) {
    let ratio = (*count - 1.0) / *count;
    let weight = ratio * ratio;
    let inverse_weight = 1.0 - weight;
    let mut blended = [0.0; 16];
    for column in 0..3 {
        for lane in 0..4 {
            let identity = if column == lane { 1.0 } else { 0.0 };
            let separate_identity_product = identity * inverse_weight;
            blended[column * 4 + lane] =
                matrix[column * 4 + lane].mul_add(weight, separate_identity_product);
        }
    }
    let mut normalized = [0.0; 16];
    for axis in (0..3).rev() {
        let original: [f32; 4] = std::array::from_fn(|lane| blended[axis * 4 + lane]);
        let mut candidate = original;
        for prior in ((axis + 1)..3).rev() {
            let basis: [f32; 4] = std::array::from_fn(|lane| normalized[prior * 4 + lane]);
            let projection = dot3(basis, original);
            for lane in 0..4 {
                let rounded_product = basis[lane] * projection;
                candidate[lane] -= rounded_product;
            }
        }
        let squared = dot3(candidate, candidate);
        let mut reciprocal = squared.sqrt().recip();
        for _ in 0..2 {
            let square = reciprocal * reciprocal;
            let half = reciprocal * 0.5;
            let residual = (-squared).mul_add(square, 1.0);
            reciprocal = half.mul_add(residual, reciprocal);
        }
        for lane in 0..4 {
            normalized[axis * 4 + lane] = candidate[lane] * reciprocal;
        }
    }
    for lane in 0..4 {
        normalized[12 + lane] = matrix[12 + lane] * weight;
    }
    normalized[13] = matrix[13];
    *matrix = normalized;
    *count -= 1.0;
}

fn dot3(left: [f32; 4], right: [f32; 4]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

/// Independent full4 scalar source DAG, not either production target kernel.
fn compose(outer: &AnimationMatrix, inner: &AnimationMatrix) -> AnimationMatrix {
    let mut result = [0.0; 16];
    for column in 0..4 {
        for lane in 0..4 {
            let base = column * 4;
            let x = if column == 3 {
                inner[base].mul_add(outer[lane], outer[12 + lane])
            } else {
                inner[base] * outer[lane]
            };
            let xy = inner[base + 1].mul_add(outer[4 + lane], x);
            result[base + lane] = inner[base + 2].mul_add(outer[8 + lane], xy);
        }
    }
    result
}
fn point(matrix: &AnimationMatrix, position: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| {
        let x = matrix[lane].mul_add(position[0], matrix[12 + lane]);
        let xy = matrix[4 + lane].mul_add(position[1], x);
        matrix[8 + lane].mul_add(position[2], xy)
    })
}
fn known_matrix(matrix: &SkeletonRootMatrix) -> AnimationMatrix {
    std::array::from_fn(|index| {
        let column = matrix.columns()[index / 4];
        if index % 4 == 3 {
            column.w.expect("actual AnimToBoard has known W")
        } else {
            column.xyz[index % 4]
        }
    })
}
fn assert_vector(actual: crate::skeleton_root_transforms::SkeletonRootVector, expected: [f32; 4]) {
    assert_eq!(
        actual.xyz.map(f32::to_bits),
        [expected[0], expected[1], expected[2]].map(f32::to_bits)
    );
    assert_eq!(actual.w.map(f32::to_bits), Some(expected[3].to_bits()));
}
