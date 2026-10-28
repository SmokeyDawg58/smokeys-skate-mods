//! Original-owner grounded prefix through actual hooks and conditional reset.
//! Two deliberate component calls use root dt0 and retained dt1/8 over the SAME
//! solved bodies, current adjusted animation and retained Vault Reckoning. They
//! first stop before IK, then complete the native state2be IK-skip continuation.
//! A third call checks a late force failure, without another physics solve.
//! Source-form consumer arithmetic is independent of the production helpers;
//! typed Part oracles preserve actual W knowledge independently of XYZ checks.

use crate::physics_state_ids::{PHYSICS_STATE_LANDINGONDECK, PHYSICS_STATE_TELEPORTING};
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    animation::AnimationMatrix,
    dynamic_assembly_session::{
        skater_pool::{SimulationSkaterAssemblyHandle, SimulationSkeletonGroundedError},
        skeleton_normal::general_owner::{
            SkeletonGeneralContinuation, SkeletonGeneralInput, SkeletonGroundedPassedOutput,
            SkeletonGroundedUpdateError,
        },
    },
    reckoning::Reckoning,
    reckoning_attributes::ReckoningVaultAttributes,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::physical_grounded::PhysicalPlayerGroundedError,
    simulation_drive_pool::{
        SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
    },
    simulation_rigid_body_pool::SimulationRigidBodyPoolSnapshot,
    skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_drives::{SkeletonDriveType, update::SkeletonDrivesUpdateCvars},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, RwLock};

type Columns = [SkeletonRootVector; 4];
const PROCESSED_GRAVITY: f32 = 73.25;

#[derive(Default)]
struct DriveCvars {
    damping: f32,
    reads: usize,
    ik_reads: usize,
}

