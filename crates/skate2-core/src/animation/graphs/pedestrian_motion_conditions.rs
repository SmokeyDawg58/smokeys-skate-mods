//! Concrete constructors and direct predicates for all102 pedestrian motion
//! declarations. Generic HasIntent/WillExpire kernels and state-path lookup are
//! reused. External services supply actual retained entity/animation facts.
use super::{ControllerState, MotionDeclarations};
use crate::{
    generated_action_load::{Attribute, StateDeclaration},
    motion_graph_conditions::{HasIntentCollaborator, HasIntentCondition,
        WillExpireCollaborator, WillExpireCondition},
};
use skate2_formats::state_graph::{StateGraphBinaryAttribute, state_graph_string_hash32};

pub(crate) enum Condition {
    HasIntent(HasIntentCondition),
    WillExpire(WillExpireCondition),
    LastState(&'static str),
    HasPlugin,
    BranchWindowEnabled,
    InTurnBranchWindow,
    InBranchWindow(&'static str),
    JumperLanding,
    JumperDownward,
}
pub(crate) struct Conditions { entries: Vec<Condition>, states: &'static [StateDeclaration] }

/// Original tBranchWindow fields; GetCurrentBranchWindow scans authored order.
pub(crate) use crate::attribute_runtime::PedestrianBranchWindow as BranchWindow;
pub(crate) struct BranchAttributes<'a> {
    pub(crate) count: i32,
    pub(crate) windows: &'a [BranchWindow],
}
pub(crate) struct BranchInfo {
    pub(crate) kind: u32,
    pub(crate) time: f32,
}
/// Optional interfaces remain optional. A missing required owner must be an
/// error, never a fabricated false value. Both shared collaborators must refer
/// to the same authority already passed to the concrete motion backend.
pub(crate) trait Services: HasIntentCollaborator + WillExpireCollaborator {
    fn has_plugin(&mut self) -> Result<bool, String>;
    fn has_animatable(&self) -> bool;
    fn branch_attributes(&self) -> Result<Option<BranchAttributes<'_>>, String>;
    fn animation_time(&self) -> Result<f32, String>;
    fn animation_transitioning(&self) -> Result<bool, String>;
    fn animation_expiration(&self) -> Result<(bool,f32), String>;
    fn branch_info(&mut self, authored_name: &str) -> Result<Option<BranchInfo>, String>;
    fn branch_window_duration(&self) -> Result<f32, String>;
    /// None is a null Context entity. SpatialAdjust's uninitialized downward
    /// value must produce an error rather than Some(false).
    fn is_jumping(&self) -> Result<Option<bool>, String>;
    fn is_jumping_downward(&self) -> Result<Option<bool>, String>;
}

fn attributes(declarations: &MotionDeclarations, index: u16) -> Result<&'static [Attribute], String> {
    let declaration = declarations.conditions.get(usize::from(index)).ok_or("motion condition index")?;
    let section = declaration.attributes;
    if section.count == 0 { return Ok(&[]); }
    let start = usize::from(section.index);
    declarations.attributes.get(start..start + usize::from(section.count)).ok_or_else(|| "motion condition attributes".into())
}
fn required(values: &'static [Attribute], key: &str) -> Result<&'static str, String> {
    values.iter().find(|v| v.key == key).map(|v| v.value).ok_or_else(|| format!("motion condition missing {key}"))
}
// Raw PPC BLT/BGT rejection and BLE acceptance retain unordered behavior.
fn not_less(a: f32, b: f32) -> bool { !matches!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less)) }
fn not_greater(a: f32, b: f32) -> bool { !matches!(a.partial_cmp(&b), Some(std::cmp::Ordering::Greater)) }
/// Same generated string-map to existing binary constructor adapter. Convert
/// only numeric keys actually consumed by these two shared condition kernels.
fn binary_attributes(values: &[Attribute]) -> Result<Vec<StateGraphBinaryAttribute>, String> {
    values.iter().map(|v| {
        let number = match v.key {
            "InTime" | "value" | "equal" | "notEqual" | "greater" | "less" | "greaterEqual" | "lessEqual" =>
                v.value.parse::<f64>().map_err(|e| e.to_string())? as f32,
            _ => 0.,
        };
        let boolean = match v.key {
            "waitForTransitions" | "abs" => match v.value {
                "true" => 1, "false" => 0, _ => return Err(format!("motion condition boolean {}", v.key)),
            },
            _ => 0,
        };
        Ok(StateGraphBinaryAttribute { key: v.key.as_bytes().to_vec(), key_hash: state_graph_string_hash32(v.key.as_bytes()),
            text: v.value.as_bytes().to_vec(), float_bits: number.to_bits(), boolean_raw: boolean })
    }).collect()
}

