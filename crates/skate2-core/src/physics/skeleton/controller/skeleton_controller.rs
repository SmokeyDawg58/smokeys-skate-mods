//! Retained selected `SkeletonController` constructor and collision dispatch.
//! Source constructor is inline 0x82e04494..44ac. The actual same-pair bridge
//! owns the processed/Skeleton association and supplies concrete mutations.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected controller observations await full physical state scheduling"
    )
)]
use std::fmt;

#[path = "skeleton_controller_air_update.rs"]
pub(crate) mod air_update;

#[cfg(test)]
#[path = "skeleton_controller_tests.rs"]
mod tests;

const STANDARD: i32 = 6;
const WORLD_ONLY_ONBOARD: i32 = 2;
const WORLD_ONLY_OFFBOARD: i32 = 1;
const OFFBOARD: i32 = 4;

/// Both methods must mutate the SAME normal Skeleton's runtime headers/groups
/// and existing post-record proxy flag. No default successful implementation.
/// The enclosing bridge invalidates actual publication before possible writes.
pub(crate) trait SkeletonControllerCollision {
    type Error;
    fn set_ground_collision(&mut self) -> Result<(), Self::Error>;
    fn collide_with_world_only(&mut self) -> Result<(), Self::Error>;
    fn collide_with_world_only_offboard(
        &mut self,
    ) -> Result<(), SkeletonControllerError<Self::Error>> {
        Err(SkeletonControllerError::PendingCollisionMode(1))
    }
    fn set_air_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        Err(SkeletonControllerError::PendingCollisionMode(7))
    }
    fn set_offboard_air_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        Err(SkeletonControllerError::PendingCollisionMode(5))
    }
    /// Needs the owner's live cvar references, sampled between Part stores.
    fn set_offboard_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        Err(SkeletonControllerError::PendingCollisionMode(4))
    }
    /// A caller without actual dynamics/readers retains the unsupported boundary.
    fn set_ragdoll_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        Err(SkeletonControllerError::PendingCollisionMode(8))
    }
}

/// Origin of selected controller fields, not completion of the whole `HiLOD`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonControllerOrigin {
    Constructor,
    Transition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SkeletonControllerValue<T> {
    pub(crate) value: T,
    pub(crate) origin: SkeletonControllerOrigin,
}
impl<T> SkeletonControllerValue<T> {
    fn constructor(value: T) -> Self {
        Self {
            value,
            origin: SkeletonControllerOrigin::Constructor,
        }
    }
    fn transition(value: T) -> Self {
        Self {
            value,
            origin: SkeletonControllerOrigin::Transition,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkeletonControllerError<E> {
    AlreadyConstructed,
    Unconstructed,
    Collision(E),
    /// Source assertion103: world-only flag false but current state1 or2.
    CurrentWorldOnlyState(i32),
    /// Source assertion72/92 after storedState/storedValid publication.
    InvalidWorldOnlyRequest(i32),
    /// `EnterState`'s unsupported enum assertion after its current-state store.
    InvalidCollisionState(i32),
    /// Source `EnterState` published mState, but this mode's real mutation has
    /// not been implemented. Never an empty successful mode transition.
    PendingCollisionMode(i32),
    /// A cvar sampled inside a mode mutation was unavailable; its prefix stays.
    Cvar(crate::input_cvar_runtime::InputCvarError),
}
impl<E: fmt::Display> fmt::Display for SkeletonControllerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyConstructed => {
                f.write_str("SkeletonController constructor already executed")
            }
            Self::Unconstructed => f.write_str("SkeletonController constructor has not executed"),
            Self::Collision(e) => write!(f, "SkeletonController collision mutation: {e}"),
            Self::CurrentWorldOnlyState(state) => write!(
                f,
                "SkeletonController inconsistent current world-only state {state}"
            ),
            Self::InvalidWorldOnlyRequest(state) => {
                write!(f, "SkeletonController invalid world-only request {state}")
            }
            Self::InvalidCollisionState(state) => {
                write!(f, "SkeletonController invalid collision state {state}")
            }
            Self::PendingCollisionMode(state) => write!(
                f,
                "SkeletonController collision mode {state} continuation unavailable"
            ),
            Self::Cvar(error) => write!(f, "SkeletonController collision cvar: {error}"),
        }
    }
}

pub(crate) struct SkeletonController {
    current: Option<SkeletonControllerValue<i32>>,
    stored: Option<SkeletonControllerValue<i32>>,
    stored_valid: Option<SkeletonControllerValue<bool>>,
    world_only: Option<SkeletonControllerValue<bool>>,
    need_air_collision: Option<SkeletonControllerValue<bool>>,
}
impl SkeletonController {
    /// ResetSystems 0x82dfffb4..c4 performs five inline stores on the
    /// retained controller; it does not re-enter collision dispatch.
    pub(crate) fn reset_after_teleport(&mut self) {
        self.current = Some(SkeletonControllerValue::transition(0));
        self.stored = Some(SkeletonControllerValue::transition(0));
        self.stored_valid = Some(SkeletonControllerValue::transition(false));
        self.world_only = Some(SkeletonControllerValue::transition(false));
        self.need_air_collision = Some(SkeletonControllerValue::transition(false));
    }
    pub(crate) fn new() -> Self {
        Self {
            current: None,
            stored: None,
            stored_valid: None,
            world_only: None,
            need_air_collision: None,
        }
    }
    pub(crate) fn current(&self) -> Option<SkeletonControllerValue<i32>> {
        self.current
    }
    pub(crate) fn stored(&self) -> Option<SkeletonControllerValue<i32>> {
        self.stored
    }
    pub(crate) fn stored_valid(&self) -> Option<SkeletonControllerValue<bool>> {
        self.stored_valid
    }
    pub(crate) fn world_only(&self) -> Option<SkeletonControllerValue<bool>> {
        self.world_only
    }
    pub(crate) fn need_air_collision(&self) -> Option<SkeletonControllerValue<bool>> {
        self.need_air_collision
    }