impl crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralCvars
    for DriveCvars
{
    fn ik_draw_feet_on_deck(&mut self) -> Result<u8, Self::Error> {
        self.ik_reads += 1;
        Ok(0) // Controlled cvar service, original default false.
    }
    fn ik_require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("draw disabled in this corpus")
    }
    fn ik_maximum_external_normal_angle(&mut self) -> Result<f32, Self::Error> {
        panic!("no external target producer in this corpus")
    }
    fn ik_target_normal_blend_speed(&mut self) -> Result<f32, Self::Error> {
        panic!("no external normal blend")
    }
    fn ik_maximum_local_delta(&mut self) -> Result<f32, Self::Error> {
        panic!("no external local target")
    }
    fn ik_maximum_world_offset(&mut self) -> Result<f32, Self::Error> {
        panic!("no external world target")
    }
}
impl SkeletonDrivesUpdateCvars for DriveCvars {
    type Error = &'static str;
    fn offboard_drive_scalar(&mut self) -> Result<f32, Self::Error> {
        panic!("actual retained proxy flag is false")
    }
    fn soft_damping(&mut self) -> Result<f32, Self::Error> {
        self.reads += 1;
        Ok(self.damping)
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "two original-owner transitions and independent consumer checks"
)]
pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    board: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    reckoning: &Reckoning<ReckoningVaultAttributes>,
) {
    crate::skeleton_ik_attributes::tests::check_original(&owner.inputs().vault);
    assert!(Arc::ptr_eq(
        world.physical_body_member(skeleton).unwrap().owner(),
        owner
    ));
    assert!(Arc::ptr_eq(
        world.physical_body_member(board).unwrap().owner(),
        owner
    ));
    let hook = world
        .skeleton(skeleton)
        .unwrap()
        .drive_owner()
        .unwrap()
        .hook()
        .unwrap()
        .clone();
    let board_matrix = world
        .board(board)
        .unwrap()
        .try_live_part_transform_typed(world.backend(), 6)
        .unwrap();
    let mapped = world
        .skeleton(skeleton)
        .unwrap()
        .animation_targets()
        .completed_volumes()
        .unwrap();
    let adjusted_hips = *world.skeleton(skeleton).unwrap().current_animation().hips();
    let adjusted_skate = world
        .skeleton(skeleton)
        .unwrap()
        .current_animation()
        .root_input(0.0)
        .unwrap()
        .skate_animation;
    let initial_poses = *world
        .skeleton(skeleton)
        .unwrap()
        .current_animation()
        .record()
        .poses();
    let initial_ik = *world
        .skeleton(skeleton)
        .unwrap()
        .current_animation()
        .ik()
        .targets();
    let reset_scalar = world
        .skeleton(skeleton)
        .unwrap()
        .current_animation()
        .record()
        .reset_scalar();
    let deck_zy = world
        .skeleton(skeleton)
        .unwrap()
        .current_animation()
        .last_deck_zy();
    let joints = world.backend().joint_snapshot();
    let drives = world.backend().drive_snapshot();
    let retained_drives = capture_drives(world, skeleton);
    let original_pool_payloads = world.backend().active_drive_payloads_for_test().unwrap();
    let mut retained_ik_data = None;
    for step in 0..2 {
        let old_hook = world.try_skeleton_hook_part_transform(&hook, 0).unwrap();
        let normal = world.skeleton(skeleton).unwrap();
        let old_position = normal.general_update().last_animation_world_position();
        let old_want_respawn = normal.general_update().want_respawn();
        let old_next_trajectory = *normal.general_update().next_trajectory_transform();
        let old_history = *normal.physics_history().record().transforms();
        let old_deltas = *normal.physics_history().record().velocity_deltas();
        let old_com = normal.physics_history().record().centre_of_mass();
        let old_com_velocity = normal.physics_history().record().centre_of_mass_velocity();
        let old_proxy_position = normal.physics_history().record().proxy_volume_position();
        let old_masses = *normal.physics_history().fractional_part_masses();
        let old_bodies = world.backend().rigid_body_snapshot();
        let mut output = SkeletonGroundedPassedOutput {
            orientation: SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
            flags_8c4: 0xffff_ffff,
        };
        let input = SkeletonGeneralInput {
            time_delta: 0.125,
            gravity: PROCESSED_GRAVITY,
            current_physics_state: if step == 0 { 1 } else { PHYSICS_STATE_TELEPORTING },
            flags_8c8: 0,
            flags_8d0: 0,
        };
        let mut drive_cvars = DriveCvars {
            damping: 0.625,
            reads: 0,
            ..DriveCvars::default()
        };
        let result = world
            .try_update_skeleton_grounded(
                skeleton,
                board,
                0.0,
                &input,
                &mut output,
                reckoning,
                &mut HostReckoningNumeric,
                &mut HostReckoningSinCosNumeric,
                &mut HostSkateboardSpawnMathNumeric,
                &mut drive_cvars,
            )
            .unwrap();
        let normal = world.skeleton(skeleton).unwrap();
        let roots = normal.root_transforms();
        let general = normal.general_update();
        let expected_hips = compose(roots.animation_to_world().columns(), &known(adjusted_hips));
        let delta: [f32; 3] = std::array::from_fn(|lane| {
            old_hook.columns()[3].xyz[lane] - expected_hips[3].xyz[lane]
        });
        let squared = (delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2];
        assert_eq!(result.reset_performed, squared > 1.0);
        assert!(
            !result.reset_performed,
            "the collision fixture already mapped and admitted these world-space hooks before solving"
        );
        assert_eq!(
            result.next,
            if step == 0 {
                SkeletonGeneralContinuation::UpdateIkContact
            } else {
                SkeletonGeneralContinuation::Complete
            }
        );
        assert_eq!(
            general.want_respawn(),
            old_want_respawn,
            "accepted hooks do not publish the failed-hook reset respawn request"
        );
        assert!(!general.add_skateboard_error());
        assert!(!general.teleporting());
        assert_eq!(output.flags_8c4, 0xfff7_ffff);
        let mut translation = known(FRESH_VOLUME_TRANSFORM);
        translation[3] = roots.predicted_board_position();
        assert_matrix(
            &output.orientation,
            &compose(
                &translation,
                &compose(roots.animation_to_board().columns(), &known(mapped[0])),
            ),
        );
        assert_matrix(general.physics_skateboard(), board_matrix.columns());
        assert_matrix(general.skate_root(), board_matrix.columns());
        assert_matrix(
            general.animation_board_to_physics_board(),
            &compose(
                roots.board_inverse().columns(),
                &compose(roots.animation_to_world().columns(), &known(adjusted_skate)),
            ),
        );
        let position = roots.animation_to_world().columns()[3];
        let seed = input.time_delta.recip();
        let first = seed.mul_add((-seed).mul_add(input.time_delta, 1.0), seed);
        let reciprocal = first.mul_add((-first).mul_add(input.time_delta, 1.0), first);
        let expected_velocity = SkeletonRootVector {
            xyz: std::array::from_fn(|lane| {
                reciprocal * (position.xyz[lane] - old_position.xyz[lane])
            }),
            w: position
                .w
                .zip(old_position.w)
                .map(|(current, old)| reciprocal * (current - old)),
        };
        assert_vector(general.animation_world_velocity(), expected_velocity);
        assert_vector(general.last_animation_world_position(), position);
        let live_hips = world.try_skeleton_hook_part_transform(&hook, 0).unwrap();
        let live_board_hook = world.try_skeleton_hook_part_transform(&hook, 1).unwrap();
        assert_matrix(
            &world
                .try_cached_skeleton_hook_part_transform(&hook, 0)
                .unwrap(),
            &expected_hips,
        );
        assert_matrix(
            &world
                .try_cached_skeleton_hook_part_transform(&hook, 1)
                .unwrap(),
            board_matrix.columns(),
        );
        assert!(
            expected_hips[3].w.is_none(),
            "unknown velocity W survives dt0 into actual requested hook cache"
        );
        for lane in 0..3 {
            assert!((live_hips.columns()[3].xyz[lane] - expected_hips[3].xyz[lane]).abs() < 0.001);
            assert!(
                (live_board_hook.columns()[3].xyz[lane] - board_matrix.columns()[3].xyz[lane])
                    .abs()
                    < 0.001
            );
        }
        if result.reset_performed {
            let bodies = world.backend().rigid_body_snapshot();
            for (part, mapped_row) in mapped.iter().enumerate() {
                let expected = compose(roots.animation_to_world().columns(), &known(*mapped_row));
                let cached = normal
                    .session()
                    .try_cached_part_transform_typed(part)
                    .unwrap();
                assert_matrix(&cached, &expected);
                let live = normal
                    .session()
                    .try_live_part_transform_host(world.backend(), part)
                    .unwrap();
                assert_eq!(
                    normal.physics_history().record().transforms()[part].map(f32::to_bits),
                    live.map(f32::to_bits)
                );
                assert_eq!(
                    normal.physics_history().record().vector_typed(
                        crate::skeleton_physics_history::lane_knowledge::RecordVectorField::Position(part)),
                    normal.session().try_live_part_transform_typed(world.backend(), part)
                        .unwrap().columns()[3]
                );
                assert_eq!(
                    normal.physics_history().record().velocities()[part].map(f32::to_bits),
                    [0; 4]
                );
                let observation = normal
                    .session()
                    .try_part_contact_spy_observation(world.backend(), part)
                    .unwrap();
                let index = usize::try_from(observation.identity().id().index()).unwrap();
                let body = &bodies.slots[index];
                let before = &old_bodies.slots[index];
                assert_eq!(body.velocity.map(f32::to_bits), [0; 3]);
                assert_eq!(body.omega.map(f32::to_bits), [0; 3]);
                assert_eq!(
                    body.force.map(f32::to_bits),
                    bodies.gravity.map(f32::to_bits)
                );
                assert_eq!(body.torque.map(f32::to_bits), [0; 3]);
                assert_eq!(
                    (
                        body.occurrence,
                        body.right_link,
                        body.left_link,
                        body.stasis,
                        body.cooldown
                    ),
                    (
                        before.occurrence,
                        before.right_link,
                        before.left_link,
                        before.stasis,
                        before.cooldown
                    )
                );
                assert_eq!(
                    body.kinematic_value.to_bits(),
                    before.kinematic_value.to_bits()
                );
            }
            assert_eq!(
                normal
                    .state()
                    .proxy_volume_target
                    .unwrap()
                    .known_lanes()
                    .expect("known controlled proxy lanes")
                    .map(f32::to_bits),
                [0; 4]
            );
            assert_eq!(
                normal
                    .current_animation()
                    .record()
                    .centre_of_mass()
                    .map(f32::to_bits),
                [0; 4]
            );
        } else {
            if result.next == SkeletonGeneralContinuation::Complete {
                assert_force_prefix(world, skeleton, &old_bodies, 24, input.gravity);
                assert_matrix(general.next_trajectory_transform(), &trajectory_identity());
                check_drives(
                    world,
                    skeleton,
                    owner,
                    &mapped,
                    &retained_drives,
                    &drive_cvars,
                );
            } else {
                // Actual hooks/IK may change their own retained state, but this
                // continuation returns before Part force/drive completion.
                let bodies = world.backend().rigid_body_snapshot();
                for part in 0..24 {
                    let observation = normal
                        .session()
                        .try_part_contact_spy_observation(world.backend(), part)
                        .unwrap();
                    let index = usize::try_from(observation.identity().id().index()).unwrap();
                    assert_eq!(bodies.slots[index], old_bodies.slots[index]);
                }
                assert_matrix(
                    general.next_trajectory_transform(),
                    old_next_trajectory.columns(),
                );
            }
            assert_eq!(
                normal
                    .physics_history()
                    .record()
                    .transforms()
                    .map(|m| m.map(f32::to_bits)),
                old_history.map(|m| m.map(f32::to_bits))
            );
            assert_vector(
                normal.state().proxy_volume_target.unwrap(),
                board_matrix.columns()[3],
            );
        }
        if step == 0 {
            check_initial_blended_world(normal, &owner.inputs().vault);
            assert_eq!(
                drive_cvars.ik_reads, 5,
                "one initial and four source common-stage draw reads"
            );
            let runtime = normal.current_animation().ik_runtime().unwrap();
            let data = runtime.data_in();
            assert_matrix(
                &data.animation_to_world,
                roots.animation_to_world().columns(),
            );
            assert_matrix(
                &data.world_to_animation,
                roots.world_to_animation().columns(),
            );
            assert_matrix(
                &data.physics_skateboard,
                general.physics_skateboard().columns(),
            );
            assert_matrix(&data.inverse_skateboard, roots.board_inverse().columns());
            assert_matrix(&data.hips, &known(adjusted_hips));
            assert_matrix(&data.skateboard, roots.board().columns());
            let actual_normal = normal.state().collision.most_vertical_toe_contact_normal;
            assert_vector(
                data.toe_contact_normal,
                SkeletonRootVector {
                    xyz: *actual_normal.xyz(),
                    w: actual_normal.known_lanes().map(|n| n[3]),
                },
            );
            assert_vector(data.hips_offset, SkeletonRootVector::known([0.0; 4]));
            assert_eq!(runtime.state().modes(), &[1, 1, 0, 0]);
            let tuning = owner
                .inputs()
                .vault
                .instance(0xc8be_3349_f2bd_493e, 0xd7ed_bd36_2d7d_2152);
            let step = f32::from_bits(
                tuning
                    .try_scalar_word(0x39fa_8c07_e835_6284, 0)
                    .unwrap()
                    .unwrap(),
            );
            let expected = if 1.0 - step >= 0.0 { step } else { 1.0 };
            assert_eq!(
                runtime.state().on_deck_blends().map(f32::to_bits),
                [expected.to_bits(), expected.to_bits(), 0, 0]
            );
            retained_ik_data = Some([
                data.animation_to_world,
                data.world_to_animation,
                data.physics_skateboard,
                data.inverse_skateboard,
                data.hips,
                data.skateboard,
            ]);
            assert_eq!(drive_cvars.reads, 0, "IK stop precedes drive services");
            assert_eq!(
                world.backend().active_drive_payloads_for_test().unwrap(),
                original_pool_payloads
            );
        } else {
            assert_eq!(drive_cvars.ik_reads, 0, "state2be does not enter IK");
            let data = normal.current_animation().ik_runtime().unwrap().data_in();
            for (actual, old) in [
                data.animation_to_world,
                data.world_to_animation,
                data.physics_skateboard,
                data.inverse_skateboard,
                data.hips,
                data.skateboard,
            ]
            .iter()
            .zip(retained_ik_data.unwrap())
            {
                assert_matrix(actual, old.columns());
            }
        }
        assert!(
            normal.current_animation().root_input(0.0).is_some(),
            "retained drive continuation never invalidates/remaps current animation"
        );
        assert_eq!(
            normal
                .animation_targets()
                .completed_volumes()
                .unwrap()
                .map(|m| m.map(f32::to_bits)),
            mapped.map(|m| m.map(f32::to_bits))
        );
        assert_eq!(
            normal
                .physics_history()
                .record()
                .velocity_deltas()
                .map(|v| v.map(f32::to_bits)),
            old_deltas.map(|v| v.map(f32::to_bits))
        );
        assert_eq!(
            normal
                .physics_history()
                .record()
                .centre_of_mass()
                .map(f32::to_bits),
            old_com.map(f32::to_bits)
        );
        assert_eq!(
            normal
                .physics_history()
                .record()
                .centre_of_mass_velocity()
                .map(f32::to_bits),
            old_com_velocity.map(f32::to_bits)
        );
        assert_eq!(
            normal
                .physics_history()
                .record()
                .proxy_volume_position()
                .map(f32::to_bits),
            old_proxy_position.map(f32::to_bits)
        );
        assert_eq!(
            normal
                .physics_history()
                .fractional_part_masses()
                .map(f32::to_bits),
            old_masses.map(f32::to_bits)
        );
        assert_eq!(
            normal
                .current_animation()
                .record()
                .poses()
                .map(|m| m.map(f32::to_bits)),
            initial_poses.map(|m| m.map(f32::to_bits))
        );
        assert_eq!(
            normal
                .current_animation()
                .ik()
                .targets()
                .map(|m| m.map(f32::to_bits)),
            initial_ik.map(|m| m.map(f32::to_bits))
        );
        assert_eq!(
            normal.current_animation().record().reset_scalar().to_bits(),
            reset_scalar.to_bits()
        );
        assert_eq!(
            normal.current_animation().last_deck_zy().to_bits(),
            deck_zy.to_bits()
        );
    }
    assert_eq!(world.backend().joint_snapshot(), joints);
    assert_eq!(world.backend().drive_snapshot(), drives);
    let payloads = world.backend().active_drive_payloads_for_test().unwrap();
    assert_eq!(
        &payloads[..3],
        &original_pool_payloads[..3],
        "board drive payloads survive"
    );
    check_late_force_failure(world, skeleton, board, reckoning);
    check_landing_ik_data(world, skeleton, board, reckoning);
    assert_matrix(
        &world
            .board(board)
            .unwrap()
            .try_live_part_transform_typed(world.backend(), 6)
            .unwrap(),
        board_matrix.columns(),
    );
}

