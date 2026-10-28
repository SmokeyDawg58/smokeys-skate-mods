//! `SkeletonController::UpdateAirCollision`0x82dd9fc8 on the retained controller.
use super::{
    SkeletonController, SkeletonControllerCollision, SkeletonControllerError,
    SkeletonControllerValue,
};

/// Raw `lhz +8c8; clrlwi ...,31` selects this bit of the big-endian word.
pub(crate) const AIR_COLLISION_FLAG: u32 = 0x0001_0000;

pub(crate) trait AirCollisionInput {
    type Error;
    fn flags_8c8(&mut self) -> Result<u32, Self::Error>;
    fn velocity_y(&mut self) -> Result<f32, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub(crate) enum AirCollisionError<I, C> {
    Input(I),
    Controller(SkeletonControllerError<C>),
}
impl<I: std::fmt::Display, C: std::fmt::Display> std::fmt::Display for AirCollisionError<I, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(e) => write!(f, "Air collision input: {e}"),
            Self::Controller(e) => e.fmt(f),
        }
    }
}
impl SkeletonController {
    /// `82dd9fd8..82dda05c`: lazy input, changed-only latch, then collision state.
    pub(crate) fn try_update_air_collision<I: AirCollisionInput, C: SkeletonControllerCollision>(
        &mut self,
        input: &mut I,
        collision: &mut C,
    ) -> Result<(), AirCollisionError<I::Error, C::Error>> {
        let previous = self
            .need_air_collision
            .ok_or(AirCollisionError::Controller(
                SkeletonControllerError::Unconstructed,
            ))?
            .value;
        let needed = input.flags_8c8().map_err(AirCollisionError::Input)? & AIR_COLLISION_FLAG != 0
            || input.velocity_y().map_err(AirCollisionError::Input)? > 0.0;
        if needed != previous {
            self.need_air_collision = Some(SkeletonControllerValue::transition(needed));
            self.try_set_collision_state(if needed { 7 } else { 6 }, collision)
                .map_err(AirCollisionError::Controller)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Input {
        flags: u32,
        velocity: Result<f32, &'static str>,
        reads: usize,
    }
    impl AirCollisionInput for Input {
        type Error = &'static str;
        fn flags_8c8(&mut self) -> Result<u32, Self::Error> {
            Ok(self.flags)
        }
        fn velocity_y(&mut self) -> Result<f32, Self::Error> {
            self.reads += 1;
            self.velocity
        }
    }
    #[derive(Default)]
    struct Collision {
        calls: Vec<i32>,
        fail: bool,
    }
    impl SkeletonControllerCollision for Collision {
        type Error = &'static str;
        fn set_ground_collision(&mut self) -> Result<(), Self::Error> {
            self.calls.push(6);
            if self.fail { Err("collision") } else { Ok(()) }
        }
        fn collide_with_world_only(&mut self) -> Result<(), Self::Error> {
            self.calls.push(2);
            if self.fail { Err("collision") } else { Ok(()) }
        }
        fn set_air_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
            self.calls.push(7);
            if self.fail {
                Err(SkeletonControllerError::Collision("collision"))
            } else {
                Ok(())
            }
        }
    }
    fn owner() -> SkeletonController {
        let mut owner = SkeletonController::new();
        owner.try_initialize_constructor::<&str>().unwrap();
        owner
    }
    #[test]
    fn big_endian_flag_skips_unavailable_velocity_and_unchanged_latch_skips_set() {
        let mut owner = owner();
        let mut input = Input {
            flags: 0x0001_0000,
            velocity: Err("must not read"),
            reads: 0,
        };
        let mut collision = Collision::default();
        owner
            .try_update_air_collision(&mut input, &mut collision)
            .unwrap();
        owner
            .try_update_air_collision(&mut input, &mut collision)
            .unwrap();
        assert_eq!(input.reads, 0);
        assert_eq!(collision.calls, [7]);
        assert!(owner.need_air_collision().unwrap().value);
    }
    #[test]
    fn ascending_only_and_wrong_halfword_do_not_enable_collision() {
        for (velocity, expected) in [
            (0.0, false),
            (-0.0, false),
            (-1.0, false),
            (f32::NAN, false),
            (1.0, true),
            (f32::INFINITY, true),
        ] {
            let mut owner = owner();
            let mut input = Input {
                flags: 1,
                velocity: Ok(velocity),
                reads: 0,
            };
            let mut collision = Collision::default();
            owner
                .try_update_air_collision(&mut input, &mut collision)
                .unwrap();
            assert_eq!(owner.need_air_collision().unwrap().value, expected);
            assert_eq!(input.reads, 1);
            assert_eq!(collision.calls, if expected { vec![7] } else { vec![] });
        }
    }
    #[test]
    fn failed_collision_retains_new_latch_and_state_in_both_directions() {
        let mut owner = owner();
        let mut collision = Collision {
            fail: true,
            ..Collision::default()
        };
        for (velocity, expected, state) in [(1.0, true, 7), (-1.0, false, 6)] {
            let mut input = Input {
                flags: 0,
                velocity: Ok(velocity),
                reads: 0,
            };
            assert_eq!(
                owner.try_update_air_collision(&mut input, &mut collision),
                Err(AirCollisionError::Controller(
                    SkeletonControllerError::Collision("collision")
                ))
            );
            assert_eq!(owner.need_air_collision().unwrap().value, expected);
            assert_eq!(owner.current().unwrap().value, state);
        }
        assert_eq!(collision.calls, [7, 6]);
    }
    #[test]
    fn latch_precedes_even_pre_enter_controller_failure() {
        let mut owner = owner();
        owner.current = Some(SkeletonControllerValue::transition(2));
        let mut input = Input {
            flags: AIR_COLLISION_FLAG,
            velocity: Err("unread"),
            reads: 0,
        };
        let mut collision = Collision::default();
        assert_eq!(
            owner.try_update_air_collision(&mut input, &mut collision),
            Err(AirCollisionError::Controller(
                SkeletonControllerError::CurrentWorldOnlyState(2)
            ))
        );
        assert!(owner.need_air_collision().unwrap().value);
        assert_eq!(owner.current().unwrap().value, 2);
        assert!(collision.calls.is_empty());
    }
    #[test]
    fn unavailable_velocity_preserves_latch_and_collision() {
        let mut owner = owner();
        let before = owner.need_air_collision();
        let mut input = Input {
            flags: 0,
            velocity: Err("velocity"),
            reads: 0,
        };
        let mut collision = Collision::default();
        assert_eq!(
            owner.try_update_air_collision(&mut input, &mut collision),
            Err(AirCollisionError::Input("velocity"))
        );
        assert_eq!(owner.need_air_collision(), before);
        assert!(collision.calls.is_empty());
    }
}
