//! Native PhysState_OffBoard_Locomotion504 lifecycle on the retained state.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle as Handle, wipeout::WipeoutSkeletonOperation,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::remaining_states::{
        OffBoardLocomotion, OwnerMember, StateKind, StatePayload,
    },
    physical_player_phys_out::construct::locomotion_state::LocomotionOutputSource,
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, try_length},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::{
        ground_control_complete::SteeringInput,
        physical_skeleton_controller::SkeletonCollisionTransition,
    },
    skeleton_processed_contact::SkeletonProcessedContact,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum LocomotionStateError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Service(&'static str, String),
}
impl std::fmt::Display for LocomotionStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OffBoardLocomotion: {self:?}")
    }
}
type E = LocomotionStateError;
fn service(site: &'static str, e: impl std::fmt::Debug) -> E {
    E::Service(site, format!("{e:?}"))
}
fn state<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    f: impl FnOnce(&mut OffBoardLocomotion) -> Result<T, E>,
) -> Result<T, E> {
    let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
    let Some(StatePayload::OffBoardLocomotion(s)) = states
        .states
        .get_mut(&StateKind::OffBoardLocomotion)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(E::Missing("constructed504"));
    };
    f(s)
}
fn processed(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<Arc<Mutex<SkeletonProcessedContact>>, E> {
    Ok(Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .processed(),
    ))
}
fn flags(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Result<u32, E> {
    processed(owner)?
        .lock()
        .map_err(|_| E::Poisoned)?
        .physics_state_fields()
        .flags_8cc
        .ok_or(E::Missing("flags8cc"))
}
impl PhysicalAssemblyWorld {
    fn validate_locomotion_state(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<(), E> {
        if !self
            .try_controller_pair_matches_owner(owner, board, skeleton)
            .map_err(|e| service("pair", e))?
        {
            return Err(E::ForeignOwner);
        }
        state(owner, |s| {
            let b = &s.base.physical;
            if !b.board.same_as(board)
                || !b.skeleton.same_as(skeleton)
                || !b.owner.upgrade().is_some_and(|v| Arc::ptr_eq(&v, owner))
            {
                return Err(E::ForeignOwner);
            }
            for (link, member) in [
                (&s.player_grab_spline, OwnerMember::PlayerGrabSpline),
                (&s.off_board_feet_ik, OwnerMember::OffBoardFeetIK),
                (&s.skateboard_controller, OwnerMember::SkateboardController),
                (
                    &s.landing_on_deck_manager,
                    OwnerMember::LandingOnDeckManager,
                ),
            ] {
                let link = link
                    .as_ref()
                    .and_then(Option::as_ref)
                    .ok_or(E::Missing("504 collaborator link"))?;
                if link.member != member
                    || !link.owner.upgrade().is_some_and(|v| Arc::ptr_eq(&v, owner))
                {
                    return Err(E::ForeignOwner);
                }
            }
            Ok(())
        })
    }
    pub(crate) fn try_enter_pair_offboard_locomotion<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        numeric: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        self.validate_locomotion_state(owner, board, skeleton)?;
        let p = processed(owner)?;
        let last = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .last
            .ok_or(E::Missing("last state"))?;
        let reset = {
            let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
            let feet = remaining.feet_ik.as_mut().ok_or(E::Missing("FeetIK"))?;
            if !feet.binding.matches(owner, &p, skeleton) {
                return Err(E::ForeignOwner);
            }
            feet.consider_reset_before_disable_ik(last)
        };
        if reset {
            self.try_wipeout_skeleton_operation(
                board,
                skeleton,
                WipeoutSkeletonOperation::DisableIk,
                numeric,
            )
            .map_err(|e| service("ConsiderReset disableIK", e))?;
        }
        owner
            .remaining()
            .lock()
            .map_err(|_| E::Poisoned)?
            .feet_ik
            .as_mut()
            .ok_or(E::Missing("FeetIK"))?
            .finish_consider_reset(504);
        self.try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::Set(4),
        )
        .map_err(|e| service("offboard collision", e))?;
        state(owner, |s| {
            s.trying_to_land_on_board = Some(false);
            s.try_to_land_on_board_delay = Some(0);
            s.is_moving_to_fast_entering_locomotion = Some(false);
            s.ground_blend = Some(0.0);
            Ok(())
        })?;
        let limit = self
            .try_offboard_locomotion_speed_limit(board)
            .map_err(|e| service("wipeout speed attribute", e))?;
        let velocity = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .centre_of_mass_velocity()
            .ok_or(E::Missing("COM velocity"))?
            .value;
        let speed = try_length(velocity, numeric).map_err(|e| service("entry speed", e))?;
        state(owner, |s| {
            s.is_moving_to_fast_entering_locomotion = Some(speed > limit);
            Ok(())
        })
    }
    pub(crate) fn try_exit_pair_offboard_locomotion(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<(), E> {
        self.validate_locomotion_state(owner, board, skeleton)
    } // native folded BLR825187c0
    pub(crate) fn try_update_pair_offboard_locomotion(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
        trig: &mut HostReckoningSinCosNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, E> {
        self.validate_locomotion_state(owner, board, skeleton)?;
        self.try_update_pair_offboard_locomotion_feet(owner, board, skeleton, refs)
            .map_err(|e| service("FeetIK.UpdateLocomotion", e))?;
        let p = processed(owner)?;
        self.try_update_constructed_board_steering(board, 0.0, &mut SteeringInput(&p))
            .map_err(|e| service("steering tilt", e))?;
        let reorient = state(owner, |s| {
            s.is_moving_to_fast_entering_locomotion
                .ok_or(E::Missing("entry too fast"))
        })?;
        let phase = self
            .try_update_pair_offboard_locomotion_skeleton(
                owner, board, skeleton, reorient, refs, ground, numeric, trig,
            )
            .map_err(|e| service("Skeleton.UpdateOffBoardLocomotion", e))?;
        state(owner, |s| {
            s.grab_object = Some(false);
            Ok(())
        })?;
        // lbz+8cc at82da06d0 reads the HIGH byte of the native word.
        if flags(owner)? & 0x0100_0000 != 0
            && !(p
                .lock()
                .map_err(|_| E::Poisoned)?
                .pushing_delay_until_repushable()
                .ok_or(E::Missing("repush delay"))?
                > 0.0)
        {
            check_offboard_grab(
                owner,
                StateKind::OffBoardLocomotion,
                refs,
                &self.debug_overlay,
                &mut crate::physical_player_grind_manager::math::GrindNum {
                    n: numeric,
                    s: trig,
                },
            )?;
        } else {
            owner
                .grab_spline()
                .lock()
                .map_err(|_| E::Poisoned)?
                .as_mut()
                .ok_or(E::Missing("PlayerGrabSpline"))?
                .try_invalidate_query(self.world_line_owners().0)
                .map_err(|e| service("InvalidateQuery", e))?;
        }
        if flags(owner)? & 0x20 != 0 {
            let trying = state(owner, |s| {
                s.trying_to_land_on_board
                    .ok_or(E::Missing("trying landing"))
            })?;
            if !trying {
                state(owner, |s| {
                    s.trying_to_land_on_board = Some(true);
                    s.try_to_land_on_board_delay = Some(2);
                    Ok(())
                })?;
            } else {
                let delay = state(owner, |s| {
                    s.try_to_land_on_board_delay
                        .ok_or(E::Missing("landing delay"))
                })?;
                if delay != 0 {
                    state(owner, |s| {
                        s.try_to_land_on_board_delay = Some(delay - 1);
                        Ok(())
                    })?;
                } else {
                    let velocity = p
                        .lock()
                        .map_err(|_| E::Poisoned)?
                        .prior_output()
                        .centre_of_mass_velocity()
                        .ok_or(E::Missing("COM velocity"))?
                        .value;
                    let assist = refs
                        .try_locomotion_max_land_on_board_assist()
                        .map_err(|e| service("landing assist", e))?;
                    let com = p
                        .lock()
                        .map_err(|_| E::Poisoned)?
                        .prior_output()
                        .centre_of_mass()
                        .ok_or(E::Missing("COM"))?
                        .value;
                    self.try_start_pair_land_on_board_query(
                        owner, skeleton, com, velocity, assist, ground, refs, numeric,
                    )
                    .map_err(|e| service("Landing.StartQuery", e))?;
                }
            }
        } else {
            state(owner, |s| {
                s.trying_to_land_on_board = Some(false);
                s.try_to_land_on_board_delay = Some(0);
                Ok(())
            })?;
        }
        Ok(phase)
    }
    pub(crate) fn try_post_pair_offboard_locomotion<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        self.validate_locomotion_state(owner, board, skeleton)?;
        self.try_update_pair_land_on_board_postphysics(owner, refs)
            .map_err(|e| service("Landing.PostPhysics", e))?;
        self.try_check_pair_offboard_wipeout(board, skeleton, owner, numeric)
            .map_err(|e| service("CheckForOffBoardWipeout", e))?;
        if state(owner, |s| {
            s.is_moving_to_fast_entering_locomotion
                .ok_or(E::Missing("entry too fast"))
        })? {
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_ground_request(23)
                .map_err(|e| service("request23", e))?;
        }
        Ok(())
    }
    pub(crate) fn try_fill_pair_offboard_locomotion(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        self.validate_locomotion_state(owner, board, skeleton)?;
        self.try_fill_pair_land_on_board_output(owner, refs)
            .map_err(|e| service("Landing.Fill", e))?;
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_fill_locomotion_state(&mut Output(owner))
            .map_err(|e| service("Locomotion.Fill", e))
    }
}
struct Output<'a>(&'a Arc<PhysicalPlayerAssemblyOwner>);
impl LocomotionOutputSource for Output<'_> {
    type Error = E;
    fn grabbing(&mut self) -> Result<bool, E> {
        state(self.0, |s| s.grab_object.ok_or(E::Missing("grab object")))
    }
    fn descriptor(&mut self) -> Result<(i32, i32), E> {
        state(self.0, |s| {
            s.grab_spline_desc
                .map(|d| (d.object_type, d.object_id))
                .ok_or(E::Missing("grab descriptor"))
        })
    }
    fn foot_down(&mut self, foot: usize) -> Result<bool, E> {
        Ok(processed(self.0)?
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .offboard_foot_influence(foot)
            .ok_or(E::Missing("foot influence"))?
            > 0.5)
    }
}