impl Conditions {
    /// HCDelegate::Init82922408 binds shared condition objects in numeric order.
    /// Retain this once with the loaded graph's objects, not once per tick.
    pub(crate) fn bind(declarations: &MotionDeclarations) -> Result<Self, String> {
        let mut entries = Vec::with_capacity(declarations.conditions.len());
        for (index, declaration) in declarations.conditions.iter().enumerate() {
            let values = attributes(declarations, index as u16)?;
            entries.push(match declaration.name {
                "HasIntent" => Condition::HasIntent(HasIntentCondition::from_attributes(&binary_attributes(values)?).map_err(|e| e.to_string())?),
                "WillExpire" => Condition::WillExpire(WillExpireCondition::from_attributes(&binary_attributes(values)?).map_err(|e| e.to_string())?),
                "LastState" => Condition::LastState(required(values, "state")?), //82caa638
                "HasPlugin" => Condition::HasPlugin,
                "BranchWindowEnabled" => Condition::BranchWindowEnabled,
                "InTurnBranchWindow" => Condition::InTurnBranchWindow,
                "InBranchWindow" => Condition::InBranchWindow(required(values, "BranchAnimInfo")?), //826f3470
                "PedestrianJumperIsLanding" => Condition::JumperLanding,
                "PedestrianJumperInDownwardMotion" => Condition::JumperDownward,
                name => return Err(format!("unbound pedestrian motion condition {name}")),
            });
        }
        Ok(Self { entries, states: declarations.states })
    }
    /// Called on a per-frame cache MISS only. `parent` is the current expression
    /// evaluation state, matching StateConditionDirect's condition+16 assignment.
    pub(crate) fn evaluate<B: Services>(&self, index: u16, parent: u16,
        controller: &ControllerState<'_>, backend: &mut B) -> Result<bool, String>
    where <B as HasIntentCollaborator>::Error: std::fmt::Debug,
          <B as WillExpireCollaborator>::Error: std::fmt::Debug,
    {
        Ok(match self.entries.get(usize::from(index)).ok_or("motion condition index")? {
            Condition::HasIntent(value) => value.activate(backend).map_err(|e| format!("motion HasIntent: {e:?}"))?,
            Condition::WillExpire(value) => value.activate(backend).map_err(|e| format!("motion WillExpire: {e:?}"))?,
            // Existing generic helper operates on the supplied declarations; it
            // does not read the onboard graph or skater state.
            Condition::LastState(name) => crate::generated_motion_load::runtime::grounded_selection::state_matches(
                self.states, parent, controller.last, name).map_err(|e| format!("motion LastState: {e:?}"))?,
            Condition::HasPlugin => backend.has_plugin()?, //82701740
            Condition::BranchWindowEnabled => { //826fd618: count !=0, not count>0
                backend.has_animatable() && backend.branch_attributes()?.is_some_and(|a| a.count != 0)
            }
            Condition::InTurnBranchWindow => { //826fd5b8 -> Pedestrian82e6e750
                match backend.branch_attributes()? {
                    None => false,
                    Some(attributes) => {
                        if backend.animation_expiration()?.0 { true }
                        else if attributes.count <= 0 { false }
                        else {
                            let time=backend.animation_time()?;
                            let windows = attributes.windows.get(..attributes.count as usize).ok_or("motion branch-window count")?;
                            // GetCurrentBranchWindow82e6e6f4/fc rejects with BLT,
                            // but the final82e6e708 accepts with BLE. Unordered
                            // therefore survives only the first two comparisons.
                            windows.iter().any(|w| not_less(w.start, 0.) && not_less(time, w.start) && time<=w.end)
                        }
                    }
                }
            }
            Condition::InBranchWindow(name) => { //827127b0; preserve inclusive bounds
                if !backend.has_animatable() { false }
                else if let Some(info) = backend.branch_info(name)? {
                    if backend.animation_transitioning()? { false }
                    else {
                        let time=backend.animation_time()?;
                        match info.kind {
                            0 => {
                                let (expired,time_to_expire)=backend.animation_expiration()?;
                                let window=backend.branch_window_duration()?;
                                //827128e0/ec are BLE acceptance, unlike the
                                // BranchPoint's BLT/BGT rejection below.
                                time<=window || time_to_expire<=window || expired
                            },
                            1 => not_less(time,info.time) &&
                                not_greater(time,backend.branch_window_duration()?+info.time),
                            _ => false,
                        }
                    }
                } else { false }
            }
            Condition::JumperLanding => backend.is_jumping()?.is_some_and(|jumping| !jumping), //826f3570
            Condition::JumperDownward => backend.is_jumping_downward()?.unwrap_or(false), //826f35c0
        })
    }
}
