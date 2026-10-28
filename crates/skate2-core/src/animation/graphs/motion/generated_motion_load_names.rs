//! Motion constructors whose retained state is named attributes and scalar settings.
use super::{boolean, float, get, packed};
use crate::{
    generated_motion_load::{Attribute, MotionLoadError},
    play_animation::{FastString30, FastString36},
};

#[derive(Debug)]
pub(crate) enum NamedMotionBehaviour {
    ComputeRepushDeadline {
        regular_attributes: bool,
    },
    ComputeTargetCoefsFromSpeedAndStrength {
        regular_attributes: bool,
    },
    CreateSlide {
        right: bool,
    },
    PowerSlideDecel {
        right: bool,
    },
    PushOut {
        is_right_foot: bool,
    },
    PushCycle {
        new_push: FastString36,
    },
    FingerFlipOut {
        grab_intent: FastString36,
        grab_blend: FastString30,
    },
    JumpInto {
        attribute: FastString30,
    },
    SetManualAngle {
        angle: FastString30,
    },
    SetManualOutTimer {
        set_time: f32,
    },
    SetSpeed {
        attribute: FastString30,
        set_manually: bool,
        set_to_value: f32,
    },
    SetGrabType {
        grab: GrabType,
    },
    EndShimmy {
        blend_out_time: f32,
    },
    SetHandPlantAnticLength {
        antic_length: FastString30,
    },
    SetDeckPitchAndYaw {
        yaw: FastString30,
        pitch: FastString30,
    },
    ChooseRandomSketchyLanding {
        random: FastString30,
    },
    SetBumpCoefficients {
        x: FastString30,
        y: FastString30,
    },
    Crouching {
        crouch: FastString30,
    },
    KickTurnSteering {
        max_height: f32,
        spin_scale: f32,
    },
    SteppingBoardNoBoard {
        board: FastString30,
        no_board: FastString30,
    },
    StandingJumpLandingCog {
        distance_to_cog: FastString30,
    },
    SetPushCoefs {
        on_first_update_only: bool,
    },
    TweakProject {
        intent_x: FastString36,
        intent_y: FastString36,
        attribute_x: FastString30,
        attribute_y: FastString30,
    },
    SetTrickAttr {
        trick: FastString30,
    },
    SetTrickHeight {
        from: FastString30,
        rename: FastString30,
        set_manually: bool,
        set_to_value: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum GrabType {
    Frontside = 1,
    Backside = 2,
    Nose = 3,
    Tail = 4,
}

pub(crate) fn supports(name: &str) -> bool {
    if matches!(
        name,
        "ComputeRepushDeadline"
            | "ComputeTargetCoefsFromSpeedAndStrength"
            | "CreateSlide"
            | "PowerSlideDecel"
            | "PushOut"
            | "PushCycle"
            | "FingerFlipOut"
            | "JumpInto"
            | "SetManualAngle"
            | "SetManualOutTimer"
            | "SetSpeed"
            | "SetGrabType"
    ) {
        return true;
    }
    matches!(
        name,
        "EndShimmy"
            | "SetHandPlantAnticLength"
            | "SetDeckPitchAndYaw"
            | "ChooseRandomSketchyLanding"
            | "SetBumpCoefficients"
            | "Crouching"
            | "KickTurnSteering"
            | "SteppingBoardNoBoard"
            | "StandingJumpLandingCOG"
            | "SetPushCoefs"
            | "TweakProject"
            | "SetTrickAttr"
            | "SetTrickHeight"
    )
}

fn required<'a>(values: &'a [Attribute], key: &'static str) -> Result<&'a str, MotionLoadError> {
    get(values, key).ok_or(MotionLoadError::InvalidAttribute {
        key,
        value: String::new(),
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "source named behaviour family dispatch preserves typed constructors"
)]
pub(crate) fn construct(
    name: &'static str,
    values: &[Attribute],
) -> Result<NamedMotionBehaviour, MotionLoadError> {
    Ok(match name {
        "ComputeRepushDeadline" => NamedMotionBehaviour::ComputeRepushDeadline {
            regular_attributes: boolean(values, "regularAttributes", true)?,
        },
        "ComputeTargetCoefsFromSpeedAndStrength" => {
            NamedMotionBehaviour::ComputeTargetCoefsFromSpeedAndStrength {
                regular_attributes: boolean(values, "regularAttributes", true)?,
            }
        }
        "CreateSlide" => NamedMotionBehaviour::CreateSlide {
            right: boolean(values, "right", true)?,
        },
        "PowerSlideDecel" => NamedMotionBehaviour::PowerSlideDecel {
            right: boolean(values, "right", true)?,
        },
        "PushOut" => NamedMotionBehaviour::PushOut {
            is_right_foot: boolean(values, "isRightFoot", true)?,
        },
        "PushCycle" => NamedMotionBehaviour::PushCycle {
            new_push: packed(
                "newPushName",
                get(values, "newPushName").unwrap_or("LeftPush"),
            )?,
        },
        "FingerFlipOut" => NamedMotionBehaviour::FingerFlipOut {
            grab_intent: packed("grabintent", required(values, "grabintent")?)?,
            grab_blend: packed("Grabbing", "Grabbing")?,
        },
        "JumpInto" => NamedMotionBehaviour::JumpInto {
            attribute: packed("attribute", required(values, "attribute")?)?,
        },
        "SetManualAngle" => NamedMotionBehaviour::SetManualAngle {
            angle: packed("manual_angle", "manual_angle")?,
        },
        "SetManualOutTimer" => NamedMotionBehaviour::SetManualOutTimer {
            set_time: float(values, "length", f32::from_bits(0x3e29_fbe7))?,
        },
        "SetSpeed" => NamedMotionBehaviour::SetSpeed {
            attribute: packed("attribute", required(values, "attribute")?)?,
            set_manually: get(values, "setToValue").is_some(),
            set_to_value: float(values, "setToValue", 0.0)?,
        },
        "SetGrabType" => {
            let text = required(values, "grab")?;
            let grab = match text {
                "FS" => GrabType::Frontside,
                "BS" => GrabType::Backside,
                "Nose" => GrabType::Nose,
                "Tail" => GrabType::Tail,
                _ => {
                    return Err(MotionLoadError::InvalidAttribute {
                        key: "grab",
                        value: text.into(),
                    });
                }
            };
            NamedMotionBehaviour::SetGrabType { grab }
        }
        "EndShimmy" => NamedMotionBehaviour::EndShimmy {
            blend_out_time: float(values, "blendtime", 0.2)?,
        },
        "SetHandPlantAnticLength" => NamedMotionBehaviour::SetHandPlantAnticLength {
            antic_length: packed("antic_length", "antic_length")?,
        },
        "SetDeckPitchAndYaw" => NamedMotionBehaviour::SetDeckPitchAndYaw {
            yaw: packed("skateyaw", get(values, "skateyaw").unwrap_or("skateyaw"))?,
            pitch: packed(
                "skatepitch",
                get(values, "skatepitch").unwrap_or("skatepitch"),
            )?,
        },
        "ChooseRandomSketchyLanding" => NamedMotionBehaviour::ChooseRandomSketchyLanding {
            random: packed("Random", "Random")?,
        },
        "SetBumpCoefficients" => NamedMotionBehaviour::SetBumpCoefficients {
            x: packed("X", required(values, "X")?)?,
            y: packed("Y", required(values, "Y")?)?,
        },
        "Crouching" => NamedMotionBehaviour::Crouching {
            crouch: packed("crouchName", required(values, "crouchName")?)?,
        },
        "KickTurnSteering" => NamedMotionBehaviour::KickTurnSteering {
            max_height: float(values, "maxHeight", 0.5)?,
            spin_scale: float(values, "spinScale", 0.5)?,
        },
        "SteppingBoardNoBoard" => NamedMotionBehaviour::SteppingBoardNoBoard {
            board: packed("Board", "Board")?,
            no_board: packed("NoBoard", "NoBoard")?,
        },
        "StandingJumpLandingCOG" => NamedMotionBehaviour::StandingJumpLandingCog {
            distance_to_cog: packed("DistToCogOnLanding", "DistToCogOnLanding")?,
        },
        "SetPushCoefs" => NamedMotionBehaviour::SetPushCoefs {
            on_first_update_only: boolean(values, "onFirstUpdateOnly", false)?,
        },
        "TweakProject" => NamedMotionBehaviour::TweakProject {
            intent_x: packed("TweakX", "TweakX")?,
            intent_y: packed("TweakY", "TweakY")?,
            attribute_x: packed("tweak_x", "tweak_x")?,
            attribute_y: packed("tweak_y", "tweak_y")?,
        },
        "SetTrickAttr" => NamedMotionBehaviour::SetTrickAttr {
            trick: packed("trick", required(values, "trick")?)?,
        },
        "SetTrickHeight" => {
            let from = packed("from", get(values, "from").unwrap_or(""))?;
            let rename = packed("rename", get(values, "rename").unwrap_or(""))?;
            let set_manually = get(values, "setToValue").is_some();
            let set_to_value = float(values, "setToValue", 0.0)?;
            required(values, "intent")?;
            required(values, "rename")?;
            if !set_manually {
                required(values, "from")?;
            }
            NamedMotionBehaviour::SetTrickHeight {
                from,
                rename,
                set_manually,
                set_to_value,
            }
        }
        _ => {
            return Err(MotionLoadError::IncompleteConstructor {
                domain: crate::generated_motion_load::ObjectDomain::Behaviour,
                index: usize::MAX,
                name,
            });
        }
    })
}
