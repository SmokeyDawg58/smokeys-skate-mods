//! Real generated Motion constructor families, starting with mandatory root.
use super::{BehaviourDeclaration, MotionLoadError, SourceBinding, data};
use crate::{
    generated_action_load::construct::BaseObject,
    play_animation::{FastString30, FastString36},
};
use skate2_formats::andale::PackedFastString;
use std::sync::Arc;
#[path = "generated_motion_load_bindings.rs"]
pub(crate) mod bindings;
#[path = "generated_motion_load_control.rs"]
pub(crate) mod control;
#[path = "generated_motion_load_filter.rs"]
pub(crate) mod filter;
#[path = "generated_motion_load_names.rs"]
pub(crate) mod names;
#[path = "generated_motion_load_plain.rs"]
pub(crate) mod plain;
#[path = "generated_motion_load_vault.rs"]
pub(crate) mod vault;

#[derive(Debug)]
pub(crate) enum MotionBehaviourPayload {
    Vault(vault::VaultMotionBehaviour),
    Named(names::NamedMotionBehaviour),
    EnterSkitchingBehaviour(MotionVaultOwner),
    SkitchingBehaviour(MotionVaultOwner),
    FilterMotionGraphIntent(Box<filter::FilteredIntent>),
    Control(control::MotionControlBehaviour),
    Plain(plain::PlainMotionBehaviour),
    CharacterGesture,
    UpdateTimeSinceTeleport,
    UpdateRidingFakie {
        high_speed_threshold: f32,
        low_speed_threshold: f32,
        time_slowly_rolling_backwards_threshold: f32,
        time_from_teleport_threshold: f32,
    },
    FakieHeadChannel {
        channel: FastString36,
    },
    AttachIntent {
        intent: FastString36,
        attribute: FastString30,
        settable: bool,
    },
    ToggleBoard(Box<ToggleBoardNames>),
    CreateAttribute(CreateAttribute),
    PlayAnimation(crate::play_animation::PlayAnimationBehaviour),
    SettingBodyTilt {
        tilt_x: FastString30,
    },
    PowerSliding,
    BodySpin,
    UpdateTimeSinceKickturn,
    StoreLandingData,
    ResetSkaterAnimation,
    ResetToGivenStance,
    SetScoreAugmentation {
        augmentation: TrickAugmentation,
        mirrored: Option<TrickAugmentation>,
    },
    SharedAction(crate::generated_action_load::BehaviourObject),
    ControlAirLegExtension {
        drive_distance_to_com: FastString30,
        bone: LegExtensionBone,
    },
}
pub(crate) struct MotionVaultOwner {
    pub(crate) attributes: crate::attribute_runtime::AttributeRuntimeInstance,
}
impl std::fmt::Debug for MotionVaultOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionVaultOwner")
            .field("null_collection", &self.attributes.is_null())
            .finish()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum LegExtensionBone {
    SkateboardRoot = 0,
    LeftToeBase = 1,
    RightToeBase = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum TrickAugmentation {
    FsPowerslide = 0,
    BsPowerslide = 1,
    FsRevert = 2,
    BsRevert = 3,
    NoseManual = 4,
    TailManual = 5,
    Wipeout = 6,
    Landing = 7,
    RideIdle = 8,
    Switching = 9,
}
#[derive(Debug)]
pub(crate) struct ToggleBoardNames {
    pub(crate) retrieve: FastString36,
    pub(crate) retrieving: FastString36,
    pub(crate) dropping: FastString36,
    pub(crate) throw: FastString36,
    pub(crate) drop: FastString36,
    pub(crate) retrieve_front: [FastString36; 3],
    pub(crate) retrieve_back: [FastString36; 3],
    pub(crate) yaw: FastString30,
    pub(crate) pitch: FastString30,
}
#[derive(Debug)]
pub(crate) struct AttributePhaseValue {
    pub(crate) active: bool,
    pub(crate) value: f32,
}
#[derive(Debug)]
pub(crate) struct CreateAttribute {
    pub(crate) name: FastString30,
    pub(crate) begin: AttributePhaseValue,
    pub(crate) update: AttributePhaseValue,
    pub(crate) end: AttributePhaseValue,
    pub(crate) settable: bool,
}
#[derive(Debug)]
pub(crate) struct MotionBehaviour {
    pub(crate) base: BaseObject,
    pub(crate) payload: MotionBehaviourPayload,
}
pub(crate) fn binding(name: &str) -> Option<SourceBinding> {
    // Every declared factory family now has its actual typed constructor.
    bindings::source_binding(name)
}

fn attributes(
    declaration: &BehaviourDeclaration,
) -> Result<&'static [super::Attribute], MotionLoadError> {
    section_attributes(declaration.attributes)
}
pub(super) fn section_attributes(
    section: super::Section,
) -> Result<&'static [super::Attribute], MotionLoadError> {
    if section.count == 0 {
        return Ok(&[]);
    }
    let start = usize::from(section.index);
    data::ATTRIBUTES
        .get(start..start + usize::from(section.count))
        .ok_or(MotionLoadError::DescriptorBounds)
}