/// Shared CheckForGrabbableObject prefix:500 at82d9c338,504 at82d9c698. No second
/// spline implementation: box geometry, candidate selection and deferred data
/// requests all use the World query chain shared with state501.
pub(crate) fn check_offboard_grab(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    kind: StateKind,
    refs: &InputCvarReferences,
    overlay: &crate::debug_overlay::DebugOverlaySlot,
    num: &mut crate::physical_player_grind_manager::math::GrindNum<'_>,
) -> Result<bool, E> {
    use crate::{
        living_world_grind_services::grab_provider::GrabBoxQuery,
        physical_player_assembly::remaining_states::{GrabDescriptor, read_floats},
        physical_player_grab_spline::geometry::GrabBox,
        physical_player_grind_manager::{math::DEG_TO_RAD, view::lanes},
    };
    let p = processed(owner)?;
    let transform = p
        .lock()
        .map_err(|_| E::Poisoned)?
        .ground_position_fields()
        .effective_animation
        .ok_or(E::Missing("effective animation+c0"))?;
    let attributes = {
        let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
        let payload = states
            .states
            .get(&kind)
            .filter(|s| s.published)
            .and_then(|s| s.value.as_ref());
        match payload {
            Some(StatePayload::OffBoard(s)) if kind == StateKind::OffBoard => s.attribs.clone(),
            Some(StatePayload::OffBoardLocomotion(s)) if kind == StateKind::OffBoardLocomotion => {
                s.attribs.clone()
            }
            _ => return Err(E::Missing("offboard grab state")),
        }
        .ok_or(E::Missing("offboard attributes"))?
    };
    // 64-bit hashes recovered from insrdi, not Hex-Rays' truncated r4 values.
    let half_dimensions = read_floats::<4>(&attributes, 0xc8b6_f949_fdff_eaae)
        .map_err(|e| service("grab half dimensions", e))?;
    let offset = read_floats::<4>(&attributes, 0xfe50_188a_8336_fdf9)
        .map_err(|e| service("grab offset", e))?;
    // Like the existing grind geometry projection, only XYZ participates in
    // reach calculations. This does not write scratch W back into Processed.
    let bounds = GrabBox::from_player(
        transform.columns().map(lanes),
        offset,
        half_dimensions,
        false,
        num,
    )
    .map_err(|e| service("InitializeGrabbableBoxTransform", e))?;
    if refs
        .try_draw_offboard_grab()
        .map_err(|e| service("grab debug cvar", e))?
    {
        overlay
            .require()
            .map_err(|e| service("offboard grab debug singleton", e))?;
    }
    let reach_start = lanes(
        p.lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .centre_of_mass()
            .ok_or(E::Missing("COM+250"))?
            .value,
    );
    let selected = {
        let mut grab = owner.grab_spline().lock().map_err(|_| E::Poisoned)?;
        let grab = grab.as_mut().ok_or(E::Missing("PlayerGrabSpline"))?;
        if grab.query_is_ready() {
            Some(
                grab.validated
                    .best_spline(reach_start, None, num)
                    .map_err(|e| service("GetBestSpline", e))?,
            )
        } else {
            None
        }
    };
    if let Some(selected) = selected {
        with_grab_fields(owner, kind, |grab_object, grab_spline_desc| {
            *grab_object = Some(selected.is_some());
            if let Some(spline) = selected {
                *grab_spline_desc = Some(GrabDescriptor {
                    object_type: spline.descriptor_type,
                    object_id: spline.descriptor_object,
                });
            }
            Ok(())
        })?;
    }
    let horizontal = read_floats::<1>(&attributes, 0xe5d0_e7b2_fbb4_f684)
        .map_err(|e| service("grab horizontal deviation", e))?[0]
        * DEG_TO_RAD;
    let normal = read_floats::<1>(&attributes, 0xb631_f1a5_ebc5_470a)
        .map_err(|e| service("grab normal deviation", e))?[0]
        * DEG_TO_RAD;
    let end_exclusion = read_floats::<1>(&attributes, 0x2523_6f55_4677_f74f)
        .map_err(|e| service("grab end exclusion", e))?[0];
    {
        let mut grab = owner.grab_spline().lock().map_err(|_| E::Poisoned)?;
        let grab = grab.as_mut().ok_or(E::Missing("PlayerGrabSpline"))?;
        grab.start_box_query(
            4,
            GrabBoxQuery {
                reach_start,
                bounds,
                end_exclusion,
                angle_limit: normal,
                horizontal_limit: horizontal,
            },
        )
        .map_err(|e| service("StartGrabSplineQuery", e))?;
    }
    let descriptor = with_grab_fields(owner, kind, |grab_object, grab_spline_desc| {
        if grab_object.ok_or(E::Missing("grab object flag"))? {
            Ok(Some(
                grab_spline_desc.ok_or(E::Missing("selected grab descriptor"))?,
            ))
        } else {
            Ok(None)
        }
    })?;
    if let Some(descriptor) = descriptor {
        let mut grab = owner.grab_spline().lock().map_err(|_| E::Poisoned)?;
        let grab = grab.as_mut().ok_or(E::Missing("PlayerGrabSpline"))?;
        grab.request_spline_data(descriptor.object_type, descriptor.object_id)
            .map_err(|e| service("RequestGrabSplineData", e))?;
    }
    Ok(descriptor.is_some())
}

fn with_grab_fields<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    kind: StateKind,
    read: impl FnOnce(
        &mut Option<bool>,
        &mut Option<crate::physical_player_assembly::remaining_states::GrabDescriptor>,
    ) -> Result<T, E>,
) -> Result<T, E> {
    let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
    match states
        .states
        .get_mut(&kind)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_mut())
    {
        Some(StatePayload::OffBoard(s)) if kind == StateKind::OffBoard => {
            read(&mut s.grab_object, &mut s.grab_spline_desc)
        }
        Some(StatePayload::OffBoardLocomotion(s)) if kind == StateKind::OffBoardLocomotion => {
            read(&mut s.grab_object, &mut s.grab_spline_desc)
        }
        _ => Err(E::Missing("offboard grab state")),
    }
}
