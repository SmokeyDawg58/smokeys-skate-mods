//! Ground.UpdateHandplants82d8d258, using the retained manager and world splines.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_handplant::HandPlantManager,
    physical_player_physics_state::CurrentPhysicalState,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum GroundHandplantsError {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Runtime(crate::physical_player_grind_manager::view::GrindError),
    Attribute(crate::attribute_runtime::AttributeRuntimeError),
    Numeric(crate::reckoning_numeric::ReckoningHostNumericError),
    Cvar(crate::input_cvar_runtime::InputCvarError),
    Overlay(crate::debug_overlay::DebugOverlayError),
}
impl std::fmt::Display for GroundHandplantsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "Ground handplants missing {v}"),
            Self::Poisoned => f.write_str("Ground handplant owner lock poisoned"),
            Self::ForeignState => {
                f.write_str("Ground handplants require the actual selected Ground owner")
            }
            Self::Pair(e) => write!(f, "Ground handplant pair: {e:?}"),
            Self::Attribute(e) => write!(f, "handplant attribute: {e:?}"),
            Self::Numeric(e) => write!(f, "handplant numeric: {e:?}"),
            Self::Cvar(e) => write!(f, "handplant cvar: {e:?}"),
            Self::Overlay(e) => write!(f, "handplant overlay: {e:?}"),
            Self::Runtime(e) => write!(f, "Ground HandPlant: {e}"),
        }
    }
}
/// Reuse Reset82db5c78; it deliberately preserves IK. The caller then performs
/// the separate2a8/2b0/2bc IK writes, preserving the low thirty flag bits.
fn reset_inactive(manager: &mut HandPlantManager) {
    manager.reset();
    manager.ik.blend = 0.0;
    manager.ik.distance = [-1.0; 4];
    manager.ik.flags &= 0x3fff_ffff;
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_ground_handplants(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), GroundHandplantsError> {
        use GroundHandplantsError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, o| Arc::clone(o))
            .map_err(E::Pair)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if state.current != CurrentPhysicalState::Ground {
                return Err(E::ForeignState);
            }
            let ground = state.ground.as_ref().ok_or(E::Missing("Ground state"))?;
            if !ground.bindings.board.same_as(board)
                || !ground.bindings.skeleton.same_as(skeleton)
                || !ground
                    .bindings
                    .owner
                    .upgrade()
                    .is_some_and(|o| Arc::ptr_eq(&o, &owner))
            {
                return Err(E::ForeignState);
            }
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let active = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .flags_8cc
            .ok_or(E::Missing("GrabWorld flag+8cc"))?
            & 0x0100_0000
            != 0;
        if active {
            use crate::{
                input_cvar_runtime::handplant::HandPlantFloat as F,
                physical_player_grind_manager::{math::GrindNum, view::lanes},
                physical_player_handplant::{
                    manager::{LaunchInfo, direction},
                    position,
                },
            };
            let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
            let mut trig = crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
            let mut num = GrindNum {
                n: &mut numeric,
                s: &mut trig,
            };
            let refs = &owner.inputs().cvars;
            let (velocity, normal, flags) = {
                let p = processed.lock().map_err(|_| E::Poisoned)?;
                (
                    p.prior_output()
                        .velocity()
                        .ok_or(E::Missing("velocity190"))?
                        .value,
                    p.prior_output()
                        .ground_normal()
                        .ok_or(E::Missing("normal1d0"))?
                        .value,
                    p.physics_state_fields()
                        .flags_8d0
                        .ok_or(E::Missing("handplant flags8d0"))?,
                )
            };
            let mut info = {
                let r = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
                let p = processed.lock().map_err(|_| E::Poisoned)?;
                LaunchInfo {
                    position: lanes(
                        p.prior_output()
                            .centre_of_mass()
                            .ok_or(E::Missing("COM250"))?
                            .value,
                    ),
                    velocity: lanes(velocity),
                    surface_normal: lanes(normal),
                    look: lanes(
                        p.prior_output()
                            .skater_look_direction()
                            .ok_or(E::Missing("look230"))?,
                    ),
                    heading: lanes(r.reckoning().ok_or(E::Missing("Reckoning"))?.heading()),
                    clamp_direction: 0,
                }
            };
            {
                let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
                let ground = state.ground.as_mut().ok_or(E::Missing("Ground"))?;
                if flags & 0x4000_0000 != 0 || flags & 0x2000_0000 == 0 {
                    ground.fields.handplant_committed_direction = 0;
                    ground.fields.handplant_frames_against_direction = 0;
                }
                if flags & 0x2000_0000 != 0 {
                    if ground.fields.handplant_committed_direction == 0 {
                        let hand = owner.handplant().lock().map_err(|_| E::Poisoned)?;
                        ground.fields.handplant_committed_direction = hand
                            .as_ref()
                            .ok_or(E::Missing("HandPlant"))?
                            .position_result
                            .direction_vs_up;
                    }
                    if ground.fields.handplant_committed_direction == 0 {
                        return Err(E::Missing("committed handplant direction assertion"));
                    }
                    let current = direction(&info, &mut num).map_err(E::Runtime)?;
                    if current == ground.fields.handplant_committed_direction {
                        ground.fields.handplant_frames_against_direction = 0;
                    } else {
                        ground.fields.handplant_frames_against_direction = ground
                            .fields
                            .handplant_frames_against_direction
                            .wrapping_add(1);
                    }
                    let a = owner.inputs().vault.instance(
                        crate::physical_player_handplant::MANAGER_CLASS,
                        crate::physical_player_handplant::DEFAULT_COLLECTION,
                    );
                    let maximum = a
                        .try_scalar_word(0xdde5_bfbc_b6a1_afd5, 0)
                        .map_err(E::Attribute)?
                        .unwrap_or(0) as i32;
                    if ground.fields.handplant_frames_against_direction > maximum {
                        ground.fields.handplant_committed_direction = 0;
                    }
                }
                info.clamp_direction = ground.fields.handplant_committed_direction;
            }
            let mut retained = owner.handplant().lock().map_err(|_| E::Poisoned)?;
            let manager = retained.as_mut().ok_or(E::Missing("HandPlantManager"))?;
            // Launch82dc4140 resets before ShouldLaunch82dbd158. An ineligible
            // launch leaves IK intact; this is not the inactive caller's reset.
            let previous = manager.position_result.position;
            manager.reset();
            let scalar = |key| -> Result<f32, E> {
                Ok(f32::from_bits(
                    manager
                        .attributes
                        .try_scalar_word(key, 0)
                        .map_err(E::Attribute)?
                        .unwrap_or(0),
                ))
            };
            let minimum_speed = scalar(0x6330_3ca6_c611_dc79)?;
            if !(minimum_speed > 0.0) {
                return Err(E::Missing("positive handplant minimum speed assertion"));
            }
            let speed =
                crate::reckoning_numeric::try_length(velocity, num.n).map_err(E::Numeric)?;
            let enough_speed = speed > minimum_speed;
            let rising = velocity.xyz[1] > 0.0;
            let minimum_slope = scalar(0xd162_2917_f6dd_215f)?;
            if !(minimum_slope >= 0.0 && minimum_slope <= 90.0) {
                return Err(E::Missing("handplant slope range assertion"));
            }
            let normal_y = normal.xyz[1];
            let angle = crate::reckoning_numeric::try_asin(normal_y.clamp(-1.0, 1.0), num.n)
                .map_err(E::Numeric)?;
            let steep = 90.0 - angle * 57.29578_f32 > minimum_slope;
            let flat = normal_y.abs() > 0.95;
            let debug = manager
                .attributes
                .try_get_data(0xa492_8ce7_9e99_ab4f, 0)
                .map_err(E::Attribute)?
                .is_some_and(|data| data.bytes().first().is_some_and(|v| *v != 0));
            if (debug
                || owner
                    .inputs()
                    .cvars
                    .try_handplant_display_debug()
                    .map_err(E::Cvar)?)
                && (!enough_speed || !rising || !steep || flat)
            {
                self.debug_overlay.require().map_err(E::Overlay)?;
            }
            if enough_speed && rising && steep && !flat {
                let half = [
                    refs.try_handplant_float(F::HalfBoxX).map_err(E::Cvar)?,
                    refs.try_handplant_float(F::HalfBoxY).map_err(E::Cvar)?,
                    refs.try_handplant_float(F::HalfBoxZ).map_err(E::Cvar)?,
                    0.,
                ];
                let bounds = crate::grind_data_octree::OctreeBounds {
                    minimum: std::array::from_fn(|i| info.position[i] - half[i]),
                    maximum: std::array::from_fn(|i| info.position[i] + half[i]),
                };
                let mut result = crate::grind_data_runtime::GrindDataQueryResult::new();
                self.grind_services
                    .try_box_test(bounds, &mut result, true, refs)
                    .map_err(|e| {
                        E::Runtime(crate::physical_player_grind_manager::view::error(e))
                    })?;
                let minimum = refs.try_handplant_float(F::MinDotWithX).map_err(E::Cvar)?;
                let primitives: Vec<_> = result.results().cloned().collect();
                if let Some(found) = position::select(
                    &manager.position_attributes,
                    &primitives,
                    info.position,
                    info.velocity,
                    info.surface_normal,
                    info.clamp_direction,
                    minimum,
                    &mut num,
                )
                .map_err(E::Runtime)?
                {
                    manager
                        .launch_selected(&info, found, previous, refs, &mut num)
                        .map_err(E::Runtime)?;
                }
            }
            let found = manager.animation.flags & 0x8000_0000 != 0;
            drop(retained);
            if found {
                self.try_handplant_pair_ik(
                    &owner,
                    board,
                    skeleton,
                    flags & 0x4000_0000 != 0 || flags & 0x2000_0000 == 0,
                    refs,
                    &mut num,
                )
                .map_err(E::Runtime)?;
            }
            return Ok(());
        }
        let mut handplant = owner.handplant().lock().map_err(|_| E::Poisoned)?;
        reset_inactive(
            handplant
                .as_mut()
                .ok_or(E::Missing("constructed HandPlantManager"))?,
        );
        Ok(()) // Caller0ea8 Skeleton.UpdateGroundSkateboard: physical_player_ground_skateboard.rs.
    }
}
#[cfg(test)]
mod tests {
    use super::{HandPlantManager, reset_inactive};
    #[test]
    fn reset_reuses_constructor_owner_and_preserves_future_and_noncleared_flags() {
        let (_, owner) = crate::physical_player_assembly::tests::issued_owner_for_grind_tests();
        let mut manager = HandPlantManager::new_sim(&owner.inputs().vault, || None);
        manager.animation.flags = u32::MAX;
        manager.trajectory_ready = true;
        manager.trajectory_selector.ready = true;
        manager.trajectory_selector.queries[0].ready = true;
        manager.ik.blend = 0.75;
        manager.ik.distance = [42.0; 4];
        manager.ik.flags = 0xc123_4567;
        manager.reset();
        assert_eq!(manager.ik.blend.to_bits(), 0.75f32.to_bits());
        reset_inactive(&mut manager);
        assert!(!manager.trajectory_ready);
        assert!(manager.trajectory_selector.ready);
        assert!(manager.trajectory_selector.queries[0].ready);
        assert_eq!(manager.animation.flags, 0x4fff_ffff);
        assert_eq!(manager.ik.blend.to_bits(), 0);
        assert_eq!(
            manager.ik.distance.map(f32::to_bits),
            [(-1.0f32).to_bits(); 4]
        );
        assert_eq!(manager.ik.flags, 0x0123_4567);
    }
}