struct RetainedDrive {
    part: usize,
    parent: usize,
    kind: SkeletonDriveType,
    frames: Arc<RwLock<SimulationDriveFrames>>,
    dynamics: Arc<RwLock<SimulationDriveDynamics>>,
    previous_frames: SimulationDriveFrames,
}

fn capture_drives(
    world: &PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Vec<RetainedDrive> {
    world
        .skeleton(skeleton)
        .unwrap()
        .drive_owner()
        .unwrap()
        .prepared()
        .eligible_bone_drives()
        .map(|drive| RetainedDrive {
            part: drive.part.index(),
            parent: drive.parent.index(),
            kind: drive.kind,
            frames: Arc::clone(drive.frames),
            dynamics: Arc::clone(drive.dynamics),
            previous_frames: *drive.frames.read().unwrap(),
        })
        .collect()
}

fn check_drives(
    world: &PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    original: &PhysicalPlayerAssemblyOwner,
    mapped: &[AnimationMatrix; 24],
    retained: &[RetainedDrive],
    cvars: &DriveCvars,
) {
    let owner = world.skeleton(skeleton).unwrap();
    let runtime = owner.drive_owner().unwrap();
    assert!(!owner.post_physics_records().proxy_enabled());
    assert_eq!(retained.len(), 44);
    let response = owner.state().collision.drive_strength;
    assert!(
        response < 1.0,
        "actual retained terrain collision selects soft drives"
    );
    assert_eq!(cvars.reads, 44);
    let vault = &original.inputs().vault;
    let animation = vault.instance(0xbabb_3ec1_1084_9e08, 0xd7ed_bd36_2d7d_2152);
    let scales = [0x0a58_9569_f5fa_0e6c, 0x13d1_a5a3_e27d_830e]
        .map(|key| f32::from_bits(animation.try_scalar_word(key, 0).unwrap().unwrap()));
    let tuning = vault.instance(0xfdf6_1860_0d39_6c3b, 0xd7ed_bd36_2d7d_2152);
    let soft_displacement = f32::from_bits(
        tuning
            .try_scalar_word(0xe1a7_f2dc_fab0_b8d0, 0)
            .unwrap()
            .unwrap(),
    );
    let soft_strength = f32::from_bits(
        tuning
            .try_scalar_word(0xf300_ffd5_99e1_709e, 0)
            .unwrap()
            .unwrap(),
    );
    let actual_payloads = world.backend().active_drive_payloads_for_test().unwrap();
    assert_eq!(actual_payloads.len(), 49);
    let mut changed = 0;
    for (ordinal, drive) in retained.iter().enumerate() {
        let part = crate::ragdoll::SkeletonPartIndex::from_index(drive.part).unwrap();
        let bone = runtime.prepared().bone(part);
        assert!(Arc::ptr_eq(&drive.frames, bone.frames(drive.kind)));
        assert!(Arc::ptr_eq(&drive.dynamics, bone.dynamics(drive.kind)));
        let frames = *drive.frames.read().unwrap();
        let dynamics = *drive.dynamics.read().unwrap();
        assert_eq!(actual_payloads[ordinal + 5], (frames, dynamics));
        changed += usize::from(frames != drive.previous_frames);
        let mut anchor = known(FRESH_VOLUME_TRANSFORM);
        anchor[3] = known(mapped[drive.part])[3];
        let expected_a = compose(&inverse(mapped[drive.part]), &anchor)[3];
        let expected_b = compose(&inverse(mapped[drive.parent]), &anchor)[3];
        assert_eq!(
            frames.position_a.map(f32::to_bits),
            [
                expected_a.xyz[0],
                expected_a.xyz[1],
                expected_a.xyz[2],
                expected_a.w.unwrap()
            ]
            .map(f32::to_bits)
        );
        assert_eq!(
            frames.position_b.map(f32::to_bits),
            [
                expected_b.xyz[0],
                expected_b.xyz[1],
                expected_b.xyz[2],
                expected_b.w.unwrap()
            ]
            .map(f32::to_bits)
        );
        let coefficients = owner.state().binding.body().attributes.drives[drive.part];
        let coefficient = if drive.kind == SkeletonDriveType::Local {
            coefficients.collision_strength
        } else {
            coefficients.collision_strength_root
        };
        let strength =
            (1.0 - coefficient).mul_add(response, coefficient) * scales[drive.kind.index()];
        assert_eq!(bone.flags(), 3);
        assert_eq!(
            bone.strengths()[drive.kind.index()].to_bits(),
            strength.to_bits()
        );
        assert_eq!(params(dynamics.linear), [0, 0, 0, 1]);
        assert_eq!(
            params(dynamics.angular),
            [
                (soft_displacement * strength).to_bits(),
                cvars.damping.to_bits(),
                ((soft_strength * strength) * f32::from_bits(0x4560_fffe)).to_bits(),
                1
            ]
        );
    }
    assert!(
        changed > 0,
        "current retained rows replace earlier controlled feedback targets"
    );
    let hook = runtime.prepared().hook_dynamics(0).read().unwrap();
    assert_eq!(params(hook.linear), [0x4415_ffff, 0, 0x470c_9fff, 2]);
}

fn params(value: SimulationDriveDynamicsParams) -> [u32; 4] {
    [
        value.spring.to_bits(),
        value.damping.to_bits(),
        value.strength.to_bits(),
        value.raw_type,
    ]
}

fn inverse(matrix: AnimationMatrix) -> Columns {
    let mut result = known(FRESH_VOLUME_TRANSFORM);
    for (column, value) in result.iter_mut().take(3).enumerate() {
        *value = SkeletonRootVector {
            xyz: [matrix[column], matrix[4 + column], matrix[8 + column]],
            w: Some(0.0),
        };
    }
    let lanes: [f32; 4] = std::array::from_fn(|lane| {
        let coordinate = |column: usize| {
            if lane == 3 {
                0.0
            } else {
                result[column].xyz[lane]
            }
        };
        let z = (0.0 - matrix[14]) * coordinate(2);
        let y = (0.0 - matrix[13]).mul_add(coordinate(1), z);
        (0.0 - matrix[12]).mul_add(coordinate(0), y)
    });
    result[3] = SkeletonRootVector {
        xyz: [lanes[0], lanes[1], lanes[2]],
        w: Some(lanes[3]),
    };
    result
}

fn trajectory_identity() -> Columns {
    let mut result = known(FRESH_VOLUME_TRANSFORM);
    result[3] = SkeletonRootVector {
        xyz: [0.0; 3],
        w: Some(0.0),
    };
    result
}

fn assert_force_prefix(
    world: &PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    before: &SimulationRigidBodyPoolSnapshot,
    completed: usize,
    gravity: f32,
) {
    let t = f32::from_bits(0x3c88_8889);
    let displacement = ((-gravity * 0.5) * t) * t;
    let seed = 1.0 / t;
    let first = seed.mul_add((-seed).mul_add(t, 1.0), seed);
    let reciprocal = first.mul_add((-first).mul_add(t, 1.0), first);
    let mut expected = before.clone();
    let owner = world.skeleton(skeleton).unwrap();
    assert_ne!(gravity.to_bits(), before.gravity[1].to_bits());
    assert_ne!(gravity.to_bits(), (-before.gravity[1]).to_bits());
    for part in 0..completed {
        let observation = owner
            .session()
            .try_part_contact_spy_observation(world.backend(), part)
            .unwrap();
        let index = usize::try_from(observation.identity().id().index()).unwrap();
        let body = &mut expected.slots[index];
        let delta = [0.0, displacement, 0.0].map(|value| (reciprocal * value) * body.inverse_mass);
        for (force, delta) in body.force.iter_mut().zip(delta) {
            *force += delta;
        }
        body.cooldown = 0;
    }
    assert_eq!(
        world.backend().rigid_body_snapshot(),
        expected,
        "only completed actual Part force/cooldown writes; all other body fields and pool identities persist"
    );
}

struct FailForceNumeric {
    fixed_calls: usize,
}
impl ReckoningNumeric for FailForceNumeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        HostReckoningNumeric.dot3(a, b).map_err(|_| "host dot")
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric
            .reciprocal_sqrt_estimate(value)
            .map_err(|_| "host rsqrt")
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric
            .truncate_i32(value)
            .map_err(|_| "host conversion")
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        if value.to_bits() == 0x3c88_8889 {
            self.fixed_calls += 1;
            if self.fixed_calls == 4 {
                return Err("force Part3");
            }
        }
        HostReckoningNumeric
            .reciprocal_estimate(value)
            .map_err(|_| "host reciprocal")
    }
}

