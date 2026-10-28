//! Original-owner four-mode landing spring -> existing Y offset -> mapped deck,
//! four IK targets and animation record. Deliberate processed state transitions
//! are component ticks within the existing solved frame, not a recovered Actor
//! schedule or additional physics solves. No history seed or alternate owner is
//! used. Finite f64 equations independently check the spring; source-form f32
//! consumer checks use the verified retained offset and original Part masses.

use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    animation::AnimationMatrix,
    attribute_runtime::AttributeRuntimeInstance,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    skeleton_animation_record::SkeletonAnimationRecordInput,
    skeleton_current_animation::{SkeletonAdjustmentInput, SkeletonCurrentAnimationCvars},
    skeleton_landing_adjust::SkeletonLandingAdjustKind as Kind,
    skeleton_landing_on_board::SkeletonLandingOnBoardInput,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

const DT: f32 = f32::from_bits(0x3c88_8889);
const HACK: f32 = f32::from_bits(0x3c75_c28f);
const SKIP_BOARD_ADJUST: u32 = 0x4_0200;
const PREPARE_JUMP: u32 = 0x20_0000;
const TARGET_BONES: [usize; 4] = [33, 32, 35, 34];
const TARGET_PARTS: [usize; 4] = [15, 19, 3, 7];

#[derive(Default)]
struct Cvars {
    calls: Vec<&'static str>,
}
impl SkeletonAnimationRecordInput for Cvars {
    type Error = &'static str;
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("COM");
        Ok(1)
    }
}
impl SkeletonCurrentAnimationCvars for Cvars {
    fn debug_landing_adjust(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("debug");
        Ok(0)
    }
    fn landing_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("debug is disabled")
    }
    fn landing_use_physics_com(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("physics COM");
        Ok(1)
    }
    fn landing_first_frame_hack(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("hack");
        Ok(HACK)
    }
    fn landing_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("frames");
        Ok(7.0)
    }
    fn offboard_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        panic!("category is controlled ground")
    }
    fn landing_on_board_show_adjust(&mut self) -> Result<u8, Self::Error> {
        panic!("LandingOnBoard is source-gated off")
    }
    fn landing_on_board_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("LandingOnBoard is source-gated off")
    }
    fn landing_on_board_negate_y(&mut self) -> Result<u8, Self::Error> {
        panic!("LandingOnBoard is source-gated off")
    }
    fn landing_on_board_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        panic!("LandingOnBoard is source-gated off")
    }
}

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
    let manual = crate::skeleton_landing_attributes::tests::check_original(&owner.inputs().vault);
    assert!(manual.iter().any(|word| *word != 0));
    let fixture = Fixture::new(world, skeleton, owner);
    let retained_velocity = world
        .skeleton(skeleton)
        .unwrap()
        .post_physics_records()
        .centre_of_mass_velocity();
    for kind in [Kind::Ground, Kind::Manual, Kind::Grind, Kind::Coffin] {
        let seed_state = if kind == Kind::Ground { 6 } else { 2 };
        let mut seed = fixture.input(seed_state, -6.0);
        seed.flags_8c8 = 1; // Actual disable/history producer, never seed_history.
        fixture.tick(world, skeleton, &seed);
        assert!(
            !world
                .skeleton(skeleton)
                .unwrap()
                .current_animation()
                .landing_adjust()
                .adjusting()
        );

        let state = if kind == Kind::Grind { 3 } else { 1 };
        let mut start = fixture.input(state, -2.0);
        if kind != Kind::Grind {
            start.flags_8cc |= 0x4000_0000;
        }
        if matches!(kind, Kind::Manual | Kind::Coffin) {
            start.balance = 1.0;
        }
        if kind == Kind::Coffin {
            start.flags_8c8 = 4;
        }
        fixture.tick(world, skeleton, &start);
        assert_eq!(
            world
                .skeleton(skeleton)
                .unwrap()
                .current_animation()
                .landing_adjust()
                .kind(),
            kind
        );
        // Ground+manual+coffin predicates overlap deliberately: priority must
        // select manual over ground and coffin over both. Grind uses prior -6
        // velocity while its new processed history is -2.
        assert_eq!(
            world
                .skeleton(skeleton)
                .unwrap()
                .current_animation()
                .landing_adjust()
                .second_last_com_velocity()
                .to_bits(),
            (-2.0_f32).to_bits()
        );

        if world
            .skeleton(skeleton)
            .unwrap()
            .current_animation()
            .landing_adjust()
            .adjusting()
        {
            // This continuing call must read the actual previous animation COM
            // even though the physical-COM cvar is still true on every call.
            fixture.tick(world, skeleton, &start);
        }
        if !world
            .skeleton(skeleton)
            .unwrap()
            .current_animation()
            .landing_adjust()
            .adjusting()
        {
            // An authored low blend may finish immediately. A fresh real
            // transition plus prepare-jump still executes the mode and Y store.
            fixture.tick(world, skeleton, &seed);
        }
        start.flags_8c4 |= PREPARE_JUMP;
        fixture.tick(world, skeleton, &start);
        let current = world.skeleton(skeleton).unwrap().current_animation();
        assert_eq!(current.landing_adjust().kind(), kind);
        assert!(!current.landing_adjust().adjusting());
        assert_eq!(current.offset().vertical_count.to_bits(), 6.0_f32.to_bits());
    }
    assert_eq!(
        world
            .skeleton(skeleton)
            .unwrap()
            .post_physics_records()
            .centre_of_mass_velocity()
            .map(f32::to_bits),
        retained_velocity.map(f32::to_bits),
        "component animation ticks cannot rewrite physical post-record velocity"
    );
}

