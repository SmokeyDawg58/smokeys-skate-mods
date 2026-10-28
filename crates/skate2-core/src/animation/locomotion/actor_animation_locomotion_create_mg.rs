//! CreateLocomotionTrajectory82cac998 MG scan and speed prefix82cacc14..cafe0.
//! Draft child of actor_animation_construction; actual adapter is still required.
use super::ActorAnimationConstruction;
use crate::play_animation::FastString30;

pub(super) trait CreateMotionTuning {
    type Error;
    /// GlobalAttributeCollection+16c, key320f3450e99ab013.
    fn sprint_increment(&mut self) -> Result<f32, Self::Error>;
    /// GlobalAttributeCollection+16c, keyf2cdca4243861327.
    fn sprint_decrement(&mut self) -> Result<f32, Self::Error>;
    fn use_sprint_button(&mut self) -> Result<bool, Self::Error>;
    /// This Actor's +6190 attribute instance, key9a266da3b97d0018, PointGraph8.
    fn desired_speed_graph(&mut self, speed: f32) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(super) enum CreateMotionError<E> {
    Missing(&'static str),
    UnknownFlags(u32),
    Tuning(E),
}
#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct CreateMotionActions {
    pub(super) consumed_20000: bool,
    pub(super) consumed_8000: bool,
    pub(super) consumed_4000: bool,
    pub(super) consumed_2000: bool,
}
fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
impl<K, W> ActorAnimationConstruction<K, W> {
    /// Called after collision-offset update and mirrored-world scratch creation.
    /// The iterator must be the actual retained MG attribute order; no sorting.
    pub(super) fn try_create_motion_prefix<T: CreateMotionTuning>(
        &mut self,
        attributes: impl IntoIterator<Item = (FastString30, f32)>,
        tuning: &mut T,
    ) -> Result<CreateMotionActions, CreateMotionError<T::Error>> {
        use CreateMotionError as E;
        let flags = self
            .player
            .as_mut()
            .ok_or(E::Missing("SkaterAnim player"))?;
        let fields = self
            .before_sprint
            .as_mut()
            .ok_or(E::Missing("locomotion fields"))?;
        let suffix = self
            .before_face
            .as_mut()
            .ok_or(E::Missing("locomotion suffix"))?;
        fields.second_flags_bits &= !0x4000_0000;
        fields.second_flags_known_mask |= 0x4000_0000;
        flags.flags_bits &= !0x10000;
        flags.flags_known_mask |= 0x10000;
        let mut sprint = false;
        let mut actions = CreateMotionActions::default();
        // Literal initializers82f7f9c8/e0/f9f8/fa10, not symbol inference.
        let jump = FastString30::encode("OB_JUMP").expect("literal");
        let mounting = FastString30::encode("OB_MOUNTING").expect("literal");
        let speed = FastString30::encode("Speed").expect("literal");
        let sprint_name = FastString30::encode("OB_Sprint").expect("literal");
        for (name, value) in attributes {
            if name == jump {
                flags.flags_bits |= 0x1000;
                flags.flags_known_mask |= 0x1000;
                break;
            }
            if name == mounting {
                flags.flags_bits |= 0x10000;
                break;
            }
            if name == speed {
                suffix.desired_speed = value;
            } else if name == sprint_name {
                // Native GT is strict; unordered takes the increment.
                let increment = if suffix.speed_clamp > f32::from_bits(0x3f733333) {
                    0.0
                } else {
                    f32::from_bits(0x3d4ccccd)
                };
                suffix.speed_clamp += increment;
                sprint = true;
                fields.second_flags_bits |= 0x4000_0000;
            } else {
                // Source checks these retained event bits only for an otherwise
                // unrecognized row. Each row consumes at most one then stops.
                for (mask, consumed) in [
                    (0x20000, &mut actions.consumed_20000),
                    (0x8000, &mut actions.consumed_8000),
                    (0x4000, &mut actions.consumed_4000),
                    (0x2000, &mut actions.consumed_2000),
                ] {
                    if flags.flags_known_mask & mask == 0 {
                        return Err(E::UnknownFlags(mask));
                    }
                    if flags.flags_bits & mask != 0 {
                        flags.flags_bits &= !mask;
                        *consumed = true;
                        break;
                    }
                }
                if actions != CreateMotionActions::default() {
                    break;
                }
            }
        }
        suffix.sprint_level = if sprint {
            tuning.sprint_increment().map_err(E::Tuning)? + suffix.sprint_level
        } else {
            suffix.sprint_level - tuning.sprint_decrement().map_err(E::Tuning)?
        };
        let lower = select(-suffix.sprint_level, 0.0, suffix.sprint_level);
        suffix.sprint_level = select(1.0 - lower, lower, 1.0);
        if tuning.use_sprint_button().map_err(E::Tuning)? {
            if fields.second_flags_bits & 0x4000_0000 != 0 {
                suffix.desired_speed = 1.0;
            } else {
                let decrement = if suffix.speed_clamp < f32::from_bits(0x3f59999a) {
                    0.0
                } else {
                    f32::from_bits(0x3d4ccccd)
                };
                suffix.speed_clamp -= decrement;
            }
        }
        if flags.flags_known_mask & 4 == 0 {
            return Err(E::UnknownFlags(4));
        }
        if flags.flags_bits & 4 != 0 {
            flags.flags_bits &= !0x40000;
            flags.flags_known_mask |= 0x40000;
        }
        let minimum = -suffix.speed_clamp;
        let lower = select(
            minimum - suffix.desired_speed,
            minimum,
            suffix.desired_speed,
        );
        suffix.desired_speed = select(suffix.speed_clamp - lower, lower, suffix.speed_clamp);
        suffix.desired_speed = tuning
            .desired_speed_graph(suffix.desired_speed)
            .map_err(E::Tuning)?;
        Ok(actions)
    }
}
