//! Actual retained `HiLOD` `StateChanger` (+300), constructed at 82e04204.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GRIND, PHYSICS_STATE_NONSPECIFIC,
};
use crate::input_cvar_runtime::{InputCvarError, InputCvarReferences};
use crate::physical_player_assembly::PhysicalPlayerAssemblyOwner;
use std::sync::{Arc, Weak};

#[path = "physical_player_state_changer_attributes.rs"]
pub(crate) mod attributes;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StateChangerError {
    AlreadyAttempted,
    NotConstructed,
}

#[derive(Debug)]
pub(crate) enum StateChangerTeleportResetError {
    NotConstructed,
    Cvar(InputCvarError),
}

pub(crate) struct PhysicalPlayerStateChangerOwner {
    attempted: bool,
    pub(crate) owner: Option<Weak<PhysicalPlayerAssemblyOwner>>,
    pub(crate) fields: Option<StateChangerFields>,
    pub(crate) attributes: Option<attributes::StateChangerAttributes>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StateChangerFields {
    pub(crate) current: Option<i32>,
    pub(crate) nonspecific_free_frames: i32,
    pub(crate) nonspecific_collision_frames: i32,
    pub(crate) something_colliding_frames: i32,
    pub(crate) two_wheel_counter: i32,
    pub(crate) three_wheel_counter: i32,
    pub(crate) post_grind_jump_counter: i32,
    pub(crate) air_frames: i32,
    pub(crate) teleport_countdown: i32,
    pub(crate) revert_exited_normally: bool,
    pub(crate) request_teleport: bool,
}

impl PhysicalPlayerStateChangerOwner {
    /// TeleportReset82dd3a30 clears seven counters, then samples the live
    /// countdown target. Current state and request flags are left intact.
    pub(crate) fn try_teleport_reset(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<(), StateChangerTeleportResetError> {
        use StateChangerTeleportResetError as E;
        let fields = self.fields.as_mut().ok_or(E::NotConstructed)?;
        fields.nonspecific_free_frames = 0;
        fields.nonspecific_collision_frames = 0;
        fields.something_colliding_frames = 0;
        fields.two_wheel_counter = 0;
        fields.three_wheel_counter = 0;
        fields.post_grind_jump_counter = 0;
        fields.air_frames = 0;
        fields.teleport_countdown = cvars.try_frames_for_teleport_countdown().map_err(E::Cvar)?;
        Ok(())
    }

    pub(crate) const fn new() -> Self {
        Self {
            attempted: false,
            owner: None,
            fields: None,
            attributes: None,
        }
    }

    /// 82dd39f0..3a2c links SAME PhysOut/Processed/board/Skeleton, then zeros
    /// counters and two flags. Current-state +10 is unwritten. `PhysicsSkater`
    /// allocation82e041e8 does not have Sim's whole-allocation clearing policy.
    pub(crate) fn try_construct(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        allocated: bool,
    ) -> Result<(), StateChangerError> {
        if self.attempted {
            return Err(StateChangerError::AlreadyAttempted);
        }
        self.attempted = true;
        if allocated {
            // The outer permits one Skeleton construction occurrence; its Body
            // admission comes later. Do not require a post-Init handle here.
            self.owner = Some(Arc::downgrade(owner));
            self.fields = Some(StateChangerFields::constructed());
            self.attributes = Some(attributes::StateChangerAttributes::bind(
                &owner.inputs().vault,
            ));
        }
        Ok(())
    }
}

impl StateChangerFields {
    pub(crate) const fn constructed() -> Self {
        Self {
            current: None,
            nonspecific_free_frames: 0,
            nonspecific_collision_frames: 0,
            something_colliding_frames: 0,
            two_wheel_counter: 0,
            three_wheel_counter: 0,
            post_grind_jump_counter: 0,
            air_frames: 0,
            teleport_countdown: 0,
            revert_exited_normally: false,
            request_teleport: false,
        }
    }

    /// CalcSuggestedState82ddbd70..94: precedes the virtual board-body read.
    pub(crate) fn begin(&mut self, current: i32) {
        if self.teleport_countdown > 0 {
            self.teleport_countdown -= 1;
        }
        self.current = Some(current);
        self.revert_exited_normally = false;
        self.request_teleport = false;
    }

    /// Ordered counter stores82ddbe5c..bf18. Native integer additions wrap.
    pub(crate) fn update_contact_counters(
        &mut self,
        current: i32,
        category: i32,
        wheels: u8,
        something_colliding: bool,
        grind_jump: bool,
    ) {
        self.nonspecific_free_frames = if current == PHYSICS_STATE_NONSPECIFIC && !something_colliding {
            self.nonspecific_free_frames.wrapping_add(1)
        } else {
            0
        };
        self.nonspecific_collision_frames = if current == PHYSICS_STATE_NONSPECIFIC && something_colliding {
            self.nonspecific_collision_frames.wrapping_add(1)
        } else {
            0
        };
        self.two_wheel_counter = if wheels > 1 {
            self.two_wheel_counter.wrapping_add(1)
        } else {
            0
        };
        self.three_wheel_counter = if wheels > 2 {
            self.three_wheel_counter.wrapping_add(1)
        } else {
            0
        };
        self.something_colliding_frames = if something_colliding {
            self.something_colliding_frames.wrapping_add(1)
        } else {
            0
        };
        self.post_grind_jump_counter = if category == PHYSICS_CATEGORY_GRIND && grind_jump {
            0
        } else {
            self.post_grind_jump_counter.wrapping_add(1)
        };
        self.air_frames = if category == PHYSICS_CATEGORY_AIR {
            self.air_frames.wrapping_add(1)
        } else {
            0
        };
    }
}

#[cfg(test)]
#[path = "physical_player_state_changer_tests.rs"]
mod tests;
