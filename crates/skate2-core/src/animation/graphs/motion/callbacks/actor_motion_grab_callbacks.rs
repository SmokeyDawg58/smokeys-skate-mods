//! Shared grab callbacks on retained Motion and animation owners.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::camera_compass::runtime::math::camera_atan2_pending;
use crate::generated_motion_load::construct::{
    control::{GestureHand, ScoringGrabs, ScoringHandPlants},
    names::GrabType,
};
use crate::play_animation::{FastString30, FastString36};
use crate::reckoning_numeric::{HostReckoningNumeric, try_length};

/// Scalar core of `TweakProject::Update82cc5238`, preserving PPC operation order.
fn project_grab_tweak_values(x: Option<f32>, y: Option<f32>) -> Option<[f32; 2]> {
    if x.is_none() && y.is_none() {
        return None;
    }
    let x = x.unwrap_or(0.0);
    let y = y.unwrap_or(0.0);
    let mut ax = x.abs();
    let mut ay = y.abs();
    let sum = ay + ax;
    let excess = sum - 1.0;
    // PPC `fcmpu; ble` branches when GT is clear, including unordered.
    if excess > 0.0 {
        let scale = 1.0_f32 / sum;
        ax = scale * ax;
        ay = scale * ay;
    }
    Some([
        if x >= 0.0 { ax } else { -ax },
        if y >= 0.0 { ay } else { -ay },
    ])
}

///82ccba6c..bb20: separate XMVectorSin/Cos, multiply magnitude, optional Y inversion.
fn polar_grab_tweak(angle: f32, magnitude: f32, invert_y: bool) -> [f32; 2] {
    let mut rounding = crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
    let sine = crate::reckoning_sin_cos::sine::try_sine(angle, &mut rounding)
        .unwrap_or_else(|never| match never {});
    let x = sine * magnitude;
    let cosine = crate::reckoning_sin_cos::try_cosine(angle, &mut rounding)
        .unwrap_or_else(|never| match never {});
    let mut y = cosine * magnitude;
    if invert_y {
        y *= -1.0;
    }
    [x, y]
}

fn adjust_hand_busy(counters: &mut [i32; 2], hand: GestureHand, delta: i32) {
    let counter = &mut counters[hand as usize];
    *counter = counter.wrapping_add(delta);
}