    /// `InitAirCollisionUpdate`82dd9fb8 stores need-air before requesting state7.
    pub(crate) fn try_init_air_collision<S: SkeletonControllerCollision>(
        &mut self,
        collision: &mut S,
    ) -> Result<(), SkeletonControllerError<S::Error>> {
        self.current.ok_or(SkeletonControllerError::Unconstructed)?;
        self.need_air_collision = Some(SkeletonControllerValue::transition(true));
        self.try_set_collision_state(7, collision)
    }

    /// Enclosing owner binds actual processed/Skeleton references at44a0/44a4.
    /// This selected constructor has no fallible native subcall after allocation.
    pub(crate) fn try_initialize_constructor<E>(
        &mut self,
    ) -> Result<(), SkeletonControllerError<E>> {
        if self.current.is_some() {
            return Err(SkeletonControllerError::AlreadyConstructed);
        }
        self.current = Some(SkeletonControllerValue::constructor(0));
        self.stored = Some(SkeletonControllerValue::constructor(0));
        self.stored_valid = Some(SkeletonControllerValue::constructor(false));
        self.world_only = Some(SkeletonControllerValue::constructor(false));
        self.need_air_collision = Some(SkeletonControllerValue::constructor(false));
        Ok(())
    }

    /// `SetSkeletonCollisionState`0x82dd9e70. Raw selected state remains observable
    /// on failure; only Ok authorizes completion of this selected transition.
    pub(crate) fn try_set_collision_state<S: SkeletonControllerCollision>(
        &mut self,
        requested: i32,
        collision: &mut S,
    ) -> Result<(), SkeletonControllerError<S::Error>> {
        use SkeletonControllerError as E;
        let world_only = self.world_only.ok_or(E::Unconstructed)?.value;
        if world_only {
            self.stored = Some(SkeletonControllerValue::transition(requested));
            self.stored_valid = Some(SkeletonControllerValue::transition(true));
            let target = match requested {
                OFFBOARD => WORLD_ONLY_OFFBOARD,
                5..=7 => WORLD_ONLY_ONBOARD,
                8 => 3,
                _ => return Err(E::InvalidWorldOnlyRequest(requested)),
            };
            self.enter(target, collision)
        } else {
            let current = self.current.expect("constructed current state").value;
            if current == 1 || current == 2 {
                return Err(E::CurrentWorldOnlyState(current));
            }
            self.enter(requested, collision)
        }
    }

    /// `StartCollideWithWorldOnly`0x82dd9db8. Stored state survives a failed Enter;
    /// worldOnly=true is published only after the selected collision mutation.
    pub(crate) fn try_start_world_only<S: SkeletonControllerCollision>(
        &mut self,
        collision: &mut S,
    ) -> Result<(), SkeletonControllerError<S::Error>> {
        let current = self
            .current
            .ok_or(SkeletonControllerError::Unconstructed)?
            .value;
        self.stored_valid = Some(SkeletonControllerValue::transition(true));
        self.stored = Some(SkeletonControllerValue::transition(current));
        let target = if current == OFFBOARD {
            WORLD_ONLY_OFFBOARD
        } else {
            WORLD_ONLY_ONBOARD
        };
        self.enter(target, collision)?;
        self.world_only = Some(SkeletonControllerValue::transition(true));
        Ok(())
    }

    /// `StopCollideWithWorldOnly`0x82dd9e10 captures validity before clearing the
    /// world-only flag, and clears validity only after a successful stored Enter.
    pub(crate) fn try_stop_world_only<S: SkeletonControllerCollision>(
        &mut self,
        collision: &mut S,
    ) -> Result<(), SkeletonControllerError<S::Error>> {
        let valid = self
            .stored_valid
            .ok_or(SkeletonControllerError::Unconstructed)?
            .value;
        self.world_only = Some(SkeletonControllerValue::transition(false));
        if valid {
            let stored = self.stored.expect("constructed stored state").value;
            self.enter(stored, collision)?;
            self.stored_valid = Some(SkeletonControllerValue::transition(false));
        } else {
            self.enter(STANDARD, collision)?;
        }
        Ok(())
    }

    fn enter<S: SkeletonControllerCollision>(
        &mut self,
        state: i32,
        collision: &mut S,
    ) -> Result<(), SkeletonControllerError<S::Error>> {
        self.current = Some(SkeletonControllerValue::transition(state));
        match state {
            WORLD_ONLY_OFFBOARD => collision.collide_with_world_only_offboard(),
            STANDARD => collision
                .set_ground_collision()
                .map_err(SkeletonControllerError::Collision),
            WORLD_ONLY_ONBOARD => collision
                .collide_with_world_only()
                .map_err(SkeletonControllerError::Collision),
            // EnterState82dd488c: ragdoll setup before world-only groups.
            // The selected current state above survives either failing subcall.
            3 => {
                collision.set_ragdoll_collision()?;
                collision
                    .collide_with_world_only()
                    .map_err(SkeletonControllerError::Collision)
            }
            8 => collision.set_ragdoll_collision(),
            7 => collision.set_air_collision(),
            5 => collision.set_offboard_air_collision(),
            4 => collision.set_offboard_collision(),
            _ => Err(SkeletonControllerError::InvalidCollisionState(state)),
        }
    }
}