fn check_late_force_failure(
    world: &mut PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    board: &SimulationSkaterAssemblyHandle,
    reckoning: &Reckoning<ReckoningVaultAttributes>,
) {
    let before = world.backend().rigid_body_snapshot();
    let history = *world
        .skeleton(skeleton)
        .unwrap()
        .physics_history()
        .record()
        .transforms();
    let mut numeric = FailForceNumeric { fixed_calls: 0 };
    let mut cvars = DriveCvars {
        damping: 0.375,
        reads: 0,
        ..DriveCvars::default()
    };
    let mut output = SkeletonGroundedPassedOutput {
        orientation: SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
        flags_8c4: 0xffff_ffff,
    };
    let result = world.try_update_skeleton_grounded(
        skeleton,
        board,
        0.0,
        &SkeletonGeneralInput {
            time_delta: 0.125,
            gravity: PROCESSED_GRAVITY,
            current_physics_state: PHYSICS_STATE_TELEPORTING,
            flags_8c8: 0,
            flags_8d0: 0,
        },
        &mut output,
        reckoning,
        &mut numeric,
        &mut HostReckoningSinCosNumeric,
        &mut HostSkateboardSpawnMathNumeric,
        &mut cvars,
    );
    assert!(matches!(
        result,
        Err(PhysicalPlayerGroundedError::Update(
            SimulationSkeletonGroundedError::Update(SkeletonGroundedUpdateError::Displacement {
                part: 3,
                ..
            })
        ))
    ));
    assert_eq!(numeric.fixed_calls, 4);
    assert_eq!(
        cvars.reads, 44,
        "all drive updates precede force suffix failure"
    );
    assert_eq!(output.flags_8c4, 0xfff7_ffff);
    assert_force_prefix(world, skeleton, &before, 3, PROCESSED_GRAVITY);
    let owner = world.skeleton(skeleton).unwrap();
    assert_eq!(
        owner
            .physics_history()
            .record()
            .transforms()
            .map(|m| m.map(f32::to_bits)),
        history.map(|m| m.map(f32::to_bits))
    );
    for drive in owner
        .drive_owner()
        .unwrap()
        .prepared()
        .eligible_bone_drives()
    {
        assert_eq!(
            drive.dynamics.read().unwrap().angular.damping.to_bits(),
            0.375_f32.to_bits()
        );
    }
    assert!(owner.current_animation().root_input(0.0).is_some());
}

