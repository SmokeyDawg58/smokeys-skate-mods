//! `FilterMotionGraphIntent82cbd460` constructor, including actual Curve inserts.
use super::{FastString36, MotionLoadError, float, get, packed};
use crate::generated_action_load::{Attribute, construct::Filter};

#[derive(Debug)]
pub(crate) struct OptionalScalar {
    pub(crate) present: bool,
    pub(crate) value: f32,
}
#[derive(Debug)]
pub(crate) struct BlendCurve {
    pub(crate) scale: f32,
    pub(crate) count: usize,
    pub(crate) coordinates: [Option<[f32; 2]>; 10],
}
impl BlendCurve {
    /// Source `Curve::Insert82c8d630` appends; it does not sort or merge x ties.
    fn insert(&mut self, x: f32, y: f32) -> Result<(), MotionLoadError> {
        if self.count > 0 && x <= self.coordinates[self.count - 1].expect("active coordinate")[0] {
            return Err(MotionLoadError::InvalidAttribute {
                key: "curve x",
                value: x.to_string(),
            });
        }
        let destination = self
            .coordinates
            .get_mut(self.count)
            .ok_or(MotionLoadError::DescriptorBounds)?;
        *destination = Some([x, y]);
        self.count += 1;
        Ok(())
    }
}
#[derive(Debug)]
pub(crate) struct FilteredIntent {
    pub(crate) intent: FastString36,
    pub(crate) default_value: f32,
    pub(crate) starting_value: f32,
    pub(crate) output_intent: FastString36,
    pub(crate) curve: BlendCurve,
    pub(crate) ramp: OptionalScalar,
    pub(crate) blend_rising: f32,
    pub(crate) blend_falling: f32,
    pub(crate) blend_out: OptionalScalar,
    pub(crate) clamp_velocity: OptionalScalar,
    pub(crate) clamp_acceleration: OptionalScalar,
    pub(crate) scale: f32,
    pub(crate) filter1: Filter,
    pub(crate) filter2: Filter,
    pub(crate) fakie_filter: Filter,
    pub(crate) mirror_filter: Filter,
}
fn optional(
    values: &[Attribute],
    key: &'static str,
    default: f32,
) -> Result<OptionalScalar, MotionLoadError> {
    Ok(OptionalScalar {
        present: get(values, key).is_some(),
        value: float(values, key, default)?,
    })
}
fn required<'a>(values: &'a [Attribute], key: &'static str) -> Result<&'a str, MotionLoadError> {
    get(values, key).ok_or(MotionLoadError::InvalidAttribute {
        key,
        value: String::new(),
    })
}
/// `ParseFilter82cb9c98` compares packed, case-folded names; unknown is NONE.
fn parse(text: &str) -> Result<Filter, MotionLoadError> {
    crate::generated_action_load::construct::parse_filter(text).map_err(MotionLoadError::Shared)
}
pub(crate) fn construct(values: &[Attribute]) -> Result<FilteredIntent, MotionLoadError> {
    let intent = packed("intent", required(values, "intent")?)?;
    let default_value = float(values, "defaultValue", 0.0)?;
    let starting_value = float(values, "startingValue", 0.0)?;
    let output_intent = packed("filteredIntent", required(values, "filteredIntent")?)?;
    let mut curve = BlendCurve {
        scale: 1.0,
        count: 0,
        coordinates: [None; 10],
    };
    let ramp = optional(values, "rampTime", 1.0)?;
    let blend_rising = float(values, "blendRising", float(values, "blend", 1.0)?)?;
    let blend_falling = float(values, "blendFalling", float(values, "blend", 1.0)?)?;
    let blend_out = optional(values, "blendOut", 1.0)?;
    let clamp_velocity = optional(values, "clampVel", 0.0)?;
    let clamp_acceleration = optional(values, "clampAcc", 0.0)?;
    if get(values, "blend").is_none()
        && (get(values, "blendRising").is_none() || get(values, "blendFalling").is_none())
    {
        return Err(MotionLoadError::InvalidAttribute {
            key: "blend",
            value: String::new(),
        });
    }
    curve.insert(0.0, 0.0)?;
    curve.insert(ramp.value, 1.0)?;
    let scale = float(values, "scale", 1.0)?;
    let mut filter1 = parse(get(values, "filter").unwrap_or("none"))?;
    if let Some(text) = get(values, "filter1") {
        filter1 = parse(text)?;
    }
    let filter2 = parse(get(values, "filter2").unwrap_or("none"))?;
    let fakie_filter = parse(get(values, "fakieFilter").unwrap_or("none"))?;
    let mirror_filter = parse(get(values, "mirrorFilter").unwrap_or("none"))?;
    Ok(FilteredIntent {
        intent,
        default_value,
        starting_value,
        output_intent,
        curve,
        ramp,
        blend_rising,
        blend_falling,
        blend_out,
        clamp_velocity,
        clamp_acceleration,
        scale,
        filter1,
        filter2,
        fakie_filter,
        mirror_filter,
    })
}
