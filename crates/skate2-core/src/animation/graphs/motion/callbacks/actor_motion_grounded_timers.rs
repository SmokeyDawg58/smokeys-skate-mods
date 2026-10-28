//! Proposed child of `actor_construction::motion_runtime_session`.
//! Three original instance-free callbacks; registration remains integrator-owned.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError};
use crate::actor_graph_construction::ActorSkaterGraphState;
use std::sync::Arc;

/// `UpdateManualOutTimer825fe1b0:` fsubs then fsel, not `f32::max`.
fn manual_countdown(value: f32, dt: f32) -> f32 {
    let next = value - dt;
    if next >= 0.0 { next } else { 0.0 }
}

impl ActorConstruction {
    /// GrabSlide.OnTransition82ca6ac0 retains a force-slide flag across a
    /// ground-grab transition; SetTurning consumes the same bit next update.
    pub(crate) fn motion_grab_slide_transition(&self, context: &BoundMotionGraphContext<ActorIdentity>, right: bool) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            if state.combined_slide_bits.known_mask & 0x8000_0000 == 0 {
                return Err(MotionUpdateError::MissingOwner("GrabSlide from fakie"));
            }
            let selected = right ^ (state.combined_slide_bits.bits & 0x8000_0000 != 0);
            let flags = &mut state.slide_bits[usize::from(selected)];
            flags.bits |= 0x2000_0000;
            flags.known_mask |= 0x2000_0000;
            Ok(())
        })
    }

    /// ShouldLeaveSlide82c9c918 selects by the captured from-fakie bit,
    /// then reads the selected SlideState's end bit (bit30).
    pub(crate) fn motion_runtime_should_leave_slide(&self, context: &BoundMotionGraphContext<ActorIdentity>, right: bool) -> Result<bool, MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            if state.combined_slide_bits.known_mask & 0x8000_0000 == 0 {
                return Err(MotionUpdateError::MissingOwner("ShouldLeaveSlide from fakie"));
            }
            let selected = right ^ (state.combined_slide_bits.bits & 0x8000_0000 != 0);
            let flags = state.slide_bits[usize::from(selected)];
            if flags.known_mask & 0x4000_0000 == 0 {
                return Err(MotionUpdateError::MissingOwner("ShouldLeaveSlide end flag"));
            }
            Ok(flags.bits & 0x4000_0000 != 0)
        })
    }

    /// IsPowerSliding Begin82c9fec0/End82c9ffb8 -> SetIsPowerSliding825fe0f0.
    pub(crate) fn motion_runtime_set_power_sliding(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        active: bool,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            state.flags.bits = (state.flags.bits & !0x0400_0000)
                | if active { 0x0400_0000 } else { 0 };
            state.flags.known_mask |= 0x0400_0000;
            Ok(())
        })
    }

    /// InCandidateSlidingState Begin82ca33d8/End82ca34e0. The virtual
    /// ISkaterMotionGraph+0x40 getter is GetSlidingStateReadOnly825fdfc8;
    /// SlidingState+0x10 is the combined flags, not the general Motion flags.
    pub(crate) fn motion_runtime_set_candidate_sliding(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        active: bool,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            state.combined_slide_bits.bits = (state.combined_slide_bits.bits & !0x4000_0000)
                | if active { 0x4000_0000 } else { 0 };
            state.combined_slide_bits.known_mask |= 0x4000_0000;
            Ok(())
        })
    }

    /// SetManualOutTimer.End82ca5310 -> SetManualOutTimer825fe170.
    pub(crate) fn motion_runtime_set_manual_out_timer(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        time: f32,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            state.manual_out_timer = time;
            Ok(())
        })
    }

    /// PowerSlideManualAtt.Update82cc8248 -> ISkaterMotionGraph.AddMgAttribute.
    pub(crate) fn motion_runtime_power_slide_manual_attribute(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), MotionUpdateError> {
        use crate::play_animation::{FastString30, FastString36};
        self.with_grounded_motion_state(context, |_| Ok(()))?;
        let name = FastString36::encode("Manual").expect("source intent");
        if let Some(value) = self.motion_runtime_intent_value(context, &name)? {
            // 82cc83c8 BO4/BI24 skips when LT is clear, including NaN.
            if value < 0.0 {
                self.with_grounded_motion_state(context, |state| {
                    state.try_append_motion_attribute(
                        FastString30::encode("balance").expect("source attribute"),
                        value,
                    )
                })?;
            }
        }
        Ok(())
    }

    pub(super) fn with_grounded_motion_state<T>(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        apply: impl FnOnce(&mut ActorSkaterGraphState) -> Result<T, MotionUpdateError>,
    ) -> Result<T, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let mut actor = self.prefix().map_err(E::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .filter(|_| actor.motion_graph_published)
            .ok_or(E::MissingOwner(
                "grounded callback cached ISkaterMotionGraph",
            ))?;
        if !actor.graph_interfaces[2]
            .as_ref()
            .is_some_and(|registered| Arc::ptr_eq(registered, graph.occurrence()))
        {
            return Err(E::ForeignOwner("grounded callback ISkaterMotionGraph"));
        }
        let state = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.state.as_mut())
            .ok_or(E::MissingOwner("Motion state"))?;
        apply(state)
    }

    /// `ManualOutTimerIsActive`82c9deb8 reads the retained Motion timer.
    /// Native floating comparison is strict, including false for NaN.
    pub(crate) fn motion_runtime_manual_out_timer_is_active(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| Ok(state.manual_out_timer > 0.0))
    }

    /// UpdateTimeSinceKickturn.Update82ca1290 ->825fe120 (dt + previous).
    /// dt must be the already-held controller's actual `GetDeltaTime` value.
    #[expect(
        clippy::assign_op_pattern,
        reason = "native fadds has dt as left operand"
    )]
    pub(crate) fn motion_runtime_update_time_since_kickturn(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        dt: f32,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            state.time_since_kickturn = dt + state.time_since_kickturn;
            Ok(())
        })
    }

    /// `ResetTimeSinceKickturn` Begin 0x82ca1378 and End 0x82ca1470
    /// both call SkaterMotionGraph 0x825fe110 on the retained timer.
    pub(crate) fn motion_runtime_reset_time_since_kickturn(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            state.time_since_kickturn = 0.0;
            Ok(())
        })
    }

    /// IsManualing Begin82c9fcd0/End82c9fdc8 -> SetIsManualing825fe0a0.
    /// The native setter changes only bit28 of Motion+0xca4.
    pub(crate) fn motion_runtime_set_is_manualing(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        manualing: bool,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            state.flags.bits =
                (state.flags.bits & !0x1000_0000) | if manualing { 0x1000_0000 } else { 0 };
            state.flags.known_mask |= 0x1000_0000;
            Ok(())
        })
    }

    /// UpdateManualOutTimer.Update82ca53f8 tests `IsManualing825fe090` first.
    pub(crate) fn motion_runtime_update_manual_out_timer(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        dt: f32,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            if state.flags.known_mask & 0x1000_0000 == 0 {
                return Err(MotionUpdateError::MissingOwner("Motion manualing flag"));
            }
            if state.flags.bits & 0x1000_0000 == 0 {
                state.manual_out_timer = manual_countdown(state.manual_out_timer, dt);
            }
            Ok(())
        })
    }

    /// UpdateManualOutTimer.End82ca5500 ->`ClearManualOutTimer825fe160`.
    pub(crate) fn motion_runtime_clear_manual_out_timer(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            state.manual_out_timer = 0.0;
            Ok(())
        })
    }

    /// `ApplyingBodyTilt` Begin82c9e098/End82c9e190 ->825fdfe0.
    /// Begin passes true; End passes false. Update is source BLR.
    pub(crate) fn motion_runtime_set_applying_body_tilt(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        applying: bool,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            state.flags.bits =
                (state.flags.bits & !0x8000_0000) | if applying { 0x8000_0000 } else { 0 };
            state.flags.known_mask |= 0x8000_0000;
            Ok(())
        })
    }

    /// IsAnticipating Begin82c9f700/End82c9f7f8 resolve the cached
    /// ISkaterMotionGraph before SetIsAnticipating825fe050 writes bit30.
    pub(crate) fn motion_set_is_anticipating(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        anticipating: bool,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            state.flags.bits =
                (state.flags.bits & !0x4000_0000) | if anticipating { 0x4000_0000 } else { 0 };
            state.flags.known_mask |= 0x4000_0000;
            Ok(())
        })
    }

    /// InitPush::Begin82ca09e8 resets the retained ISkaterMotionGraph push
    /// structure in source store order; its first float is left unchanged.
    pub(crate) fn motion_runtime_init_push(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), MotionUpdateError> {
        self.with_grounded_motion_state(context, |state| {
            state.push_bits.bits &= !0x8000_0000;
            state.push_bits.known_mask |= 0x8000_0000;
            state.push[1] = 0.0;
            state.push[5] = -1.0;
            state.push[6] = -1.0;
            state.push[7] = -1.0;
            state.push[2] = -1.0;
            state.push[3] = -1.0;
            state.push[4] = -1.0;
            Ok(())
        })
    }
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "exact native countdown outputs")]
mod tests {
    use super::manual_countdown;

    #[test]
    fn native_countdown_fsel_preserves_negative_zero_and_clears_unordered() {
        assert_eq!(manual_countdown(3.0, 0.5), 2.5);
        assert_eq!(manual_countdown(0.5, 3.0).to_bits(), 0);
        assert_eq!(manual_countdown(-0.0, 0.0).to_bits(), (-0.0_f32).to_bits());
        assert_eq!(manual_countdown(f32::NAN, 1.0).to_bits(), 0);
        assert_eq!(manual_countdown(1.0, f32::NAN).to_bits(), 0);
        assert_eq!(manual_countdown(f32::INFINITY, f32::INFINITY).to_bits(), 0);
        assert_eq!(manual_countdown(0.0, -1.0), 1.0);
    }
}
