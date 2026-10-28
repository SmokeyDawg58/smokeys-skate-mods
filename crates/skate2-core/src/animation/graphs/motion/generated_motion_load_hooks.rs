//! Four real generated Motion transition-hook constructors.
use super::{HookDeclaration, MotionLoadError, SourceBinding, construct};
use crate::{generated_action_load::construct::BaseObject, play_animation::FastString36};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct MotionHook {
    pub(crate) base: BaseObject,
    pub(crate) payload: HookPayload,
}
#[derive(Debug)]
pub(crate) enum HookPayload {
    GrabSlide {
        right: bool,
    },
    OverrideNextAnimationTransition {
        transition_type: TransitionType,
        blend_length: f32,
        blend_with_current_frame: bool,
        transition_under: bool,
        use_channel_from_weights: bool,
    },
    InitializeLocomotion {
        landing: bool,
        running_out_back: bool,
        running_out_front: bool,
        dismounting: bool,
        starting_foot: u32,
    },
    MongoPushToAntic {
        channel: FastString36,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum TransitionType {
    Play = 1,
    Blend = 2,
    ChannelBlend = 3,
    Sequence = 4,
}

/// Hook callbacks describe `Base::Process`, `AddParam`, `Hook::Process`, `OnTransition`.
pub(crate) fn binding(name: &str) -> Option<SourceBinding> {
    let (name, factory_helper, allocation_size, vtable, on_transition) = match name {
        "GrabSlide" => ("GrabSlide", 0x82c9_2068, 0x1c, 0x8230_0de0, 0x82ca_6ac0),
        "OverideNextAnimTransitionHook" => (
            "OverideNextAnimTransitionHook",
            0x82c9_20c8,
            0x28,
            0x8230_0e14,
            0x82ca_6be8,
        ),
        "InitializeLocomotion" => (
            "InitializeLocomotion",
            0x82c9_2128,
            0x20,
            0x8230_0ef8,
            0x82ca_6cc0,
        ),
        "MongoPushToAntic" => (
            "MongoPushToAntic",
            0x82cb_f4a8,
            0x30,
            0x8230_346c,
            0x82cb_eb00,
        ),
        _ => return None,
    };
    Some(SourceBinding {
        name,
        factory_helper,
        allocation_size,
        vtable,
        callbacks: [0x8251_87c0, 0x82ce_ec78, 0x8251_87c0, on_transition],
    })
}
#[expect(
    clippy::float_cmp,
    reason = "source truncated startingFoot is restricted to exact integer zero or one"
)]
pub(crate) fn construct(
    declaration: &HookDeclaration,
    name: Arc<str>,
) -> Result<MotionHook, MotionLoadError> {
    let values = construct::section_attributes(declaration.attributes)?;
    let base = BaseObject::new(name);
    let payload = match declaration.name {
        "GrabSlide" => HookPayload::GrabSlide {
            right: construct::boolean(values, "right", true)?,
        },
        "OverideNextAnimTransitionHook" => HookPayload::OverrideNextAnimationTransition {
            transition_type: match construct::get(values, "transType") {
                Some("play") => TransitionType::Play,
                Some("sequence") => TransitionType::Sequence,
                Some("channelblend") => TransitionType::ChannelBlend,
                _ => TransitionType::Blend,
            },
            blend_length: construct::float(values, "time", 0.2)?,
            blend_with_current_frame: construct::boolean(values, "blendWithCurrentFrame", false)?,
            transition_under: construct::boolean(values, "transitionUnder", false)?,
            use_channel_from_weights: construct::boolean(values, "useChannelFromWeights", false)?,
        },
        "InitializeLocomotion" => {
            let landing = construct::boolean(values, "isLanding", false)?;
            let running_out_back = construct::boolean(values, "isRunningOutBack", false)?;
            let running_out_front = construct::boolean(values, "isRunningOutFront", false)?;
            let dismounting = construct::boolean(values, "isDismounting", false)?;
            let starting = construct::float(values, "startingFoot", 0.0)?.trunc();
            // Source float-to-int conversion precedes unsigned assertion <2.
            let starting_foot = if starting == 0.0 {
                0
            } else if starting == 1.0 {
                1
            } else {
                return Err(MotionLoadError::InvalidAttribute {
                    key: "startingFoot",
                    value: starting.to_string(),
                });
            };
            HookPayload::InitializeLocomotion {
                landing,
                running_out_back,
                running_out_front,
                dismounting,
                starting_foot,
            }
        }
        "MongoPushToAntic" => HookPayload::MongoPushToAntic {
            channel: construct::packed(
                "anim",
                construct::get(values, "anim").ok_or(MotionLoadError::InvalidAttribute {
                    key: "anim",
                    value: String::new(),
                })?,
            )?,
        },
        name => {
            return Err(MotionLoadError::IncompleteConstructor {
                domain: super::ObjectDomain::Hook,
                index: usize::MAX,
                name,
            });
        }
    };
    Ok(MotionHook { base, payload })
}
