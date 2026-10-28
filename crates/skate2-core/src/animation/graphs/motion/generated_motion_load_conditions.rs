//! Generated Motion condition constructors, preserving the shared base semantics.
use super::{ConditionDeclaration, MotionLoadError, construct};
use crate::{
    generated_action_load::{
        ConditionClass,
        construct::{BaseObject, Comparison, ConditionMask},
    },
    play_animation::{FastString30, FastString36},
};
use std::sync::Arc;
#[path = "generated_motion_load_condition_bindings.rs"]
pub(crate) mod bindings;
#[path = "generated_motion_load_condition_plain.rs"]
pub(crate) mod plain;

#[derive(Debug)]
pub(crate) enum MotionCondition {
    Shared(crate::generated_action_load::ConditionObject),
    Motion {
        base: BaseObject,
        mask: ConditionMask,
        payload: ConditionPayload,
    },
}
#[derive(Debug)]
pub(crate) enum ConditionPayload {
    Plain(plain::PlainCondition),
    Comparison {
        class: plain::ComparisonCondition,
        comparison: Comparison,
    },
    CanEnterSlide {
        right: bool,
    },
    ShouldLeaveSlide {
        right: bool,
    },
    ComVelCompare {
        comparison: Comparison,
        axis: Axis,
    },
    HasIntent {
        comparison: Comparison,
        intent: FastString36,
    },
    HasFilteredMotionGraphIntent {
        comparison: Comparison,
        intent: FastString36,
    },
    HasTweak {
        comparison: Comparison,
        x: FastString36,
        y: FastString36,
    },
    InStateForTime {
        comparison: Comparison,
        state: String,
    },
    LastState {
        state: String,
    },
    InTimeWindow {
        start: f32,
        frame_length: f32,
    },
    IsMovingObject {
        angle_start: f32,
        angle_end: f32,
    },
    IsProSkater {
        skater: FastString30,
    },
    HasLandingType {
        landing: LandingType,
    },
    IsSkitchShimmying {
        direction: Option<construct::vault::ShimmyDirection>,
    },
    MongoPushFootToFar {
        contact: FastString30,
        right_bone: FastString30,
        left_bone: FastString30,
    },
    IsInJumpAir {
        comparison: Comparison,
        foot: Option<Foot>,
        debug_draw_offset: u32,
    },
    IsLocomotionFootDown {
        comparison: Comparison,
        foot: Option<Foot>,
        debug_draw_offset: u32,
    },
    CurrentGrabType {
        grab: construct::names::GrabType,
    },
    ShouldPlayHandPlantAnim {
        desired: HandPlantAnimation,
    },
    SkitchingPosition {
        desired: SkitchPosition,
    },
    WillExpire {
        time: f32,
        wait_for_transitions: bool,
        time_tag: String,
    },
    AngleAdjustByAnimationEndLessThan {
        max_delta_angle: f32,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum Axis {
    X = 0,
    Y = 1,
    Z = 2,
    All = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum LandingType {
    Straight = 0,
    Sketchy = 1,
    Spin = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum Foot {
    Right = 0,
    Left = 1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum HandPlantAnimation {
    Antic = 0,
    Into = 1,
    Out = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum SkitchPosition {
    Front = 1,
    Back = 2,
    Both = 3,
}

#[expect(
    clippy::too_many_lines,
    reason = "source condition family dispatch preserves each typed constructor"
)]
pub(crate) fn construct(
    declaration: &ConditionDeclaration,
    name: Arc<str>,
    max_delta_angle: &mut impl FnMut() -> Result<f32, MotionLoadError>,
) -> Result<MotionCondition, MotionLoadError> {
    let values = construct::section_attributes(declaration.attributes)?;
    if let Ok(class) = ConditionClass::from_name(declaration.name) {
        return crate::generated_action_load::construct::condition_from_attributes(
            class, name, values,
        )
        .map(MotionCondition::Shared)
        .map_err(MotionLoadError::Shared);
    }
    let base = BaseObject::new(name);
    let mask = crate::generated_action_load::construct::mask(construct::get(values, "mask"))
        .map_err(MotionLoadError::Shared)?;
    let comparison = || {
        crate::generated_action_load::construct::comparison_from_attributes(
            declaration.name,
            values,
        )
        .map_err(MotionLoadError::Shared)
    };
    let get = |key: &'static str| {
        construct::get(values, key).ok_or(MotionLoadError::InvalidAttribute {
            key,
            value: String::new(),
        })
    };
    let payload = if let Some(class) = plain::PlainCondition::from_name(declaration.name) {
        ConditionPayload::Plain(class)
    } else if let Some(class) = plain::ComparisonCondition::from_name(declaration.name) {
        ConditionPayload::Comparison {
            class,
            comparison: comparison()?,
        }
    } else {
        match declaration.name {
            "AngleAdjustByAnimationEndLessThan" => {
                ConditionPayload::AngleAdjustByAnimationEndLessThan {
                    max_delta_angle: construct::float(values, "MaxDeltaAngle", max_delta_angle()?)?,
                }
            }
            "CanEnterSlide" => ConditionPayload::CanEnterSlide {
                right: construct::boolean(values, "right", true)?,
            },
            "ShouldLeaveSlide" => ConditionPayload::ShouldLeaveSlide {
                right: construct::boolean(values, "right", true)?,
            },
            "ComVelCompare" => ConditionPayload::ComVelCompare {
                comparison: comparison()?,
                axis: match construct::get(values, "axis")
                    .unwrap_or("0")
                    .as_bytes()
                    .first()
                {
                    Some(b'x' | b'X') => Axis::X,
                    Some(b'y' | b'Y') => Axis::Y,
                    Some(b'z' | b'Z') => Axis::Z,
                    _ => Axis::All,
                },
            },
            "HasIntent" => ConditionPayload::HasIntent {
                comparison: comparison()?,
                intent: construct::packed("intent", get("intent")?)?,
            },
            "HasFilteredMotionGraphIntent" => ConditionPayload::HasFilteredMotionGraphIntent {
                comparison: comparison()?,
                intent: construct::packed("intent", get("intent")?)?,
            },
            "HasTweak" => ConditionPayload::HasTweak {
                comparison: comparison()?,
                x: construct::packed("TweakX", "TweakX")?,
                y: construct::packed("TweakY", "TweakY")?,
            },
            "InStateForTime" => ConditionPayload::InStateForTime {
                comparison: comparison()?,
                state: get("state")?.into(),
            },
            "LastState" => ConditionPayload::LastState {
                state: get("state")?.into(),
            },
            "InTimeWindow" => ConditionPayload::InTimeWindow {
                start: construct::float(values, "StartTime", 0.0)?,
                frame_length: construct::float(values, "WindowFrameLength", 0.0)?,
            },
            "IsMovingObject" => ConditionPayload::IsMovingObject {
                angle_start: construct::float(values, "anglestart", 0.0)?
                    * f32::from_bits(0x3c8e_fa35),
                angle_end: construct::float(values, "angleend", 0.0)? * f32::from_bits(0x3c8e_fa35),
            },
            "IsProSkater" => ConditionPayload::IsProSkater {
                skater: construct::packed("skater", get("skater")?)?,
            },
            "HasLandingType" => ConditionPayload::HasLandingType {
                landing: match construct::get(values, "landingType").unwrap_or("straight") {
                    "spin" => LandingType::Spin,
                    "sketchy" => LandingType::Sketchy,
                    _ => LandingType::Straight,
                },
            },
            "IsSkitchShimmying" => ConditionPayload::IsSkitchShimmying {
                direction: match construct::get(values, "direction") {
                    Some("right") => Some(construct::vault::ShimmyDirection::Right),
                    Some("left") => Some(construct::vault::ShimmyDirection::Left),
                    _ => None,
                },
            },
            "MongoPushFootToFar" => ConditionPayload::MongoPushFootToFar {
                contact: construct::packed("push_contact", "push_contact")?,
                right_bone: construct::packed("RightToeBase", "RightToeBase")?,
                left_bone: construct::packed("LeftToeBase", "LeftToeBase")?,
            },
            "IsInJumpAir" | "IsLocomotionFootDown" => {
                let comparison = comparison()?;
                let text = construct::get(values, "foot").unwrap_or("Right");
                let foot = match text {
                    "Right" => Some(Foot::Right),
                    "Left" => Some(Foot::Left),
                    _ => None,
                };
                let debug_draw_offset = if foot == Some(Foot::Right) {
                    if declaration.name == "IsInJumpAir" {
                        30
                    } else {
                        15
                    }
                } else {
                    0
                };
                if declaration.name == "IsInJumpAir" {
                    ConditionPayload::IsInJumpAir {
                        comparison,
                        foot,
                        debug_draw_offset,
                    }
                } else {
                    ConditionPayload::IsLocomotionFootDown {
                        comparison,
                        foot,
                        debug_draw_offset,
                    }
                }
            }
            "CurrentGrabType" => {
                let text = get("grab")?;
                let grab = match text {
                    "FS" => construct::names::GrabType::Frontside,
                    "BS" => construct::names::GrabType::Backside,
                    "Nose" => construct::names::GrabType::Nose,
                    "Tail" => construct::names::GrabType::Tail,
                    _ => {
                        return Err(MotionLoadError::InvalidAttribute {
                            key: "grab",
                            value: text.into(),
                        });
                    }
                };
                ConditionPayload::CurrentGrabType { grab }
            }
            "ShouldPlayHandPlantAnim" => {
                let text = construct::get(values, "anim").unwrap_or("antic");
                let desired = match text {
                    "antic" => HandPlantAnimation::Antic,
                    "into" => HandPlantAnimation::Into,
                    "out" => HandPlantAnimation::Out,
                    _ => {
                        return Err(MotionLoadError::InvalidAttribute {
                            key: "anim",
                            value: text.into(),
                        });
                    }
                };
                ConditionPayload::ShouldPlayHandPlantAnim { desired }
            }
            "SkitchingPosition" => {
                let text = construct::get(values, "side").unwrap_or("");
                let desired = match text {
                    "front" => SkitchPosition::Front,
                    "back" => SkitchPosition::Back,
                    "both" => SkitchPosition::Both,
                    _ => {
                        return Err(MotionLoadError::InvalidAttribute {
                            key: "side",
                            value: text.into(),
                        });
                    }
                };
                ConditionPayload::SkitchingPosition { desired }
            }
            "WillExpire" => {
                let time = construct::float(values, "InTime", 0.0)?;
                let wait_for_transitions = construct::boolean(values, "waitForTransitions", true)?;
                let text = construct::get(values, "InTimeTag").unwrap_or("");
                if text.len() >= 37 {
                    return Err(MotionLoadError::InvalidAttribute {
                        key: "InTimeTag",
                        value: text.into(),
                    });
                }
                ConditionPayload::WillExpire {
                    time,
                    wait_for_transitions,
                    time_tag: text.into(),
                }
            }
            name => {
                return Err(MotionLoadError::IncompleteConstructor {
                    domain: super::ObjectDomain::Condition,
                    index: usize::MAX,
                    name,
                });
            }
        }
    };
    Ok(MotionCondition::Motion {
        base,
        mask,
        payload,
    })
}