struct Fixture<'a> {
    pose: &'a [AnimationMatrix],
    raw: [AnimationMatrix; 24],
    raw_ik: [AnimationMatrix; 4],
    weights: [f32; 24],
    roots: [SkeletonRootVector; 4],
    physical_com: [f32; 4],
    physical_root_position: [f32; 4],
    original: Original,
}
impl<'a> Fixture<'a> {
    fn new(
        world: &PhysicalAssemblyWorld,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &'a PhysicalPlayerAssemblyOwner,
    ) -> Self {
        let normal = world.skeleton(skeleton).unwrap();
        let data = &normal.state().binding.body().data;
        let pose = &owner.inputs().pose_matrices;
        let masses = *normal.physics_history().part_masses();
        let total = masses.into_iter().fold(0.0_f32, |sum, mass| sum + mass);
        Self {
            pose,
            raw: std::array::from_fn(|part| {
                compose(&pose[data.bone_ids()[part]], &data.bone_to_volume()[part])
            }),
            raw_ik: std::array::from_fn(|target| {
                compose(
                    &pose[TARGET_BONES[target]],
                    &data.bone_to_volume()[TARGET_PARTS[target]],
                )
            }),
            weights: masses.map(|mass| mass * (1.0 / total)),
            roots: *normal.root_transforms().animation_to_board().columns(),
            physical_com: normal.physics_history().record().centre_of_mass(),
            physical_root_position: normal.physics_history().record().positions()[0],
            original: Original(
                owner
                    .inputs()
                    .vault
                    .instance(0x74bc_9c93_affe_6aac, 0xd7ed_bd36_2d7d_2152),
            ),
        }
    }
    fn input(&self, state: i32, velocity: f32) -> SkeletonAdjustmentInput {
        let mut deck = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
        deck[13] = self.physical_root_position[1];
        SkeletonAdjustmentInput {
            category: 0,
            state: 0,
            filtered_state: state,
            flags_8c4: 0,
            flags_8c8: 0,
            flags_8cc: SKIP_BOARD_ADJUST,
            balance: 0.0,
            centre_of_mass_velocity: [5.0, velocity, 7.0, 99.0],
            up: [0.0, 1.0, 0.0, 0.0],
            landing_on_board: SkeletonLandingOnBoardInput {
                flags_8d0: 0,
                deck_transform: deck,
                landing_on_deck_adjust: [0.0; 4],
                time_to_land: 0.0,
            },
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "independent retained spring and all mapped/IK/record consumer assertions"
    )]
    fn tick(
        &self,
        world: &mut PhysicalAssemblyWorld,
        skeleton: &SimulationSkaterAssemblyHandle,
        input: &SkeletonAdjustmentInput,
    ) {
        let normal = world.skeleton(skeleton).unwrap();
        let current = normal.current_animation();
        let old_com = current.record().centre_of_mass();
        let old_com_to_deck = current.record().com_to_deck();
        let old_deck_zy = current.last_deck_zy();
        let before = Snapshot::from(current.landing_adjust());
        let expected = self.expected(before, input, old_com[1]);
        let old_y = current.offset().transform[13];
        let old_y_count = current.offset().vertical_count;
        let old_full_count = current.offset().full_count;
        assert!(!current.offset().full_just_set && !current.offset().vertical_just_set);
        let (expected_y, expected_count) = if let Some((y, count)) = expected.offset {
            (y, count)
        } else if old_y_count > 0.0 {
            let quotient = (old_y_count - 1.0) / old_y_count;
            (f64::from(old_y * (quotient * quotient)), old_y_count - 1.0)
        } else {
            (f64::from(old_y), old_y_count)
        };
        let mut passed = 321.0_f32;
        let mut cvars = Cvars::default();
        world
            .try_process_skeleton_current_animation(
                skeleton,
                self.pose,
                input,
                &mut passed,
                &mut cvars,
            )
            .unwrap();
        assert_eq!(cvars.calls, expected.calls);
        let normal = world.skeleton(skeleton).unwrap();
        let current = normal.current_animation();
        let landing = current.landing_adjust();
        assert_eq!(landing.kind(), expected.state.kind);
        assert_eq!(landing.adjusting(), expected.state.adjusting);
        assert_eq!(landing.time().to_bits(), expected.state.time.to_bits());
        close(
            landing.y_position(),
            expected.state.position,
            "retained Y position",
        );
        close(
            landing.y_velocity(),
            expected.state.velocity,
            "retained Y velocity",
        );
        close(
            landing.last_animation_position(),
            expected.state.last_animation,
            "previous-animation input",
        );
        close(
            landing.desired_com_for_grind(),
            expected.state.desired_grind,
            "retained grind desired COM",
        );
        assert_eq!(
            landing.second_last_com_velocity().to_bits(),
            input.centre_of_mass_velocity[1].to_bits()
        );
        assert_eq!(landing.last_filtered_state(), input.filtered_state);
        close(
            current.offset().transform[13],
            expected_y,
            "actual spring Y offset",
        );
        assert_eq!(
            current.offset().vertical_count.to_bits(),
            expected_count.to_bits()
        );
        assert_eq!(
            current.offset().full_count.to_bits(),
            if old_full_count > 0.0 {
                old_full_count - 1.0
            } else {
                old_full_count
            }
            .to_bits()
        );
        assert!(!current.offset().full_just_set && !current.offset().vertical_just_set);
        let offset = &current.offset().transform;
        let mut volumes = self.raw;
        volumes[0] = compose(offset, &self.raw[0]);
        let ik = self.raw_ik.map(|matrix| compose(offset, &matrix));
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
        assert_eq!(
            current.raw_skateboard().map(f32::to_bits),
            self.raw[0].map(f32::to_bits)
        );
        assert_eq!(
            current.hips().map(f32::to_bits),
            volumes[23].map(f32::to_bits)
        );
        let mut centre = [0.0_f32; 4];
        for (matrix, weight) in volumes.iter().zip(self.weights) {
            for lane in 0..4 {
                centre[lane] = matrix[12 + lane].mul_add(weight, centre[lane]);
            }
        }
        assert_eq!(
            current.record().centre_of_mass().map(f32::to_bits),
            centre.map(f32::to_bits)
        );
        assert_eq!(
            current.record().centre_of_mass_delta().map(f32::to_bits),
            std::array::from_fn::<_, 4, _>(|lane| centre[lane] - old_com[lane]).map(f32::to_bits)
        );
        let com_to_deck = subtract(
            point(&self.roots, centre),
            point(
                &self.roots,
                std::array::from_fn(|lane| volumes[0][12 + lane]),
            ),
        );
        assert_vector(current.record().com_to_deck(), com_to_deck);
        assert_vector(
            current.record().com_to_deck_delta(),
            subtract(com_to_deck, old_com_to_deck),
        );
        assert_eq!(passed.to_bits(), (volumes[0][9] - old_deck_zy).to_bits());
        assert_eq!(current.last_deck_zy().to_bits(), volumes[0][9].to_bits());
        assert_eq!(
            current
                .root_input(DT)
                .unwrap()
                .skate_animation
                .map(f32::to_bits),
            volumes[0].map(f32::to_bits)
        );
    }

