//! Native PhysState_LandingOnDeck505 lifecycle; uses the constructor-owned state/manager.
use crate::physics_state_ids::{PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_OFFBOARD_LOCOMOTION};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle as Handle, wipeout::WipeoutSkeletonOperation,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences, landing_state::LandingCvar as C},
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{LandingOnDeck, OwnerMember, StateKind, StatePayload},
    },
    physical_player_phys_out::construct::{
        landing_state::LandingOutputSource, skeleton_fill_suffix::plane_angle,
    },
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    physical_player_remaining_constructor_owners::LandingOnDeck as Manager,
    reckoning::grounded_input::{GroundInputRead, GroundVector},
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningNumeric, length_from_squared_and_estimate,
        wrap_turns_to_radians,
    },
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::physical_skeleton_controller::SkeletonCollisionTransition,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix as M, SkeletonRootVector as V},
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};
const DT: f32 = f32::from_bits(0x3c888889);
const INV_TWO_PI: f32 = f32::from_bits(0x3e22f983);
const EPS: f32 = 1.0 / 65536.0;
const UP: V = V::known([0., 1., 0., 0.]);
#[derive(Debug)]
pub(crate) enum LandingStateError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    Service(&'static str, String),
}
impl std::fmt::Display for LandingStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LandingOnDeck lifecycle: {self:?}")
    }
}
type E = LandingStateError;
pub(crate) fn service(stage: &'static str, e: impl std::fmt::Debug) -> E {
    E::Service(stage, format!("{e:?}"))
}
fn state<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    f: impl FnOnce(&mut LandingOnDeck) -> Result<T, E>,
) -> Result<T, E> {
    let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
    let Some(StatePayload::LandingOnDeck(s)) = states
        .states
        .get_mut(&StateKind::LandingOnDeck)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(E::Missing("constructed state505"));
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
fn manager<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    skeleton: &Handle,
    f: impl FnOnce(&mut Manager) -> Result<T, E>,
) -> Result<T, E> {
    let p = processed(owner)?;
    let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
    let m = remaining
        .landing
        .as_mut()
        .ok_or(E::Missing("Landing manager"))?;
    if !m.skeleton.matches(owner, &p, skeleton) || !m.world.matches(owner) {
        return Err(E::ForeignOwner);
    }
    f(m)
}
fn float(refs: &InputCvarReferences, key: C) -> Result<f32, E> {
    refs.try_landing_state_float(key).map_err(E::Cvar)
}
fn boolean(refs: &InputCvarReferences, key: C) -> Result<bool, E> {
    refs.try_landing_state_bool(key).map_err(E::Cvar)
}
fn attr(a: &AttributeRuntimeInstance, key: u64) -> Result<f32, E> {
    Ok(f32::from_bits(
        a.try_scalar_word(key, 0)
            .map_err(E::Attribute)?
            .unwrap_or(0),
    ))
}
fn lod_attr(owner: &Arc<PhysicalPlayerAssemblyOwner>, key: u64) -> Result<f32, E> {
    state(owner, |s| {
        attr(
            s.lod_attribs.as_ref().ok_or(E::Missing("LOD attributes"))?,
            key,
        )
    })
}
fn flag(owner: &Arc<PhysicalPlayerAssemblyOwner>, mask: u32) -> Result<bool, E> {
    Ok(processed(owner)?
        .lock()
        .map_err(|_| E::Poisoned)?
        .physics_state_fields()
        .flags_8d0
        .ok_or(E::Missing("flags8d0"))?
        & mask
        != 0)
}
fn com(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Result<V, E> {
    Ok(processed(owner)?
        .lock()
        .map_err(|_| E::Poisoned)?
        .prior_output()
        .centre_of_mass()
        .ok_or(E::Missing("COM"))?
        .value)
}
fn velocity(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Result<V, E> {
    Ok(processed(owner)?
        .lock()
        .map_err(|_| E::Poisoned)?
        .prior_output()
        .centre_of_mass_velocity()
        .ok_or(E::Missing("COM velocity"))?
        .value)
}
fn fsel(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0. { positive } else { negative }
}
fn closest_angle(angle: f32) -> f32 {
    let angle = wrap_turns_to_radians(angle * INV_TWO_PI);
    // Raw BLE skips positive case when GT-clear, including unordered.
    let sign = if angle > 0. { 1. } else { -1. };
    let mut a = angle * sign;
    if a > std::f32::consts::FRAC_PI_2 {
        a -= std::f32::consts::PI;
    }
    sign * a
}
impl PhysicalAssemblyWorld {
    fn validate_landing_state(
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
            let link = s
                .landing_on_deck_manager
                .as_ref()
                .and_then(|v| v.as_ref())
                .ok_or(E::Missing("manager link"))?;
            if link.member != OwnerMember::LandingOnDeckManager
                || !link.owner.upgrade().is_some_and(|v| Arc::ptr_eq(&v, owner))
            {
                return Err(E::ForeignOwner);
            }
            Ok(())
        })
    }
    fn landing_debug(&self, refs: &InputCvarReferences) -> Result<(), E> {
        if boolean(refs, C::Debug)? {
            self.debug_overlay
                .require()
                .map_err(|e| service("debug overlay", e))?;
        }
        Ok(())
    }
    fn landing_raw_hips(&self, skeleton: &Handle) -> Result<M, E> {
        let n = self
            .skeleton(skeleton)
            .map_err(|e| service("Skeleton", e))?;
        // GetLastAnimVolumeTransform82b3d498 reads SkeletonState+10,
        // not the independently mutable live volume-pose buffer+3040.
        let hips = &n.current_animation().record().poses()[23];
        Ok(n.root_transforms()
            .animation_to_world()
            .compose(&M::known(*hips)))
    }
    pub(crate) fn try_enter_pair_landing_state<N: ReckoningNumeric>(
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
        self.validate_landing_state(owner, board, skeleton)?;
        self.try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::Set(6),
        )
        .map_err(|e| service("STANDARD collision", e))?;
        let last = processed(owner)?
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .last
            .ok_or(E::Missing("last state"))?;
        if last != PHYSICS_STATE_OFFBOARD_LOCOMOTION && last != PHYSICS_STATE_OFFBOARD_AIR {
            manager(owner, skeleton, |m| {
                m.reset_after_teleport();
                Ok(())
            })?;
        }
        state(owner, |s| {
            s.heavy_landing_wipeout = Some(false);
            s.on_deck = Some(false);
            s.flip_board = Some(false);
            s.time_to_land = Some(0.);
            s.hippy_jumping = Some(false);
            s.turn_speed = Some(0.);
            s.just_hippy_jumped = Some(0);
            s.turn_speed_adjusted = Some(0.);
            s.transition_angle = Some(0.);
            Ok(())
        })?;
        let hips = self.landing_raw_hips(skeleton)?;
        let p = processed(owner)?;
        let target = {
            let p = p.lock().map_err(|_| E::Poisoned)?;
            let x = p
                .ground_position_fields()
                .effective_animation
                .ok_or(E::Missing("effective animation"))?
                .columns()[0];
            if p.physics_state_fields()
                .flags_8cc
                .ok_or(E::Missing("flags8cc"))?
                & 8
                != 0
            {
                x.map(|v| f32::from_bits(v.to_bits() ^ 0x80000000))
            } else {
                x
            }
        };
        let angle = plane_angle(hips.columns()[1], target, UP, numeric)
            .map_err(|e| service("transition plane angle", e))?;
        state(owner, |s| {
            s.transition_angle = Some(wrap_turns_to_radians(angle * INV_TWO_PI));
            Ok(())
        })?;
        if boolean(refs, C::Debug)? {
            let _ = self.landing_raw_hips(skeleton)?;
            self.debug_overlay
                .require()
                .map_err(|e| service("debug overlay", e))?;
        }
        self.try_wipeout_skeleton_operation(
            board,
            skeleton,
            WipeoutSkeletonOperation::DisableIk,
            numeric,
        )
        .map_err(|e| service("DisableIK", e))?;
        self.try_disable_board_animation(board)
            .map_err(|e| service("DisableBoardAnimation", e))?;
        self.try_set_board_standard(board)
            .map_err(|e| service("SetStandard", e))?;
        if flag(owner, 0x8000)? {
            let jump = self
                .try_calculate_pair_hippy_jump_velocity(owner, board, skeleton, numeric)
                .map_err(|e| service("HippyJumpVel", e))?;
            let position = com(owner)?;
            manager(owner, skeleton, |m| {
                m.init_trajectory::<std::convert::Infallible>(position, jump)
                    .map_err(|e| service("InitTrajectory", e))
            })?;
            state(owner, |s| {
                s.just_hippy_jumped = Some(2);
                Ok(())
            })?;
            self.try_start_pair_landing_state_wobble(owner, skeleton, false)
                .map_err(|e| service("takeoff wobble", e))?;
        } else if {
            let last = processed(owner)?
                .lock()
                .map_err(|_| E::Poisoned)?
                .physics_state_fields()
                .last
                .ok_or(E::Missing("last state"))?;
            last != PHYSICS_STATE_OFFBOARD_LOCOMOTION && last != PHYSICS_STATE_OFFBOARD_AIR
        } {
            let position = com(owner)?;
            let velocity = velocity(owner)?;
            manager(owner, skeleton, |m| {
                m.init_trajectory::<std::convert::Infallible>(position, velocity)
                    .map_err(|e| service("InitTrajectory", e))
            })?;
        } else {
            manager(owner, skeleton, |m| {
                m.trajectory_time += DT;
                Ok(())
            })?;
            let position = com(owner)?;
            manager(owner, skeleton, |m| {
                m.correct_trajectory(position, refs, numeric, &self.debug_overlay)
                    .map_err(|e| service("CorrectTrajectory", e))
            })?;
        }
        let hippy = flag(owner, 0x8000)?;
        state(owner, |s| {
            s.hippy_jumping = Some(hippy);
            Ok(())
        })
    }
    pub(crate) fn try_exit_pair_landing_state(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<(), E> {
        self.validate_landing_state(owner, board, skeleton)?;
        manager(owner, skeleton, |m| {
            m.reset_after_teleport();
            Ok(())
        })
    }
    pub(crate) fn try_update_pair_landing_state(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
        sin_cos: &mut HostReckoningSinCosNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, E> {
        self.validate_landing_state(owner, board, skeleton)?;
        let p = processed(owner)?;
        let old_ttl = state(owner, |s| s.time_to_land.ok_or(E::Missing("time to land")))?;
        let frames = old_ttl * f32::from_bits(0x426fffff);
        let (a, b) = {
            let p = p.lock().map_err(|_| E::Poisoned)?;
            (
                p.vector(GroundVector::EffectiveZ)
                    .map_err(|e| service("effectiveZ", e))?,
                p.ground_position_fields()
                    .effective_animation
                    .ok_or(E::Missing("effective animation"))?
                    .columns()[2],
            )
        };
        let result = self
            .try_update_pair_landing_manager(owner, skeleton, ground, refs, numeric)
            .map_err(|e| service("Landing.Update", e))?;
        let mut angle = plane_angle(a, b, UP, numeric).map_err(|e| service("angle", e))?;
        let timer = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .state_timer
            .ok_or(E::Missing("state timer"))?;
        let transition_time = float(refs, C::AngleTransitionTime)?;
        if timer < transition_time {
            let transition = state(owner, |s| {
                s.transition_angle.ok_or(E::Missing("transition angle"))
            })?;
            angle = -((1. - timer / transition_time) * transition) + angle;
        }
        let old_turn = state(owner, |s| s.turn_speed.ok_or(E::Missing("turn speed")))?;
        angle = wrap_turns_to_radians(old_turn.mul_add(frames, angle) * INV_TWO_PI);
        state(owner, |s| {
            s.flip_board = Some(angle.abs() > std::f32::consts::FRAC_PI_2);
            Ok(())
        })?;
        if old_ttl > EPS {
            let ascending = velocity(owner)?.xyz[1] > 0.;
            let mut spin = 0.;
            if ascending
                && state(owner, |s| s.hippy_jumping.ok_or(E::Missing("hippy")))?
                && boolean(refs, C::EnableSpin)?
            {
                spin = p
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .animation_attribute_fields()
                    .physics_body_spin
                    .ok_or(E::Missing("body spin"))?;
            }
            let mut desired = old_turn;
            // BLE is !GT; unordered selects the auto-turn branch.
            let delta_key = if spin.abs() > EPS {
                desired = -(lod_attr(owner, 0x0729_2400_dc2f_83eb)? * spin);
                0x29d0_8b52_21ef_b58f
            } else {
                let maximum = lod_attr(owner, 0xc918_12c5_4db2_e23b)?;
                if ascending {
                    angle = closest_angle(angle);
                }
                let threshold =
                    lod_attr(owner, 0x2b77_be5c_70b6_5127)? * f32::from_bits(0x3c8efa35);
                if angle.abs() > threshold {
                    let offset = if angle > 0. {
                        let x = angle - threshold;
                        fsel(-x, 0., x)
                    } else {
                        let x = threshold + angle;
                        fsel(-x, x, 0.)
                    };
                    let turn = old_turn - offset / frames;
                    let lower = fsel(-maximum - turn, -maximum, turn);
                    desired = fsel(maximum - lower, lower, maximum);
                }
                0xce82_748b_1a0a_32c4
            };
            let delta = lod_attr(owner, delta_key)?;
            let lower = old_turn - delta;
            let upper = old_turn + delta;
            let limited = fsel(lower - desired, lower, desired);
            let turn = fsel(upper - limited, limited, upper);
            state(owner, |s| {
                s.turn_speed = Some(turn);
                s.turn_speed_adjusted = Some(turn);
                Ok(())
            })?;
            let t = float(refs, C::AngleTransitionTime)?;
            if timer < t {
                let transition = state(owner, |s| {
                    s.transition_angle.ok_or(E::Missing("transition angle"))
                })?;
                let adjusted = -((transition / t).mul_add(DT, -turn));
                state(owner, |s| {
                    s.turn_speed_adjusted = Some(adjusted);
                    Ok(())
                })?;
            }
        }
        let turn = state(owner, |s| {
            s.turn_speed_adjusted.ok_or(E::Missing("adjusted turn"))
        })?;
        let phase = self
            .try_update_pair_offboard_air_skeleton(
                owner,
                board,
                skeleton,
                result.position,
                turn,
                true,
                refs,
                ground,
                numeric,
                sin_cos,
            )
            .map_err(|e| service("Skeleton.UpdateOffBoardAir", e))?;
        let mut ttl = result.time_to_land;
        state(owner, |s| {
            s.time_to_land = Some(ttl);
            Ok(())
        })?;
        let current = manager(owner, skeleton, |m| Ok(m.current_velocity()))?;
        if current.xyz[1] < 0. {
            ttl = self.landing_accurate_time(owner, skeleton, refs, numeric)?;
            state(owner, |s| {
                s.time_to_land = Some(ttl);
                s.on_deck = Some(ttl < f32::from_bits(0x3c83126f));
                Ok(())
            })?;
            self.try_calculate_pair_land_on_board_ik(owner, skeleton, ttl, refs, numeric)
                .map_err(|e| service("AccurateIK", e))?;
            if state(owner, |s| s.on_deck.ok_or(E::Missing("on deck")))? {
                self.try_start_pair_landing_state_wobble(owner, skeleton, true)
                    .map_err(|e| service("landing wobble", e))?;
            }
            self.landing_debug(refs)?;
        }
        let raw = self
            .skeleton(skeleton)
            .map_err(|e| service("Skeleton", e))?
            .current_animation()
            .raw_skateboard();
        let z = V::known([raw[8], raw[9], raw[10], raw[11]]);
        let dot = numeric
            .dot_vector(z, V::known([0., 0., 1., 0.]))
            .map_err(|e| service("raw board Z dot", e))?
            .abs();
        // Raw BGE skips DisableIK for unordered too.
        if dot < float(refs, C::MinDotForFeetIk)? {
            self.landing_debug(refs)?;
            self.try_wipeout_skeleton_operation(
                board,
                skeleton,
                WipeoutSkeletonOperation::DisableIk,
                numeric,
            )
            .map_err(|e| service("DisableIK", e))?;
        } else if boolean(refs, C::EnableFeetIk)? && ttl < float(refs, C::TimeToEnableIk)? {
            self.try_enable_pair_landing_ik(owner, board, skeleton)?;
        }
        state(owner, |s| {
            let v = s.just_hippy_jumped.ok_or(E::Missing("just hippy jumped"))?;
            if v > 0 {
                s.just_hippy_jumped = Some(v - 1);
            }
            Ok(())
        })?;
        let threshold = self
            .try_landing_heavy_wipeout_threshold(board, skeleton, owner, numeric)
            .map_err(|e| service("heavy landing attribute", e))?;
        let ttl = state(owner, |s| s.time_to_land.ok_or(E::Missing("time to land")))?;
        let heavy =
            ttl < float(refs, C::TimeForLandingWipeout)? && velocity(owner)?.xyz[1] < -threshold;
        state(owner, |s| {
            s.heavy_landing_wipeout = Some(heavy);
            Ok(())
        })?;
        Ok(phase)
    }
    fn landing_accurate_time<N: ReckoningNumeric>(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &Handle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<f32, E>
    where
        N::Error: std::fmt::Debug,
    {
        let normal = self
            .skeleton(skeleton)
            .map_err(|e| service("Skeleton", e))?;
        let left = normal
            .try_query_part_transform(15)
            .map_err(|e| service("left toe", e))?
            .columns()[3];
        let right = normal
            .try_query_part_transform(19)
            .map_err(|e| service("right toe", e))?
            .columns()[3];
        let sum = left.zip(right, |a, b| a + b);
        let offset = float(refs, C::DeckYOffset)?;
        let p = processed(owner)?;
        let p = p.lock().map_err(|_| E::Poisoned)?;
        let deck = p.deck_transform().ok_or(E::Missing("deck"))?.columns()[3].xyz[1];
        let wheels = p
            .physics_state_fields()
            .num_wheels_in_contact
            .ok_or(E::Missing("wheel contacts"))?;
        let board_y = p
            .prior_output()
            .velocity()
            .ok_or(E::Missing("board velocity"))?
            .value
            .xyz[1];
        let com_y = p
            .prior_output()
            .centre_of_mass_velocity()
            .ok_or(E::Missing("COM velocity"))?
            .value
            .xyz[1];
        drop(p);
        accurate_time(
            sum.xyz[1],
            offset,
            deck,
            com_y - board_y,
            wheels != 0,
            numeric,
        )
    }
    pub(crate) fn try_post_pair_landing_state<N: ReckoningNumeric>(
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
        self.validate_landing_state(owner, board, skeleton)?;
        self.try_update_pair_land_on_board_postphysics(owner, refs)
            .map_err(|e| service("Landing.Post", e))?;
        if !manager(owner, skeleton, |m| Ok(m.can_land_on_deck()))? {
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_ground_request(24)
                .map_err(|e| service("request24", e))?;
        }
        if state(owner, |s| {
            s.heavy_landing_wipeout.ok_or(E::Missing("heavy landing"))
        })? {
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_ground_request(27)
                .map_err(|e| service("request27", e))?;
        }
        self.try_check_landing_skeleton_air_wipeout(board, skeleton, owner, numeric)
            .map_err(|e| service("SkeletonCollisionWipeoutAir", e))
    }
    pub(crate) fn try_fill_pair_landing_state(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        self.validate_landing_state(owner, board, skeleton)?;
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_fill_landing_state(&mut Output { owner, refs })
            .map_err(|e| service("state Fill", e))?;
        self.try_fill_pair_land_on_board_output(owner, refs)
            .map_err(|e| service("Landing.Fill", e))
    }
}
fn accurate_time<N: ReckoningNumeric>(
    toe_sum_y: f32,
    offset: f32,
    deck_y: f32,
    relative_y: f32,
    wheels: bool,
    numeric: &mut N,
) -> Result<f32, E>
where
    N::Error: std::fmt::Debug,
{
    let height = deck_y - toe_sum_y.mul_add(0.5, offset);
    let gravity = if wheels { -9.8 } else { -4.9 };
    let discriminant = relative_y.mul_add(relative_y, (height * gravity) * 2.);
    if discriminant > 0. {
        let estimate = numeric
            .reciprocal_sqrt_estimate(discriminant)
            .map_err(|e| service("accurate TTL sqrt", e))?;
        Ok((-relative_y - length_from_squared_and_estimate(discriminant, estimate)) / gravity)
    } else {
        Ok(0.)
    }
}
struct Output<'a> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    refs: &'a InputCvarReferences,
}
impl LandingOutputSource for Output<'_> {
    type Error = E;
    fn wipeout_time(&mut self) -> Result<f32, E> {
        float(self.refs, C::WipeoutTime)
    }
    fn time_to_land(&mut self) -> Result<f32, E> {
        state(self.owner, |s| s.time_to_land.ok_or(E::Missing("TTL")))
    }
    fn on_deck(&mut self) -> Result<bool, E> {
        state(self.owner, |s| s.on_deck.ok_or(E::Missing("on deck")))
    }
    fn just_hippy(&mut self) -> Result<bool, E> {
        state(self.owner, |s| {
            s.just_hippy_jumped
                .map(|x| x > 0)
                .ok_or(E::Missing("just hippy"))
        })
    }
    fn flip(&mut self) -> Result<bool, E> {
        state(self.owner, |s| s.flip_board.ok_or(E::Missing("flip")))
    }
    fn hippy(&mut self) -> Result<bool, E> {
        state(self.owner, |s| s.hippy_jumping.ok_or(E::Missing("hippy")))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closest_axis_preserves_signed_direction_and_wraps_both_halves() {
        let pi = std::f32::consts::PI;
        assert!((closest_angle(pi * 0.75) + pi * 0.25).abs() < 1e-6);
        assert!((closest_angle(-pi * 0.75) - pi * 0.25).abs() < 1e-6);
        assert!(closest_angle(f32::NAN).is_nan());
    }
    #[test]
    fn accurate_time_uses_toe_mean_offset_and_relative_velocity() {
        let mut n = HostReckoningNumeric;
        let a = accurate_time(2., -0.15, 0., -2., true, &mut n).unwrap();
        let b = accurate_time(2., -0.15, 0., -2., false, &mut n).unwrap();
        assert!(a > 0.25 && a < 0.27);
        assert!(b > a);
        assert_eq!(accurate_time(0., 0., 1., 0., true, &mut n).unwrap(), 0.);
        assert_eq!(
            accurate_time(f32::NAN, 0., 0., 0., true, &mut n).unwrap(),
            0.
        );
    }
}
