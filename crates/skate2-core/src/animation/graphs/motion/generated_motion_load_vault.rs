//! Source constructors retaining actual Attrib instances and generated layouts.
use super::{boolean, get, packed};
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::{Attribute, MotionLoadError},
    play_animation::FastString30,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum VaultMotionBehaviour {
    SkitchShimmying {
        attributes: AttributeRuntimeInstance,
        desired_direction: ShimmyDirection,
    },
    HippyJumpAntic {
        trick_height: FastString30,
        attributes: AttributeRuntimeInstance,
        layout: AnimationMotionLayout,
    },
    MovingObject(Box<MovingObject>),
}

#[derive(Debug)]
pub(crate) enum AnimationMotionLayout {
    Cached(AttributeRuntimeData),
    /// `Gen::anim_motion828b1590` substitutes `DefaultDataArea(0x5c0)`.
    /// `DefaultDataArea828b5340` returns zero-initialized `gDefaultDataArea83059d30`.
    DefaultZero,
}
impl AnimationMotionLayout {
    pub(crate) fn bytes(&self) -> &[u8] {
        static DEFAULT: [u8; 0x5c0] = [0; 0x5c0];
        match self {
            Self::Cached(data) => data.bytes(),
            Self::DefaultZero => &DEFAULT,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum ShimmyDirection {
    Left = 1,
    Right = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum MovingDirection {
    Still = 0,
    PushLeft = 1,
    PushRight = 2,
    PushLeftToRight = 3,
    PushRightToLeft = 4,
    PushToPullLeft = 5,
    PushToPullRight = 6,
    PullLeft = 7,
    PullRight = 8,
    PullLeftToRight = 9,
    PullRightToLeft = 10,
    PullToPushLeft = 11,
    PullToPushRight = 12,
}
#[derive(Debug)]
pub(crate) struct MovingObject {
    pub(crate) attributes: AttributeRuntimeInstance,
    pub(crate) direction: MovingDirection,
    pub(crate) x_speed: FastString30,
    pub(crate) z_speed: FastString30,
    pub(crate) angle: FastString30,
    pub(crate) strength: FastString30,
    pub(crate) strafe_rotation: FastString30,
    pub(crate) crouch: FastString30,
    pub(crate) blend_out_over_next_cycle: bool,
}
pub(crate) fn supports(name: &str) -> bool {
    matches!(
        name,
        "SkitchShimmyingBehaviour" | "HippyJumpAntic" | "MovingObject"
    )
}

pub(crate) fn construct(
    name: &'static str,
    values: &[Attribute],
    vault: &Arc<AttributeRuntimeVault>,
) -> Result<VaultMotionBehaviour, MotionLoadError> {
    Ok(match name {
        "SkitchShimmyingBehaviour" => {
            let attributes = vault.instance(0x07d3_9b41_aace_e50f, 0xd7ed_bd36_2d7d_2152);
            let text = get(values, "direction").unwrap_or("");
            let desired_direction = match text {
                "right" => ShimmyDirection::Right,
                "left" => ShimmyDirection::Left,
                _ => {
                    return Err(MotionLoadError::InvalidAttribute {
                        key: "direction",
                        value: text.into(),
                    });
                }
            };
            VaultMotionBehaviour::SkitchShimmying {
                attributes,
                desired_direction,
            }
        }
        "HippyJumpAntic" => {
            let trick_height = packed("TrickHeight", "TrickHeight")?;
            let attributes = vault.instance(0x90c3_decf_0105_631e, 0x3d79_dab3_3e8b_d2be);
            let layout = match attributes
                .try_cached_layout(0x5c0)
                .map_err(MotionLoadError::AttributeRuntime)?
            {
                Some(data) => AnimationMotionLayout::Cached(data),
                None => AnimationMotionLayout::DefaultZero,
            };
            VaultMotionBehaviour::HippyJumpAntic {
                trick_height,
                attributes,
                layout,
            }
        }
        "MovingObject" => {
            let attributes = vault.instance(0x97b0_272a_2a7e_54f8, 0xd7ed_bd36_2d7d_2152);
            let x_speed = packed("xSpeed", "")?;
            let z_speed = packed("zSpeed", "")?;
            let angle = packed("angle", "angle")?;
            let strength = packed("strength", "strength")?;
            let strafe_rotation = packed("straferotation", "straferotation")?;
            let crouch = packed("crouch", "crouch")?;
            let blend_out_over_next_cycle = boolean(values, "BlendOutOverNextCycle", false)?;
            let text = get(values, "direction").unwrap_or("");
            let direction = match text {
                "PUSHLEFT" => MovingDirection::PushLeft,
                "PUSHRIGHT" => MovingDirection::PushRight,
                "PUSHLEFTToRIGHT" => MovingDirection::PushLeftToRight,
                "PUSHRIGHTToLEFT" => MovingDirection::PushRightToLeft,
                "PUSHToPULLLEFT" => MovingDirection::PushToPullLeft,
                "PUSHToPULLRIGHT" => MovingDirection::PushToPullRight,
                "PULLLEFT" => MovingDirection::PullLeft,
                "PULLRIGHT" => MovingDirection::PullRight,
                "PULLLEFTToRIGHT" => MovingDirection::PullLeftToRight,
                "PULLRIGHTToLEFT" => MovingDirection::PullRightToLeft,
                "PULLToPUSHLEFT" => MovingDirection::PullToPushLeft,
                "PULLToPUSHRIGHT" => MovingDirection::PullToPushRight,
                "still" => MovingDirection::Still,
                _ => {
                    return Err(MotionLoadError::InvalidAttribute {
                        key: "direction",
                        value: text.into(),
                    });
                }
            };
            VaultMotionBehaviour::MovingObject(Box::new(MovingObject {
                attributes,
                direction,
                x_speed,
                z_speed,
                angle,
                strength,
                strafe_rotation,
                crouch,
                blend_out_over_next_cycle,
            }))
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
