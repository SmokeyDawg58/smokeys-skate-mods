//! Actual final board output getters; no geometry-derived wheel-base substitute.
use super::SkateboardBodyConstruction;
use crate::{
    physical_player_phys_out::construct::board_fill::board_speed_from_squared_and_estimate,
    skateboard_attributes::{
        SkateboardAttributeClass, SkateboardAttributeError, SkateboardAttributeRead,
    },
    skateboard_contact_surface::SkateboardContactSurface,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
};
#[derive(Debug, PartialEq)]
pub(crate) enum BoardSlideSpeedError<N> {
    InvalidPart(usize),
    Unknown(&'static str),
    Numeric(N),
}
impl<N: std::fmt::Debug> std::fmt::Display for BoardSlideSpeedError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPart(p) => write!(f, "slide-speed Part{p} outside native seven parts"),
            Self::Unknown(field) => write!(f, "slide speed requires {field}"),
            Self::Numeric(e) => write!(f, "slide-speed numeric: {e:?}"),
        }
    }
}
impl SkateboardContactSurface {
    pub(crate) fn try_part_slide_speed<N: SkeletonCollisionNumeric>(
        &self,
        part: usize,
        numeric: &mut N,
    ) -> Result<f32, BoardSlideSpeedError<N::Error>> {
        use BoardSlideSpeedError as E;
        if part >= 7 {
            return Err(E::InvalidPart(part));
        }
        if !self.collision_state.in_contact[part].ok_or(E::Unknown("selected inContact"))? {
            return Ok(0.0);
        }
        let relative = self.collision_state.relative_velocity[part]
            .ok_or(E::Unknown("selected relative velocity"))?;
        let normal = self.collision_state.normal[part].ok_or(E::Unknown("selected normal"))?;
        let dot = numeric.dot3(normal.xyz, relative.xyz).map_err(E::Numeric)?;
        let tangent = std::array::from_fn(|lane| {
            let component = normal.xyz[lane] * dot;
            relative.xyz[lane] - component
        });
        let squared = numeric.dot3(tangent, tangent).map_err(E::Numeric)?;
        let estimate = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        Ok(board_speed_from_squared_and_estimate(squared, estimate))
    }
}
pub(crate) fn try_deck_to_wheel_base<A: SkateboardAttributeRead>(
    attributes: &mut A,
) -> Result<f32, A::Error> {
    let radius = attributes.lookup(SkateboardAttributeClass::Wheels, 0x1264_6f5e_842c_4db3)?;
    let vertical = attributes.lookup(SkateboardAttributeClass::Trucks, 0x2be0_8dd7_a920_7622)?;
    // The first pointer is retained across the second lookup, then both floats
    // load in native order. Missing pointers use the existing DefaultDataArea.
    let radius = attributes.float(&radius)?;
    let vertical = attributes.float(&vertical)?;
    Ok(radius - vertical)
}
impl SkateboardBodyConstruction {
    pub(crate) fn try_output_deck_to_wheel_base(
        &mut self,
    ) -> Result<f32, SkateboardAttributeError> {
        try_deck_to_wheel_base(&mut self.attributes)
    }
}
#[cfg(test)]
#[path = "skateboard_body_output_getters_tests.rs"]
mod tests;