    #[expect(
        clippy::too_many_lines,
        reason = "independent finite equations for four source modes and retained-field expectations"
    )]
    fn expected(
        &self,
        mut state: Snapshot,
        input: &SkeletonAdjustmentInput,
        previous_com_y: f32,
    ) -> Expected {
        let a = &self.original;
        let maximum = a.scalar(0x5199_e2e7_49e7_220b);
        assert!(maximum > 0.0 && maximum.is_finite());
        let old_state = state.last_state;
        let offboard_ground = old_state == 6 && input.filtered_state == 1;
        let air_ground = old_state == 2 && input.filtered_state == 1;
        let ground = (offboard_ground || air_ground) && input.flags_8cc & 0x4000_0000 != 0;
        let manual = air_ground && input.balance != 0.0;
        let grind = old_state == 2 && input.filtered_state == 3 && input.flags_8c4 & 0x20 == 0;
        let coffin = input.flags_8c8 & 4 != 0 && (grind || air_ground);
        let disable = input.flags_8c8 & 1 != 0 && input.flags_8c8 & 4 == 0;
        if disable {
            state.adjusting = false;
        }
        let mut calls = vec!["debug"];
        let mut first_ground = false;
        if !state.adjusting && !disable && (ground || manual || grind || coffin) {
            state.kind = if coffin {
                Kind::Coffin
            } else if manual {
                Kind::Manual
            } else if ground {
                Kind::Ground
            } else {
                Kind::Grind
            };
            state.adjusting = true;
            state.time = 0.0;
            let start_velocity = if grind || coffin {
                state.previous_velocity
            } else {
                f64::from(input.centre_of_mass_velocity[1])
            };
            state.velocity = start_velocity.max(-maximum).min(0.0);
            state.position =
                f64::from(self.physical_com[1]) - f64::from(self.physical_root_position[1]);
            state.last_animation = f64::from(f32::MAX);
            first_ground = state.kind == Kind::Ground;
            calls.push("debug");
        }
        let mut offset = None;
        if state.adjusting {
            calls.push("physics COM");
            let animation = if offboard_ground {
                f64::from(input.centre_of_mass_velocity[1]) * f64::from(DT)
                    + f64::from(self.physical_com[1])
                    - f64::from(input.landing_on_board.deck_transform[13])
            } else {
                f64::from(previous_com_y) - f64::from(self.raw[0][13])
            };
            let duration = a.scalar(0x1d4c_65d8_fd17_9fe2);
            assert!(duration > 0.0 && duration.is_finite());
            let (k1, k2, blend, mut stop) = match state.kind {
                Kind::Ground => (
                    a.scalar(0xac53_2ffc_deba_62d8),
                    a.scalar(0x2471_dd04_ee7e_e637),
                    1.0,
                    input.filtered_state != 1 || input.flags_8cc & 0x4000_0000 == 0,
                ),
                Kind::Manual => (
                    a.scalar(0x01ba_609b_b65c_adb5),
                    a.scalar(0x475a_3cb8_59bd_b0a6),
                    a.manual(f64::from(state.time)),
                    input.filtered_state != 1,
                ),
                Kind::Grind => (
                    a.scalar(0x1098_9560_be00_f690),
                    a.scalar(0xbf50_36b2_3111_79e8),
                    a.graph(0x804c_e67e_a039_46fa, f64::from(state.time)),
                    input.filtered_state != 3,
                ),
                Kind::Coffin => (
                    0.0,
                    0.0,
                    1.0,
                    input.filtered_state != 1
                        || f64::from(state.time) > duration
                        || input.flags_8c8 & 4 == 0,
                ),
            };
            if state.kind == Kind::Coffin {
                let denominator = a.scalar(0x4f4e_3f45_66ec_34e3);
                assert!(denominator > 0.0);
                let strength = (state.velocity.abs() / denominator).clamp(0.0, 1.0);
                state.position = a.scalar(0x2f99_6144_e5ef_4eea)
                    + a.graph(0x25e6_e31b_c284_1e1f, f64::from(state.time) / duration) * strength;
            } else {
                let mut desired = a.scalar(0x9091_e734_f21a_004d);
                if state.kind == Kind::Grind {
                    if f64::from(state.time) > a.scalar(0x8dac_44f9_0326_2ddb) {
                        let speed = a.scalar(0x74e3_fc7c_dd2f_8c97);
                        desired = animation
                            .clamp(state.desired_grind - speed, state.desired_grind + speed);
                    }
                    state.desired_grind = desired;
                }
                let displacement = state.position - desired;
                let old_velocity = state.velocity;
                let acceleration = -(displacement * k1 + old_velocity * k2);
                let upper = if state.kind == Kind::Ground {
                    0.0
                } else {
                    maximum
                };
                state.velocity =
                    (old_velocity + acceleration * f64::from(DT)).clamp(-maximum, upper);
                calls.push("debug");
                let position =
                    displacement + state.velocity.midpoint(old_velocity) * f64::from(DT) + desired;
                state.position = position.max(a.scalar(0x2a69_0d7e_8c1e_dcf4));
            }
            let mut correction = animation - state.position;
            if first_ground {
                calls.push("hack");
                correction = (correction
                    + (f64::from(HACK) * state.velocity)
                        .clamp(f64::from(f32::from_bits(0xbdcc_cccd)), 0.0))
                .max(0.0);
            }
            calls.push("debug");
            if state.kind == Kind::Ground
                && f64::from(state.time) > a.scalar(0x0175_8b41_9109_fdae)
                && state.last_animation < animation
                && correction > 0.0
            {
                stop = true;
            }
            if state.kind == Kind::Grind && input.flags_8c4 & 0x20 != 0 {
                stop = true;
            }
            if state.time > 1.5 {
                stop = true;
            }
            let jump = input.flags_8c4 & PREPARE_JUMP != 0;
            if (!stop && blend >= f64::from(f32::from_bits(0x3d4c_cccd))) || jump {
                let count = if state.kind == Kind::Coffin {
                    a.scalar_f32(0x9aba_ff27_0d53_f138)
                } else {
                    calls.push("frames");
                    7.0
                };
                offset = Some((correction * blend, if jump { 6.0 } else { count }));
                if jump {
                    state.adjusting = false;
                }
            } else {
                state.adjusting = false;
            }
            state.time += DT;
            state.last_animation = animation;
        }
        state.last_state = input.filtered_state;
        state.previous_velocity = f64::from(input.centre_of_mass_velocity[1]);
        calls.push("COM");
        Expected {
            state,
            offset,
            calls,
        }
    }
}

