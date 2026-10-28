//! Actual trick output values, shared with the native animation/input owner.
use crate::play_animation::{EMPTY_FAST_STRING_36, FastString36};

/// `AnimOutTrickData`.Reset825f68f0. The real `kInvalidTrick` initializer82f7be08
/// assigns an empty string; it is distinct from Andale's `nullString` identity.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AnimOutTrickData {
    pub(crate) name: FastString36,
    pub(crate) gesture: FastString36,
    pub(crate) bits: u32,
    pub(crate) tweak_x: f32,
    pub(crate) tweak_y: f32,
}
impl AnimOutTrickData {
    pub(crate) fn reset_values() -> Self {
        Self {
            name: EMPTY_FAST_STRING_36,
            gesture: EMPTY_FAST_STRING_36,
            bits: 0,
            tweak_x: 0.0,
            tweak_y: 0.0,
        }
    }
}

/// `PhysOut_Trick` constructor82e2f118 and Reset82e2d530.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_field_names,
    reason = "retain original PDB field meanings across output schemas"
)]
pub(super) struct Trick {
    pub(super) current_name: FastString36,
    pub(super) completed_name: FastString36,
    pub(super) current_animation: AnimOutTrickData,
    pub(super) is_gesturing: bool,
    pub(super) gesture: i32,
    pub(super) wipeout_gesture_x: f32,
    pub(super) wipeout_gesture_y: f32,
    pub(super) has_completed_trick: bool,
    pub(super) is_push_tricking: bool,
}
impl Trick {
    pub(super) fn reset_values() -> Self {
        Self {
            current_name: EMPTY_FAST_STRING_36,
            completed_name: EMPTY_FAST_STRING_36,
            current_animation: AnimOutTrickData::reset_values(),
            is_gesturing: false,
            gesture: 35, // eAnimGestureNames_Count; raw li5,0x23 at82e2d5e4.
            wipeout_gesture_x: 0.0,
            wipeout_gesture_y: 0.0,
            has_completed_trick: false,
            is_push_tricking: false,
        }
    }
}