impl ActorConstruction {
    /// ScoringGrabs::Create82ccb910, called by Update82c8ef88 each frame.
    /// Shared XY/polar inputs and authored-neighbor directional boundaries.
    pub(crate) fn motion_score_fs_grab_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        scoring: &ScoringGrabs,
    ) -> Result<(), E> {
        self.motion_score_directional_grab(context, scoring, 0x0200_0000)
    }
    /// Create82ccbe58 shares the same XY sectors; SetTrickDataHandPlant825fe3e0
    /// differs in its trick bit (0x01000000).
    pub(crate) fn motion_score_handplant_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        scoring: &ScoringHandPlants,
    ) -> Result<(), E> {
        let shared = ScoringGrabs {
            grab: scoring.name,
            directions: std::array::from_fn(|i| {
                crate::generated_motion_load::construct::control::DirectionalGrab {
                    name: scoring.directions[i].name,
                    active: scoring.directions[i].active,
                }
            }),
            has_tweak: scoring.has_tweak,
            intent_x: scoring.intent_x,
            intent_y: scoring.intent_y,
            intent_angle: FastString36 { words: [0; 6] },
            intent_magnitude: FastString36 { words: [0; 6] },
            use_xy: true,
            invert_y: false,
        };
        self.motion_score_directional_grab(context, &shared, 0x0100_0000)
    }
    fn motion_score_directional_grab(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        scoring: &ScoringGrabs,
        trick_bit: u32,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let mut x = 0.0_f32;
        let mut y = 0.0_f32;
        let mut name = scoring.grab;
        if scoring.has_tweak {
            if scoring.use_xy {
                x = self
                    .motion_runtime_filtered_intent_value(context, &scoring.intent_x)?
                    .unwrap_or(0.0);
                y = self
                    .motion_runtime_filtered_intent_value(context, &scoring.intent_y)?
                    .unwrap_or(0.0);
            } else {
                let angle = self
                    .motion_runtime_filtered_intent_value(context, &scoring.intent_angle)?
                    .unwrap_or(0.0);
                let magnitude = self
                    .motion_runtime_filtered_intent_value(context, &scoring.intent_magnitude)?
                    .unwrap_or(0.0);
                [x, y] = polar_grab_tweak(angle, magnitude, scoring.invert_y);
            }
            let mut numeric = HostReckoningNumeric;
            let length = try_length([x, y, 0.0, 0.0], &mut numeric).map_err(E::ScoringNumeric)?;
            if length > 0.5 {
                let angle = camera_atan2_pending(y, x, &mut numeric)
                    .map_err(E::ScoringNumeric)?
                    .finish();
                let angle = if y < 0.0 {
                    angle + f32::from_bits(0x40c9_0fdb)
                } else {
                    angle
                };
                let [up, left, down, right] = scoring.directions.each_ref().map(|d| d.active);
                let intervals = [
                    (
                        if right { 44.0 } else { 359.0 },
                        if left { 134.0 } else { 181.0 },
                    ),
                    (
                        if up { 44.0 } else { 89.0 },
                        if down { 226.0 } else { 271.0 },
                    ),
                    (
                        if left { 224.0 } else { 179.0 },
                        if right { 316.0 } else { 1.0 },
                    ),
                    (
                        if down { 314.0 } else { 269.0 },
                        if up { 46.0 } else { 91.0 },
                    ),
                ];
                let degree = 0.017_453_292_f32;
                let mut matched = false;
                for (direction, (start, end)) in scoring.directions.iter().zip(intervals) {
                    if !direction.active {
                        continue;
                    }
                    let start = start * degree;
                    let end = end * degree;
                    // InAngle82c8c2d0..2ec: LT/GT-clear includes unordered.
                    let inside = if end < start {
                        !(angle < start) || !(angle > end)
                    } else {
                        !(angle < start) && !(angle > end)
                    };
                    if inside {
                        name = direction.name;
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    return Err(E::PendingAnimationOperation(
                        "ScoringGrabs unmatched direction",
                    ));
                }
            }
        }
        let mut words = [0; 6];
        words[..5].copy_from_slice(&name.words);
        let mut actor = self.prefix().map_err(E::Actor)?;
        let state = actor
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .and_then(|motion| motion.state.as_mut())
            .ok_or(E::MissingOwner("ScoringGrabs Motion state"))?;
        state.trick_bits |= trick_bit;
        state.trick_name = FastString36 { words };
        state.trick_tweak = [x, y];
        Ok(())
    }

    /// `TweakProject::Update82cc5238`: read the filtered Motion intents,
    /// project their magnitudes onto the unit L1 diamond, then publish both
    /// animation scalars when either filtered intent exists.
    pub(crate) fn motion_project_grab_tweak(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        intent_x: &FastString36,
        intent_y: &FastString36,
        attribute_x: FastString30,
        attribute_y: FastString30,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(0)?;
        let x = self.motion_runtime_filtered_intent_value(context, intent_x)?;
        let y = self.motion_runtime_filtered_intent_value(context, intent_y)?;
        let Some([projected_x, projected_y]) = project_grab_tweak_values(x, y) else {
            return Ok(());
        };
        let mut animation = animation.lock().map_err(|_| E::Lock)?;
        animation
            .try_add_gesture_attribute(attribute_x, projected_x, false, -1)
            .map_err(E::AnimationWrite)?;
        animation
            .try_add_gesture_attribute(attribute_y, projected_y, false, -1)
            .map_err(E::AnimationWrite)
    }

    /// `SetGrabType::Begin82ca6718` and `End82ca67e8` write the cached
    /// ISkaterAnim's full grab-type word, clearing it only on End.
    pub(crate) fn motion_set_grab_type(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        grab: Option<GrabType>,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let owner = self.lifecycle_animation(2)?;
        owner
            .lock()
            .map_err(|_| E::Lock)?
            .try_set_grab_type(grab.map_or(0, |grab| grab as u32))
            .map_err(E::MissingOwner)
    }

    /// `HandBusy::Begin82c9e880` / `End82c9e9c8` resolve ISkaterMotionGraph,
    /// then GetGrabStateWrite825fdfb8: BS counter+0, FS counter+4.
    pub(crate) fn motion_adjust_hand_busy(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        hand: GestureHand,
        delta: i32,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let mut actor = self.prefix().map_err(E::Actor)?;
        let state = actor
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .and_then(|motion| motion.state.as_mut())
            .ok_or(E::MissingOwner("HandBusy Motion GrabState"))?;
        adjust_hand_busy(&mut state.grab_busy, hand, delta);
        Ok(())
    }

    /// `EndGesture::Begin82c9e768`: BS GestureLeft / FS GestureRight,
    /// then GestureBoth, in that order.
    pub(crate) fn motion_end_hand_gestures(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        hand: GestureHand,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        // Native compares cached and registered IChannelAnimatable, then
        // returns when both are null (82c9e7d4/82c9e7f4).
        {
            let actor = self.prefix().map_err(E::Actor)?;
            let registered = actor.interfaces.binary_search(
                &crate::actor_animation_construction::ActorAnimationInterface::ChannelAnimatable.source_name(),
            ).is_ok();
            match (registered, actor.animation_interfaces[1].as_ref()) {
                (false, None) => return Ok(()),
                (true, Some(_)) => {}
                _ => return Err(E::ForeignOwner("EndGesture channel interface")),
            }
        }
        let owner = self.lifecycle_animation(1)?;
        let mut animation = owner.lock().map_err(|_| E::Lock)?;
        let first = match hand {
            GestureHand::Backside => "GestureLeft",
            GestureHand::Frontside => "GestureRight",
        };
        for name in [first, "GestureBoth"] {
            animation
                .try_end_gesture_channel(&FastString36::encode(name).expect("native gesture name"))
                .map_err(E::AnimationWrite)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{GestureHand, adjust_hand_busy, polar_grab_tweak, project_grab_tweak_values};

    #[test]
    fn polar_grab_uses_sine_x_cosine_y_and_inverts_only_y() {
        assert_eq!(polar_grab_tweak(0.0, 0.75, false), [0.0, 0.75]);
        assert_eq!(polar_grab_tweak(0.0, 0.75, true), [0.0, -0.75]);
        let a = polar_grab_tweak(0.6, 0.8, false);
        let b = polar_grab_tweak(0.6, 0.8, true);
        assert_eq!(a[0].to_bits(), b[0].to_bits());
        assert_eq!((-a[1]).to_bits(), b[1].to_bits());
        let zero = polar_grab_tweak(0.0, 0.0, true);
        assert_eq!(zero[1].to_bits(), (-0.0f32).to_bits());
    }

    #[test]
    fn hand_busy_counters_are_distinct_and_wrap() {
        let mut counters = [0, i32::MAX];
        adjust_hand_busy(&mut counters, GestureHand::Frontside, 1);
        assert_eq!(counters, [0, i32::MIN]);
        adjust_hand_busy(&mut counters, GestureHand::Backside, -1);
        assert_eq!(counters, [-1, i32::MIN]);
        adjust_hand_busy(&mut counters, GestureHand::Frontside, -1);
        adjust_hand_busy(&mut counters, GestureHand::Backside, 1);
        assert_eq!(counters, [0, i32::MAX]);
    }

    #[test]
    fn tweak_projection_retains_presence_sign_and_source_unordered_branch() {
        assert_eq!(project_grab_tweak_values(None, None), None);
        assert_eq!(
            project_grab_tweak_values(Some(0.75), Some(-0.75)),
            Some([0.5, -0.5])
        );
        assert_eq!(
            project_grab_tweak_values(Some(-0.0), None)
                .unwrap()
                .map(f32::to_bits),
            [0, 0]
        );
        assert_eq!(
            project_grab_tweak_values(None, Some(-0.25)),
            Some([0.0, -0.25])
        );
        let [x, y] = project_grab_tweak_values(Some(0.25), Some(f32::NAN)).unwrap();
        assert_eq!(x.to_bits(), 0.25_f32.to_bits());
        assert!(y.is_nan());
    }
}