#[expect(
    clippy::too_many_lines,
    reason = "one actual landing branch with independent producer and preserved-owner checks"
)]
fn check_landing_ik_data(
    world: &mut PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    board: &SimulationSkaterAssemblyHandle,
    reckoning: &Reckoning<ReckoningVaultAttributes>,
) {
    let normal = world.skeleton(skeleton).unwrap();
    let previous_deck = normal.current_animation().record().poses()[0];
    let mapped = normal
        .animation_targets()
        .completed_volumes()
        .unwrap()
        .map(|m| m.map(f32::to_bits));
    let raw = normal
        .animation_targets()
        .animation_rows()
        .map(|m| m.unwrap().map(f32::to_bits));
    let body_before = world.backend().rigid_body_snapshot();
    let payloads = world.backend().active_drive_payloads_for_test().unwrap();
    let mut output = SkeletonGroundedPassedOutput {
        orientation: SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
        flags_8c4: 0,
    };
    let mut cvars = DriveCvars {
        damping: 0.875,
        reads: 0,
        ..DriveCvars::default()
    };
    let progress = world
        .try_update_skeleton_grounded(
            skeleton,
            board,
            0.0,
            &SkeletonGeneralInput {
                time_delta: 0.125,
                gravity: PROCESSED_GRAVITY,
                current_physics_state: PHYSICS_STATE_LANDINGONDECK,
                flags_8c8: 0,
                flags_8d0: 1 << 28,
            },
            &mut output,
            reckoning,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
            &mut HostSkateboardSpawnMathNumeric,
            &mut cvars,
        )
        .unwrap();
    assert!(!progress.reset_performed);
    assert_eq!(progress.next, SkeletonGeneralContinuation::UpdateIkContact);
    assert_eq!(cvars.reads, 0);
    assert_eq!(cvars.ik_reads, 5);
    let normal = world.skeleton(skeleton).unwrap();
    check_initial_blended_world(
        normal,
        &world
            .physical_body_member(skeleton)
            .unwrap()
            .owner()
            .inputs()
            .vault,
    );
    let runtime = normal.current_animation().ik_runtime().unwrap();
    let data = runtime.data_in();
    let expected = compose(
        normal.root_transforms().animation_to_world().columns(),
        &known(previous_deck),
    );
    assert_matrix(&data.skateboard, &expected);
    assert_matrix(&data.physics_skateboard, &expected);
    assert!(
        expected[3].w.is_none(),
        "actual packed velocity W remains unknown"
    );
    assert_ne!(
        data.physics_skateboard,
        *normal.general_update().physics_skateboard(),
        "landing branch must not reuse the physical deck copy"
    );
    // Inverse consumes onlyXYZ and reconstructsW; the sentinel fourth lanes
    // cannot affect this independent scalar inverse oracle.
    let scratch = std::array::from_fn(|i| {
        if i % 4 == 3 {
            71.0
        } else {
            expected[i / 4].xyz[i % 4]
        }
    });
    assert_matrix(&data.inverse_skateboard, &inverse(scratch));
    assert_eq!(
        runtime.state().on_deck_blends().map(f32::to_bits),
        [0; 4],
        "source bit28 decays previously retained foot blends"
    );
    assert_eq!(
        normal
            .animation_targets()
            .completed_volumes()
            .unwrap()
            .map(|m| m.map(f32::to_bits)),
        mapped
    );
    assert_eq!(
        normal
            .animation_targets()
            .animation_rows()
            .map(|m| m.unwrap().map(f32::to_bits)),
        raw
    );
    assert_eq!(
        world.backend().rigid_body_snapshot(),
        body_before,
        "no force/solve follows unfinished active IK"
    );
    assert_eq!(
        world.backend().active_drive_payloads_for_test().unwrap(),
        payloads
    );
}

