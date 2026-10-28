//! The player's physics state ids under the debug executable's own names:
//! `Sk8::Physics::ePhysicsState`, what each `PhysState_*::GetType` returns and the physics output
//! publishes as `physics_state`, and `Sk8::Physics::ePhysicsCategory`, a state's hundreds, published
//! as `physics_state_category`.
#![allow(dead_code, reason = "the complete native enums; code names the values it reads")]

pub(crate) const PHYSICS_STATE_INVALID: i32 = 0;
pub(crate) const PHYSICS_STATE_PHYSICS_GROUND: i32 = 100;
pub(crate) const PHYSICS_STATE_SLIDE_GROUND: i32 = 101;
pub(crate) const PHYSICS_STATE_REVERT_GROUND: i32 = 102;
pub(crate) const PHYSICS_STATE_GROUND_ANIMATION: i32 = 103;
pub(crate) const PHYSICS_STATE_SKITCHING: i32 = 104;
pub(crate) const PHYSICS_STATE_FOLLOW_PATH: i32 = 105;
pub(crate) const PHYSICS_STATE_PHYSICS_AIR: i32 = 200;
pub(crate) const PHYSICS_STATE_KNOWN_AIR: i32 = 201;
pub(crate) const PHYSICS_STATE_WIPEOUT_GROUND: i32 = 300;
pub(crate) const PHYSICS_STATE_GRIND_BOARDSLIDE: i32 = 400;
pub(crate) const PHYSICS_STATE_GRIND_5050: i32 = 401;
pub(crate) const PHYSICS_STATE_GRIND_TIPSLIDE: i32 = 402;
pub(crate) const PHYSICS_STATE_GRIND_5_O: i32 = 403;
pub(crate) const PHYSICS_STATE_GRIND_BACKSLASH: i32 = 404;
pub(crate) const PHYSICS_STATE_OFFBOARD: i32 = 500;
pub(crate) const PHYSICS_STATE_OFFBOARD_PUSHING: i32 = 501;
/// Also what `PhysState_OffBoard_Jump::GetType` (0x82d98590) returns.
pub(crate) const PHYSICS_STATE_OFFBOARD_AIR: i32 = 502;
/// The enum's value; `PhysState_OffBoard_Jump::GetType` returns `PHYSICS_STATE_OFFBOARD_AIR`.
pub(crate) const PHYSICS_STATE_OFFBOARD_JUMP: i32 = 503;
pub(crate) const PHYSICS_STATE_OFFBOARD_LOCOMOTION: i32 = 504;
pub(crate) const PHYSICS_STATE_LANDINGONDECK: i32 = 505;
pub(crate) const PHYSICS_STATE_HANDPLANT: i32 = 600;
pub(crate) const PHYSICS_STATE_FOOTPLANT: i32 = 601;
pub(crate) const PHYSICS_STATE_BONELESS: i32 = 602;
pub(crate) const PHYSICS_STATE_SLEEPING: i32 = 700;
pub(crate) const PHYSICS_STATE_NONSPECIFIC: i32 = 701;
pub(crate) const PHYSICS_STATE_TELEPORTING: i32 = 702;
pub(crate) const PHYSICS_STATE_UPPER_LIMIT: i32 = 703;

pub(crate) const PHYSICS_CATEGORY_INVALID: i32 = 0;
pub(crate) const PHYSICS_CATEGORY_GROUND: i32 = 100;
pub(crate) const PHYSICS_CATEGORY_AIR: i32 = 200;
pub(crate) const PHYSICS_CATEGORY_WIPEOUT: i32 = 300;
pub(crate) const PHYSICS_CATEGORY_GRIND: i32 = 400;
pub(crate) const PHYSICS_CATEGORY_OFFBOARD: i32 = 500;
pub(crate) const PHYSICS_CATEGORY_GROUNDTRICK: i32 = 600;
pub(crate) const PHYSICS_CATEGORY_OTHER: i32 = 700;
