//! Motion control and grab constructor families from their original class helpers.
use super::{FastString30, FastString36, MotionLoadError, boolean, float, get, packed};
use crate::generated_action_load::Attribute;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum PhysicsAnimationMode {
    FollowAnimation = 0,
    ForcePhysics = 1,
    ForceAnimation = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum GestureHand {
    Backside = 0,
    Frontside = 1,
}
#[derive(Debug)]
pub(crate) enum MotionControlBehaviour {
    ForcePhysics(PhysicsAnimationMode),
    ControlGrindCrouch {
        drive_distance_to_com: FastString30,
    },
    HandBusy(GestureHand),
    EndGesture(GestureHand),
    SetDistComToBoard {
        attribute: FastString30,
        set_manually: bool,
        adjust_for_velocity: bool,
    },
    DisableTricks {
        timer_length: f32,
    },
    SetTurning {
        angle: FastString30,
        direction: FastString30,
        quickness: FastString30,
        holding: FastString30,
        speed: FastString30,
    },
    ScoringGrabs(Box<ScoringGrabs>),
    ScoringHandPlants(Box<ScoringHandPlants>),
    Wipeout {
        lean: FastString30,
        twist: FastString30,
        gesture_x: FastString30,
        gesture_y: FastString30,
    },
    PlayWipeoutRecoverAnim {
        transition_length: f32,
    },
    LandingOffBoardAir {
        impact: FastString30,
        close_to_land: FastString30,
    },
    ScoringTrick {
        name: FastString30,
    },
    GrindControlFade {
        distance_board_to_cog: FastString30,
        twist: FastString30,
        twist_intent: FastString36,
    },
}
#[derive(Debug)]
pub(crate) struct DirectionalGrab {
    pub(crate) name: FastString30,
    pub(crate) active: bool,
}
#[derive(Debug)]
pub(crate) struct ScoringGrabs {
    pub(crate) grab: FastString30,
    /// Original constructor visits up,left,down,right in this order.
    pub(crate) directions: [DirectionalGrab; 4],
    pub(crate) has_tweak: bool,
    pub(crate) intent_x: FastString36,
    pub(crate) intent_y: FastString36,
    pub(crate) intent_angle: FastString36,
    pub(crate) intent_magnitude: FastString36,
    pub(crate) use_xy: bool,
    pub(crate) invert_y: bool,
}
#[derive(Debug)]
pub(crate) struct ScoringHandPlants {
    pub(crate) name: FastString30,
    pub(crate) directions: [DirectionalGrab; 4],
    pub(crate) has_tweak: bool,
    pub(crate) intent_x: FastString36,
    pub(crate) intent_y: FastString36,
}
pub(crate) fn supports(name: &str) -> bool {
    if name == "ScoringHandPlants" {
        return true;
    }
    if matches!(
        name,
        "Wipeout"
            | "PlayWipeoutRecoverAnim"
            | "LandingOffBoardAir"
            | "ScoringTrick"
            | "GrindControlFade"
    ) {
        return true;
    }
    matches!(
        name,
        "ForcePhysics"
            | "ControlGrindCrouch"
            | "HandBusy"
            | "EndGesture"
            | "SetDistComToBoard"
            | "DisableTricks"
            | "SetTurning"
            | "ScoringGrabs"
    )
}
fn required<'a>(values: &'a [Attribute], key: &'static str) -> Result<&'a str, MotionLoadError> {
    get(values, key).ok_or(MotionLoadError::InvalidAttribute {
        key,
        value: String::new(),
    })
}
fn hand(values: &[Attribute]) -> GestureHand {
    if get(values, "hand").unwrap_or("bs") == "bs" {
        GestureHand::Backside
    } else {
        GestureHand::Frontside
    }
}
fn grabs(values: &[Attribute]) -> Result<ScoringGrabs, MotionLoadError> {
    let mut result = ScoringGrabs {
        grab: packed("grab", "")?,
        directions: std::array::from_fn(|_| DirectionalGrab {
            name: FastString30::encode("").expect("empty packed name"),
            active: false,
        }),
        has_tweak: false,
        intent_x: packed("x", "")?,
        intent_y: packed("y", "")?,
        intent_angle: packed("angle", "")?,
        intent_magnitude: packed("magnitude", "")?,
        use_xy: true,
        invert_y: boolean(values, "invertY", false)?,
    };
    result.grab = packed("grabName", required(values, "grabName")?)?;
    for (index, key) in ["up", "left", "down", "right"].into_iter().enumerate() {
        if let Some(value) = get(values, key) {
            result.directions[index].name = packed(key, value)?;
            result.directions[index].active = true;
            result.has_tweak = true;
        }
    }
    if result.has_tweak {
        if !((get(values, "intentX").is_some() && get(values, "intentY").is_some())
            || (get(values, "angle").is_some() && get(values, "magnitude").is_some()))
        {
            return Err(MotionLoadError::InvalidAttribute {
                key: "tweak input",
                value: String::new(),
            });
        }
        if get(values, "intentX").is_some() {
            result.intent_x = packed("intentX", required(values, "intentX")?)?;
            result.intent_y = packed("intentY", required(values, "intentY")?)?;
            result.use_xy = true;
        } else {
            result.intent_magnitude = packed("magnitude", required(values, "magnitude")?)?;
            result.intent_angle = packed("angle", required(values, "angle")?)?;
            result.use_xy = false;
        }
    }
    Ok(result)
}
fn handplants(values: &[Attribute]) -> Result<ScoringHandPlants, MotionLoadError> {
    let mut result = ScoringHandPlants {
        name: packed("handplantname", required(values, "handplantname")?)?,
        directions: std::array::from_fn(|_| DirectionalGrab {
            name: FastString30::encode("").expect("empty packed name"),
            active: false,
        }),
        has_tweak: false,
        intent_x: packed("x", "")?,
        intent_y: packed("y", "")?,
    };
    for (index, key) in ["up", "left", "down", "right"].into_iter().enumerate() {
        if let Some(value) = get(values, key) {
            result.directions[index].name = packed(key, value)?;
            result.directions[index].active = true;
            result.has_tweak = true;
        }
    }
    if result.has_tweak {
        result.intent_x = packed("intentX", required(values, "intentX")?)?;
        result.intent_y = packed("intentY", required(values, "intentY")?)?;
    }
    Ok(result)
}
pub(crate) fn construct(
    name: &'static str,
    values: &[Attribute],
) -> Result<MotionControlBehaviour, MotionLoadError> {
    Ok(match name {
        "ScoringHandPlants" => {
            MotionControlBehaviour::ScoringHandPlants(Box::new(handplants(values)?))
        }
        "Wipeout" => MotionControlBehaviour::Wipeout {
            lean: packed("lean", "Lean")?,
            twist: packed("twist", "Twist")?,
            gesture_x: packed("gestureX", "WipeoutGestureX")?,
            gesture_y: packed("gestureY", "WipeoutGestureY")?,
        },
        "PlayWipeoutRecoverAnim" => MotionControlBehaviour::PlayWipeoutRecoverAnim {
            transition_length: float(values, "transTime", 0.1)?,
        },
        "LandingOffBoardAir" => MotionControlBehaviour::LandingOffBoardAir {
            impact: packed("impact", "Impact")?,
            close_to_land: packed("close", "CloseToLand")?,
        },
        "ScoringTrick" => MotionControlBehaviour::ScoringTrick {
            name: packed("trick", required(values, "trick")?)?,
        },
        "GrindControlFade" => MotionControlBehaviour::GrindControlFade {
            distance_board_to_cog: packed(
                "distBoardToCogAnimAttribute",
                required(values, "distBoardToCogAnimAttribute")?,
            )?,
            twist: packed(
                "twistAnimAttribute",
                required(values, "twistAnimAttribute")?,
            )?,
            twist_intent: packed("twistMGIntent", required(values, "twistMGIntent")?)?,
        },
        "ForcePhysics" => MotionControlBehaviour::ForcePhysics(
            match get(values, "force").unwrap_or("FOLLOW_ANIMATION_DATA") {
                "FOLLOW_ANIMATION_DATA" => PhysicsAnimationMode::FollowAnimation,
                "FORCE_PHYSICS_SKATEBOARD" => PhysicsAnimationMode::ForcePhysics,
                "FORCE_ANIM_SKATEBOARD" => PhysicsAnimationMode::ForceAnimation,
                value => {
                    return Err(MotionLoadError::InvalidAttribute {
                        key: "force",
                        value: value.into(),
                    });
                }
            },
        ),
        "ControlGrindCrouch" => MotionControlBehaviour::ControlGrindCrouch {
            drive_distance_to_com: packed(
                "driveDistToCom",
                get(values, "driveDistToCom").unwrap_or("disttocog"),
            )?,
        },
        "HandBusy" => MotionControlBehaviour::HandBusy(hand(values)),
        "EndGesture" => MotionControlBehaviour::EndGesture(hand(values)),
        "SetDistComToBoard" => MotionControlBehaviour::SetDistComToBoard {
            attribute: packed("attribute", required(values, "attribute")?)?,
            set_manually: boolean(values, "setManually", false)?,
            adjust_for_velocity: boolean(values, "adjustForVel", false)?,
        },
        "DisableTricks" => MotionControlBehaviour::DisableTricks {
            timer_length: float(values, "length", 0.5)?,
        },
        "SetTurning" => MotionControlBehaviour::SetTurning {
            angle: packed("angleName", get(values, "angleName").unwrap_or("angle"))?,
            direction: packed("dirName", get(values, "dirName").unwrap_or("dir"))?,
            quickness: packed(
                "quicknessName",
                get(values, "quicknessName").unwrap_or("quickness"),
            )?,
            holding: packed(
                "holdingName",
                get(values, "holdingName").unwrap_or("holding"),
            )?,
            speed: packed("speedName", get(values, "speedName").unwrap_or("speedtuck"))?,
        },
        "ScoringGrabs" => MotionControlBehaviour::ScoringGrabs(Box::new(grabs(values)?)),
        _ => {
            return Err(MotionLoadError::IncompleteConstructor {
                domain: super::super::ObjectDomain::Behaviour,
                index: usize::MAX,
                name,
            });
        }
    })
}