fn check_initial_blended_world(
    normal: &crate::dynamic_assembly_session::skeleton_normal::SkeletonNormalAssembly,
    vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
) {
    let animation = normal.current_animation();
    let runtime = animation.ik_runtime().unwrap();
    let data = runtime.data_in();
    let state = runtime.state();
    let previous = animation.record().poses();
    let old_inverse = inverse(previous[0]);
    let original = vault.instance(0xc8be_3349_f2bd_493e, 0xd7ed_bd36_2d7d_2152);
    let extent_data = original
        .try_get_data(0x121e_67e2_b409_6a41, 0)
        .unwrap()
        .unwrap();
    let extent: [f32; 4] = std::array::from_fn(|lane| {
        f32::from_be_bytes(
            extent_data.bytes()[4 * lane..4 * lane + 4]
                .try_into()
                .unwrap(),
        )
    });
    let mut hybrid_board = *data.physics_skateboard.columns();
    let mut previous_position = known(FRESH_VOLUME_TRANSFORM);
    previous_position[3] = known(previous[0])[3];
    hybrid_board[3] = compose(data.animation_to_world.columns(), &previous_position)[3];
    for entry in 0..4 {
        let part = animation.ik().parts()[entry].unwrap().index();
        let deck = compose(&old_inverse, &known(animation.ik().targets()[entry]));
        let initial_world = compose(data.animation_to_world.columns(), &known(previous[part]));
        assert_matrix(&state.part_to_deck()[entry], &deck);
        assert_vector(state.part_positions()[entry], deck[3]);
        let outside = (0..3)
            .any(|lane| -extent[lane] > deck[3].xyz[lane] || deck[3].xyz[lane] > extent[lane]);
        assert_eq!(state.parts_on_deck()[entry], u32::from(!outside));
        let blend = state.on_deck_blends()[entry];
        if state.modes()[entry] == 1 && blend > 0.0 {
            assert_vector(
                state.part_to_world()[entry].columns()[3],
                base_position(initial_world[3], compose(&hybrid_board, &deck)[3], blend),
            );
        } else {
            assert_matrix(&state.part_to_world()[entry], &initial_world);
        }
        if let Some(parent) = animation.ik().parents()[entry] {
            let relation = compose(&inverse(previous[part]), &known(previous[parent.index()]));
            let parent_deck = compose(&deck, &relation);
            let parent_world = compose(&initial_world, &relation);
            assert_matrix(&state.parent_to_deck()[entry], &parent_deck);
            if state.modes()[entry] == 1 && blend > 0.0 {
                assert_vector(
                    state.parent_to_world()[entry].columns()[3],
                    base_position(
                        parent_world[3],
                        compose(&hybrid_board, &parent_deck)[3],
                        blend,
                    ),
                );
            } else {
                assert_matrix(&state.parent_to_world()[entry], &parent_world);
            }
        }
    }
}

