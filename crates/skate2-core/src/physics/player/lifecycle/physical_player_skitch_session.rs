//! Live Skitch104 Enter/Update82d94678/82da10a0; uses the retained state/pair.
use super::ground_pre_solve::HostGroundPreparationNumeric;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle as Handle,
    input_cvar_runtime::reckoning::ReckoningCvarAdapter,
    input_cvar_runtime::{
        InputCvarReferences,
        skitch::{SkitchBool as B, SkitchFloat as F, SkitchInt as I},
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{
            GrabDescriptor, Skitching, StateKind, StatePayload,
            skitch_ik::SkitchIkInput,
            skitch_state::{SkitchControlCvars, SkitchExitForce},
        },
    },
    physical_player_grab_spline::manager::GrabMessageKind,
    physical_player_grind_manager::{
        math::GrindNum,
        view::{GrindError as E, error, lanes, need, root},
    },
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

pub(crate) fn state<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
    f: impl FnOnce(&mut Skitching) -> Result<T, E>,
) -> Result<T, E> {
    let mut states = owner.remaining_states().lock().map_err(error)?;
    let s = need(
        states
            .states
            .get_mut(&StateKind::Skitching)
            .filter(|s| s.published)
            .and_then(|s| s.value.as_mut()),
        "retained Skitch104",
    )?;
    let StatePayload::Skitching(s) = s else {
        return Err(error("Skitch payload"));
    };
    let b = &s.base.physical;
    if !b.board.same_as(board)
        || !b.skeleton.same_as(skeleton)
        || !b.owner.upgrade().is_some_and(|v| Arc::ptr_eq(&v, owner))
    {
        return Err(error("foreign Skitch pair"));
    }
    f(s)
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_enter_pair_skitch(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<(), E> {
        state(owner, board, skeleton, |s| {
            s.try_enter(|| self.try_disable_board_animation(board))
                .map_err(error)
        })
    }
    pub(crate) fn try_update_pair_skitch(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, E> {
        let p = Arc::clone(owner.reckoning().lock().map_err(error)?.processed());
        let valid = need(
            p.lock().map_err(error)?.physics_state_fields().flags_8d0,
            "skitch ready flags",
        )? & 0x0100_0000
            != 0;
        state(owner, board, skeleton, |s| {
            s.grab_spline_valid = Some(valid);
            Ok(())
        })?;
        let frames = refs
            .try_skitch_int(I::kNumberOfFramesToDisableHandsAfterSkitch)
            .map_err(error)?;
        need(self.skaters.as_mut(), "skater pool")?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.disable_hand_collisions(frames)
            })
            .map_err(error)?;
        let controls = SkitchControlCvars {
            min_turn_for_exit: refs
                .try_skitch_float(F::kSkitchMinTurnForExit)
                .map_err(error)?,
            stick_hand_slow_down: refs
                .try_skitch_float(F::kStickHandSlowDownEffect)
                .map_err(error)?,
            force_stick: refs.try_skitch_bool(B::kForceSkitchStick).map_err(error)?,
            frames_to_exit: refs
                .try_skitch_int(I::kFramesToExitSkitching)
                .map_err(error)?,
        };
        if valid {
            state(owner, board, skeleton, |s| {
                s.grab_spline = Some(
                    need(
                        p.lock().map_err(error)?.grab_spline(false),
                        "primary spline",
                    )?
                    .clone(),
                );
                Ok(())
            })?;
            let body = need(
                self.board_construction(board).map_err(error)?,
                "board construction",
            )?;
            let height = need(
                body.wheel_lines().lock().map_err(error)?.height,
                "wheel height",
            )?;
            let time = need(body.post_fields(), "board post fields")?.time_since_wheels_touching;
            let (deck, turn, spin, normal, mirrored, y, z) = {
                let p = p.lock().map_err(error)?;
                let a = p.animation_attribute_fields();
                (
                    need(p.deck_transform(), "deck transform")?
                        .columns()
                        .map(lanes),
                    need(a.turn, "turn")?,
                    need(p.jump_events().spin(), "spin")?,
                    lanes(need(p.prior_output().ground_normal(), "ground normal")?.value),
                    need(p.physics_state_fields().flags_8cc, "stance")? & 8 != 0,
                    need(a.world_grab_y, "world grab Y")?,
                    need(a.world_grab_z, "world grab Z")?,
                )
            };
            let authored = refs
                .try_use_authored_vehicle_grab_splines()
                .map_err(error)?;
            state(owner, board, skeleton, |s| {
                s.update_local_physical(height, time).map_err(error)?;
                s.update_local_control(turn, spin);
                s.ground_normal = Some(
                    if refs
                        .try_skitch_bool(B::kSkitchingUseBetterSteerForceNormals)
                        .map_err(error)?
                    {
                        [0., 1., 0., 0.]
                    } else {
                        normal
                    },
                );
                let mut n = GrindNum {
                    n: &mut numeric.reckoning,
                    s: &mut numeric.sin_cos,
                };
                s.update_grab_geometry(deck, authored, &mut n)
                    .map_err(error)?;
                s.check_state_change(&controls).map_err(error)?;
                s.update_follow_point(
                    deck[2],
                    refs.try_skitch_bool(B::kEnableCentrifugalAcceleration)
                        .map_err(error)?,
                    refs.try_skitch_bool(B::kEnableFakeFollowPointPhysics)
                        .map_err(error)?,
                    &mut n,
                )
                .map_err(error)?;
                s.update_skitch_state(&controls).map_err(error)
            })?;
            use crate::ragdoll::SkeletonPartIndex as Part;
            need(self.skaters.as_mut(), "skater pool")?
                .try_with_jump_pair(&self.backend, board, skeleton, |normal, actual| {
                    if !Arc::ptr_eq(owner, actual) {
                        return Err(error("foreign Skitch skeleton"));
                    }
                    let bone = |part: Part| -> Result<[f32; 4], E> {
                        let row = need(
                            normal.animation_targets().animation_row(part as usize),
                            "Skitch raw bone",
                        )?;
                        Ok([row[12], row[13], row[14], row[15]])
                    };
                    let input = SkitchIkInput {
                        anim_to_world: normal
                            .root_transforms()
                            .animation_to_world()
                            .columns()
                            .map(lanes),
                        left_hand: bone(Part::LeftHand)?,
                        right_hand: bone(Part::RightHand)?,
                        left_arm: bone(Part::LeftArm)?,
                        right_arm: bone(Part::RightArm)?,
                        world_grab_y: y,
                        world_grab_z: z,
                        mirrored,
                        authored,
                        force_left: refs
                            .try_skitch_bool(B::kSkitchForceLeftHand)
                            .map_err(error)?,
                        force_right: refs
                            .try_skitch_bool(B::kSkitchForceRightHand)
                            .map_err(error)?,
                        crouch_spring: refs
                            .try_skitch_float(F::kSkitchCrouchSpringScalar)
                            .map_err(error)?,
                    };
                    let targets = state(owner, board, skeleton, |s| {
                        s.update_hand_ik(
                            input,
                            &mut GrindNum {
                                n: &mut numeric.reckoning,
                                s: &mut numeric.sin_cos,
                            },
                        )
                        .map_err(error)
                    })?;
                    for (index, target) in targets.into_iter().enumerate() {
                        if let Some(target) = target {
                            if refs.try_skitch_bool(B::kSkitchEnableIK).map_err(error)? {
                                normal
                                    .try_set_world_hand_ik_target(
                                        index == 0,
                                        root(target.position),
                                        target.maximum,
                                        target.blend,
                                        &mut numeric.reckoning,
                                    )
                                    .map_err(error)?;
                            }
                        }
                    }
                    Ok::<_, E>(())
                })
                .map_err(error)??;
            self.try_update_skitch_board(owner, board, skeleton, refs, numeric)?;
            let offset = state(owner, board, skeleton, |s| {
                s.deck_animation_offset(
                    refs.try_skitch_float(F::kDeckBlendAmount).map_err(error)?,
                    refs.try_skitch_float(F::kDeckMaxOffsetDelta)
                        .map_err(error)?,
                    refs.try_skitch_float(F::kMinDeckOffsetAngle)
                        .map_err(error)?,
                    &mut GrindNum {
                        n: &mut numeric.reckoning,
                        s: &mut numeric.sin_cos,
                    },
                )
                .map_err(error)
            })?;
            if let Some(offset) = offset {
                let flat = std::array::from_fn(|i| offset[i / 4][i % 4]);
                self.try_set_skeleton_skateboard_offset(
                    skeleton,
                    &flat,
                    &mut SkitchOffsetCvars(refs),
                )
                .map_err(error)?;
            }
            let descriptor = state(owner, board, skeleton, |s| {
                let spline = need(s.grab_spline.as_ref(), "Skitch spline")?;
                Ok(GrabDescriptor {
                    object_type: spline.descriptor_type,
                    object_id: spline.descriptor_object,
                })
            })?;
            let mut grab = owner.grab_spline().lock().map_err(error)?;
            let grab = need(grab.as_mut(), "PlayerGrabSpline")?;
            grab.request_spline_data(descriptor.object_type, descriptor.object_id)?;
            grab.defer_message(descriptor, GrabMessageKind::NotifyGrabbing)?;
            if need(p.lock().map_err(error)?.known_flags_8c8(), "locality flags")? & 0x4000_0000
                == 0
            {
                grab.defer_message(descriptor, GrabMessageKind::NotifyLocalPlayer)?;
            }
        }
        // CheckForExit tests the FIRST byte8cc bit0, distinct from selection's bit23.
        let requested = need(
            p.lock().map_err(error)?.physics_state_fields().flags_8cc,
            "grab input",
        )? & 0x0100_0000
            != 0;
        let exit = state(owner, board, skeleton, |s| {
            s.check_exit(requested, &controls).map_err(error)
        })?;
        if exit != SkitchExitForce::None {
            let mass = need(p.lock().map_err(error)?.physical_mass(), "physical mass")?;
            let force = state(owner, board, skeleton, |s| {
                s.exit_force(
                    exit,
                    mass,
                    &mut GrindNum {
                        n: &mut numeric.reckoning,
                        s: &mut numeric.sin_cos,
                    },
                )
                .map_err(error)
            })?;
            self.queue_skitch_force(board, 6, force)?;
        }
        state(owner, board, skeleton, |s| {
            s.finish_exit_check(&controls).map_err(error)
        })?;
        let phase = owner
            .reckoning()
            .lock()
            .map_err(error)?
            .try_update_grounded_live(&p, &mut ReckoningCvarAdapter(refs), &mut numeric.reckoning)
            .map_err(error)?;
        let progress = self
            .try_update_pair_ground_skateboard(
                board,
                skeleton,
                ground,
                refs,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                &mut numeric.spawn_math,
            )
            .map_err(error)?;
        if progress.next != crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralContinuation::Complete {
            return Err(error("incomplete Skitch Skeleton"));
        }
        state(owner, board, skeleton, |s| {
            s.first_eval = Some(false);
            Ok(())
        })?;
        Ok(phase)
    }
}
struct SkitchOffsetCvars<'a>(&'a InputCvarReferences);
impl crate::skeleton_skateboard_offset::SkeletonSkateboardOffsetCvars for SkitchOffsetCvars<'_> {
    type Error = crate::input_cvar_runtime::InputCvarError;
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.0.try_skitch_offset_blend_frames()
    }
}
