//! Original occurrence constructors, using the shared exported declarations.
use super::GuardBehaviour;
use crate::{pedestrian_graph::conditions::want_type, pedestrian_graph_data as data};

/// Actual entity attributes and cvar values, read by the concrete owner.
pub(crate) struct GuardTuning {
    pub(crate) alert_time: Option<f32>,
    pub(crate) patrol_interest_time: Option<f32>,
    pub(crate) warn_interest_time: Option<f32>,
}
impl GuardTuning {
    /// LWPatrolZoneOwner::Reset82e51780 binds the original default collection
    /// for every pedestrian; this is not a made-up guard resource reference.
    pub(crate) fn load(
        vault: &std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        chase: &crate::attribute_runtime::AttributeRuntimeInstance,
        warn_interest_time: f32,
    ) -> Result<Self, String> {
        let patrol = vault.instance(
            0xb5de_32c9_7060_5efa,
            skate2_formats::attrib::string_hash64("default"),
        );
        if chase.is_null() || patrol.is_null() {
            return Err("native guard tuning collection absent".into());
        }
        let scalar = |instance: &crate::attribute_runtime::AttributeRuntimeInstance,
                      key|
         -> Result<f32, String> {
            Ok(f32::from_bits(
                instance
                    .try_scalar_word(key, 0)
                    .map_err(|e| e.to_string())?
                    .unwrap_or(0),
            ))
        };
        Ok(Self {
            alert_time: Some(scalar(chase, 0xfa76_2b2e_a065_a6d7)?),
            patrol_interest_time: Some(scalar(&patrol, 0x1f4e_7aea_9a02_da2a)?),
            warn_interest_time: Some(warn_interest_time),
        })
    }
}
pub(crate) fn attribute(occurrence: u16, key: &str) -> Result<Option<&'static str>, String> {
    let declaration = data::BEHAVIOURS
        .get(usize::from(occurrence))
        .ok_or_else(|| format!("invalid pedestrian occurrence {occurrence}"))?;
    let section = declaration.attributes;
    if section.count == 0 {
        return Ok(None);
    }
    let start = usize::from(section.index);
    let end = start + usize::from(section.count);
    let attributes = data::ATTRIBUTES
        .get(start..end)
        .ok_or_else(|| format!("invalid pedestrian attributes {occurrence}"))?;
    Ok(attributes.iter().find(|a| a.key == key).map(|a| a.value))
}
fn want(occurrence: u16, key: &str) -> Result<usize, String> {
    let name = attribute(occurrence, key)?
        .ok_or_else(|| format!("missing native {key} for occurrence {occurrence}"))?;
    want_type(name).ok_or_else(|| format!("invalid native want {name}"))
}
pub(crate) fn boolean(occurrence: u16, key: &str, default: bool) -> Result<bool, String> {
    // GenAttributeMap::GetBool828fb8a8 accepts lowercase literal names only.
    match attribute(occurrence, key)? {
        None => Ok(default),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(v) => Err(format!("invalid native Boolean {key}={v}")),
    }
}
pub(crate) fn float(occurrence: u16, key: &str, default: Option<f32>) -> Result<f32, String> {
    match attribute(occurrence, key)? {
        Some(v) => v
            .parse()
            .map_err(|_| format!("invalid native float {key}={v}")),
        None => default.ok_or_else(|| format!("missing native float {key}")),
    }
}
/// MoodMan::GetEndChaseReasonEnumFromString82ce6cb8.
pub(crate) fn end_chase_reason(name: &str) -> Option<i32> {
    [
        "default",
        "aggressivecapture",
        "returntopatrolzone",
        "lostinterest",
    ]
    .iter()
    .position(|s| s.eq_ignore_ascii_case(name))
    .map(|n| n as i32)
}
impl GuardBehaviour {
    pub(crate) fn bind(occurrence: u16, tuning: &GuardTuning) -> Result<Option<Self>, String> {
        let name = data::BEHAVIOURS
            .get(usize::from(occurrence))
            .ok_or_else(|| format!("invalid pedestrian occurrence {occurrence}"))?
            .name;
        Ok(Some(match name {
            "UnsetWant" => Self::UnsetWant {
                want: want(occurrence, "want")?,
            },
            "KnowAboutWantTarget" => Self::KnowAboutWantTarget {
                want: want(occurrence, "want")?, //ctor826eda80
            },
            "UnsetWantAndSuppressMoodForATime" => Self::UnsetWantAndSuppressMood {
                want: want(occurrence, "want")?,
                time: float(occurrence, "timeout", Some(1.0))?, //ctor826edd48
            },
            "ActivateRelatedWant" => Self::ActivateRelatedWant {
                original: want(occurrence, "originalWant")?,
                related: want(occurrence, "relatedWant")?,
                deactivate_original: boolean(occurrence, "deactivateOriginalWant", true)?,
            },
            "SetAlertTimer" => Self::SetAlertTimer {
                authored_alert_time: tuning.alert_time.ok_or("SetAlertTimer live tuning absent")?,
            },
            "SetPatrolZoneMaintainInterestTimerOnBegin" => Self::SetPatrolInterestTimer {
                authored_interest_time: tuning.patrol_interest_time.ok_or("SetPatrolInterestTimer live tuning absent")?,
            },
            "ChaserEndChase" | "ChaserGroupEndChase" => {
                let value =
                    attribute(occurrence, "reason")?.ok_or("missing native end chase reason")?;
                let reason = end_chase_reason(value)
                    .ok_or_else(|| format!("invalid native end chase reason {value}"))?;
                if name == "ChaserEndChase" {
                    Self::ChaserEndChase { reason }
                } else {
                    Self::ChaserGroupEndChase {
                        reason,
                        aggressive_capture_reason: 1,
                    }
                }
            }
            "ChannelWarnWantTarget" => Self::ChannelWarnWantTarget {
                want: want(occurrence, "want")?,
                deactivate_want: boolean(occurrence, "deactivateWant", true)?,
                warn_interest_time: tuning.warn_interest_time.ok_or("ChannelWarnWantTarget live tuning absent")?,
            },
            "RegisterTazer" => Self::RegisterTazer {
                want: want(occurrence, "want")?,
            },
            "UnregisterTazerOnEnd" => Self::UnregisterTazerOnEnd,
            "EndTaze" => Self::EndTaze,
            "NewChasee" => Self::NewChasee,
            "EndChase" => Self::EndChase,
            "MoodResetAboutChasee" => Self::MoodResetAboutChasee,
            "SuppressMoodAboutChasee" => Self::SuppressMoodAboutChasee {
                time: float(occurrence, "timeout", None)?,
            },
            "ReturnToPatrolZone" => Self::ReturnToPatrolZone,
            "SetAltTargetToChaseePosition" => Self::SetAltTargetToChaseePosition,
            "KnowAboutChasee" => Self::KnowAboutChasee,
            "BlockChasee" => {
                let distance = float(occurrence, "attargetdist", Some(1.0))?;
                Self::BlockChasee {
                    at_target_distance_squared: distance * distance,
                    slow_speed: float(occurrence, "slowspeed", Some(1.0))?,
                }
            }
            "PursueChasee" => Self::PursueChasee,
            _ => return Ok(None),
        }))
    }
}