fn base_position(
    old: SkeletonRootVector,
    candidate: SkeletonRootVector,
    blend: f32,
) -> SkeletonRootVector {
    let complement = 1.0 - blend;
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| {
            candidate.xyz[lane].mul_add(blend, old.xyz[lane] * complement)
        }),
        w: candidate
            .w
            .zip(old.w)
            .map(|(candidate, old)| candidate.mul_add(blend, old * complement)),
    }
}

fn known(matrix: AnimationMatrix) -> Columns {
    std::array::from_fn(|column| SkeletonRootVector {
        xyz: std::array::from_fn(|lane| matrix[column * 4 + lane]),
        w: Some(matrix[column * 4 + 3]),
    })
}

fn compose(outer: &Columns, inner: &Columns) -> Columns {
    std::array::from_fn(|column| {
        let coefficients = inner[column].xyz;
        let coordinate = |lane: usize| {
            let first = if column == 3 {
                coefficients[0].mul_add(outer[0].xyz[lane], outer[3].xyz[lane])
            } else {
                coefficients[0] * outer[0].xyz[lane]
            };
            let second = coefficients[1].mul_add(outer[1].xyz[lane], first);
            coefficients[2].mul_add(outer[2].xyz[lane], second)
        };
        let w = outer[0]
            .w
            .zip(outer[1].w)
            .zip(outer[2].w)
            .and_then(|((x, y), z)| {
                let first = if column == 3 {
                    coefficients[0].mul_add(x, outer[3].w?)
                } else {
                    coefficients[0] * x
                };
                Some(coefficients[2].mul_add(z, coefficients[1].mul_add(y, first)))
            });
        SkeletonRootVector {
            xyz: std::array::from_fn(coordinate),
            w,
        }
    })
}

fn assert_vector(actual: SkeletonRootVector, expected: SkeletonRootVector) {
    assert_eq!(actual.xyz.map(f32::to_bits), expected.xyz.map(f32::to_bits));
    assert_eq!(actual.w.map(f32::to_bits), expected.w.map(f32::to_bits));
}

fn assert_matrix(actual: &SkeletonRootMatrix, expected: &Columns) {
    for (&actual, &expected) in actual.columns().iter().zip(expected) {
        assert_vector(actual, expected);
    }
}