struct Snapshot {
    adjusting: bool,
    kind: Kind,
    time: f32,
    position: f64,
    velocity: f64,
    previous_velocity: f64,
    last_animation: f64,
    last_state: i32,
    desired_grind: f64,
}
impl Snapshot {
    fn from(state: &crate::skeleton_landing_adjust::SkeletonLandingAdjust) -> Self {
        Self {
            adjusting: state.adjusting(),
            kind: state.kind(),
            time: state.time(),
            position: f64::from(state.y_position()),
            velocity: f64::from(state.y_velocity()),
            previous_velocity: f64::from(state.second_last_com_velocity()),
            last_animation: f64::from(state.last_animation_position()),
            last_state: state.last_filtered_state(),
            desired_grind: f64::from(state.desired_com_for_grind()),
        }
    }
}
struct Expected {
    state: Snapshot,
    offset: Option<(f64, f32)>,
    calls: Vec<&'static str>,
}
struct Original(AttributeRuntimeInstance);
impl Original {
    fn scalar_f32(&self, key: u64) -> f32 {
        self.0.try_get_data(key, 0).unwrap().map_or(0.0, |data| {
            f32::from_be_bytes(data.bytes().try_into().unwrap())
        })
    }
    fn scalar(&self, key: u64) -> f64 {
        f64::from(self.scalar_f32(key))
    }
    fn graph(&self, key: u64, time: f64) -> f64 {
        self.0
            .try_get_data(key, 0)
            .unwrap()
            .map_or(0.0, |data| graph(data.bytes(), time))
    }
    fn manual(&self, time: f64) -> f64 {
        self.0
            .try_cached_layout(0x220)
            .unwrap()
            .map_or(0.0, |data| graph(&data.bytes()[0x140..0x170], time))
    }
}
fn graph(bytes: &[u8], time: f64) -> f64 {
    let values: Vec<_> = bytes[0x10..0x30]
        .chunks_exact(4)
        .map(|word| f64::from(f32::from_be_bytes(word.try_into().unwrap())))
        .collect();
    if time < values[0] {
        return values[4];
    }
    if time >= values[3] {
        return values[7];
    }
    let upper = (1..4).find(|index| time < values[*index]).unwrap();
    let width = values[upper] - values[upper - 1];
    if width <= 0.0 {
        values[4 + upper]
    } else {
        values[3 + upper]
            + (values[4 + upper] - values[3 + upper]) * (time - values[upper - 1]) / width
    }
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
        (f64::from(actual) - expected).abs() <= 0.00003 * (1.0 + expected.abs()),
        "{label}: {actual} vs {expected}"
    );
}
