//! Concrete constructors required by the mounted Default camera graph.
//! Leaf execution remains a separate producer; construction evaluates no condition.
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_action_load::{
        GeneratedActionLoadError,
        construct::{
            BehaviourPayload, Comparison, ConstructorAttributes, comparison_from_lookup,
            print_text_from_lookup,
        },
    },
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric},
    state_graph_runtime::{LeafCreateRequest, LeafKind, StateGraphLeafFactory},
};
use skate2_formats::state_graph::{StateGraphBinaryAttribute, state_graph_string_hash32};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CameraCondition {
    OnRoad,
    BoardOnGround,
    WipingOut,
    InManual,
    Grinding,
    WasGrinding,
    Reentry,
    PreparingJump,
    LedgeLeft,
    LedgeRight,
    TurningCentredTime,
    SubjectSpeed,
    PredictedAirTime,
    PredictedLandingHeight,
    PredictedMaxHeight,
    PredictedHeightDistanceRatio,
    FrontsideAngle,
    GrindInclineAngle,
    TransferAngle,
    LaunchInclineAngle,
    UnderThreat,
    Offboard,
    AirOffboard,
    Skitching,
    MovingObject,
    FootPlant,
    HandPlant,
    Boneless,
    HippyJump,
    RunningOut,
    CameraType,
    PreviousShot,
    PainterRegion,
    Volume,
}
/// Actual factory family and source address; no generic state-name inference.
const CONDITIONS: [(&str, CameraCondition, u32, bool); 34] = [
    (
        "SkaterIsOnRoad",
        CameraCondition::OnRoad,
        0x82e3_2b28,
        false,
    ),
    (
        "IsPlayerSkateboardOnGround",
        CameraCondition::BoardOnGround,
        0x82e3_2b88,
        false,
    ),
    (
        "IsWipingOut",
        CameraCondition::WipingOut,
        0x82e3_2be8,
        false,
    ),
    ("IsInManual", CameraCondition::InManual, 0x82e3_2ca8, false),
    (
        "SkaterIsGrinding",
        CameraCondition::Grinding,
        0x82e3_2d08,
        false,
    ),
    (
        "SkaterWasGrinding",
        CameraCondition::WasGrinding,
        0x82e3_2d68,
        false,
    ),
    ("IsReentry", CameraCondition::Reentry, 0x82e3_2f48, false),
    (
        "IsPreparingToJump",
        CameraCondition::PreparingJump,
        0x82e3_3008,
        false,
    ),
    (
        "IsGrindLedgeLeft",
        CameraCondition::LedgeLeft,
        0x82e3_3068,
        false,
    ),
    (
        "IsGrindLedgeRight",
        CameraCondition::LedgeRight,
        0x82e3_30c8,
        false,
    ),
    (
        "TurningCentredTime",
        CameraCondition::TurningCentredTime,
        0x82e3_3128,
        true,
    ),
    (
        "CameraSubjectSpeed",
        CameraCondition::SubjectSpeed,
        0x82e3_31e8,
        true,
    ),
    (
        "PredictedAirTime",
        CameraCondition::PredictedAirTime,
        0x82e3_3368,
        true,
    ),
    (
        "PredictedLandingHeight",
        CameraCondition::PredictedLandingHeight,
        0x82e3_33c8,
        true,
    ),
    (
        "PredictedMaxHeight",
        CameraCondition::PredictedMaxHeight,
        0x82e3_3428,
        true,
    ),
    (
        "PredictedHeightDistanceRatio",
        CameraCondition::PredictedHeightDistanceRatio,
        0x82e3_3488,
        true,
    ),
    (
        "FrontsideAngle",
        CameraCondition::FrontsideAngle,
        0x82e3_34e8,
        true,
    ),
    (
        "GrindInclineAngle",
        CameraCondition::GrindInclineAngle,
        0x82e3_3668,
        true,
    ),
    (
        "TransferAngle",
        CameraCondition::TransferAngle,
        0x82e3_36c8,
        true,
    ),
    (
        "LaunchInclineAngle",
        CameraCondition::LaunchInclineAngle,
        0x82e3_3728,
        true,
    ),
    (
        "SkaterIsUnderThreat",
        CameraCondition::UnderThreat,
        0x82e3_38a8,
        false,
    ),
    ("IsOffboard", CameraCondition::Offboard, 0x82e3_3a28, false),
    (
        "IsAirOffboard",
        CameraCondition::AirOffboard,
        0x82e3_3a88,
        false,
    ),
    (
        "SkaterIsSkitching",
        CameraCondition::Skitching,
        0x82e3_3ae8,
        false,
    ),
    (
        "SkaterIsMovingObject",
        CameraCondition::MovingObject,
        0x82e3_3b48,
        false,
    ),
    (
        "SkaterIsPerformingFootPlant",
        CameraCondition::FootPlant,
        0x82e3_3ba8,
        false,
    ),
    (
        "SkaterIsPerformingHandPlant",
        CameraCondition::HandPlant,
        0x82e3_3c08,
        false,
    ),
    (
        "SkaterIsPerformingBoneless",
        CameraCondition::Boneless,
        0x82e3_3c68,
        false,
    ),
    (
        "SkaterIsPerformingHippyJump",
        CameraCondition::HippyJump,
        0x82e3_3cc8,
        false,
    ),
    (
        "SkaterIsRunningOut",
        CameraCondition::RunningOut,
        0x82e3_3d28,
        false,
    ),
    (
        "IsCameraTypeActive",
        CameraCondition::CameraType,
        0x82e3_3968,
        false,
    ),
    (
        "IsPreviousShot",
        CameraCondition::PreviousShot,
        0x82e3_9e50,
        false,
    ),
    (
        "IsInWorldPainterRegion",
        CameraCondition::PainterRegion,
        0x82e3_ab10,
        false,
    ),
    ("IsInVolume", CameraCondition::Volume, 0x82e3_ab70, false),
];
pub(crate) enum CameraConditionData {
    Plain,
    Comparison(Comparison),
    CameraType(i32),
    PreviousShots(Vec<u32>),
    PainterRegion { layer: u32, region: Arc<str> },
    Volume(Arc<str>),
}
pub(crate) enum CameraLeaf {
    Condition {
        class: CameraCondition,
        data: CameraConditionData,
    },
    ChooseShot {
        shots: Vec<Arc<str>>,
        transition_in: f32,
        transition_out: f32,
    },
    PrintText(BehaviourPayload),
    SlowMotion {
        attributes: AttributeRuntimeInstance,
    },
}
#[derive(Debug)]
pub(crate) enum CameraFactoryError {
    Encoding,
    Unsupported,
    Allocation,
    Attributes(GeneratedActionLoadError),
    Numeric(ReckoningHostNumericError),
}
impl std::fmt::Display for CameraFactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "camera graph constructor: {self:?}")
    }
}
impl std::error::Error for CameraFactoryError {}
struct BinaryAttributes<'a>(&'a [StateGraphBinaryAttribute]);
impl BinaryAttributes<'_> {
    fn find(&self, key: &str) -> Option<&StateGraphBinaryAttribute> {
        let hash = state_graph_string_hash32(key.as_bytes());
        self.0.iter().find(|attribute| attribute.key_hash == hash)
    }
    fn float(&self, key: &str, default: f32) -> f32 {
        self.find(key)
            .map_or(default, StateGraphBinaryAttribute::float_value)
    }
}
impl ConstructorAttributes for BinaryAttributes<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.find(key).map(|attribute| {
            std::str::from_utf8(&attribute.text).expect("constructor admitted UTF-8 attributes")
        })
    }
    fn required_float(&self, key: &'static str) -> Result<f32, GeneratedActionLoadError> {
        self.find(key)
            .map(StateGraphBinaryAttribute::float_value)
            .ok_or(GeneratedActionLoadError::MissingAttribute(key))
    }
    fn boolean(&self, key: &'static str, default: bool) -> Result<bool, GeneratedActionLoadError> {
        Ok(self
            .find(key)
            .map_or(default, |attribute| attribute.boolean_raw != 0))
    }
}
pub(crate) struct CameraLeafFactory {
    conditions: Vec<Vec<u8>>,
    behaviours: Vec<Vec<u8>>,
    vault: Arc<AttributeRuntimeVault>,
}
impl CameraLeafFactory {
    pub(crate) fn new(vault: Arc<AttributeRuntimeVault>) -> Self {
        Self {
            conditions: CONDITIONS
                .iter()
                .map(|(name, ..)| name.as_bytes().to_vec())
                .collect(),
            behaviours: ["CameraChooseShot", "PrintText2D", "SlowMotionController"]
                .map(|name| name.as_bytes().to_vec())
                .into(),
            vault,
        }
    }
    fn condition(
        name: &[u8],
        attrs: &BinaryAttributes<'_>,
    ) -> Result<CameraLeaf, CameraFactoryError> {
        use CameraFactoryError as E;
        let (_, class, _, comparison) = CONDITIONS
            .iter()
            .find(|(n, ..)| n.as_bytes() == name)
            .ok_or(E::Unsupported)?;
        let data = if *comparison {
            CameraConditionData::Comparison(comparison_from_lookup(attrs).map_err(E::Attributes)?)
        } else {
            match class {
                CameraCondition::CameraType => CameraConditionData::CameraType(
                    HostReckoningNumeric
                        .truncate_i32(attrs.required_float("type").map_err(E::Attributes)?)
                        .map_err(E::Numeric)?,
                ),
                CameraCondition::PreviousShot => {
                    let mut hashes = Vec::new();
                    while let Some(shot) = attrs.get(&format!("shot{}", hashes.len() + 1)) {
                        hashes.try_reserve(1).map_err(|_| E::Allocation)?;
                        hashes.push(state_graph_string_hash32(shot.as_bytes()));
                    }
                    CameraConditionData::PreviousShots(hashes)
                }
                CameraCondition::PainterRegion => CameraConditionData::PainterRegion {
                    layer: 12,
                    region: Arc::from(attrs.required("type").map_err(E::Attributes)?),
                },
                CameraCondition::Volume => CameraConditionData::Volume(Arc::from(
                    attrs.required("volume").map_err(E::Attributes)?,
                )),
                _ => CameraConditionData::Plain,
            }
        };
        Ok(CameraLeaf::Condition {
            class: *class,
            data,
        })
    }
}
impl StateGraphLeafFactory for CameraLeafFactory {
    type Payload = CameraLeaf;
    type Error = CameraFactoryError;
    fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
        match kind {
            LeafKind::Condition => &self.conditions,
            LeafKind::Behaviour => &self.behaviours,
            LeafKind::Hook => &[],
        }
    }
    fn create(&mut self, request: LeafCreateRequest<'_>) -> Result<CameraLeaf, CameraFactoryError> {
        use CameraFactoryError as E;
        // Explicit authored UTF-8 admission; numeric values are never reparsed.
        if request
            .attributes
            .iter()
            .any(|a| std::str::from_utf8(&a.text).is_err())
        {
            return Err(E::Encoding);
        }
        let attrs = BinaryAttributes(request.attributes);
        if request.kind == LeafKind::Condition {
            return Self::condition(request.registered_name, &attrs);
        }
        match request.registered_name {
            b"CameraChooseShot" => {
                let mut shots = Vec::new();
                while let Some(shot) = attrs.get(&format!("shot{}", shots.len() + 1)) {
                    shots.try_reserve(1).map_err(|_| E::Allocation)?;
                    shots.push(Arc::from(shot));
                }
                let transition_in = attrs.float("transitionIn", -1.0);
                let transition_out = attrs.float("transitionOut", -1.0);
                Ok(CameraLeaf::ChooseShot {
                    shots,
                    transition_in,
                    transition_out,
                })
            }
            b"PrintText2D" => Ok(CameraLeaf::PrintText(
                print_text_from_lookup(&attrs).map_err(E::Attributes)?,
            )),
            b"SlowMotionController" => Ok(CameraLeaf::SlowMotion {
                attributes: self
                    .vault
                    .instance(0x0a12_455a_1506_9596, 0xd7ed_bd36_2d7d_2152),
            }),
            _ => Err(E::Unsupported),
        }
    }
}

#[cfg(test)]
#[path = "camera_graph_factory_tests.rs"]
mod tests;