/// Adapter for immutable generated declarations into the existing typed
/// `PlayAnimation` parser. Only fields read numerically are converted; textual
/// fields do not acquire invented numeric semantics.
fn play_attributes(
    values: &[super::Attribute],
) -> Result<Vec<skate2_formats::state_graph::StateGraphBinaryAttribute>, MotionLoadError> {
    values
        .iter()
        .map(|attribute| {
            let value = match attribute.key {
                "time" => float(values, "time", 0.0)?,
                "playBackSpeed" => float(values, "playBackSpeed", 0.0)?,
                "defaultValue" => float(values, "defaultValue", 0.0)?,
                _ => 0.0,
            };
            let boolean = match attribute.key {
                "blendWithCurrentFrame" => boolean(values, "blendWithCurrentFrame", false)?,
                "transitionUnder" => boolean(values, "transitionUnder", false)?,
                "useChannelFromWeights" => boolean(values, "useChannelFromWeights", false)?,
                "normalize" => boolean(values, "normalize", false)?,
                _ => false,
            };
            Ok(skate2_formats::state_graph::StateGraphBinaryAttribute {
                key: attribute.key.as_bytes().to_vec(),
                key_hash: skate2_formats::state_graph::state_graph_string_hash32(
                    attribute.key.as_bytes(),
                ),
                text: attribute.value.as_bytes().to_vec(),
                float_bits: value.to_bits(),
                boolean_raw: u8::from(boolean),
            })
        })
        .collect()
}
pub(crate) fn add_parameter(
    object: &mut MotionBehaviour,
    section: super::Section,
    index: usize,
) -> Result<(), MotionLoadError> {
    match &mut object.payload {
        MotionBehaviourPayload::PlayAnimation(animation) => animation
            .add_param(&play_attributes(section_attributes(section)?)?)
            .map_err(MotionLoadError::PlayAnimation),
        _ => Err(MotionLoadError::UnsupportedParameters(index)),
    }
}
pub(super) fn get<'a>(values: &'a [super::Attribute], key: &str) -> Option<&'a str> {
    values
        .iter()
        .find(|attribute| attribute.key == key)
        .map(|attribute| attribute.value)
}
#[expect(
    clippy::cast_possible_truncation,
    reason = "GenAttributeMap GetFloat uses CRT atof double followed by source frsp"
)]
pub(super) fn float(
    values: &[super::Attribute],
    key: &'static str,
    default: f32,
) -> Result<f32, MotionLoadError> {
    let Some(text) = get(values, key) else {
        return Ok(default);
    };
    let value = text
        .parse::<f64>()
        .map_err(|_| MotionLoadError::InvalidAttribute {
            key,
            value: text.into(),
        })? as f32;
    if !value.is_finite() {
        return Err(MotionLoadError::InvalidAttribute {
            key,
            value: text.into(),
        });
    }
    Ok(value)
}
pub(super) fn packed<const N: usize>(
    field: &'static str,
    text: &str,
) -> Result<PackedFastString<N>, MotionLoadError> {
    PackedFastString::encode(text).map_err(|_| MotionLoadError::PackedString(field))
}
pub(super) fn boolean(
    values: &[super::Attribute],
    key: &'static str,
    default: bool,
) -> Result<bool, MotionLoadError> {
    match get(values, key) {
        None => Ok(default),
        Some("false") => Ok(false),
        Some("true") => Ok(true),
        Some(value) => Err(MotionLoadError::InvalidAttribute {
            key,
            value: value.into(),
        }),
    }
}
fn create_attribute(values: &[super::Attribute]) -> Result<CreateAttribute, MotionLoadError> {
    let name = get(values, "attName").ok_or(MotionLoadError::InvalidAttribute {
        key: "attName",
        value: String::new(),
    })?;
    let mut result = CreateAttribute {
        name: packed("attName", name)?,
        begin: AttributePhaseValue {
            active: false,
            value: 0.0,
        },
        update: AttributePhaseValue {
            active: false,
            value: 0.0,
        },
        end: AttributePhaseValue {
            active: false,
            value: 0.0,
        },
        settable: false,
    };
    if get(values, "always").is_some() {
        result.begin.active = true;
        result.begin.value = float(values, "always", 0.0)?;
        result.update.active = true;
        result.update.value = float(values, "always", 0.0)?;
        result.end.active = true;
        // Native82cab89c stores +38 UPDATE again; END+3c stays zero.
        result.update.value = float(values, "always", 0.0)?;
        return Ok(result);
    }
    if get(values, "begin").is_none()
        && get(values, "update").is_none()
        && get(values, "end").is_none()
    {
        result.update.active = true;
        return Ok(result);
    }
    for (key, phase) in [
        ("begin", &mut result.begin),
        ("update", &mut result.update),
        ("end", &mut result.end),
    ] {
        if get(values, key).is_some() {
            phase.active = true;
            phase.value = float(values, key, 0.0)?;
        }
    }
    result.settable = boolean(values, "set", false)?;
    Ok(result)
}
fn augmentation(key: &'static str, text: &str) -> Result<TrickAugmentation, MotionLoadError> {
    Ok(match text {
        "FSPowerslide" => TrickAugmentation::FsPowerslide,
        "BSPowerslide" => TrickAugmentation::BsPowerslide,
        "FSRevert" => TrickAugmentation::FsRevert,
        "BSRevert" => TrickAugmentation::BsRevert,
        "NoseManual" => TrickAugmentation::NoseManual,
        "TailManual" => TrickAugmentation::TailManual,
        "Wipeout" => TrickAugmentation::Wipeout,
        "Landing" => TrickAugmentation::Landing,
        "RideIdle" => TrickAugmentation::RideIdle,
        "Switching" => TrickAugmentation::Switching,
        _ => {
            return Err(MotionLoadError::InvalidAttribute {
                key,
                value: text.into(),
            });
        }
    })
}
pub(crate) fn behaviour(
    declaration: &BehaviourDeclaration,
    name: Arc<str>,
) -> Result<MotionBehaviour, MotionLoadError> {
    behaviour_with_vault(declaration, name, None)
}
#[expect(
    clippy::too_many_lines,
    reason = "source behaviour family dispatch preserves typed constructors"
)]
pub(crate) fn behaviour_with_vault(
    declaration: &BehaviourDeclaration,
    name: Arc<str>,
    vault: Option<&Arc<crate::attribute_runtime::AttributeRuntimeVault>>,
) -> Result<MotionBehaviour, MotionLoadError> {
    let values = attributes(declaration)?;
    let base = BaseObject::new(name);
    let payload = match declaration.name {
        name if self::vault::supports(name) => MotionBehaviourPayload::Vault(
            self::vault::construct(name, values, vault.ok_or(MotionLoadError::MissingVault)?)?,
        ),
        name if names::supports(name) => {
            MotionBehaviourPayload::Named(names::construct(name, values)?)
        }
        "EnterSkitchingBehaviour" | "SkitchingBehaviour" => {
            let owner = MotionVaultOwner {
                attributes: vault
                    .ok_or(MotionLoadError::MissingVault)?
                    .instance(0x07d3_9b41_aace_e50f, 0xd7ed_bd36_2d7d_2152),
            };
            if declaration.name == "EnterSkitchingBehaviour" {
                MotionBehaviourPayload::EnterSkitchingBehaviour(owner)
            } else {
                MotionBehaviourPayload::SkitchingBehaviour(owner)
            }
        }
        "FilterMotionGraphIntent" => {
            MotionBehaviourPayload::FilterMotionGraphIntent(Box::new(filter::construct(values)?))
        }
        name if control::supports(name) => {
            MotionBehaviourPayload::Control(control::construct(name, values)?)
        }
        "CharacterGesture" => MotionBehaviourPayload::CharacterGesture,
        "UpdateTimeSinceTeleport" => MotionBehaviourPayload::UpdateTimeSinceTeleport,
        "PowerSliding" => MotionBehaviourPayload::PowerSliding,
        "BodySpin" => MotionBehaviourPayload::BodySpin,
        "UpdateTimeSinceKickturn" => MotionBehaviourPayload::UpdateTimeSinceKickturn,
        "StoreLandingData" => MotionBehaviourPayload::StoreLandingData,
        "ResetSkaterAnimation" => MotionBehaviourPayload::ResetSkaterAnimation,
        "ResetToGivenStance" => MotionBehaviourPayload::ResetToGivenStance,
        "SettingBodyTilt" => MotionBehaviourPayload::SettingBodyTilt {
            tilt_x: packed("tilt_x", get(values, "tilt_x").unwrap_or("tilt_x"))?,
        },
        "SetScoreAugmentation" => {
            let mut augmentation = augmentation(
                "augment",
                get(values, "augment").ok_or(MotionLoadError::InvalidAttribute {
                    key: "augment",
                    value: String::new(),
                })?,
            )?;
            let mirrored = if let Some(text) = get(values, "mirrorAugment") {
                let selected = self::augmentation("mirrorAugment", text)?;
                if selected == TrickAugmentation::Switching {
                    // Source82c8ef44 writes primary+1c, leaves mirror+20 unknown.
                    augmentation = selected;
                    None
                } else {
                    Some(selected)
                }
            } else {
                Some(augmentation)
            };
            MotionBehaviourPayload::SetScoreAugmentation {
                augmentation,
                mirrored,
            }
        }
        "PrintText2D" => MotionBehaviourPayload::SharedAction(
            crate::generated_action_load::construct::behaviour_from_attributes(
                crate::generated_action_load::BehaviourClass::PrintText2D,
                Arc::clone(&base.name),
                values,
            )
            .map_err(MotionLoadError::Shared)?,
        ),
        "UpdateRidingFakie" => MotionBehaviourPayload::UpdateRidingFakie {
            high_speed_threshold: float(values, "highSpeedThreshold", -1.0)?,
            low_speed_threshold: float(values, "lowSpeedThreshold", -0.5)?,
            time_slowly_rolling_backwards_threshold: float(
                values,
                "timeSlowlyRollingBackwardsThreshold",
                0.2,
            )?,
            time_from_teleport_threshold: float(values, "timeFromTeleportThreshold", 3.0)?,
        },
        "FakieHeadChannel" => MotionBehaviourPayload::FakieHeadChannel {
            channel: packed("fakie", "fakie")?,
        },
        "AttachIntent" => MotionBehaviourPayload::AttachIntent {
            intent: packed("intent", get(values, "intent").unwrap_or("0"))?,
            attribute: packed("attr", get(values, "attr").unwrap_or("0"))?,
            settable: match get(values, "set") {
                None | Some("false") => false,
                Some("true") => true,
                Some(value) => {
                    return Err(MotionLoadError::InvalidAttribute {
                        key: "set",
                        value: value.into(),
                    });
                }
            },
        },
        "ToggleBoard" => MotionBehaviourPayload::ToggleBoard(Box::new(ToggleBoardNames {
            retrieve: packed("retrieve", "OB_RetrieveBoard")?,
            retrieving: packed("retrieving", "OB_RetrievingBoard")?,
            dropping: packed("dropping", "OB_DroppingBoard")?,
            throw: packed("throw", "OFB_THROW_0")?,
            drop: packed("drop", "OFB_THROW_90")?,
            retrieve_front: [
                packed("frontInto", "OFB_RETRIEVE_HIGH_FRONT_INTO")?,
                packed("frontCycle", "OFB_RETRIEVE_HIGH_FRONT_CYC")?,
                packed("frontOut", "OFB_RETRIEVE_HIGH_FRONT_OUT")?,
            ],
            retrieve_back: [
                packed("backInto", "OFB_RETRIEVE_HIGH_BACK_INTO")?,
                packed("backCycle", "OFB_RETRIEVE_HIGH_BACK_CYC")?,
                packed("backOut", "OFB_RETRIEVE_HIGH_BACK_OUT")?,
            ],
            yaw: packed("yaw", "yaw")?,
            pitch: packed("pitch", "pitch")?,
        })),
        "CreateAttribute" => MotionBehaviourPayload::CreateAttribute(create_attribute(values)?),
        "PlayAnimation" => MotionBehaviourPayload::PlayAnimation(
            crate::play_animation::PlayAnimationBehaviour::from_attributes(&play_attributes(
                values,
            )?)
            .map_err(MotionLoadError::PlayAnimation)?,
        ),
        "ControlAirLegExtension" => MotionBehaviourPayload::ControlAirLegExtension {
            drive_distance_to_com: packed(
                "driveDistToCom",
                get(values, "driveDistToCom").unwrap_or("disttocog"),
            )?,
            bone: match get(values, "bone").unwrap_or("Skateboard_Root") {
                "RightToeBase" => LegExtensionBone::RightToeBase,
                "LeftToeBase" => LegExtensionBone::LeftToeBase,
                _ => LegExtensionBone::SkateboardRoot,
            },
        },
        name if plain::PlainMotionBehaviour::from_name(name).is_some() => {
            MotionBehaviourPayload::Plain(
                plain::PlainMotionBehaviour::from_name(name).expect("matched source class"),
            )
        }
        name => {
            return Err(MotionLoadError::IncompleteConstructor {
                domain: super::ObjectDomain::Behaviour,
                index: usize::MAX,
                name,
            });
        }
    };
    Ok(MotionBehaviour { base, payload })
}
