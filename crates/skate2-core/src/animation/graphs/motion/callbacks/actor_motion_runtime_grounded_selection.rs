//! Proposed child of `generated_motion_load::runtime`, named `grounded_selection`.
//! Uses the retained Motion component's distinct surface and Z speed fields.
use super::MotionUpdateError as Error;
use crate::{
    actor_construction::{ActorConstruction, ActorIdentity},
    generated_action_load::{NULL_INDEX, StateDeclaration, construct::Comparison},
    generated_motion_load::ids::{condition as cond, expression as expr},
    motion_graph_entity_authority::BoundMotionGraphContext,
};

fn com_velocity_operand(
    velocity: crate::skeleton_root_transforms::SkeletonRootVector,
    axis: crate::generated_motion_load::conditions::Axis,
) -> Result<f32, crate::reckoning_numeric::ReckoningHostNumericError> {
    use crate::{
        generated_motion_load::conditions::Axis,
        reckoning_numeric::{HostReckoningNumeric, try_length},
    };
    Ok(match axis {
        Axis::X => velocity.xyz[0],
        Axis::Y => velocity.xyz[1],
        Axis::Z => velocity.xyz[2],
        Axis::All => try_length(velocity, &mut HostReckoningNumeric)?,
    })
}

impl ActorConstruction {
    /// `IsRidingFakie82c9a308` checks cached `ISkaterAnim` against UCOM before
    /// `GetIsRidingFakie82cb7530` reads flag20000000 at SkaterAnim+e4c.
    pub(crate) fn motion_runtime_is_riding_fakie(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, Error> {
        use crate::play_animation_lifecycle::{
            PlayAnimationInterfaceKind, PlayAnimationInterfaceResolution,
            PlayAnimationLifecycleBackend,
        };
        let mut source =
            crate::actor_construction::motion_runtime_session::play_animation::PlayAnimationActor {
                actor: self,
                context,
            };
        if source.resolve_interface(PlayAnimationInterfaceKind::SkaterAnim)?
            != PlayAnimationInterfaceResolution::Resolved
        {
            return Err(Error::ForeignOwner("IsRidingFakie ISkaterAnim"));
        }
        self.runtime_is_riding_fakie_from_checked_context()
    }

    /// Shared `IsRidingFakie::ActivationFunc` 0x82c9a308 read after the
    /// caller has checked its own Action or Motion graph context.
    pub(crate) fn runtime_is_riding_fakie_from_checked_context(&self) -> Result<bool, Error> {
        let owner = self.lifecycle_animation(2)?;
        let animation = owner.lock().map_err(|_| Error::Lock)?;
        let player = animation
            .player()
            .ok_or(Error::MissingOwner("SkaterAnim player"))?;
        if player.flags_known_mask & 0x2000_0000 == 0 {
            return Err(Error::MissingOwner("SkaterAnim riding fakie flag"));
        }
        Ok(player.flags_bits & 0x2000_0000 != 0)
    }

    /// PhysicsSpeedCompare.ActivationFunc82c99808: actual Motion+a4/a8.
    pub(crate) fn motion_runtime_speed_compare(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        comparison: Comparison,
        along_skate_z: bool,
    ) -> Result<bool, Error> {
        self.check_motion_runtime_context(context)?;
        self.runtime_speed_compare_from_checked_context(comparison, along_skate_z)
    }

    /// Shared native `PhysicsSpeedCompare::ActivationFunc` 0x82c99808.
    /// The caller verifies Action or Motion context before this owner read.
    pub(crate) fn runtime_speed_compare_from_checked_context(
        &self,
        comparison: Comparison,
        along_skate_z: bool,
    ) -> Result<bool, Error> {
        let physical = self.cached_physical_allocation().map_err(Error::Physical)?;
        let output = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?;
        let value = if along_skate_z {
            output.try_motion_z_speed()
        } else {
            output.try_motion_surface_speed()
        }
        .map_err(Error::PhysOut)?;
        Ok(crate::generated_action_load::conditions::compare(
            comparison, value,
        ))
    }

    /// ComVelCompare::ActivationFunc82c99920 resolves IPhysical GetOutput,
    /// reads SystemReckoning's live COM velocity and compares its chosen axis.
    pub(crate) fn motion_runtime_com_velocity_compare(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        comparison: Comparison,
        axis: crate::generated_motion_load::conditions::Axis,
    ) -> Result<bool, Error> {
        use crate::physical_player_phys_out::PhysOutSystemVector;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(Error::Physical)?;
        let output = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?;
        let velocity = output
            .try_landing_system_vector(PhysOutSystemVector::CentreOfMassVelocity)
            .map_err(Error::PhysOut)?;
        let value = com_velocity_operand(velocity, axis).map_err(Error::ConditionNumeric)?;
        Ok(crate::generated_action_load::conditions::compare(
            comparison, value,
        ))
    }

    /// `ApexReached::ActivationFunc`82c9b460: SystemReckoning COM velocity +10,
    /// lane Y splatted into vcmpgtfp128 (zero > Y); unordered values fail.
    pub(crate) fn motion_runtime_apex_reached(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, Error> {
        use crate::physical_player_phys_out::PhysOutSystemVector;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(Error::Physical)?;
        let velocity = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_landing_system_vector(PhysOutSystemVector::CentreOfMassVelocity)
            .map_err(Error::PhysOut)?;
        Ok(0.0 > velocity.xyz[1])
    }
}

#[cfg(test)]
mod com_velocity_tests {
    use super::com_velocity_operand;
    use crate::{
        generated_motion_load::conditions::Axis, skeleton_root_transforms::SkeletonRootVector,
    };

    #[test]
    fn samples_xyz_or_native_xyz_length_without_w() {
        let velocity = SkeletonRootVector {
            xyz: [3.0, -4.0, 0.0],
            w: None,
        };
        assert_eq!(com_velocity_operand(velocity, Axis::X).unwrap(), 3.0);
        assert_eq!(com_velocity_operand(velocity, Axis::Y).unwrap(), -4.0);
        assert_eq!(com_velocity_operand(velocity, Axis::Z).unwrap(), 0.0);
        assert!((com_velocity_operand(velocity, Axis::All).unwrap() - 5.0).abs() < 0.0001);
        assert_eq!(
            com_velocity_operand(
                SkeletonRootVector::known([0.0, 0.0, 0.0, f32::NAN]),
                Axis::All
            )
            .unwrap()
            .to_bits(),
            0.0_f32.to_bits()
        );
    }
}

/// Pre8290aa50 switch; preserve short circuit and the controller's shared cache.
/// This does not admit any missing lifecycle callback or Sustain expression.
pub(super) fn expression(
    index: u16,
    condition: &mut impl FnMut(u16) -> Result<bool, Error>,
) -> Result<bool, Error> {
    Ok(match index {
        expr::MOVING_1014 => !condition(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_0_15)?, //829104a4
        expr::BRAKE_INTO => condition(cond::HAS_INTENT_BRAKE)?,                      //8291051c
        expr::LEFT_SLIDE => condition(cond::CAN_ENTER_SLIDE_NOT_RIGHT)?,             //829106b4
        expr::RIGHT_SLIDE_1168 => condition(cond::CAN_ENTER_SLIDE)?,                 //82910834
        expr::REVERT_BS => condition(cond::HAS_INTENT_SLIDE_BS_180)?, //829108f4: SlideBs180
        expr::REVERT_FS => condition(cond::HAS_INTENT_SLIDE_FS_180)?, //82910954: SlideFs180
        expr::STATIONARY_1015 => condition(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_0_15)?, //829105f4
        expr::PUSH_1028 => !condition(cond::DISABLE_PUSH_BRAKE)?,     //82911434
        expr::TO_ANTIC_CYC => condition(cond::WILL_EXPIRE_IN_0_07_NOT_WAIT)?, //82911688: WillExpire InTime=.07
        expr::IDLE_1025_TO_SWITCH_1031 => {
            condition(cond::IS_RIDING_FAKIE)?
                && (condition(cond::IS_RIDING_SWITCH)?
                    || condition(cond::IN_PARENT_STATE_FOR_TIME_GREATER_0_6)?)
        } //8291a6d4
        expr::MANUAL_1027 => {
            condition(cond::HAS_INTENT_MANUAL_ENGAGE_TIME_GREATER_0_2)?
                && (!condition(cond::LAST_STATE_IN_AIR)?
                    || !condition(cond::COM_VEL_COMPARE_AXIS_Y_LESS_NEG_8)?
                    || !condition(cond::SURFACE_SLOPE_GREATER_45)?)
        } //8291bb74
        expr::FROM_FAKIE_1016 => {
            condition(cond::IS_RIDING_FAKIE)? && condition(cond::HAS_INTENT_KICK_TURN_2ND)?
        } //829159b4
        expr::RIGHT_1017 => {
            condition(cond::HAS_INTENT_KICK_TURN_GREATER_0)?
                && condition(cond::HAS_INTENT_KICK_TURN)?
        } //82915a7c
        expr::LEFT_1018 => {
            condition(cond::HAS_INTENT_KICK_TURN_LESS_0)? && condition(cond::HAS_INTENT_KICK_TURN)?
        } //82915b48
        expr::KICK_TURN_715 => {
            condition(cond::HAS_INTENT_NEW_KICK_TURN)?
                || (!condition(cond::LAST_STATE_TURNING_IDLE)?
                    && !condition(cond::LAST_STATE_KICK_TURN)?)
        } //8291a5ac
        expr::TO_WIPE_OUT => {
            condition(cond::HAS_INTENT_WIPE_OUT_REQUEST)?
                || condition(cond::IS_PHYSICS_WIPING)?
                || condition(cond::PHYSICS_WANTS_WIPE_OUT)?
                || condition(cond::PHYS_FILTERED_STATE_WIPEOUT)?
        } //8291ad2c
        expr::AGGRESSIVE_OR_LOOSE_1169 => {
            condition(cond::IS_PRO_SKATER_LOOSE)?
                || condition(cond::IS_PRO_SKATER_AGGRESSIVE)?
                || condition(cond::IS_PRO_SKATER_MARK_GONZALES_2ND)?
                || condition(cond::IS_PRO_SKATER_DANNY_WAY)?
                || condition(cond::IS_PRO_SKATER_JERRY_HSU_2ND)?
        } //8291cc38
        _ => return Err(Error::PendingExpression(index)),
    })
}

fn declaration(states: &[StateDeclaration], index: u16) -> Result<&StateDeclaration, Error> {
    states
        .get(usize::from(index))
        .ok_or(Error::InvalidState(index))
}

/// GetSubState8292a890/82955eb0. Search immediate children, then ancestors;
/// dotted paths resolve their prefix first and retain searchparent=true.
pub(super) fn substate(
    states: &[StateDeclaration],
    mut parent: u16,
    name: &str,
) -> Result<u16, Error> {
    if parent == NULL_INDEX {
        parent = 0;
    }
    if let Some((prefix, suffix)) = name.rsplit_once('.')
        && !prefix.is_empty()
    {
        parent = substate(states, parent, prefix)?;
        if parent == NULL_INDEX {
            return Ok(NULL_INDEX);
        }
        return substate(states, parent, suffix);
    }
    loop {
        let state = declaration(states, parent)?;
        let children = state.children;
        for offset in 0..children.count {
            let child = children.index.checked_add(offset).ok_or(Error::Capacity)?;
            if declaration(states, child)?.name == name {
                return Ok(child);
            }
        }
        parent = state.parent;
        if parent == NULL_INDEX {
            return Ok(NULL_INDEX);
        }
    }
}

/// `LastState82c94a00` and `CurrentState82c94ae0` resolve relative to parent,
/// then ask whether the selected retained leaf descends from that target.
/// Invoke inside the already-held controller; never acquire it again here.
pub(crate) fn state_matches(
    states: &[StateDeclaration],
    parent: u16,
    mut leaf: u16,
    name: &str,
) -> Result<bool, Error> {
    let target = substate(states, parent, name)?;
    while leaf != NULL_INDEX {
        if leaf == target {
            return Ok(true);
        }
        leaf = declaration(states, leaf)?.parent;
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_idle_transitions_preserve_lazy_reads() {
        for (index, true_conditions, expected, expected_reads) in [
            (253, vec![], true, vec![18]),
            (253, vec![18], false, vec![18]),
            (414, vec![], false, vec![14]),
            (414, vec![14, 337], true, vec![14, 337]),
            (414, vec![14, 338], true, vec![14, 337, 338]),
            (414, vec![14], false, vec![14, 337, 338]),
            (429, vec![], false, vec![345]),
            (429, vec![345], true, vec![345, 346]),
            (429, vec![345, 346], true, vec![345, 346, 347]),
            (429, vec![345, 346, 347], true, vec![345, 346, 347, 348]),
            (
                429,
                vec![345, 346, 347, 348],
                false,
                vec![345, 346, 347, 348],
            ),
        ] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(index, &mut |condition| {
                    reads.push(condition);
                    Ok(true_conditions.contains(&condition))
                })
                .unwrap(),
                expected
            );
            assert_eq!(reads, expected_reads);
        }
        for (index, failure) in [(253, 18), (414, 337), (429, 347)] {
            assert!(matches!(
                expression(index, &mut |condition| {
                    if condition == failure {
                        Err(Error::MissingOwner("native condition"))
                    } else {
                        Ok(true)
                    }
                }),
                Err(Error::MissingOwner("native condition"))
            ));
        }
    }

    #[test]
    fn anticipation_transition_uses_only_cached_will_expire_381() {
        for (answer, expected) in [(false, false), (true, true)] {
            let mut reads = Vec::new();
            assert_eq!(
                expression(259, &mut |index| {
                    reads.push(index);
                    Ok(answer)
                })
                .unwrap(),
                expected
            );
            assert_eq!(reads, [381]);
        }
        assert!(matches!(
            expression(259, &mut |index| {
                assert_eq!(index, 381);
                Err(Error::MissingOwner("WillExpire"))
            }),
            Err(Error::MissingOwner("WillExpire"))
        ));
    }

    #[test]
    fn wipeout_pre_preserves_lazy_conditions_and_failures() {
        for stop in 2..=6 {
            let mut reads = Vec::new();
            let result = expression(420, &mut |index| {
                reads.push(index);
                Ok(index == stop)
            })
            .unwrap();
            assert_eq!(result, stop <= 5);
            assert_eq!(reads, (2..=stop.min(5)).collect::<Vec<_>>());
        }
        for stop in 2..=5 {
            let mut reads = Vec::new();
            assert!(matches!(
                expression(420, &mut |index| {
                    reads.push(index);
                    if index == stop {
                        Err(Error::MissingOwner("live wipeout condition"))
                    } else {
                        Ok(false)
                    }
                }),
                Err(Error::MissingOwner("live wipeout condition"))
            ));
            assert_eq!(reads, (2..=stop).collect::<Vec<_>>());
        }
    }

    #[test]
    fn kickturn_short_circuit_does_not_consume_later_conditions() {
        for (true_condition, expected, calls) in [
            (Some(290), true, vec![290]),
            (Some(312), false, vec![290, 312]),
            (Some(313), false, vec![290, 312, 313]),
            (None, true, vec![290, 312, 313]),
        ] {
            let mut actual = Vec::new();
            let result = expression(413, &mut |index| {
                actual.push(index);
                Ok(Some(index) == true_condition)
            })
            .unwrap();
            assert_eq!(result, expected);
            assert_eq!(actual, calls);
        }
    }

    #[test]
    fn kickturn_propagates_reached_failure_and_other_expressions_stay_pending() {
        assert!(matches!(
            expression(413, &mut |index| {
                if index == 312 {
                    Err(Error::InvalidState(7))
                } else {
                    Ok(false)
                }
            }),
            Err(Error::InvalidState(7))
        ));
        assert!(matches!(
            expression(228, &mut |_| panic!("unsupported")),
            Err(Error::PendingExpression(228))
        ));
    }

    #[test]
    fn kickturn_children_preserve_first_operand_and_presence_check() {
        for (expression_index, first, second) in [(346, 14, 314), (347, 316, 291), (348, 317, 291)]
        {
            let mut calls = Vec::new();
            assert!(
                !expression(expression_index, &mut |index| {
                    calls.push(index);
                    Ok(false)
                })
                .unwrap()
            );
            assert_eq!(calls, [first]);
            calls.clear();
            assert!(
                expression(expression_index, &mut |index| {
                    calls.push(index);
                    Ok(true)
                })
                .unwrap()
            );
            assert_eq!(calls, [first, second]);
        }
    }

    #[test]
    fn original_idle_children_select_default_without_special_pro_match() {
        let states = &crate::generated_motion_load::data::STATES;
        let idle = &states[1025];
        let mut calls = Vec::new();
        let mut selected = NULL_INDEX;
        for index in idle.children.index..idle.children.index + idle.children.count {
            let child = &states[usize::from(index)];
            if child.active
                && (child.expression == NULL_INDEX
                    || expression(child.expression, &mut |condition| {
                        calls.push(condition);
                        Ok(false)
                    })
                    .unwrap())
            {
                selected = index;
                break;
            }
        }
        assert_eq!(selected, 1170);
        assert_eq!(calls, [331, 332, 333, 334, 335]);
        calls.clear();
        assert!(
            expression(437, &mut |index| {
                calls.push(index);
                Ok(index == 332)
            })
            .unwrap()
        );
        assert_eq!(calls, [331, 332]);
    }

    #[test]
    fn original_table_resolves_relative_names_and_last_descendants() {
        let states = &crate::generated_motion_load::data::STATES;
        assert_eq!(substate(states, 715, "Turning.Idle").unwrap(), 1025);
        assert_eq!(substate(states, 715, "KickTurn").unwrap(), 715);
        assert!(state_matches(states, 715, 1016, "KickTurn").unwrap());
        assert!(state_matches(states, 715, 1169, "Turning.Idle").unwrap());
        assert!(!state_matches(states, 715, 6, "Turning.Idle").unwrap());
        assert!(!state_matches(states, 715, NULL_INDEX, "KickTurn").unwrap());
        assert!(!state_matches(states, 715, 1016, "Kick").unwrap());
    }
}

#[cfg(test)]
mod revert_expression_tests {
    use super::*;

    #[test]
    fn revert_pre_uses_actual_separate_motion_intent_conditions() {
        for (index, expected_condition) in [(220, 318), (224, 324), (226, 329), (227, 330)] {
            for expected in [false, true] {
                let mut reads = Vec::new();
                assert_eq!(
                    expression(index, &mut |condition| {
                        reads.push(condition);
                        Ok(expected)
                    })
                    .unwrap(),
                    expected
                );
                assert_eq!(reads, [expected_condition]);
            }
            assert!(matches!(
                expression(index, &mut |condition| {
                    assert_eq!(condition, expected_condition);
                    Err(Error::MissingOwner("Motion intents"))
                }),
                Err(Error::MissingOwner("Motion intents"))
            ));
        }
    }
}
