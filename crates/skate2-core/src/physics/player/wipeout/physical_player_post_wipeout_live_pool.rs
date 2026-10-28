//! Same retained pair for error/wobble, selected foot/hand and mandatory IK update.
//! Head and pose publication remain the caller continuation.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    actor_construction::anim_out_phys_in::pose_lanes::{PoseAccessError, PoseBuffer},
    dynamic_assembly_session::skeleton_normal::post_wipeout_head_orientation::HeadOrientationError,
    input_cvar_runtime::{
        InputCvarError,
        head_tracking_live::{HeadOrientationRuntime, HeadTrackingLiveError},
    },
    skeleton_head_tracking::post_wipeout_update::HeadTrackingError,
    skeleton_root_transforms::SkeletonRootVector,
};
use crate::{
    dynamic_assembly_session::skeleton_normal::post_wipeout_foot_live::{
        FootIkLiveDebug, FootIkLiveError, FootIkLiveInput, ik_update::IkPostUpdateLiveDebug,
    },
    skeleton_ik_state::post_wipeout_foot::FootIkError,
};
use crate::{
    dynamic_assembly_session::skeleton_normal::post_wipeout_prefix::{
        PostWipeoutBeforeIk, PostWipeoutPrefixError, PostWipeoutPrefixOwners, RootAdjustmentError,
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_skateboard_wobble::post_wipeout_update::WobbleUpdateDebug,
};
use std::sync::{Arc, Mutex};
pub(crate) struct PostWipeoutBeforePose {
    pub(crate) captured: PostWipeoutBeforeIk,
    pub(crate) update_pose: bool,
}
#[derive(Debug)]
pub(crate) enum PostWipeoutPoolError<N, D, S> {
    Pool(SimulationSkaterPoolError),
    ForeignOwner,
    Missing(&'static str),
    Poisoned,
    Prefix(PostWipeoutPrefixError<N, D>),
    Root(RootAdjustmentError<S>),
    Foot(FootIkError<FootIkLiveError<D>, N, S>),
    Hand(FootIkError<FootIkLiveError<D>, N, S>),
    IkUpdate(FootIkError<FootIkLiveError<D>, N, S>),
    PoseAccess(PoseAccessError),
    Head(HeadTrackingError<HeadTrackingLiveError, N>),
    Orientation(HeadOrientationError<InputCvarError, N, S>),
    Cvar(InputCvarError),
}
impl<N: std::fmt::Debug, D: std::fmt::Debug, S: std::fmt::Debug> std::fmt::Display
    for PostWipeoutPoolError<N, D, S>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => e.fmt(f),
            Self::ForeignOwner => f.write_str("post-wipeout pair mismatch"),
            Self::Missing(v) => write!(f, "post-wipeout missing {v}"),
            Self::Poisoned => f.write_str("post-wipeout lock poisoned"),
            Self::Prefix(e) => e.fmt(f),
            Self::Root(e) => e.fmt(f),
            Self::Foot(e) | Self::Hand(e) | Self::IkUpdate(e) => e.fmt(f),
            Self::PoseAccess(e) => e.fmt(f),
            Self::Head(e) => e.fmt(f),
            Self::Orientation(e) => e.fmt(f),
            Self::Cvar(e) => e.fmt(f),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    #[expect(
        clippy::too_many_arguments,
        reason = "distinct retained owners and original numeric services"
    )]
    #[expect(
        clippy::type_complexity,
        reason = "separate numeric, drawing and SinCos failure types"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "retain original call order and shared owner lock lifetimes"
    )]
    pub(crate) fn try_post_wipeout_through_ik<
        N: ReckoningNumeric,
        D: WobbleUpdateDebug
            + IkPostUpdateLiveDebug
            + FootIkLiveDebug<Error = <D as WobbleUpdateDebug>::Error>,
        S: ReckoningSinCosNumeric,
    >(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        board_handle: &SimulationSkaterAssemblyHandle,
        skeleton_handle: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        wipeout_starting: bool,
        numeric: &mut N,
        debug: &mut D,
        rounding: &mut S,
    ) -> Result<
        PostWipeoutBeforePose,
        PostWipeoutPoolError<N::Error, <D as WobbleUpdateDebug>::Error, S::Error>,
    > {
        use PostWipeoutPoolError as E;
        let si = self
            .skeleton_mutation_index(backend, skeleton_handle)
            .map_err(E::Pool)?;
        let bi = self.member_index(board_handle).map_err(E::Pool)?;
        let (kind, bo) = self
            .try_physical_member_owner(backend, board_handle)
            .map_err(E::Pool)?;
        if kind != super::super::physical::SimulationPhysicalBodyKind::Board {
            return Err(E::Pool(SimulationSkaterPoolError::WrongMemberKind));
        }
        let (_, so) = self
            .try_physical_member_owner(backend, skeleton_handle)
            .map_err(E::Pool)?;
        if !Arc::ptr_eq(bo, expected_owner) || !Arc::ptr_eq(so, expected_owner) {
            return Err(E::ForeignOwner);
        }
        let (board, skeleton) = if bi < si {
            let (a, b) = self.members.split_at_mut(si);
            (&a[bi], &mut b[0])
        } else {
            let (a, b) = self.members.split_at_mut(bi);
            (&b[0], &mut a[si])
        };
        let MemberContent::Board {
            candidate: board, ..
        } = &board.content
        else {
            unreachable!("checked board")
        };
        let MemberContent::Skeleton {
            candidate: skeleton,
            ..
        } = &mut skeleton.content
        else {
            unreachable!("checked skeleton")
        };
        let construction = skeleton
            .construction
            .as_mut()
            .ok_or(E::Missing("Skeleton construction"))?;
        let ctor = &mut construction.constructor;
        let binding = ctor.bindings.as_ref().ok_or(E::Missing("bindings"))?;
        if !Arc::ptr_eq(&binding.processed, processed)
            || binding
                .owner
                .upgrade()
                .is_none_or(|o| !Arc::ptr_eq(&o, expected_owner))
        {
            return Err(E::ForeignOwner);
        }
        let p = processed.lock().map_err(|_| E::Poisoned)?;
        let outer = ctor.outer.as_mut().ok_or(E::Missing("Skeleton outer"))?;
        let before = skeleton
            .owner
            .try_post_wipeout_prefix(
                PostWipeoutPrefixOwners {
                    processed: &p,
                    board: board.session(),
                    backend,
                    board_construction: board
                        .construction
                        .as_deref()
                        .ok_or(E::Missing("board construction"))?,
                    outer,
                    wobble: ctor.wobble.as_mut().ok_or(E::Missing("Skeleton wobble"))?,
                },
                wipeout_starting,
                numeric,
                debug,
            )
            .map_err(E::Prefix)?;
        let before = skeleton
            .owner
            .try_post_wipeout_root_adjustment(before, outer, rounding)
            .map_err(E::Root)?;
        if !before.teleporting && (!before.airborne || before.board_contact) && !before.offboard {
            skeleton
                .owner
                .try_post_wipeout_foot_live(
                    FootIkLiveInput {
                        processed: &p,
                        backend,
                        physics_animation: construction
                            .body_reader()
                            .ok_or(E::Missing("Skeleton body attributes"))?
                            .global_physics_animation_instance(),
                        deck: &before.deck,
                        wipeout_starting: before.wipeout_starting,
                        debug,
                    },
                    numeric,
                    rounding,
                )
                .map_err(E::Foot)?;
        }
        if !before.teleporting && before.airborne && before.board_contact {
            skeleton
                .owner
                .try_post_wipeout_hand_live(
                    FootIkLiveInput {
                        processed: &p,
                        backend,
                        physics_animation: construction
                            .body_reader()
                            .ok_or(E::Missing("Skeleton body attributes"))?
                            .global_physics_animation_instance(),
                        deck: &before.deck,
                        wipeout_starting: before.wipeout_starting,
                        debug,
                    },
                    numeric,
                    rounding,
                )
                .map_err(E::Hand)?;
        }
        skeleton
            .owner
            .try_post_wipeout_ik_update_live(
                FootIkLiveInput {
                    processed: &p,
                    backend,
                    physics_animation: construction
                        .body_reader()
                        .ok_or(E::Missing("Skeleton body attributes"))?
                        .global_physics_animation_instance(),
                    deck: &before.deck,
                    wipeout_starting: before.wipeout_starting,
                    debug,
                },
                numeric,
                rounding,
            )
            .map_err(E::IkUpdate)?;
        let ctor = &mut construction.constructor;
        let input = ctor
            .bindings
            .as_ref()
            .and_then(|binding| binding.animation_input.as_ref())
            .ok_or(E::Missing("actual animation input"))?;
        let actual = expected_owner
            .animation_physics_input
            .lock()
            .map_err(|_| E::Poisoned)?;
        if actual
            .as_ref()
            .is_none_or(|actual| !Arc::ptr_eq(actual, input))
        {
            return Err(E::ForeignOwner);
        }
        drop(actual);
        let mut pose = input.lock().map_err(|_| E::Poisoned)?;
        let head_index = ctor.init.named_bones[6]
            .and_then(|bone| usize::try_from(bone).ok())
            .ok_or(E::Missing("actual Head bone"))?;
        let captured_head = pose
            .pose_matrix_typed(PoseBuffer::Global, head_index)
            .map_err(E::PoseAccess)?;
        let mut axis_angle = SkeletonRootVector::known([0.0; 4]);
        let cvars = &expected_owner.inputs().cvars;
        ctor.head
            .as_mut()
            .ok_or(E::Missing("HeadTracking owner"))?
            .try_post_wipeout_update(
                skeleton.owner.root_transforms().world_to_animation(),
                &captured_head,
                &mut axis_angle,
                cvars,
                numeric,
            )
            .map_err(E::Head)?;
        skeleton
            .owner
            .try_post_wipeout_head_orientation(
                &mut pose,
                &ctor.init,
                captured_head,
                axis_angle,
                &mut HeadOrientationRuntime(cvars),
                numeric,
                rounding,
            )
            .map_err(E::Orientation)?;
        let update_pose = cvars.try_update_skeleton_pose_buffer().map_err(E::Cvar)?;
        Ok(PostWipeoutBeforePose {
            captured: before,
            update_pose,
        })
    }
}
